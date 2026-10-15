mod artifact;
pub mod config;
pub mod db;
pub mod downstream;
pub mod edinet;
mod financials;
mod inventory;
mod linkbase;
pub mod paths;
mod potential_terms;
mod preferred_terms;
pub mod price_lookup;
pub mod schema;
pub mod screening;
mod share_classes;
pub mod shikiho;
#[path = "taxonomy_registry.rs"]
mod taxonomy;
#[cfg(test)]
mod taxonomy_20260617_accounting_policy_tests;
#[cfg(test)]
mod taxonomy_accounting_scope_guard_tests;
#[cfg(test)]
mod taxonomy_accounting_guard_active_tests;
#[cfg(test)]
mod taxonomy_accounting_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_accrued_expenses_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_accumulated_depreciation_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_affiliate_payables_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_amortization_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_bulk_current_liability_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_bulk_same_meaning_reviewed_aliases_20260617_tests;
#[cfg(test)]
mod taxonomy_context_accounting_guard_tests;
#[cfg(test)]
mod taxonomy_contract_assets_tests;
#[cfg(test)]
mod taxonomy_contract_liabilities_accounting_scope_tests;
#[cfg(test)]
mod taxonomy_contract_receivables_payables_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_current_payables_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_edinet_jgaap_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_factchecked_receivable_payable_100_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_factchecked_working_capital_bulk_aliases_tests;
#[cfg(test)]
mod taxonomy_factchecked_working_capital_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_high_frequency_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_industry_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_jppfs_amortization_cash_flow_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_jppfs_high_frequency_batch2_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_jppfs_high_frequency_batch3_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_jppfs_statement_role_suffix_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_non_trade_receivables_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_paid_in_capital_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_reviewed_alias_chain_tests;
#[cfg(test)]
mod taxonomy_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_same_meaning_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_us_gaap_cash_flow_reviewed_aliases_tests;
#[cfg(test)]
mod taxonomy_working_capital_and_accumulated_assets_reviewed_aliases_tests;
mod types;
mod xml_util;

#[derive(Debug, Clone)]
pub struct InventoriesTagMismatchError {
    pub message: String,
}

impl InventoriesTagMismatchError {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for InventoriesTagMismatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for InventoriesTagMismatchError {}

