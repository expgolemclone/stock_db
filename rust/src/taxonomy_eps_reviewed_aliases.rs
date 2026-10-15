use super::base;
use base::{CanonicalConcept, MatchKind, TaxonomyFamily};

use crate::types::ConceptKey;

fn reviewed_eps_field(family: TaxonomyFamily, local_name: &str) -> Option<&'static str> {
    match family {
        TaxonomyFamily::EdinetJppfs
        | TaxonomyFamily::EdinetJpigp
        | TaxonomyFamily::EdinetJpcrp
        | TaxonomyFamily::Ifrs => match local_name {
            "BasicEarningsLossPerShare"
            | "BasicEarningsLossPerShareIFRS"
            | "BasicEarningsLossPerShareIFRSSummaryOfBusinessResults"
            | "BasicEarningsLossPerShareKeyFinancialData"
            | "BasicEarningsLossPerShareSummaryOfBusinessResults"
            | "BasicEarningsLossPerShareUSGAAPSummaryOfBusinessResults"
            | "BasicEarningsPerShare"
            | "BasicEarningsPerShareIFRS"
            | "BasicEarningsPerShareSummaryOfBusinessResult"
            | "BasicEarningsPerShareSummaryOfBusinessResults" => Some("earnings_per_share"),
            "DilutedEarningsLossPerShare"
            | "DilutedEarningsLossPerShareIFRS"
            | "DilutedEarningsLossPerShareIFRSSummaryOfBusinessResults"
            | "DilutedEarningsLossPerShareUSGAAPSummaryOfBusinessResults"
            | "DilutedEarningsPerShare"
            | "DilutedEarningsPerShareIFRS"
            | "DilutedEarningsPerShareIFRSSummaryOfBusinessResults"
            | "DilutedEarningsPerShareSummaryOfBusinessResults" => {
                Some("diluted_earnings_per_share")
            }
            _ => None,
        },
        TaxonomyFamily::UsGaap => match local_name {
            "EarningsPerShareBasic" | "EarningsPerShareBasicAndDiluted" => {
                Some("earnings_per_share")
            }
            "EarningsPerShareDiluted" => Some("diluted_earnings_per_share"),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn reviewed_alias(concept: &ConceptKey) -> Option<CanonicalConcept> {
    let family = base::classify_namespace_uri(&concept.0)?;
    let field = reviewed_eps_field(family, &concept.1)?;
    Some(CanonicalConcept {
        item_name: field.to_string(),
        priority: if concept.1.contains("SummaryOfBusinessResult")
            || concept.1.contains("KeyFinancialData")
        {
            5
        } else {
            20
        },
        match_kind: MatchKind::TaxonomyFamilyAlias,
    })
}

pub fn statement_for_item_name(item_name: &str) -> Option<&'static str> {
    match item_name {
        "earnings_per_share" | "diluted_earnings_per_share" => Some("pl"),
        _ => base::statement_for_item_name(item_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_reported_eps_across_supported_standards_and_edinet_eras() {
        for (namespace, local_name, expected) in [
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jppfs/2016-02-29/jppfs_cor",
                "BasicEarningsPerShareSummaryOfBusinessResult",
                "earnings_per_share",
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2020-11-01/jpcrp_cor",
                "BasicEarningsLossPerShareKeyFinancialData",
                "earnings_per_share",
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2022-11-01/jpcrp_cor",
                "BasicEarningsLossPerShareUSGAAPSummaryOfBusinessResults",
                "earnings_per_share",
            ),
            (
                "http://disclosure.edinet-fsa.go.jp/taxonomy/jpigp/2025-11-01/jpigp_cor",
                "DilutedEarningsLossPerShareIFRSSummaryOfBusinessResults",
                "diluted_earnings_per_share",
            ),
            (
                "https://fasb.org/us-gaap/2025",
                "EarningsPerShareBasic",
                "earnings_per_share",
            ),
            (
                "https://fasb.org/us-gaap/2025",
                "EarningsPerShareDiluted",
                "diluted_earnings_per_share",
            ),
        ] {
            let mapped = reviewed_alias(&(namespace.to_string(), local_name.to_string())).unwrap();
            assert_eq!(mapped.item_name, expected);
        }
    }

    #[test]
    fn preferred_share_specific_eps_is_not_collapsed_into_common_eps() {
        let concept = (
            "http://disclosure.edinet-fsa.go.jp/taxonomy/jpcrp/2025-11-01/jpcrp_cor".to_string(),
            "ClassAPreferredStockBasicEarningsLossPerShareSummaryOfBusinessResults".to_string(),
        );
        assert!(reviewed_alias(&concept).is_none());
    }

    #[test]
    fn eps_is_profit_and_loss_data() {
        assert_eq!(statement_for_item_name("earnings_per_share"), Some("pl"));
        assert_eq!(
            statement_for_item_name("diluted_earnings_per_share"),
            Some("pl")
        );
    }
}
