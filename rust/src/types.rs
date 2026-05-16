use std::collections::HashMap;

/// (namespace_uri, local_name) — mirrors the Python ConceptKey tuple.
pub type ConceptKey = (String, String);

/// Explicit dimension member attached to an XBRL context.
#[derive(Debug, Clone)]
pub struct ExplicitMember {
    pub dimension: ConceptKey,
    pub member: ConceptKey,
}

/// Parsed context information extracted from XBRL documents.
#[derive(Debug, Clone)]
pub struct ContextInfo {
    pub period: Option<String>,
    pub _instant: Option<String>,
    pub is_instant: bool,
    pub has_dimensions: bool,
    pub is_non_consolidated: bool,
    pub dimension_count: usize,
    pub explicit_members: Vec<ExplicitMember>,
}

/// Kind of XBRL unit (affects which fact bucket a value is stored in).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnitKind {
    JPY,
    Shares,
    Other,
}

/// Parsed unit information — tracks what kind of unit each ID represents.
pub type UnitMap = HashMap<String, UnitKind>;

/// One extracted share class count.
#[derive(Debug, Clone)]
pub struct ShareClassFact {
    pub period: String,
    pub class_key: String,
    pub class_name: String,
    pub shares: f64,
    pub is_preferred: bool,
    pub source_kind: String,
    pub fact_priority: u8,
}

/// Loaded XBRL artifact containing all extracted fact buckets.
#[derive(Debug, Clone)]
pub struct LoadedXbrlArtifact {
    pub path: String,
    pub is_dir: bool,
    /// period → (ConceptKey → Option<f64>)
    pub financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// period → (ConceptKey → Option<f64>)
    pub inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// period → (ConceptKey → Option<f64>)
    pub non_consolidated_financial_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// period → (ConceptKey → Option<f64>)
    pub non_consolidated_inventory_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// period → (ConceptKey → Option<f64>)
    pub non_consolidated_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// period → (ConceptKey → Option<f64>) — shares-denominated facts (unit=shares).
    pub shares_facts: HashMap<String, HashMap<ConceptKey, Option<f64>>>,
    /// Share-class issued share counts extracted from dimensioned or class-specific facts.
    pub share_class_facts: Vec<ShareClassFact>,
}

/// An edge in a calculation linkbase graph.
#[derive(Debug, Clone)]
pub struct CalculationEdge {
    pub child: ConceptKey,
    pub weight: f64,
}

/// Context selection mode for fact extraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextMode {
    /// Instant context with no dimensions — used for primary inventory balance sheet.
    Instant,
    /// Any period context with no dimensions — used for primary financial facts.
    Financial,
    /// Instant context with only NonConsolidatedMember — used when no primary facts exist.
    NonConsolidatedInstant,
    /// Any period context with only NonConsolidatedMember — used when no primary facts exist.
    NonConsolidatedFinancial,
    /// Non-consolidated with exactly 1 dimension — used for dividend fallback.
    NonConsolidatedDividend,
}
