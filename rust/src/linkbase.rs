use std::collections::HashMap;
use std::path::{Path, PathBuf};

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::types::ConceptKey;
use crate::xml_util;

pub type LookupByPath = HashMap<(PathBuf, String), ConceptKey>;
pub type LookupByBasename = HashMap<(String, String), ConceptKey>;
pub type PresentationGraphs = HashMap<String, HashMap<ConceptKey, Vec<PresentationArc>>>;

#[derive(Debug, Clone)]
pub struct PresentationArc {
    pub child: ConceptKey,
    pub order: f64,
}

pub fn build_concept_lookup(artifact_root: &Path) -> (LookupByPath, LookupByBasename) {
    let mut by_path: LookupByPath = HashMap::new();
    let mut by_basename: LookupByBasename = HashMap::new();

    let mut xsd_paths: Vec<PathBuf> = Vec::new();
    collect_files_recursive(artifact_root, "xsd", &mut xsd_paths);
    xsd_paths.sort();

    for xsd_path in &xsd_paths {
        let content = match std::fs::read_to_string(xsd_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let target_ns = extract_target_namespace(&content);
        if target_ns.is_empty() {
            continue;
        }
        extract_xsd_elements(
            &content,
            xsd_path,
            &target_ns,
            &mut by_path,
            &mut by_basename,
        );
    }

    (by_path, by_basename)
}

pub fn build_presentation_graphs(
    artifact_root: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    prefix_namespaces: &HashMap<String, String>,
) -> PresentationGraphs {
    let mut graphs: PresentationGraphs = HashMap::new();
    let mut pre_paths: Vec<PathBuf> = Vec::new();
    collect_files_recursive(artifact_root, "xml", &mut pre_paths);
    pre_paths.retain(|p| {
        p.file_name()
            .is_some_and(|n| n.to_string_lossy().ends_with("_pre.xml"))
    });
    pre_paths.sort();

    for pre_path in &pre_paths {
        let content = match std::fs::read_to_string(pre_path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        parse_presentation_linkbase(
            &content,
            pre_path,
            by_path,
            by_basename,
            prefix_namespaces,
            &mut graphs,
        );
    }
    graphs
}

fn collect_files_recursive(root: &Path, ext: &str, result: &mut Vec<PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, ext, result);
            } else if path.extension().is_some_and(|e| e == ext) {
                result.push(path);
            }
        }
    }
}

fn extract_target_namespace(content: &str) -> String {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local == "schema" {
                    for attr in e.attributes().flatten() {
                        let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        if xml_util::local_name(&k) == "targetNamespace" {
                            return String::from_utf8_lossy(&attr.value).to_string();
                        }
                    }
                }
                break;
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
    String::new()
}

fn extract_xsd_elements(
    content: &str,
    xsd_path: &Path,
    target_ns: &str,
    by_path: &mut LookupByPath,
    by_basename: &mut LookupByBasename,
) {
    let mut reader = Reader::from_str(content);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local != "element" {
                    continue;
                }
                let mut name = String::new();
                let mut element_id = String::new();
                for attr in e.attributes().flatten() {
                    let k = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let v = String::from_utf8_lossy(&attr.value).to_string();
                    match xml_util::local_name(&k).as_str() {
                        "name" => name = v,
                        "id" => element_id = v,
                        _ => {}
                    }
                }
                if name.is_empty() {
                    continue;
                }
                let concept = (target_ns.to_string(), name);
                if !element_id.is_empty() {
                    let key = (
                        xsd_path
                            .to_path_buf()
                            .canonicalize()
                            .unwrap_or_else(|_| xsd_path.to_path_buf()),
                        element_id.clone(),
                    );
                    by_path.insert(key, concept.clone());
                    by_basename.insert(
                        (
                            xsd_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string(),
                            element_id,
                        ),
                        concept,
                    );
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

fn parse_presentation_linkbase(
    content: &str,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    prefix_namespaces: &HashMap<String, String>,
    graphs: &mut PresentationGraphs,
) {
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current_role = String::new();
    let mut locators: HashMap<String, ConceptKey> = HashMap::new();
    let mut in_pres_link = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);
                if local == "presentationLink" {
                    in_pres_link = true;
                    locators.clear();
                    current_role = attr_value(&e, "role").unwrap_or_default();
                } else if in_pres_link && local == "loc" {
                    store_locator(
                        &e,
                        base_path,
                        by_path,
                        by_basename,
                        prefix_namespaces,
                        &mut locators,
                    );
                } else if in_pres_link && local == "presentationArc" {
                    store_presentation_arc(&e, &current_role, &locators, graphs);
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let local = xml_util::local_name(&tag);
                if in_pres_link && local == "loc" {
                    store_locator(
                        &e,
                        base_path,
                        by_path,
                        by_basename,
                        prefix_namespaces,
                        &mut locators,
                    );
                } else if in_pres_link && local == "presentationArc" {
                    store_presentation_arc(&e, &current_role, &locators, graphs);
                }
            }
            Ok(Event::End(e)) => {
                let local = xml_util::local_name(&String::from_utf8_lossy(e.name().as_ref()));
                if local == "presentationLink" {
                    in_pres_link = false;
                }
            }
            Ok(Event::Eof) => break,
            _ => {}
        }
        buf.clear();
    }
}

fn store_locator(
    event: &quick_xml::events::BytesStart<'_>,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    prefix_namespaces: &HashMap<String, String>,
    locators: &mut HashMap<String, ConceptKey>,
) {
    let label = attr_value(event, "label").unwrap_or_default();
    let href = attr_value(event, "href").unwrap_or_default();
    if label.is_empty() || href.is_empty() {
        return;
    }
    if let Some(concept) =
        resolve_locator_concept(&href, base_path, by_path, by_basename, prefix_namespaces)
    {
        locators.insert(label, concept);
    }
}

fn store_presentation_arc(
    event: &quick_xml::events::BytesStart<'_>,
    role: &str,
    locators: &HashMap<String, ConceptKey>,
    graphs: &mut PresentationGraphs,
) {
    let from_label = attr_value(event, "from").unwrap_or_default();
    let to_label = attr_value(event, "to").unwrap_or_default();
    if let (Some(from), Some(to)) = (locators.get(&from_label), locators.get(&to_label)) {
        let order = attr_value(event, "order")
            .and_then(|value| value.parse::<f64>().ok())
            .unwrap_or(0.0);
        let children = graphs
            .entry(role.to_string())
            .or_default()
            .entry(from.clone())
            .or_default();
        if !children.iter().any(|edge| edge.child == *to) {
            children.push(PresentationArc {
                child: to.clone(),
                order,
            });
            children.sort_by(|left, right| {
                left.order
                    .partial_cmp(&right.order)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| left.child.1.cmp(&right.child.1))
            });
        }
    }
}

fn attr_value(event: &quick_xml::events::BytesStart<'_>, target_local: &str) -> Option<String> {
    for attr in event.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        if xml_util::local_name(&key).eq_ignore_ascii_case(target_local) {
            return Some(String::from_utf8_lossy(&attr.value).to_string());
        }
    }
    None
}

fn resolve_locator_concept(
    href: &str,
    base_path: &Path,
    by_path: &LookupByPath,
    by_basename: &LookupByBasename,
    prefix_namespaces: &HashMap<String, String>,
) -> Option<ConceptKey> {
    let (href_path, fragment) = href.rsplit_once('#')?;
    if fragment.is_empty() {
        return None;
    }

    if !href_path.is_empty() {
        if href_path.contains("://") {
            let basename = Path::new(href_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            let decoded = urldecode(&basename);
            if let Some(concept) = by_basename.get(&(decoded, fragment.to_string())) {
                return Some(concept.clone());
            }
            return resolve_fragment_concept(fragment, prefix_namespaces);
        }
        let resolved = base_path
            .parent()
            .unwrap_or(base_path)
            .join(urldecode(href_path));
        let canonical = resolved.canonicalize().unwrap_or(resolved);
        if let Some(concept) = by_path.get(&(canonical.clone(), fragment.to_string())) {
            return Some(concept.clone());
        }
        let basename = canonical
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if let Some(concept) = by_basename.get(&(basename, fragment.to_string())) {
            return Some(concept.clone());
        }
        return resolve_fragment_concept(fragment, prefix_namespaces);
    }

    let canonical = base_path
        .canonicalize()
        .unwrap_or_else(|_| base_path.to_path_buf());
    by_path
        .get(&(canonical, fragment.to_string()))
        .cloned()
        .or_else(|| resolve_fragment_concept(fragment, prefix_namespaces))
}

fn resolve_fragment_concept(
    fragment: &str,
    prefix_namespaces: &HashMap<String, String>,
) -> Option<ConceptKey> {
    let mut prefixes: Vec<&str> = prefix_namespaces
        .keys()
        .map(String::as_str)
        .filter(|prefix| !prefix.is_empty())
        .collect();
    prefixes.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));

    for prefix in prefixes {
        let Some(local_name) = fragment
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix('_'))
        else {
            continue;
        };
        if local_name.is_empty() {
            continue;
        }
        if let Some(namespace) = prefix_namespaces.get(prefix) {
            return Some((namespace.clone(), local_name.to_string()));
        }
    }
    None
}

fn urldecode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let hi = chars.next();
            let lo = chars.next();
            if let (Some(h), Some(l)) = (hi, lo) {
                if let (Some(hv), Some(lv)) = (hex_digit(h), hex_digit(l)) {
                    result.push(char::from(hv << 4 | lv));
                    continue;
                }
            }
            result.push('%');
        } else {
            result.push(char::from(b));
        }
    }
    result
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
