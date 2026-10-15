use std::collections::{HashMap, HashSet};

use crate::share_classes;
use crate::types::{LoadedXbrlArtifact, PreferredShareTermFact, ShareClassFact, TextFact};

const TERMS_TEXT_LIMIT: usize = 20_000;

#[derive(Debug, Clone)]
struct PreferredClass {
    class_key: String,
    class_name: String,
    shares: Option<f64>,
}

pub fn parse_preferred_share_terms_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Vec<PreferredShareTermFact> {
    let preferred_classes = preferred_classes_from_share_rows(
        &share_classes::parse_share_classes_from_artifact(artifact),
    );
    let mut rows = Vec::new();

    for text_fact in &artifact.text_facts {
        let source_kind = source_kind_for_concept(&text_fact.source_concept);
        if !should_parse_preferred_share_terms(&source_kind) {
            continue;
        }
        let mut classes = matching_preferred_classes(&preferred_classes, &text_fact.text);
        if classes.is_empty() {
            classes = inferred_preferred_classes(&text_fact.text);
        }

        for class_info in classes {
            rows.push(build_term_fact(
                text_fact,
                &class_info,
                &preferred_classes,
                &source_kind,
            ));
        }
    }

    dedupe_and_sort(rows)
}

fn should_parse_preferred_share_terms(source_kind: &str) -> bool {
    matches!(
        source_kind,
        "issued_share_change_text" | "classes_of_shares_text" | "preferred_share_text"
    )
}

fn preferred_classes_from_share_rows(rows: &[ShareClassFact]) -> Vec<PreferredClass> {
    let mut by_key: HashMap<(String, String), PreferredClass> = HashMap::new();
    for row in rows {
        if !row.is_preferred {
            continue;
        }
        let key = (row.class_key.clone(), row.class_name.clone());
        let class_info = PreferredClass {
            class_key: row.class_key.clone(),
            class_name: row.class_name.clone(),
            shares: (row.shares > 0.0).then_some(row.shares),
        };
        let replace = by_key
            .get(&key)
            .is_none_or(|existing| existing.shares.is_none() && class_info.shares.is_some());
        if replace {
            by_key.insert(key, class_info);
        }
    }

    let mut classes: Vec<PreferredClass> = by_key.into_values().collect();
    classes.sort_by(|a, b| a.class_name.cmp(&b.class_name));
    classes
}

fn matching_preferred_classes(classes: &[PreferredClass], text: &str) -> Vec<PreferredClass> {
    let normalized_text = normalize_for_match(text);
    classes
        .iter()
        .filter(|class_info| contains_class_name(&normalized_text, &class_info.class_name))
        .cloned()
        .collect()
}

fn inferred_preferred_classes(text: &str) -> Vec<PreferredClass> {
    let normalized = normalize_for_match(text);
    let prefixes = [
        "A種",
        "B種",
        "C種",
        "D種",
        "E種",
        "第一種",
        "第二種",
        "第三種",
        "第1種",
        "第2種",
        "第3種",
    ];
    let mut classes = Vec::new();
    for prefix in prefixes {
        for suffix in ["優先株式", "種類株式"] {
            let class_name = format!("{prefix}{suffix}");
            if !normalized.contains(&class_name) {
                continue;
            }
            classes.push(PreferredClass {
                class_key: class_name.clone(),
                class_name,
                shares: None,
            });
        }
    }
    if classes.is_empty() && text.contains("優先株式") {
        classes.push(PreferredClass {
            class_key: "preferred_shares".to_string(),
            class_name: "優先株式".to_string(),
            shares: None,
        });
    }
    classes
}

fn build_term_fact(
    text_fact: &TextFact,
    class_info: &PreferredClass,
    all_classes: &[PreferredClass],
    source_kind: &str,
) -> PreferredShareTermFact {
    let preferred_shares =
        extract_preferred_shares(&text_fact.text, &class_info.class_name).or(class_info.shares);
    let conversion_ratio_to_common =
        extract_conversion_ratio_to_common(&text_fact.text, &class_info.class_name);
    let converted_common_shares = conversion_ratio_to_common
        .and_then(|ratio| preferred_shares.map(|s| s * ratio))
        .or_else(|| extract_converted_common_shares(&text_fact.text, &class_info.class_name))
        .or_else(|| {
            infer_one_for_one_converted_shares(
                &text_fact.text,
                class_info,
                all_classes,
                preferred_shares,
            )
        });

    PreferredShareTermFact {
        period: text_fact.period.clone(),
        class_key: class_info.class_key.clone(),
        class_name: class_info.class_name.clone(),
        terms_text: truncate_chars(&text_fact.text, TERMS_TEXT_LIMIT),
        preferred_shares,
        converted_common_shares,
        conversion_ratio_to_common,
        issue_price_yen: extract_labeled_yen(
            &text_fact.text,
            &["発行価格", "払込金額", "払込価格"],
        ),
        capitalized_amount_yen: extract_labeled_yen(&text_fact.text, &["資本組入額"]),
        source_concept: text_fact.source_concept.clone(),
        source_kind: source_kind.to_string(),
    }
}

fn contains_class_name(normalized_text: &str, class_name: &str) -> bool {
    normalized_text.contains(&normalize_for_match(class_name))
}

fn extract_preferred_shares(text: &str, class_name: &str) -> Option<f64> {
    for alias in class_aliases(class_name) {
        for segment in segments_after_alias(text, &alias, 60) {
            if let Some(value) = parse_preferred_share_count(&segment) {
                return Some(value);
            }
        }
    }
    None
}

fn parse_preferred_share_count(segment: &str) -> Option<f64> {
    let unit_pos = segment.find("株")?;
    let before = &segment[..unit_pos];
    let value = parse_number_at_end(before)?;
    let after = &segment[unit_pos + "株".len()..];
    if value == 1.0 && looks_like_conversion_ratio_suffix(after) {
        return None;
    }
    Some(value)
}

fn looks_like_conversion_ratio_suffix(after_share_unit: &str) -> bool {
    let suffix = take_chars_from_byte(after_share_unit, 0, 12);
    suffix.starts_with("につき")
        || suffix.starts_with("に対し")
        || suffix.starts_with("あたり")
        || suffix.starts_with("当たり")
        || (suffix.starts_with('を') && suffix.contains("普通株式"))
}

fn extract_conversion_ratio_to_common(text: &str, class_name: &str) -> Option<f64> {
    let normalized = normalize_for_match(text);
    for alias in class_aliases(class_name) {
        let normalized_alias = normalize_for_match(&alias);
        let mut search_start = 0usize;
        while let Some(pos) = normalized[search_start..].find(&normalized_alias) {
            let start = search_start + pos + normalized_alias.len();
            let segment = take_chars_from_byte(&normalized, start, 140);
            for marker in ["1株につき普通株式", "1株を普通株式", "1株につき"] {
                if let Some(marker_pos) = segment.find(marker) {
                    let after = &segment[marker_pos + marker.len()..];
                    if !ratio_context_allows_conversion(segment, marker_pos, after) {
                        continue;
                    }
                    if let Some(value) = parse_number_before_unit(after, "株") {
                        return Some(value);
                    }
                }
            }
            search_start = start;
        }
    }
    None
}

fn ratio_context_allows_conversion(segment: &str, marker_pos: usize, after_marker: &str) -> bool {
    let before_near = last_chars(&segment[..marker_pos], 40);
    let after_near = take_chars_from_byte(after_marker, 0, 40);
    let context = format!("{before_near}{after_near}");
    if context.contains("株式分割") || context.contains("分割") {
        return false;
    }
    context.contains("転換")
        || context.contains("変更")
        || context.contains("取得請求")
        || context.contains("交付")
        || context.contains("対価")
}

fn extract_converted_common_shares(text: &str, class_name: &str) -> Option<f64> {
    let normalized = normalize_for_match(text);
    for alias in class_aliases(class_name) {
        let normalized_alias = normalize_for_match(&alias);
        let mut search_start = 0usize;
        while let Some(pos) = normalized[search_start..].find(&normalized_alias) {
            let start = search_start + pos;
            let segment = take_chars_from_byte(&normalized, start, 180);
            let Some(common_pos) = segment.find("普通株式") else {
                search_start = start + normalized_alias.len();
                continue;
            };
            let before_common = &segment[..common_pos];
            let has_conversion_word =
                segment.contains("転換") || segment.contains("変更") || segment.contains("交付");
            let is_ratio_phrase = before_common.contains("1株につき");
            let common_abs_pos = start + common_pos;
            let prefix_to_common = &normalized[..common_abs_pos];
            let has_multiple_preferred_classes = prefix_to_common.matches("優先株式").count() > 1;
            if has_conversion_word
                && !is_ratio_phrase
                && !has_multiple_preferred_classes
                && before_common.matches("優先株式").count() <= 1
            {
                let after_common = &segment[common_pos + "普通株式".len()..];
                if let Some(value) = parse_number_before_unit(after_common, "株") {
                    return Some(value);
                }
            }
            search_start = start + normalized_alias.len();
        }
    }
    None
}

fn infer_one_for_one_converted_shares(
    text: &str,
    class_info: &PreferredClass,
    all_classes: &[PreferredClass],
    preferred_shares: Option<f64>,
) -> Option<f64> {
    let preferred_shares = preferred_shares?;
    let normalized = normalize_for_match(text);
    if !(normalized.contains("普通株式")
        && (normalized.contains("転換") || normalized.contains("変更")))
    {
        return None;
    }

    let mut sum = 0.0;
    let mut found = false;
    for other in all_classes {
        if !contains_class_name(&normalized, &other.class_name) {
            continue;
        }
        if let Some(shares) = extract_preferred_shares(text, &other.class_name).or(other.shares) {
            sum += shares;
            found = true;
        }
    }
    if !found || sum <= 0.0 {
        return None;
    }

    let converted_total = extract_first_common_shares_after_preferred(text)?;
    if (converted_total - sum).abs() <= 1.0
        && contains_class_name(&normalized, &class_info.class_name)
    {
        Some(preferred_shares)
    } else {
        None
    }
}

fn extract_first_common_shares_after_preferred(text: &str) -> Option<f64> {
    let normalized = normalize_for_match(text);
    let preferred_pos = normalized.find("優先株式")?;
    let common_pos = normalized[preferred_pos..].find("普通株式")? + preferred_pos;
    let after_common = &normalized[common_pos + "普通株式".len()..];
    parse_number_before_unit(after_common, "株")
}

fn extract_labeled_yen(text: &str, labels: &[&str]) -> Option<f64> {
    let normalized = normalize_for_match(text);
    for label in labels {
        let mut search_start = 0usize;
        while let Some(pos) = normalized[search_start..].find(label) {
            let start = search_start + pos + label.len();
            let segment = take_chars_from_byte(&normalized, start, 80);
            if let Some(value) = parse_number_before_unit(segment, "円") {
                return Some(value);
            }
            search_start = start;
        }
    }
    None
}

fn segments_after_alias(text: &str, alias: &str, max_chars: usize) -> Vec<String> {
    let normalized_text = normalize_for_match(text);
    let normalized_alias = normalize_for_match(alias);
    let mut segments = Vec::new();
    let mut search_start = 0usize;
    while let Some(pos) = normalized_text[search_start..].find(&normalized_alias) {
        let start = search_start + pos + normalized_alias.len();
        let segment = take_chars_from_byte(&normalized_text, start, max_chars).to_string();
        segments.push(segment);
        search_start = start;
    }
    segments
}

fn parse_number_before_unit(segment: &str, unit: &str) -> Option<f64> {
    let unit_pos = segment.find(unit)?;
    let before = &segment[..unit_pos];
    parse_number_at_end(before)
}

fn parse_number_at_end(raw: &str) -> Option<f64> {
    let mut number_reversed = Vec::new();
    for ch in raw.chars().rev() {
        if is_number_char(ch) || ch == '-' || ch == '△' {
            number_reversed.push(ch);
        } else if number_reversed.is_empty() && is_trailing_number_separator(ch) {
            continue;
        } else if number_reversed.is_empty() {
            return None;
        } else if !number_reversed.is_empty() {
            break;
        }
    }
    if number_reversed.is_empty() {
        return None;
    }
    let raw: String = number_reversed.into_iter().rev().collect();
    parse_number(&raw)
}

fn is_trailing_number_separator(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '　' | ')' | '）' | ']' | '】' | '}' | '、' | '，' | ',' | ':' | '：'
        )
}

fn take_chars_from_byte(raw: &str, start: usize, max_chars: usize) -> &str {
    if start >= raw.len() {
        return "";
    }
    let suffix = &raw[start..];
    let end = suffix
        .char_indices()
        .nth(max_chars)
        .map(|(idx, _)| start + idx)
        .unwrap_or(raw.len());
    &raw[start..end]
}

fn parse_number(raw: &str) -> Option<f64> {
    let normalized = normalize_number(raw);
    if normalized.is_empty() {
        return None;
    }
    normalized.parse::<f64>().ok()
}

fn normalize_number(raw: &str) -> String {
    raw.chars()
        .filter_map(|ch| match ch {
            '０'..='９' => Some(((ch as u32 - '０' as u32) as u8 + b'0') as char),
            '0'..='9' => Some(ch),
            '．' | '.' => Some('.'),
            '－' | '-' => Some('-'),
            '△' => Some('-'),
            ',' | '，' => None,
            _ => None,
        })
        .collect()
}

fn is_number_char(ch: char) -> bool {
    matches!(
        ch,
        '0'..='9' | '０'..='９' | ',' | '，' | '.' | '．'
    )
}

fn class_aliases(class_name: &str) -> Vec<String> {
    let mut aliases = vec![class_name.to_string()];
    let ascii = to_ascii_alnum(class_name);
    if ascii != class_name {
        aliases.push(ascii);
    }
    aliases.sort();
    aliases.dedup();
    aliases
}

fn normalize_for_match(raw: &str) -> String {
    to_ascii_alnum(raw)
        .chars()
        .filter_map(|ch| {
            if ch.is_whitespace() || ch == '　' {
                None
            } else if ch == '，' {
                Some(',')
            } else if ch == '：' {
                Some(':')
            } else {
                Some(ch)
            }
        })
        .collect()
}

fn to_ascii_alnum(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'Ａ'..='Ｚ' => ((ch as u32 - 'Ａ' as u32) as u8 + b'A') as char,
            'ａ'..='ｚ' => ((ch as u32 - 'ａ' as u32) as u8 + b'a') as char,
            '０'..='９' => ((ch as u32 - '０' as u32) as u8 + b'0') as char,
            _ => ch,
        })
        .collect()
}

fn source_kind_for_concept(source_concept: &str) -> String {
    let local = source_concept
        .rsplit_once('#')
        .map(|(_, local)| local)
        .unwrap_or(source_concept);
    if local.contains("ChangesInNumberOfIssuedSharesStatedCapitalEtcTextBlock") {
        "issued_share_change_text".to_string()
    } else if local.contains("ClassesOfSharesEtcTextBlock") {
        "classes_of_shares_text".to_string()
    } else if local.contains("IssuedShares")
        || local.contains("SharesAndTreasuryShares")
        || local.contains("VotingRights")
    {
        "issued_shares_text".to_string()
    } else if local.contains("PerShareInformation") {
        "per_share_information_text".to_string()
    } else if local.contains("BusinessResults") {
        "business_results_text".to_string()
    } else {
        "preferred_share_text".to_string()
    }
}

fn last_chars(raw: &str, max_chars: usize) -> String {
    let mut chars = raw.chars().rev().take(max_chars).collect::<Vec<_>>();
    chars.reverse();
    chars.into_iter().collect()
}

fn truncate_chars(raw: &str, max_chars: usize) -> String {
    let mut output = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx == max_chars {
            output.push_str("...");
            return output;
        }
        output.push(ch);
    }
    output
}

fn dedupe_and_sort(rows: Vec<PreferredShareTermFact>) -> Vec<PreferredShareTermFact> {
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let key = (
            row.period.clone(),
            row.class_key.clone(),
            row.source_concept.clone(),
            row.terms_text.clone(),
        );
        if seen.insert(key) {
            result.push(row);
        }
    }
    result.sort_by(|a, b| {
        b.period
            .cmp(&a.period)
            .then_with(|| a.class_name.cmp(&b.class_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_conversion_ratio_and_prices() {
        let text = "Ａ種優先株式100株は、Ａ種優先株式1株につき普通株式2株に転換されます。発行価格：2,500円 資本組入額：1,250円";

        assert_eq!(extract_preferred_shares(text, "Ａ種優先株式"), Some(100.0));
        assert_eq!(
            extract_conversion_ratio_to_common(text, "Ａ種優先株式"),
            Some(2.0)
        );
        assert_eq!(extract_converted_common_shares(text, "Ａ種優先株式"), None);
        assert_eq!(extract_labeled_yen(text, &["発行価格"]), Some(2500.0));
        assert_eq!(extract_labeled_yen(text, &["資本組入額"]), Some(1250.0));
    }

    #[test]
    fn does_not_treat_one_share_ratio_as_preferred_share_count() {
        let text = "Ａ種優先株式は、Ａ種優先株式1株につき普通株式2株に転換されます。";

        assert_eq!(extract_preferred_shares(text, "Ａ種優先株式"), None);
    }

    #[test]
    fn infers_one_for_one_when_total_common_equals_total_preferred() {
        let text = "Ａ種優先株式590,657株、Ｂ種優先株式500,000株及びＣ種優先株式560,000株は普通株式1,650,657株に変更しております。";
        let classes = vec![
            PreferredClass {
                class_key: "A".to_string(),
                class_name: "Ａ種優先株式".to_string(),
                shares: None,
            },
            PreferredClass {
                class_key: "B".to_string(),
                class_name: "Ｂ種優先株式".to_string(),
                shares: None,
            },
            PreferredClass {
                class_key: "C".to_string(),
                class_name: "Ｃ種優先株式".to_string(),
                shares: None,
            },
        ];

        assert_eq!(
            infer_one_for_one_converted_shares(text, &classes[0], &classes, Some(590657.0)),
            Some(590657.0)
        );
    }

    #[test]
    fn rejects_stock_split_ratio_after_preferred_share_conversion() {
        let text = "Ｄ種優先株式54,887株を自己株式として取得し、対価として普通株式54,887株を交付した。普通株式1株につき200株の株式分割を行った。";

        assert_eq!(
            extract_conversion_ratio_to_common(text, "Ｄ種優先株式"),
            None
        );
        assert_eq!(
            extract_converted_common_shares(text, "Ｄ種優先株式"),
            Some(54_887.0)
        );
    }

    #[test]
    fn classifies_issued_share_notes_as_issued_shares_text() {
        assert_eq!(
            source_kind_for_concept(
                "jpcrp_cor:NotesRegardingIssuedSharesAndTreasurySharesTextBlock"
            ),
            "issued_shares_text"
        );
        assert!(!should_parse_preferred_share_terms("issued_shares_text"));
    }

    #[test]
    fn does_not_parse_next_class_name_as_share_unit() {
        let segment = "900900900Ｂ種優先株式";

        assert_eq!(parse_preferred_share_count(segment), None);
    }
}
