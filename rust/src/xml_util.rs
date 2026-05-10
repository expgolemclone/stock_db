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
    XHTML_NS,
    IX_NS,
    XBRLI_NS,
    XBRLDI_NS,
    XLINK_NS,
    LINK_NS,
    XS_NS,
    XML_NS,
];

/// Patterns for fact document files.
pub const FACT_FILE_EXTENSIONS: &[&str] = &["xhtml", "html", "htm", "xbrl"];

/// Inventory total tag names.
pub const INVENTORY_TOTAL_TAGS: &[&str] = &[
    "Inventories",
    "InventoriesCA",
    "InventoriesCAIFRS",
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
    if short_name.starts_with("ConstructionInProgress") {
        return short_name != "ConstructionInProgressCAIFRS";
    }
    IGNORED_INVENTORY_SUBSTRINGS
        .iter()
        .any(|frag| short_name.contains(frag))
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

/// Parse an XBRL numeric value from raw text, applying scale and sign.
pub fn parse_xbrl_value(raw: &str, _decimals: &str, scale: &str, sign: &str) -> Option<f64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "−" || trimmed == "—" || trimmed == "－" {
        return None;
    }
    let clean = trimmed.replace(',', "").replace('△', "");
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
    if sign == "negative" || trimmed.starts_with('△') {
        result = -result;
    }
    Some(result)
}

/// Read file content as UTF-8 string.
pub fn read_file_content(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Check if file content looks like parseable iXBRL.
pub fn is_valid_xbrl_text(content: &str) -> bool {
    let has_nonfraction = content.contains("<ix:nonfraction")
        || content.contains("<ix:nonFraction")
        || content.contains("<IX:nonfraction")
        || content.contains("<IX:NONFRACTION");
    let has_fiscal_end = content.contains("CurrentFiscalYearEndDateDEI");
    has_nonfraction && has_fiscal_end
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
            ctx.period.is_some()
                && !ctx.is_non_consolidated
                && !ctx.has_dimensions
        }
        crate::types::ContextMode::NonConsolidatedInstant => {
            ctx.is_instant
                && ctx.period.is_some()
                && ctx.is_non_consolidated
                && ctx.dimension_count == 1
        }
        crate::types::ContextMode::NonConsolidatedFinancial => {
            ctx.period.is_some()
                && ctx.is_non_consolidated
                && ctx.dimension_count == 1
        }
        crate::types::ContextMode::NonConsolidatedDividend => {
            ctx.period.is_some()
                && ctx.is_non_consolidated
                && ctx.dimension_count == 1
        }
    }
}

/// Build a quick lookup set from a slice of tag names.
pub fn inventory_total_set() -> std::collections::HashSet<&'static str> {
    INVENTORY_TOTAL_TAGS.iter().copied().collect()
}

/// Build a quick lookup set from the component tag names.
pub fn inventory_component_set() -> std::collections::HashSet<&'static str> {
    INVENTORY_COMPONENT_TAGS.iter().copied().collect()
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
