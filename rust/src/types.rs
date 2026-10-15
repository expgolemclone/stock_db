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
    /// Canonical period key derived from the context end/instant date (YYYY-MM).
    pub period: Option<String>,
    /// Duration-context start date. `None` for instant facts.
    pub period_start: Option<String>,
    /// Context end date or instant date, preserving the full YYYY-MM-DD value.
    pub period_end: Option<String>,
    pub _instant: Option<String>,
    pub is_instant: bool,
    pub has_dimensions: bool,
    pub is_non_consolidated: bool,
    pub dimension_count: usize,
    pub explicit_members: Vec<ExplicitMember>,
}

/// Kind of XBRL unit (affects which fact bucket a value is stored in).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitKind {
    JPY,
    Shares,
    /// Monetary amount per ordinary share, represented by an XBRL divide unit.
    JPYPerShare,
    Other,
}

impl UnitKind {
    pub fn as_str(self) -> &'static str {
        match self {
            UnitKind::JPY => "JPY",
            UnitKind::Shares => "shares",
            UnitKind::JPYPerShare => "JPY/share",
            UnitKind::Other => "other",
        }
    }
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

/// One text fact retained for preferred-share term extraction.
#[derive(Debug, Clone)]
pub struct TextFact {
    pub period: String,
    pub source_concept: String,
    pub text: String,
    pub tables: Vec<TextTable>,
}

/// A simple row/column view of tables embedded in an XBRL text block.
#[derive(Debug, Clone)]
pub struct TextTable {
    pub rows: Vec<Vec<String>>,
}

/// One raw numeric fact that belongs to a financial statement presentation role.
#[derive(Debug, Clone)]
pub struct RawStatementFact {
    pub statement: String,
    pub period: String,
    pub period_start: Option<String>,
    pub period_end: String,
    pub duration_days: Option<i64>,
    pub consolidation_scope: String,
    pub context_id: String,
    pub dimensions_json: String,
    pub concept: ConceptKey,
    pub role_uri: String,
    pub unit: String,
    pub unit_ref: String,
    /// Original XBRL `decimals` attribute. `INF` and empty values are preserved.
    pub decimals: String,
    pub value: Option<f64>,
}

/// One presentation linkbase edge for a financial statement role.
#[derive(Debug, Clone)]
pub struct StatementPresentationEdge {
    pub statement: String,
    pub role_uri: String,
    pub parent: ConceptKey,
    pub parent_label: String,
    pub child: ConceptKey,
    pub child_order: f64,
    pub child_label: String,
}

/// One extracted preferred-share term useful for dilution calculations.
#[derive(Debug, Clone)]
pub struct PreferredShareTermFact {
    pub period: String,
    pub class_key: String,
    pub class_name: String,
    pub terms_text: String,
    pub preferred_shares: Option<f64>,
    pub converted_common_shares: Option<f64>,
    pub conversion_ratio_to_common: Option<f64>,
    pub issue_price_yen: Option<f64>,
    pub capitalized_amount_yen: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
}

/// One period-end potential-equity instrument useful for dilution detection.
#[derive(Debug, Clone)]
pub struct PotentialEquityInstrumentFact {
    pub period: String,
    pub instrument_key: String,
    pub instrument_type: String,
    pub instrument_name: String,
    pub terms_text: String,
    pub period_end_common_shares: Option<f64>,
    pub exercise_price_yen: Option<f64>,
    pub conversion_price_yen: Option<f64>,
    pub face_value_yen: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
}

/// One statement about whether period-end potential equity exists.
#[derive(Debug, Clone)]
pub struct PotentialEquityStatusFact {
    pub period: String,
    pub status_key: String,
    pub has_potential_equity: bool,
    pub terms_text: String,
    pub source_concept: String,
    pub source_kind: String,
}

/// One diluted-EPS weighted average common-share adjustment.
#[derive(Debug, Clone)]
pub struct DilutedEpsAdjustmentFact {
    pub period: String,
    pub adjustment_key: String,
    pub adjustment_type: String,
    pub adjustment_name: String,
    pub terms_text: String,
    pub weighted_average_common_shares: Option<f64>,
    pub source_concept: String,
    pub source_kind: String,
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
    /// Raw balance sheet facts from statement presentation roles.
    pub raw_balance_sheet_facts: Vec<RawStatementFact>,
    /// Raw profit/loss facts from statement presentation roles.
    pub raw_profit_loss_facts: Vec<RawStatementFact>,
    /// Raw cash-flow facts from statement presentation roles.
    pub raw_cash_flow_facts: Vec<RawStatementFact>,
    /// Presentation hierarchy edges for statement roles.
    pub statement_presentation_edges: Vec<StatementPresentationEdge>,
    /// Share-class issued share counts extracted from dimensioned or class-specific facts.
    pub share_class_facts: Vec<ShareClassFact>,
    /// Text facts retained for preferred-share term extraction.
    pub text_facts: Vec<TextFact>,
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
    /// Instant context with only NonConsolidatedMember — retained for raw/audit use.
    NonConsolidatedInstant,
    /// Any period context with only NonConsolidatedMember — retained for raw/audit use.
    NonConsolidatedFinancial,
    /// Non-consolidated with exactly 1 dimension — retained for dividend evidence.
    NonConsolidatedDividend,
}
