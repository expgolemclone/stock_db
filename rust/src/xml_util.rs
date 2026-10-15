use std::collections::HashMap;
use std::path::Path;

use crate::types::{ConceptKey, ContextInfo};

// XML namespace constants
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
pub const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";
pub const IX_NS: &str = "http://www.xbrl.org/2008/inlineXBRL";
pub const XBRLI_NS: &str = "http://www.xbrl.org/2003/instance";
pub const XBRLDI_NS: &str = "http://xbrl.org/2006/xbrldi";
pub const XLINK_NS: &str = "http://www.w3.org/1999/xlink";
pub const LINK_NS: &str = "http://www.xbrl.org/2003/linkbase";
pub const XS_NS: &str = "http://www.w3.org/2001/XMLSchema";

/// Namespaces whose facts should be skipped in instance document extraction.
pub const IGNORED_FACT_NAMESPACES: &[&str] = &[
    XHTML_NS, IX_NS, XBRLI_NS, XBRLDI_NS, XLINK_NS, LINK_NS, XS_NS, XML_NS,
];

/// Shares-denominated tag names used by the normalized financial parser.
pub const SHARES_OUTSTANDING_TAGS: &[&str] = &[
    "NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc",
    "NumberOfIssuedSharesAsOfFilingDateIssuedSharesTotalNumberOfSharesEtc",
    "NumberOfIssuedSharesIssuedSharesTotalNumberOfSharesEtc",
];

const ISSUED_SHARES_FACT_PRIORITIES: &[(&str, u8)] = &[
    (
        "NumberOfIssuedSharesAsOfFiscalYearEndIssuedSharesTotalNumberOfSharesEtc",
        0,
    ),
    (
        "NumberOfIssuedSharesAsOfFilingDateIssuedSharesTotalNumberOfSharesEtc",
        1,
    ),
    ("NumberOfIssuedSharesIssuedSharesTotalNumberOfSharesEtc", 2),
];

/// Patterns for fact document files.
pub const FACT_FILE_EXTENSIONS: &[&str] = &["xhtml", "html", "htm", "xbrl"];

/// Inventory total tag names.
pub const INVENTORY_TOTAL_TAGS: &[&str] = &[
    "Inventories",
    "InventoriesCA",
    "InventoriesCAIFRS",
    "InventoriesCAIFRSIFRS",
    "InventoriesIFRS",
    "InventoriesAssetsIFRS",
];

/// Inventory component tag names.
pub const INVENTORY_COMPONENT_TAGS: &[&str] = &[
    // Japan GAAP
    "BeautyMaterialsCA",
    "EducationalMaterialsCA",
    "EquipmentAndMaterials",
    "FinishedGoods",
    "FinishedGoodsAndSemiFinishedGoodsCA",
    "FinishedGoodsAndWorkInProcess",
    "FinishedGoodsAndWorkInProcessCA",
    "FinishedGoodsCA",
    "FinishedGoodsCAIFRS",
    "FinishedGoodsIncludingSemiFinishedGoods",
    "FoodInventoryOnStoreCAAssets",
    "IngredientsAndProductionSuppliesCA",
    "InventoriesOfJointProjectInvestmentCA",
    "MaintenanceSupplies",
    "MaterialsAndStocksCA",
    "Merchandise",
    "MerchandiseAndFinishedGoods",
    "MerchandiseAndFinishedGoodsSemiFinishedGoods",
    "MerchandiseCA",
    "MerchandiseEtcCA",
    "MerchandiseLentCA",
    "MerchandiseSuppliesCA",
    "OtherInventories",
    "PartlyFinishedGoodsCA",
    "PFIProjectsAndOtherInventoriesCA",
    "ProgramInventories",
    "ProgramRightsAndWorkInProcess",
    "PurchasedGoodsMaterialsAndSuppliesCA",
    "RawMaterials",
    "RawMaterialsAndSupplies",
    "RawMaterialsAndSuppliesCA",
    "RawMaterialsAndSuppliesCNS",
    "RawMaterialsCA",
    "RawMaterialsCAGAS",
    "RawMaterialsInTransit",
    "RealEstateForSale",
    "RealEstateForSaleAndDevelopmentProjectsInProgressCNS",
    "RealEstateForSaleCNS",
    "RealEstateForSaleInProcess",
    "RealEstateForSaleInProcessCA",
    "RealEstateForSaleInProcessAndOtherCA",
    "RealEstateForSaleInTrustCA",
    "RentalInventoryAssetsCA",
    "SemiFinishedGoods",
    "SemiFinishedGoodsAndWorkInProcessCA",
    "Supplies",
    "SuppliesCA",
    "TeachingMaterials",
    "TemporaryMaterials",
    "TrustBeneficiaryRightOfRealEstateForSaleInventories",
    "UndeliveredMerchandise",
    "WorkInProcess",
    "WorkInProcessAndPartlyFinishedConstructionCA",
    "WorkInProcessCA",
    "WorkInProcessCAAssets",
    "WorkInProcessContentsAssetsCA",
    "WorkInProcessConstructionCA",
    // Japan GAAP - real estate / construction
    "CostsOnRealEstateBusiness",
    "CostsOnUncompletedConstructionContractsCNS",
    "CostsOnUncompletedConstructionContractsAndOtherCNS",
    "CostsOnUncompletedServices",
    "DevelopmentProjectsInProgress",
    "GoodsInTransit",
    "InvestmentInRealEstateForSaleCA",
    "LandAndBuildingsForSaleInLots",
    "LandForSaleInLots",
    "MerchandizeAndFinishedGoods",
    // IFRS
    "ConstructionInProgressCAIFRS",
    "FinishedGoodsCAIFRS",
    "MerchandiseAndFinishedGoodsCAIFRS",
    "MerchandiseAssetsIFRS",
    "MerchandiseCAIFRS",
    "OtherInventoriesAssetsIFRS",
    "OtherInventoriesCAIFRS",
    "ProductionSuppliesCAIFRS",
    "RawMaterialsAndOthersCAIFRS",
    "RawMaterialsAndSuppliesCAIFRS",
    "RawMaterialsPurchasedComponentsAndSuppliesCAIFRS",
    "RawMaterialsWorkInProgressAndSuppliesCAIFRS",
    "RawMaterialsCAIFRS",
    "RealEstateForSaleAssetsIFRS",
    "RealEstateForSaleCAIFRS",
    "RealEstateForSaleInProcessCAIFRS",
    "SemiFinishedGoodsAndWorkInProcessCAIFRS",
    "SemiFinishedGoodsCAIFRS",
    "SuppliesAndOtherCAIFRS",
    "SuppliesAndRawMaterialsCAIFRS",
    "TelecommunicationsTerminalEquipmentAndMaterialsToBeSoldCAIFRS",
    "WorkInProcessAndRawMaterialsCAIFRS",
    "WorkInProcessAssetsIFRS",
    "WorkInProcessCAIFRS",
];

/// Exact inventory-like tag names that should be ignored.
pub const IGNORED_INVENTORY_TAGS: &[&str] = &[
    // Financial-sector "stocks" are investment/insurance assets, not inventories.
    "StocksAssetsINS",
    "StocksAssetsInvestmentSecuritiesBNK",
];

/// Substrings that indicate an inventory-like tag should be ignored.
pub const IGNORED_INVENTORY_SUBSTRINGS: &[&str] = &[
    "AccountsReceivable",
    "AccumulatedDepreciation",
    "AccumulatedImpairment",
    "AcquisitionCost",
    "Adjustment",
    "AllowanceFor",
    "Amortization",
    "Beginning",
    "ChangeIn",
    "ChangesIn",
    "Compensation",
    "CostOf",
    "DecreaseIncrease",
    "DifferenceBetweenCostOf",
    "Disposal",
    "Ending",
    "ExportPriceAdjustment",
    "GainOn",
    "GrossProfit",
    "IfDifferentFromBsBalance",
    "IncreaseDecrease",
    "LossOn",
    "NCA",
    "NetCOS",
    "NetSales",
    "Notes",
    "OfWhich",
    "PaymentsFor",
    "ProceedsFrom",
    "ProductsStocks",
    "ProvisionFor",
    "Purchase",
    "Receivable",
    "Recycled",
    "Redemption",
    "ReserveFor",
    "StocksOf",
    "ScheduledToBeSold",
    "Subtotal",
    "ThatAreScheduledToBeSold",
    "TextBlock",
    "ToBeSoldForMoreThan",
    "ToBeSoldMoreThan",
    "TotalBeginning",
    "TransferFrom",
    "TransferTo",
    "Valuation",
    "WriteDown",
];

/// Keywords to detect inventory-like concept names.
pub const INVENTORY_CANDIDATE_KEYWORDS: &[&str] = &[
    "BeautyMaterials",
    "EducationalMaterials",
    "FinishedGoods",
    "FoodInventory",
    "IngredientsAndProductionSupplies",
    "Inventor",
    "MaintenanceSupplies",
    "Materials",
    "Merchandise",
    "OtherInventories",
    "ProgramInventories",
    "RawMaterials",
    "RealEstateForSale",
    "RentalInventory",
    "SemiFinishedGoods",
    "Stocks",
    "Supplies",
    "TeachingMaterials",
    "TemporaryMaterials",
    "UndeliveredMerchandise",
    "WorkInProcess",
];

/// Check if a short name looks inventory-related.
pub fn is_inventory_like(short_name: &str) -> bool {
    INVENTORY_CANDIDATE_KEYWORDS
        .iter()
        .any(|kw| short_name.contains(kw))
}

/// Check if an inventory-like tag should be ignored.
pub fn is_ignored_inventory_candidate(short_name: &str) -> bool {
    if IGNORED_INVENTORY_TAGS.contains(&short_name) {
        return true;
    }
    if short_name.starts_with("ConstructionInProgress") {
        return short_name != "ConstructionInProgressCAIFRS";
    }
    IGNORED_INVENTORY_SUBSTRINGS
        .iter()
        .any(|frag| short_name.contains(frag))
}

/// Check if a shares-denominated fact should be retained.
pub fn is_relevant_shares_fact(short_name: &str) -> bool {
    SHARES_OUTSTANDING_TAGS.contains(&short_name)
}

pub fn issued_shares_fact_priority(short_name: &str) -> Option<u8> {
    ISSUED_SHARES_FACT_PRIORITIES
        .iter()
        .find_map(|(name, priority)| (*name == short_name).then_some(*priority))
}

pub fn concept_key_string(concept: &ConceptKey) -> String {
    if concept.0.is_empty() {
        concept.1.clone()
    } else {
        format!("{}#{}", concept.0, concept.1)
    }
}

pub fn is_total_share_class_member(local_name: &str) -> bool {
    local_name == "TotalClassesOfSharesMember"
}

pub fn share_class_name_from_member(local_name: &str, label: Option<&str>) -> String {
    if let Some(cleaned) = clean_share_class_label(label) {
        return cleaned;
    }
    match local_name {
        "OrdinaryShareMember" => "普通株式".to_string(),
        "ClassAPreferredSharesMember" => "Ａ種優先株式".to_string(),
        "ClassBPreferredSharesMember" => "Ｂ種優先株式".to_string(),
        "ClassOnePreferredSharesMember" => "第一種優先株式".to_string(),
        "ClassTwoPreferredSharesMember" => "第二種優先株式".to_string(),
        "ClassASharesMember" => "Ａ種種類株式".to_string(),
        _ => local_name.trim_end_matches("Member").to_string(),
    }
}

pub fn share_class_name_from_concept(local_name: &str, label: Option<&str>) -> String {
    if let Some(cleaned) = clean_share_class_label(label) {
        return cleaned;
    }
    local_name
        .replace("TotalNumberOfIssuedShares", "")
        .replace("NumberOfIssuedShares", "")
        .replace("SummaryOfBusinessResults", "")
        .replace("KeyFinancialData", "")
        .trim_matches('_')
        .to_string()
}

pub fn is_preferred_share_class(local_name: &str, class_name: &str) -> bool {
    local_name.contains("PreferredShares")
        || local_name.contains("PreferredShare")
        || class_name.contains("優先株式")
        || (class_name.contains("種類株式") && !class_name.contains("普通株式"))
        || class_name.to_ascii_lowercase().contains("preferred shares")
}

pub fn is_class_specific_issued_shares_concept(local_name: &str, label: Option<&str>) -> bool {
    let has_issued_shares = local_name.contains("TotalNumberOfIssuedShares")
        || local_name.contains("NumberOfIssuedShares");
    if !has_issued_shares || SHARES_OUTSTANDING_TAGS.contains(&local_name) {
        return false;
    }
    let label_text = label.unwrap_or("");
    let haystack = format!("{local_name} {label_text}");
    haystack.contains("PreferredShares")
        || haystack.contains("OrdinaryShares")
        || haystack.contains("ClassA")
        || haystack.contains("ClassB")
        || haystack.contains("ClassC")
        || haystack.contains("ClassD")
        || haystack.contains("ClassE")
        || haystack.contains("優先株式")
        || haystack.contains("普通株式")
        || haystack.contains("種類株式")
}

pub fn plain_text_from_xbrl_text(raw: &str) -> String {
    let without_tags = strip_xml_like_tags(raw);
    let decoded = decode_common_html_entities(&without_tags);
    collapse_whitespace(&decoded)
}

pub fn is_relevant_potential_equity_text_fact(local_name: &str, text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let has_potential_equity = text.contains("優先株式")
        || text.contains("種類株式")
        || text.contains("新株予約権")
        || text.contains("ストック・オプション")
        || text.contains("ストックオプション")
        || text.contains("転換社債")
        || text.contains("新株予約権付社債")
        || text.contains("ワラント")
        || text.contains("潜在株式")
        || text.contains("希薄")
        || text.contains("譲渡制限付株式")
        || text.contains("株式報酬")
        || lower.contains("preferred share")
        || lower.contains("preferred stock")
        || lower.contains("share acquisition right")
        || lower.contains("stock option")
        || lower.contains("convertible")
        || lower.contains("warrant")
        || lower.contains("potential share")
        || lower.contains("dilut")
        || lower.contains("rsu")
        || lower.contains("psu");
    if !has_potential_equity {
        return false;
    }

    let has_term_hint = text.contains("普通株式")
        || text.contains("転換")
        || text.contains("変更")
        || text.contains("行使")
        || text.contains("取得請求")
        || text.contains("取得条項")
        || text.contains("目的となる株式")
        || text.contains("発行価格")
        || text.contains("払込金額")
        || text.contains("行使価額")
        || text.contains("転換価額")
        || text.contains("資本組入額")
        || text.contains("希薄")
        || text.contains("潜在株式")
        || lower.contains("convert")
        || lower.contains("conversion")
        || lower.contains("exercise")
        || lower.contains("option")
        || lower.contains("warrant")
        || lower.contains("dilut");
    if !has_term_hint {
        return false;
    }

    local_name.ends_with("TextBlock")
        && (local_name.contains("IssuedShares")
            || local_name.contains("ClassesOfShares")
            || local_name.contains("ShareAcquisitionRights")
            || local_name.contains("ShareOption")
            || local_name.contains("ShareOptions")
            || local_name.contains("SharesAndTreasuryShares")
            || local_name.contains("PerShareInformation")
            || local_name.contains("StatedCapital"))
}

fn strip_xml_like_tags(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut in_tag = false;
    for ch in raw.chars() {
        if ch == '<' {
            in_tag = true;
            output.push(' ');
        } else if ch == '>' {
            in_tag = false;
            output.push(' ');
        } else if !in_tag {
            output.push(ch);
        }
    }
    output
}

fn decode_common_html_entities(raw: &str) -> String {
    raw.replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&ensp;", " ")
        .replace("&emsp;", " ")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn collapse_whitespace(raw: &str) -> String {
    let mut output = String::with_capacity(raw.len());
    let mut last_was_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() || ch == '\u{00a0}' {
            if !last_was_space {
                output.push(' ');
                last_was_space = true;
            }
        } else {
            output.push(ch);
            last_was_space = false;
        }
    }
    output.trim().to_string()
}

fn clean_share_class_label(label: Option<&str>) -> Option<String> {
    let label = label?;
    for raw_part in label.split(['、', ',']) {
        let part = raw_part
            .trim()
            .trim_end_matches(" [メンバー]")
            .trim_end_matches(" [member]")
            .trim();
        if part.is_empty()
            || part.contains("発行済株式総数")
            || part.contains("経営指標等")
            || part.eq_ignore_ascii_case("key financial data")
        {
            continue;
        }
        if part.contains("株式") || part.to_ascii_lowercase().contains("shares") {
            return Some(part.to_string());
        }
    }
    None
}

/// Extract the local name from a namespaced tag like `{uri}local` or `prefix:local`.
pub fn local_name(tag: &str) -> String {
    if let Some(idx) = tag.rfind('}') {
        tag[idx + 1..].to_string()
    } else if let Some(idx) = tag.rfind(':') {
        tag[idx + 1..].to_string()
    } else {
        tag.to_string()
    }
}

/// Extract the namespace URI from a namespaced tag like `{uri}local`.
pub fn namespace_uri(tag: &str) -> &str {
    if tag.starts_with('{') {
        if let Some(idx) = tag.find('}') {
            return &tag[1..idx];
        }
    }
    ""
}

/// Resolve a QName (prefix:local) using the namespace map.
pub fn resolve_qname(
    name: &str,
    nsmap: &HashMap<String, String>,
    fallback_uri: &str,
) -> Option<ConceptKey> {
    if name.is_empty() {
        return None;
    }
    // Already resolved: {uri}local
    if name.starts_with('{') {
        if let Some(idx) = name.find('}') {
            let uri = &name[1..idx];
            let local = &name[idx + 1..];
            return Some((uri.to_string(), local.to_string()));
        }
    }
    // Prefixed: prefix:local
    if let Some(colon) = name.find(':') {
        let prefix = &name[..colon];
        let local = &name[colon + 1..];
        if let Some(uri) = nsmap.get(prefix) {
            return Some((uri.clone(), local.to_string()));
        }
        return None;
    }
    // Unprefixed — use fallback or default namespace
    if !fallback_uri.is_empty() {
        return Some((fallback_uri.to_string(), name.to_string()));
    }
    if let Some(uri) = nsmap.get("") {
        return Some((uri.clone(), name.to_string()));
    }
    None
}

/// Parse a context period from an instant date string (YYYY-MM-DD → YYYY-MM).
pub fn context_period_from_instant(instant_text: Option<&str>) -> Option<String> {
    instant_text.and_then(|t| {
        if t.len() >= 7 {
            Some(t[..7].to_string())
        } else {
            None
        }
    })
}

/// Characters that represent a "dash" meaning no value (not zero).
const DASH_CHARS: &[char] = &['−', '—', '－', '\u{FF0D}', '\u{2015}'];

/// Characters treated as negative-number prefixes alongside the sign attribute.
const NEGATIVE_PREFIX_CHARS: &[char] = &['△', '▲'];

/// Parse an XBRL numeric value from raw text, applying scale and sign.
///
/// Handles Japanese numeric conventions commonly found in EDINET filings:
/// - `△1,234` / `▲1,234` — negative prefix (white/filled triangle)
/// - `(1,234)` — parenthesized negative (accounting convention)
/// - Full-width comma `，` mixed with half-width `,`
/// - Inline XBRL `sign="-"` and legacy `sign="negative"` attributes
/// - `20円0銭` — yen-sen format
pub fn parse_xbrl_value(raw: &str, _decimals: &str, scale: &str, sign: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || DASH_CHARS.contains(&trimmed.chars().next().unwrap_or('\0')) && trimmed.len() == 1
    {
        return None;
    }

    // Detect parenthesized negative: (1,234) → negative 1234
    let is_paren_negative = trimmed.starts_with('(') && trimmed.ends_with(')');
    let unparen = if is_paren_negative {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };

    // Detect leading negative prefix (△ or ▲)
    let is_prefix_negative =
        NEGATIVE_PREFIX_CHARS.contains(&unparen.chars().next().unwrap_or('\0'));

    // Strip commas (both half-width and full-width) and negative prefix chars
    let mut clean = String::with_capacity(unparen.len());
    for ch in unparen.chars() {
        if ch == ',' || ch == '\u{FF0C}' {
            continue;
        }
        if NEGATIVE_PREFIX_CHARS.contains(&ch) {
            continue;
        }
        clean.push(ch);
    }
    if clean.is_empty() {
        return None;
    }

    // Handle "円銭" format: "123円45銭"
    let value = if let Some(rest) = clean.strip_suffix("銭") {
        if let Some(yen_pos) = rest.find("円") {
            let yen: f64 = rest[..yen_pos].parse().ok()?;
            let sen: f64 = rest[yen_pos + 3..].parse().ok()?;
            yen + sen / 100.0
        } else {
            clean.parse().ok()?
        }
    } else {
        clean.parse().ok()?
    };

    let mut result = value;
    if !scale.is_empty() {
        if let Ok(s) = scale.parse::<i32>() {
            result *= 10f64.powi(s);
        }
    }
    if matches!(sign.trim(), "-" | "negative") || is_prefix_negative || is_paren_negative {
        result = -result;
    }
    Some(result)
}

/// Read file content as UTF-8 string, falling back to Shift-JIS (Windows-31J)
/// transcoding when UTF-8 decoding fails.
///
/// Some older EDINET filings use Shift-JIS encoding. This function tries
/// UTF-8 first, then falls back to SHIFT_JIS → UTF-8 via `encoding_rs`.
pub fn read_file_content(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    // Try UTF-8 first (the common case for modern filings).
    match std::str::from_utf8(&bytes) {
        Ok(s) => Some(s.to_string()),
        Err(_) => {
            // Fall back to Shift-JIS (Windows-31J / CP932) for legacy filings.
            let (cow, _encoding, _had_errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
            Some(cow.into_owned())
        }
    }
}

/// Check if file content looks like parseable iXBRL.
pub fn is_valid_xbrl_text(content: &str) -> bool {
    let has_fiscal_end = content.contains("CurrentFiscalYearEndDateDEI");
    has_inline_xbrl_fact(content) && has_fiscal_end
}

/// Check if file content contains inline XBRL fact elements.
pub fn has_inline_xbrl_fact(content: &str) -> bool {
    let normalized = content.to_ascii_lowercase();
    normalized.contains("<ix:nonfraction") || normalized.contains("<ix:nonnumeric")
}

/// Collect fact document paths from an artifact directory.
pub fn iter_fact_document_paths(root: &Path) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for ext in FACT_FILE_EXTENSIONS {
        if let Ok(entries) = glob_entries(root, ext) {
            for path in entries {
                if path.is_file() && seen.insert(path.clone()) {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths
}

fn glob_entries(root: &Path, ext: &str) -> Result<Vec<std::path::PathBuf>, ()> {
    let mut result = Vec::new();
    glob_recursive(root, ext, &mut result);
    Ok(result)
}

fn glob_recursive(dir: &Path, ext: &str, result: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                glob_recursive(&path, ext, result);
            } else if path.extension().is_some_and(|e| e == ext) {
                result.push(path);
            }
        }
    }
}

/// Determine if a context should be used based on the given mode.
pub fn should_use_context(ctx: &ContextInfo, mode: crate::types::ContextMode) -> bool {
    match mode {
        crate::types::ContextMode::Instant => {
            ctx.is_instant
                && ctx.period.is_some()
                && !ctx.is_non_consolidated
                && !ctx.has_dimensions
        }
        crate::types::ContextMode::Financial => {
            ctx.period.is_some() && !ctx.is_non_consolidated && !ctx.has_dimensions
        }
        crate::types::ContextMode::NonConsolidatedInstant => {
            ctx.is_instant
                && ctx.period.is_some()
                && ctx.is_non_consolidated
                && ctx.dimension_count == 1
        }
        crate::types::ContextMode::NonConsolidatedFinancial => {
            ctx.period.is_some() && ctx.is_non_consolidated && ctx.dimension_count == 1
        }
        crate::types::ContextMode::NonConsolidatedDividend => {
            ctx.period.is_some() && ctx.is_non_consolidated && ctx.dimension_count == 1
        }
    }
}

/// Check if a tag name is an inventory total tag (allocation-free).
pub fn is_inventory_total(name: &str) -> bool {
    INVENTORY_TOTAL_TAGS.contains(&name)
}

/// Check if a tag name is an inventory component tag (allocation-free).
pub fn is_inventory_component(name: &str) -> bool {
    INVENTORY_COMPONENT_TAGS.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xbrl_value_basic() {
        assert_eq!(parse_xbrl_value("1000", "", "", ""), Some(1000.0));
    }

    #[test]
    fn test_parse_xbrl_value_negative() {
        assert_eq!(parse_xbrl_value("500", "", "", "negative"), Some(-500.0));
    }

    #[test]
    fn test_parse_xbrl_value_inline_minus_sign() {
        assert_eq!(parse_xbrl_value("500", "", "", "-"), Some(-500.0));
    }

    #[test]
    fn test_parse_xbrl_value_scale() {
        assert_eq!(parse_xbrl_value("500", "", "3", ""), Some(500_000.0));
    }

    #[test]
    fn test_parse_xbrl_value_negative_scale() {
        assert_eq!(
            parse_xbrl_value("500", "-3", "3", "negative"),
            Some(-500_000.0)
        );
    }

    #[test]
    fn test_parse_xbrl_value_empty() {
        assert_eq!(parse_xbrl_value("", "", "", ""), None);
    }

    #[test]
    fn test_parse_xbrl_value_dash() {
        assert_eq!(parse_xbrl_value("−", "", "", ""), None);
    }

    #[test]
    fn test_parse_xbrl_value_triangle() {
        assert_eq!(parse_xbrl_value("△500", "", "", ""), Some(-500.0));
    }

    #[test]
    fn test_parse_xbrl_value_filled_triangle() {
        assert_eq!(parse_xbrl_value("▲1,234", "", "", ""), Some(-1234.0));
    }

    #[test]
    fn test_parse_xbrl_value_fullwidth_comma() {
        assert_eq!(parse_xbrl_value("1\u{FF0C}234", "", "", ""), Some(1234.0));
    }

    #[test]
    fn test_parse_xbrl_value_parenthesized_negative() {
        assert_eq!(parse_xbrl_value("(1,234)", "", "", ""), Some(-1234.0));
    }

    #[test]
    fn test_parse_xbrl_value_filled_triangle_with_fullwidth_comma() {
        assert_eq!(parse_xbrl_value("▲1\u{FF0C}234", "", "", ""), Some(-1234.0));
    }

    #[test]
    fn test_parse_xbrl_value_em_dash_is_none() {
        assert_eq!(parse_xbrl_value("—", "", "", ""), None);
    }

    #[test]
    fn test_parse_xbrl_value_fullwidth_minus_is_none() {
        assert_eq!(parse_xbrl_value("\u{FF0D}", "", "", ""), None);
    }

    #[test]
    fn test_parse_xbrl_value_yen_sen() {
        assert_eq!(parse_xbrl_value("20円0銭", "", "", ""), Some(20.0));
    }

    #[test]
    fn test_local_name() {
        assert_eq!(local_name("{http://example.com}Foo"), "Foo");
        assert_eq!(local_name("prefix:Bar"), "Bar");
        assert_eq!(local_name("Baz"), "Baz");
    }

    #[test]
    fn test_namespace_uri() {
        assert_eq!(
            namespace_uri("{http://example.com}Foo"),
            "http://example.com"
        );
        assert_eq!(namespace_uri("prefix:Bar"), "");
    }

    #[test]
    fn test_resolve_qname_braced() {
        let nsmap = HashMap::new();
        let result = resolve_qname("{http://example.com}Foo", &nsmap, "");
        assert_eq!(
            result,
            Some(("http://example.com".to_string(), "Foo".to_string()))
        );
    }

    #[test]
    fn test_resolve_qname_prefixed() {
        let mut nsmap = HashMap::new();
        nsmap.insert("ext".to_string(), "http://example.com".to_string());
        let result = resolve_qname("ext:Foo", &nsmap, "");
        assert_eq!(
            result,
            Some(("http://example.com".to_string(), "Foo".to_string()))
        );
    }
}
