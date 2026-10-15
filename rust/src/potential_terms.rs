use std::collections::HashSet;

use crate::preferred_terms;
use crate::types::{
    DilutedEpsAdjustmentFact, LoadedXbrlArtifact, PotentialEquityInstrumentFact,
    PotentialEquityStatusFact, TextFact, TextTable,
};

const TERMS_TEXT_LIMIT: usize = 20_000;

pub fn parse_potential_equity_instruments_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Vec<PotentialEquityInstrumentFact> {
    let mut rows = preferred_terms::parse_preferred_share_terms_from_artifact(artifact)
        .into_iter()
        .filter(|row| allows_preferred_share_potential_equity(&row.source_kind))
        .filter(|row| row.preferred_shares.is_some_and(|shares| shares > 0.0))
        .map(|row| {
            let period_end_common_shares = row.converted_common_shares.or_else(|| {
                match (row.preferred_shares, row.conversion_ratio_to_common) {
                    (Some(preferred), Some(ratio)) => Some(preferred * ratio),
                    _ => None,
                }
            });
            PotentialEquityInstrumentFact {
                period: row.period,
                instrument_key: row.class_key,
                instrument_type: "preferred_share".to_string(),
                instrument_name: row.class_name,
                terms_text: row.terms_text,
                period_end_common_shares,
                exercise_price_yen: None,
                conversion_price_yen: None,
                face_value_yen: None,
                source_concept: row.source_concept,
                source_kind: row.source_kind,
            }
        })
        .collect::<Vec<_>>();

    for text_fact in &artifact.text_facts {
        if let Some(row) = build_term_fact(text_fact) {
            rows.push(row);
        }
    }

    dedupe_and_sort(rows)
}

pub fn parse_diluted_eps_adjustments_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Vec<DilutedEpsAdjustmentFact> {
    let rows = artifact
        .text_facts
        .iter()
        .filter_map(build_diluted_eps_adjustment_fact)
        .collect::<Vec<_>>();
    dedupe_and_sort_adjustments(rows)
}

pub fn parse_potential_equity_statuses_from_artifact(
    artifact: &LoadedXbrlArtifact,
) -> Vec<PotentialEquityStatusFact> {
    let rows = artifact
        .text_facts
        .iter()
        .filter_map(build_status_fact)
        .collect::<Vec<_>>();
    dedupe_and_sort_statuses(rows)
}

fn build_term_fact(text_fact: &TextFact) -> Option<PotentialEquityInstrumentFact> {
    if is_absence_only(&text_fact.text) {
        return None;
    }

    let instrument_type = classify_instrument(&text_fact.text)?;
    if instrument_type == "preferred_share" {
        return None;
    }
    let source_kind = source_kind_for_concept(&text_fact.source_concept);
    if !allows_period_end_instrument_quantification(&source_kind) {
        return None;
    }

    let instrument_name = infer_instrument_name(&text_fact.text, instrument_type);
    let exercise_price_yen = extract_labeled_yen(
        &text_fact.text,
        &["行使価額", "権利行使価額", "払込金額", "行使時の払込金額"],
    );
    let conversion_price_yen =
        extract_labeled_yen(&text_fact.text, &["転換価額", "転換価格", "修正価額"]);
    let face_value_yen = extract_labeled_yen(
        &text_fact.text,
        &["社債残高", "残高", "額面総額", "発行総額"],
    );
    let period_end_common_shares = extract_period_end_common_shares(text_fact, instrument_type)
        .or_else(|| match (face_value_yen, conversion_price_yen) {
            (Some(face), Some(price)) if price != 0.0 => Some(face / price),
            _ => None,
        });

    Some(PotentialEquityInstrumentFact {
        period: text_fact.period.clone(),
        instrument_key: format!("{}:{}", instrument_type, normalize_key(&instrument_name)),
        instrument_type: instrument_type.to_string(),
        instrument_name,
        terms_text: truncate_chars(&text_fact.text, TERMS_TEXT_LIMIT),
        period_end_common_shares,
        exercise_price_yen,
        conversion_price_yen,
        face_value_yen,
        source_concept: text_fact.source_concept.clone(),
        source_kind,
    })
}

fn build_status_fact(text_fact: &TextFact) -> Option<PotentialEquityStatusFact> {
    let source_kind = source_kind_for_concept(&text_fact.source_concept);
    if source_kind != "per_share_information_text" {
        return None;
    }
    let normalized = normalize_for_match(&text_fact.text);
    let has_potential_equity = if is_explicit_potential_equity_absence(&normalized) {
        false
    } else if is_unquantified_potential_equity_presence(&normalized) {
        true
    } else {
        return None;
    };
    let status_key = if has_potential_equity {
        "unquantified_potential_equity"
    } else {
        "no_potential_equity"
    };
    Some(PotentialEquityStatusFact {
        period: text_fact.period.clone(),
        status_key: status_key.to_string(),
        has_potential_equity,
        terms_text: truncate_chars(&text_fact.text, TERMS_TEXT_LIMIT),
        source_concept: text_fact.source_concept.clone(),
        source_kind,
    })
}

fn is_explicit_potential_equity_absence(normalized: &str) -> bool {
    if normalized.contains("希薄化効果を有している潜在株式が存在しない")
        || normalized.contains("希薄化効果を有する潜在株式が存在しない")
    {
        return false;
    }
    normalized.contains("潜在株式が存在しない")
        || normalized.contains("潜在株式は存在しない")
        || normalized.contains("潜在株式が存在しておりません")
        || normalized.contains("潜在株式は存在しておりません")
}

fn is_unquantified_potential_equity_presence(normalized: &str) -> bool {
    normalized.contains("潜在株式は存在するものの")
        || normalized.contains("潜在株式が存在するものの")
        || normalized.contains("潜在株式は存在しておりますが")
        || normalized.contains("潜在株式が存在しておりますが")
        || normalized.contains("潜在株式は存在するが")
        || normalized.contains("潜在株式が存在するが")
        || normalized.contains("希薄化効果を有しない")
        || normalized.contains("逆希薄化効果")
}

fn build_diluted_eps_adjustment_fact(text_fact: &TextFact) -> Option<DilutedEpsAdjustmentFact> {
    let source_kind = source_kind_for_concept(&text_fact.source_concept);
    if source_kind != "per_share_information_text" {
        return None;
    }
    if !contains_diluted_eps_context(&text_fact.text) {
        return None;
    }
    let weighted_average_common_shares = extract_weighted_average_common_shares(text_fact)?;
    Some(DilutedEpsAdjustmentFact {
        period: text_fact.period.clone(),
        adjustment_key: "diluted_eps_common_share_increase".to_string(),
        adjustment_type: "diluted_eps_common_share_increase".to_string(),
        adjustment_name: "普通株式増加額".to_string(),
        terms_text: truncate_chars(&text_fact.text, TERMS_TEXT_LIMIT),
        weighted_average_common_shares: Some(weighted_average_common_shares),
        source_concept: text_fact.source_concept.clone(),
        source_kind,
    })
}

fn contains_diluted_eps_context(text: &str) -> bool {
    let normalized = normalize_for_match(text);
    normalized.contains("潜在株式調整後")
        || normalized.contains("普通株式増加額")
        || normalized.contains("希薄化")
        || normalized.to_ascii_lowercase().contains("dilut")
}

fn extract_weighted_average_common_shares(text_fact: &TextFact) -> Option<f64> {
    extract_weighted_average_common_shares_from_tables(&text_fact.tables)
        .or_else(|| extract_weighted_average_common_shares_from_text(&text_fact.text))
        .filter(|value| is_plausible_share_count(*value, "diluted_eps_adjustment"))
}

fn extract_weighted_average_common_shares_from_tables(tables: &[TextTable]) -> Option<f64> {
    for table in tables {
        for row in &table.rows {
            let joined = normalize_for_match(&row.join(" "));
            if !(joined.contains("普通株式増加額")
                || joined.contains("普通株式増加数")
                || joined.contains("普通株式増加"))
            {
                continue;
            }
            let multiplier = share_count_multiplier(&joined);
            let parsed_cells = row
                .iter()
                .skip(1)
                .map(|cell| parse_standalone_share_cell(cell).map(|value| value * multiplier))
                .collect::<Vec<_>>();
            if !parsed_cells.is_empty() {
                return parsed_cells
                    .last()
                    .copied()
                    .flatten()
                    .filter(|value| *value > 0.0);
            }
            if let Some(value) = extract_share_count_after_marker(&joined, "普通株式増加") {
                return Some(value);
            }
        }
    }
    None
}

fn extract_weighted_average_common_shares_from_text(text: &str) -> Option<f64> {
    let normalized = normalize_for_number_scan(text);
    extract_share_count_after_marker(&normalized, "普通株式増加")
}

fn extract_share_count_after_marker(text: &str, marker: &str) -> Option<f64> {
    let mut search_start = 0usize;
    while let Some(pos) = text[search_start..].find(marker) {
        let marker_pos = search_start + pos;
        let after_marker = take_chars_from_byte(text, marker_pos + marker.len(), 260);
        if after_marker.starts_with("の主要な内訳") {
            search_start = marker_pos + marker.len();
            continue;
        }
        let segment = truncate_at_diluted_eps_stop(after_marker);
        let multiplier = share_count_multiplier(&format!("{marker}{segment}"));
        let tokens = extract_number_or_dash_tokens(segment);
        return match tokens.last() {
            Some(ShareToken::Number(value)) if *value > 0.0 => Some(*value * multiplier),
            _ => None,
        };
    }
    None
}

fn parse_standalone_share_cell(cell: &str) -> Option<f64> {
    let normalized = normalize_for_match(cell);
    if normalized.is_empty() || normalized == "-" || normalized == "－" || normalized == "―" {
        return None;
    }
    if normalized.contains('円') || normalized.contains("千円") || normalized.contains("百万円")
    {
        return None;
    }
    parse_number_at_end(&normalized)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ShareToken {
    Number(f64),
    Dash,
}

fn extract_number_or_dash_tokens(segment: &str) -> Vec<ShareToken> {
    let mut values = Vec::new();
    let mut buf = String::new();
    for ch in segment.chars() {
        if is_number_char(ch) || ch == '△' {
            buf.push(ch);
        } else if matches!(ch, '-' | '－' | '―') {
            if !buf.is_empty() {
                push_share_token(&mut values, &buf);
                buf.clear();
            }
            values.push(ShareToken::Dash);
        } else if !buf.is_empty() {
            push_share_token(&mut values, &buf);
            buf.clear();
        }
    }
    if !buf.is_empty() {
        push_share_token(&mut values, &buf);
    }
    values
}

fn push_share_token(tokens: &mut Vec<ShareToken>, raw: &str) {
    if raw.chars().all(|ch| matches!(ch, '-' | '－' | '―')) {
        tokens.push(ShareToken::Dash);
    } else if let Some(value) = parse_number(raw) {
        tokens.push(ShareToken::Number(value));
    }
}

fn truncate_at_diluted_eps_stop(raw: &str) -> &str {
    let stop_markers = [
        "（うち",
        "(うち",
        "希薄化効果",
        "1株当たり純資産",
        "１株当たり純資産",
        "純資産の部",
        "普通株式に係る期末",
    ];
    let stop = stop_markers
        .iter()
        .filter_map(|marker| raw.find(marker))
        .min()
        .unwrap_or(raw.len());
    &raw[..stop]
}

fn share_count_multiplier(context: &str) -> f64 {
    if context.contains("千株") {
        1_000.0
    } else {
        1.0
    }
}

fn allows_preferred_share_potential_equity(source_kind: &str) -> bool {
    matches!(
        source_kind,
        "issued_share_change_text" | "classes_of_shares_text" | "preferred_share_text"
    )
}

fn classify_instrument(text: &str) -> Option<&'static str> {
    let lower = text.to_ascii_lowercase();
    if text.contains("優先株式") || text.contains("種類株式") || lower.contains("preferred share")
    {
        return Some("preferred_share");
    }
    if text.contains("転換社債")
        || text.contains("新株予約権付社債")
        || lower.contains("convertible bond")
    {
        return Some("convertible_bond");
    }
    if text.contains("ストック・オプション")
        || text.contains("ストックオプション")
        || lower.contains("stock option")
    {
        return Some("stock_option");
    }
    if text.contains("新株予約権")
        || text.contains("ワラント")
        || lower.contains("share acquisition right")
        || lower.contains("warrant")
    {
        return Some("share_acquisition_right");
    }
    if text.contains("譲渡制限付株式")
        || text.contains("株式報酬")
        || lower.contains("rsu")
        || lower.contains("psu")
        || lower.contains("share-based")
        || lower.contains("share based")
    {
        return Some("share_based_compensation");
    }
    if text.contains("潜在株式")
        || text.contains("希薄")
        || lower.contains("potential share")
        || lower.contains("dilut")
    {
        return Some("other_potential_equity");
    }
    None
}

fn is_absence_only(text: &str) -> bool {
    let normalized = normalize_for_match(text);
    let absence = normalized.contains("該当事項はありません")
        || normalized.contains("該当事項なし")
        || normalized.contains("該当ありません")
        || normalized.to_ascii_lowercase().contains("not applicable");
    if !absence {
        return false;
    }
    !(normalized.contains("潜在株式は存在")
        || normalized.contains("潜在株式が存在")
        || normalized.contains("存在するものの"))
}

fn infer_instrument_name(text: &str, instrument_type: &str) -> String {
    let normalized = normalize_for_match(text);
    if let Some(name) = extract_round_name(&normalized, "新株予約権") {
        return name;
    }
    match instrument_type {
        "convertible_bond" => "転換社債/新株予約権付社債".to_string(),
        "stock_option" => "ストックオプション".to_string(),
        "share_acquisition_right" => "新株予約権".to_string(),
        "share_based_compensation" => "株式報酬".to_string(),
        _ => "潜在株式".to_string(),
    }
}

fn extract_round_name(normalized: &str, suffix: &str) -> Option<String> {
    let suffix_pos = normalized.find(suffix)?;
    let prefix = &normalized[..suffix_pos];
    let mut start = suffix_pos;
    for (idx, ch) in prefix.char_indices().rev() {
        let ok = matches!(
            ch,
            '第' | '回' | '号' | '種' | '0'..='9' | '０'..='９'
                | '一' | '二' | '三' | '四' | '五' | '六' | '七' | '八' | '九' | '十'
        );
        if ok {
            start = idx;
        } else if start != suffix_pos {
            break;
        }
    }
    if start == suffix_pos {
        return None;
    }
    Some(format!("{}{}", &normalized[start..suffix_pos], suffix))
}

fn allows_period_end_instrument_quantification(source_kind: &str) -> bool {
    matches!(
        source_kind,
        "share_options_text"
            | "share_acquisition_rights_text"
            | "potential_equity_text"
            | "classes_of_shares_text"
    )
}

fn extract_period_end_common_shares(text_fact: &TextFact, instrument_type: &str) -> Option<f64> {
    extract_period_end_common_shares_from_tables(&text_fact.tables)
        .or_else(|| extract_potential_common_shares(&text_fact.text))
        .filter(|value| is_plausible_share_count(*value, instrument_type))
}

fn extract_period_end_common_shares_from_tables(tables: &[TextTable]) -> Option<f64> {
    for table in tables {
        for row in &table.rows {
            let joined = normalize_for_match(&row.join(" "));
            if !(joined.contains("目的となる株式")
                || joined.contains("目的である株式")
                || joined.contains("未行使")
                || joined.contains("期末")
                || joined.contains("残高"))
            {
                continue;
            }
            if !(joined.contains("普通株式") || joined.to_ascii_lowercase().contains("common"))
            {
                continue;
            }
            if let Some(value) = row.iter().rev().find_map(|cell| {
                extract_shares_after_marker(&normalize_for_match(cell), "普通株式")
                    .or_else(|| parse_number_before_unit(&normalize_for_match(cell), "株"))
            }) {
                return Some(value);
            }
        }
    }
    None
}

fn extract_potential_common_shares(text: &str) -> Option<f64> {
    let normalized = normalize_for_match(text);
    match (
        extract_count_after_marker(&normalized, "新株予約権", "個"),
        extract_shares_after_marker(&normalized, "1個につき普通株式"),
    ) {
        (Some(rights), Some(shares_per_right)) => return Some(rights * shares_per_right),
        _ => {}
    }

    for marker in [
        "目的となる株式の数",
        "目的となる株式の種類、内容及び数",
        "目的となる株式の種類、数",
        "目的である株式の数",
        "普通株式",
    ] {
        if let Some(value) = extract_shares_after_marker(&normalized, marker) {
            if value != 1.0 || !marker.contains("普通株式") {
                return Some(value);
            }
        }
    }
    None
}

fn extract_shares_after_marker(text: &str, marker: &str) -> Option<f64> {
    let pos = text.find(marker)?;
    let segment = take_chars_from_byte(text, pos + marker.len(), 160);
    parse_number_before_unit(segment, "株")
}

fn extract_count_after_marker(text: &str, marker: &str, unit: &str) -> Option<f64> {
    let pos = text.find(marker)?;
    let segment = take_chars_from_byte(text, pos + marker.len(), 40);
    parse_number_before_unit(segment, unit)
}

fn extract_labeled_yen(text: &str, labels: &[&str]) -> Option<f64> {
    let normalized = normalize_for_match(text);
    for label in labels {
        let mut search_start = 0usize;
        while let Some(pos) = normalized[search_start..].find(label) {
            let start = search_start + pos + label.len();
            let segment = take_chars_from_byte(&normalized, start, 100);
            if let Some(value) = parse_yen_amount(segment) {
                return Some(value);
            }
            search_start = start;
        }
    }
    None
}

fn parse_yen_amount(segment: &str) -> Option<f64> {
    let yen_pos = segment.find('円')?;
    let before_yen = &segment[..yen_pos];
    let multiplier = if before_yen.ends_with("百万円") || before_yen.ends_with("百万") {
        1_000_000.0
    } else if before_yen.ends_with("千円") || before_yen.ends_with('千') {
        1_000.0
    } else {
        1.0
    };
    parse_number_at_end(before_yen).map(|value| value * multiplier)
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

fn parse_number(raw: &str) -> Option<f64> {
    let normalized = normalize_number(raw);
    if normalized.is_empty() {
        return None;
    }
    normalized.parse::<f64>().ok()
}

fn is_plausible_share_count(value: f64, _kind: &str) -> bool {
    value.is_finite() && value >= 0.0 && value <= 10_000_000_000.0
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

fn normalize_for_match(raw: &str) -> String {
    raw.chars()
        .filter_map(|ch| match ch {
            'Ａ'..='Ｚ' => Some(((ch as u32 - 'Ａ' as u32) as u8 + b'A') as char),
            'ａ'..='ｚ' => Some(((ch as u32 - 'ａ' as u32) as u8 + b'a') as char),
            '０'..='９' => Some(((ch as u32 - '０' as u32) as u8 + b'0') as char),
            '，' => Some(','),
            '：' => Some(':'),
            ch if ch.is_whitespace() || ch == '　' => None,
            ch => Some(ch),
        })
        .collect()
}

fn normalize_for_number_scan(raw: &str) -> String {
    raw.chars()
        .map(|ch| match ch {
            'Ａ'..='Ｚ' => ((ch as u32 - 'Ａ' as u32) as u8 + b'A') as char,
            'ａ'..='ｚ' => ((ch as u32 - 'ａ' as u32) as u8 + b'a') as char,
            '０'..='９' => ((ch as u32 - '０' as u32) as u8 + b'0') as char,
            '，' => ',',
            '：' => ':',
            ch if ch.is_whitespace() || ch == '　' => ' ',
            ch => ch,
        })
        .collect()
}

fn normalize_key(raw: &str) -> String {
    normalize_for_match(raw)
        .chars()
        .map(|ch| if ch == ':' { '_' } else { ch })
        .collect()
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

fn source_kind_for_concept(source_concept: &str) -> String {
    let local = source_concept
        .rsplit_once('#')
        .map(|(_, local)| local)
        .unwrap_or(source_concept);
    if local.contains("EmployeeShareOption") || local.contains("ShareOptions") {
        "share_options_text".to_string()
    } else if local.contains("ShareAcquisitionRights") {
        "share_acquisition_rights_text".to_string()
    } else if local.contains("PerShareInformation") {
        "per_share_information_text".to_string()
    } else if local.contains("BusinessResults") {
        "business_results_text".to_string()
    } else if local.contains("IssuedShares") {
        "issued_shares_text".to_string()
    } else {
        "potential_equity_text".to_string()
    }
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

fn dedupe_and_sort(rows: Vec<PotentialEquityInstrumentFact>) -> Vec<PotentialEquityInstrumentFact> {
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let key = (
            row.period.clone(),
            row.instrument_key.clone(),
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
            .then_with(|| a.instrument_type.cmp(&b.instrument_type))
            .then_with(|| a.instrument_name.cmp(&b.instrument_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });
    result
}

fn dedupe_and_sort_adjustments(
    rows: Vec<DilutedEpsAdjustmentFact>,
) -> Vec<DilutedEpsAdjustmentFact> {
    let mut seen: HashSet<(String, String, String, String)> = HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let key = (
            row.period.clone(),
            row.adjustment_key.clone(),
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
            .then_with(|| a.adjustment_type.cmp(&b.adjustment_type))
            .then_with(|| a.adjustment_name.cmp(&b.adjustment_name))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });
    result
}

fn dedupe_and_sort_statuses(
    rows: Vec<PotentialEquityStatusFact>,
) -> Vec<PotentialEquityStatusFact> {
    let mut seen: HashSet<(String, String, bool, String, String)> = HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let key = (
            row.period.clone(),
            row.status_key.clone(),
            row.has_potential_equity,
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
            .then_with(|| a.status_key.cmp(&b.status_key))
            .then_with(|| a.source_kind.cmp(&b.source_kind))
            .then_with(|| a.source_concept.cmp(&b.source_concept))
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{TextFact, TextTable};

    fn row_for(text: &str) -> Option<PotentialEquityInstrumentFact> {
        build_term_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:DetailsOfEmployeeShareOptionProgramTextBlock".to_string(),
            text: text.to_string(),
            tables: Vec::new(),
        })
    }

    #[test]
    fn parses_share_acquisition_right_common_shares() {
        let row = row_for("第１回新株予約権 新株予約権の目的となる株式の数 普通株式10,000株")
            .expect("row");
        assert_eq!(row.instrument_type, "share_acquisition_right");
        assert_eq!(row.instrument_name, "第1回新株予約権");
        assert_eq!(row.period_end_common_shares, Some(10_000.0));
    }

    #[test]
    fn parses_right_count_times_shares_per_right() {
        let row = row_for("新株予約権100個 1個につき普通株式100株を交付する。").expect("row");
        assert_eq!(row.period_end_common_shares, Some(10_000.0));
    }

    #[test]
    fn does_not_create_instrument_from_per_share_information() {
        let row = build_term_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式は存在するものの、１株当たり当期純損失であるため記載しておりません。"
                .to_string(),
            tables: Vec::new(),
        });
        assert!(row.is_none());
    }

    #[test]
    fn skips_absence_only_text() {
        assert!(row_for("新株予約権等に関する事項 該当事項はありません。").is_none());
    }

    #[test]
    fn extracts_diluted_eps_weighted_average_from_table() {
        let row = build_diluted_eps_adjustment_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益 普通株式増加額".to_string(),
            tables: vec![TextTable {
                rows: vec![vec![
                    "普通株式増加額(株)".to_string(),
                    "―".to_string(),
                    "831,295".to_string(),
                ]],
            }],
        })
        .expect("adjustment");
        assert_eq!(row.weighted_average_common_shares, Some(831_295.0));
    }

    #[test]
    fn diluted_eps_text_does_not_fall_through_to_net_asset_section() {
        let row = build_diluted_eps_adjustment_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益金額 普通株式増加数 (千株) 0 ― （うち新株予約権） (千株) (0) (―) 希薄化効果を有しないため、潜在株式調整後１株当たり当期純利益の算定に含めなかった潜在株式の概要 ― ― ２．１株当たり純資産額の算定上の基礎 純資産の部の合計額から控除する金額 (千円) 27,564 32,476".to_string(),
            tables: Vec::new(),
        });

        assert!(row.is_none());
    }

    #[test]
    fn diluted_eps_text_uses_current_period_value_before_stop_marker() {
        let row = build_diluted_eps_adjustment_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益 普通株式増加数（株） － 1,563,721 希薄化効果を有しないため、潜在株式調整後１株当たり当期純利益の算定に含めなかった潜在株式の概要 － －".to_string(),
            tables: Vec::new(),
        })
        .expect("adjustment");

        assert_eq!(row.weighted_average_common_shares, Some(1_563_721.0));
    }

    #[test]
    fn diluted_eps_text_applies_thousand_share_unit() {
        let row = build_diluted_eps_adjustment_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益 普通株式増加数(千株) 3.1 4.1 希薄化効果を有しないため、潜在株式調整後１株当たり当期純利益の算定に含めなかった潜在株式の概要 － －".to_string(),
            tables: Vec::new(),
        })
        .expect("adjustment");

        assert_eq!(row.weighted_average_common_shares, Some(4_100.0));
    }

    #[test]
    fn ignores_issued_shares_text_as_period_end_instrument() {
        let row = build_term_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesRegardingIssuedSharesAndTreasurySharesTextBlock"
                .to_string(),
            text: "普通株式1,337,529,084株。ストック・オプションの権利行使による減少93,968株"
                .to_string(),
            tables: Vec::new(),
        });
        assert!(row.is_none());
    }

    #[test]
    fn extracts_explicit_absence_status() {
        let row = build_status_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益については、潜在株式が存在しないため記載しておりません。".to_string(),
            tables: Vec::new(),
        })
        .expect("status");
        assert_eq!(row.status_key, "no_potential_equity");
        assert!(!row.has_potential_equity);
    }

    #[test]
    fn ignores_business_results_status_text() {
        let row = build_status_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:BusinessResultsOfReportingCompanyTextBlock".to_string(),
            text: "潜在株式調整後１株当たり当期純利益 (円) ― ― ― ― ―".to_string(),
            tables: Vec::new(),
        });

        assert!(row.is_none());
    }

    #[test]
    fn does_not_treat_non_dilutive_phrase_as_absence() {
        let row = build_status_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "希薄化効果を有している潜在株式が存在しないため記載しておりません。".to_string(),
            tables: Vec::new(),
        });
        assert!(row.is_none());
    }

    #[test]
    fn extracts_unquantified_presence_status() {
        let row = build_status_fact(&TextFact {
            period: "2025-03".to_string(),
            source_concept: "jpcrp_cor:NotesPerShareInformationTextBlock".to_string(),
            text: "潜在株式は存在するものの、１株当たり当期純損失であるため記載しておりません。"
                .to_string(),
            tables: Vec::new(),
        })
        .expect("status");
        assert_eq!(row.status_key, "unquantified_potential_equity");
        assert!(row.has_potential_equity);
    }
}
