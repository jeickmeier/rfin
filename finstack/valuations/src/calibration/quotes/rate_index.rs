//! Rate index conventions for calibration.
//!
//! This module provides index-driven conventions resolution for rate indices,
//! allowing swaps to infer OIS-vs-term behavior from the referenced floating
//! index identifier (Bloomberg/FinCad style).

use crate::instruments::irs::FloatingLegCompounding;
use finstack_core::dates::{BusinessDayConvention, DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
use finstack_core::Result;
use std::collections::HashMap;
use std::sync::OnceLock;

use super::json_registry::{build_lookup_map_mapped, RegistryFile};

/// Type of rate index for convention determination.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub(crate) enum RateIndexKind {
    /// Overnight Risk-Free Rate index (e.g., SOFR, SONIA, ESTR).
    OvernightRfr,
    /// Term index with a fixed period (e.g., 3M LIBOR, 6M EURIBOR).
    Term,
    /// Unknown or generic index.
    Unknown,
}

/// This structure captures the necessary details for pricing instruments
/// tied to this index, such as payment frequency, resets, and compounding.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RateIndexConventions {
    /// Operating currency of the index.
    pub currency: Currency,
    /// Index category (Overnight vs Term).
    pub kind: RateIndexKind,
    /// Index tenor (None for overnight indices).
    pub tenor: Option<Tenor>,
    /// Market standard day count convention.
    pub day_count: DayCount,
    /// Typical payment frequency for swaps referencing this index.
    pub default_payment_frequency: Tenor,
    /// Business days between accrual end and payment.
    pub default_payment_delay_days: i32,
    /// Business days between fixing and accrual start.
    pub default_reset_lag_days: i32,
    /// Methodology for compounding overnight rates (OIS only).
    pub ois_compounding: Option<FloatingLegCompounding>,

    // =========================================================================
    // Swap market defaults (combined from prior currency-market conventions)
    // =========================================================================

    /// Market-standard calendar identifier for swaps referencing this index.
    pub market_calendar_id: String,
    /// Market-standard spot settlement lag (business days) for swaps referencing this index.
    pub market_settlement_days: i32,
    /// Market-standard business day convention for scheduling swaps referencing this index.
    pub market_business_day_convention: BusinessDayConvention,
    /// Market-standard fixed leg day count for swaps referencing this index.
    pub default_fixed_leg_day_count: DayCount,
    /// Market-standard fixed leg frequency for swaps referencing this index.
    pub default_fixed_leg_frequency: Tenor,
}

impl RateIndexConventions {
    /// Try to resolve conventions for an index identifier from the embedded registry.
    pub(crate) fn try_for_index(index: &IndexId) -> Option<&'static Self> {
        registry_conventions(index.as_str())
    }

    /// Resolve conventions for an index identifier from the embedded registry.
    ///
    /// This is intentionally **strict**: if the index is not present in
    /// `rate_index_conventions.json`, an error is returned.
    pub(crate) fn require_for_index(index: &IndexId) -> Result<&'static Self> {
        Self::try_for_index(index).ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "Missing rate index conventions for '{}'. Add it to finstack/valuations/data/conventions/rate_index_conventions.json",
                index
            ))
        })
    }

    /// Returns true if the index string is clearly an overnight RFR (OIS-suitable).
    ///
    /// This is intentionally heuristic and designed for market-style identifiers like
    /// `USD-SOFR-OIS` and `GBP-SONIA-OIS`.
    /// Returns true if the index identifier is a recognized overnight RFR in the registry.
    pub(crate) fn is_overnight_rfr_index(index: &IndexId) -> bool {
        Self::try_for_index(index).is_some_and(|c| c.kind == RateIndexKind::OvernightRfr)
    }
}

// ============================================================================
// JSON registry (explicit conventions only)
// ============================================================================

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct RateIndexConventionsRecord {
    currency: Currency,
              kind: RateIndexKind,
    #[serde(default)]
    tenor: Option<String>,
              day_count: DayCount,
    default_payment_frequency: String,
              default_payment_delay_days: i32,
              default_reset_lag_days: i32,
    #[serde(default)]
    ois_compounding: Option<OisCompoundingSpec>,
    market_calendar_id: String,
    market_settlement_days: i32,
    market_business_day_convention: BusinessDayConvention,
    default_fixed_leg_day_count: DayCount,
    default_fixed_leg_frequency: String,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
enum OisCompoundingSpec {
    Sofr,
    Sonia,
    Estr,
    Tona,
    Fedfunds,
    Saron,
    CompoundedInArrears {
        lookback_days: i32,
        observation_shift: Option<i32>,
    },
}

impl OisCompoundingSpec {
    fn to_compounding(&self) -> FloatingLegCompounding {
        match self {
            Self::Sofr => FloatingLegCompounding::sofr(),
            Self::Sonia => FloatingLegCompounding::sonia(),
            Self::Estr => FloatingLegCompounding::estr(),
            Self::Tona => FloatingLegCompounding::tona(),
            Self::Fedfunds => FloatingLegCompounding::fedfunds(),
            Self::Saron => FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 2,
                observation_shift: None,
            },
            Self::CompoundedInArrears {
                lookback_days,
                observation_shift,
            } => FloatingLegCompounding::CompoundedInArrears {
                lookback_days: *lookback_days,
                observation_shift: *observation_shift,
            },
        }
    }
}

impl RateIndexConventionsRecord {
    fn into_conventions(self) -> RateIndexConventions {
        let tenor = self.tenor.map(|s| {
            Tenor::parse(&s).unwrap_or_else(|e| {
                panic!("Invalid `tenor` in rate index conventions registry: '{}': {}", s, e)
            })
        });

        let default_payment_frequency =
            Tenor::parse(&self.default_payment_frequency).unwrap_or_else(|e| {
                panic!(
                    "Invalid `default_payment_frequency` in rate index conventions registry: '{}': {}",
                    self.default_payment_frequency, e
                )
            });

        let default_fixed_leg_frequency =
            Tenor::parse(&self.default_fixed_leg_frequency).unwrap_or_else(|e| {
                panic!(
                    "Invalid `default_fixed_leg_frequency` in rate index conventions registry: '{}': {}",
                    self.default_fixed_leg_frequency, e
                )
            });

        let ois_compounding = self.ois_compounding.as_ref().map(|s| s.to_compounding());

        // Basic invariants: avoid silently encoding impossible combinations.
        match self.kind {
            RateIndexKind::OvernightRfr => {
                if tenor.is_some() {
                    panic!("Overnight RFR index conventions must not specify a tenor");
                }
                if ois_compounding.is_none() {
                    panic!("Overnight RFR index conventions must specify `ois_compounding`");
                }
            }
            RateIndexKind::Term | RateIndexKind::Unknown => {
                if ois_compounding.is_some() {
                    panic!("Non-overnight index conventions must not specify `ois_compounding`");
                }
            }
        }

        RateIndexConventions {
            currency: self.currency,
            kind: self.kind,
            tenor,
            day_count: self.day_count,
            default_payment_frequency,
            default_payment_delay_days: self.default_payment_delay_days,
            default_reset_lag_days: self.default_reset_lag_days,
            ois_compounding,
            market_calendar_id: self.market_calendar_id,
            market_settlement_days: self.market_settlement_days,
            market_business_day_convention: self.market_business_day_convention,
            default_fixed_leg_day_count: self.default_fixed_leg_day_count,
            default_fixed_leg_frequency,
        }
    }
}

fn normalize_registry_id(id: &str) -> String {
    // Keep this in sync with `tokenize_index()` normalization:
    // - normalize "€STR" spelling into ASCII
    // - uppercase for case-insensitive matching
    // - collapse separators so "USD_SOFR_OIS" and "USD-SOFR-OIS" match
    let normalized = id.replace('€', "E").to_uppercase();
    normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn rate_index_conventions_registry() -> &'static HashMap<String, RateIndexConventions> {
    static REGISTRY: OnceLock<HashMap<String, RateIndexConventions>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let json = include_str!("../../../data/conventions/rate_index_conventions.json");
        let file: RegistryFile<RateIndexConventionsRecord> = serde_json::from_str(json)
            .expect("Failed to parse embedded rate index conventions registry JSON");
        build_lookup_map_mapped(file, normalize_registry_id, |rec| rec.clone().into_conventions())
    })
}

/// Registry of explicit per-index conventions loaded from embedded JSON.
fn registry_conventions(index: &str) -> Option<&'static RateIndexConventions> {
    let key = normalize_registry_id(index);
    rate_index_conventions_registry().get(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_term_index_tenor() {
        let c = RateIndexConventions::require_for_index(&IndexId::new("USD-SOFR-3M"))
            .expect("registry");
        assert_eq!(c.kind, RateIndexKind::Term);
        assert_eq!(c.tenor, Some(Tenor::parse("3M").expect("tenor")));
        assert_eq!(
            c.default_payment_frequency,
            Tenor::parse("3M").expect("tenor")
        );
    }

    #[test]
    fn treats_ois_index_as_overnight_rfr_defaults() {
        let c = RateIndexConventions::require_for_index(&IndexId::new("USD-SOFR-OIS"))
            .expect("registry");
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert!(matches!(
            c.ois_compounding,
            Some(FloatingLegCompounding::CompoundedInArrears { .. })
        ));
    }

    #[test]
    fn treats_fedfunds_ois_index_as_no_lookback() {
        let c = RateIndexConventions::require_for_index(&IndexId::new("USD-FEDFUNDS-OIS"))
            .expect("registry");
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }

    #[test]
    fn treats_generic_usd_ois_as_no_lookback_by_default() {
        let c = RateIndexConventions::require_for_index(&IndexId::new("USD-OIS"))
            .expect("registry");
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }

    #[test]
    fn treats_usd_sofr_index_as_no_lookback_by_default() {
        let c = RateIndexConventions::require_for_index(&IndexId::new("USD-SOFR"))
            .expect("registry");
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }
}
