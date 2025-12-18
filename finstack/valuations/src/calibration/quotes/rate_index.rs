//! Rate index conventions for calibration.
//!
//! This module provides index-driven conventions resolution for rate indices,
//! allowing swaps to infer OIS-vs-term behavior from the referenced floating
//! index identifier (Bloomberg/FinCad style).

use crate::instruments::irs::FloatingLegCompounding;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};
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
}

impl RateIndexConventions {
    /// Resolve conventions for a given index identifier.
    ///
    /// Periodically updated registry and heuristic token parsing are used
    /// to determine if the index is a term index or an overnight RFR.
    pub(crate) fn for_index_with_currency(index: &IndexId, currency_hint: Currency) -> Self {
        // Registry-first resolution: prefer explicit per-index conventions for market accuracy.
        if let Some(c) = registry_conventions(index.as_str()) {
            return c;
        }

        // Fallback: heuristic token parsing for legacy/unregistered indices.
        let tokens = tokenize_index(index.as_str());
        let currency = tokens
            .first()
            .and_then(|t| parse_currency_token(t))
            .unwrap_or(currency_hint);

        // Identify tenor tokens (e.g. "3M", "6M", "1Y") when present.
        let tenor = tokens.iter().find_map(|t| Tenor::parse(t).ok());

        let is_overnight_rfr = is_overnight_rfr_tokens(&tokens) && tenor.is_none();

        let kind = if is_overnight_rfr {
            RateIndexKind::OvernightRfr
        } else if tenor.is_some() {
            RateIndexKind::Term
        } else {
            RateIndexKind::Unknown
        };

        let day_count = default_index_day_count(currency, &tokens, kind);

        let (default_payment_frequency, default_payment_delay_days, default_reset_lag_days) =
            match kind {
                RateIndexKind::OvernightRfr => (Tenor::annual(), 2, 0),
                _ => (default_term_payment_frequency(tenor), 0, -2),
            };

        let ois_compounding = match kind {
            RateIndexKind::OvernightRfr => Some(default_ois_compounding(currency, &tokens)),
            _ => None,
        };

        Self {
            currency,
            kind,
            tenor,
            day_count,
            default_payment_frequency,
            default_payment_delay_days,
            default_reset_lag_days,
            ois_compounding,
        }
    }

    /// Returns true if the index string is clearly an overnight RFR (OIS-suitable).
    ///
    /// This is intentionally heuristic and designed for market-style identifiers like
    /// `USD-SOFR-OIS` and `GBP-SONIA-OIS`.
    /// Returns true if the index identifier is a recognized overnight RFR.
    pub(crate) fn is_overnight_rfr_index(index: &IndexId) -> bool {
        if let Some(c) = registry_conventions(index.as_str()) {
            return c.kind == RateIndexKind::OvernightRfr;
        }
        let tokens = tokenize_index(index.as_str());
        let tenor = tokens.iter().find_map(|t| Tenor::parse(t).ok());
        is_overnight_rfr_tokens(&tokens) && tenor.is_none()
    }
}

// ============================================================================
// JSON registry (explicit conventions)
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

fn tokenize_index(index: &str) -> Vec<String> {
    // Normalize common "€STR" spelling into "ESTR" so tokenization can remain ASCII-oriented.
    // Also uppercase for case-insensitive matching.
    let normalized = index.replace('€', "E").to_uppercase();
    normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .collect()
}

/// Registry of explicit per-index conventions loaded from embedded JSON.
fn registry_conventions(index: &str) -> Option<RateIndexConventions> {
    let key = normalize_registry_id(index);
    rate_index_conventions_registry().get(&key).cloned()
}

fn parse_currency_token(token: &str) -> Option<Currency> {
    match token {
        "USD" => Some(Currency::USD),
        "EUR" => Some(Currency::EUR),
        "GBP" => Some(Currency::GBP),
        "JPY" => Some(Currency::JPY),
        "CHF" => Some(Currency::CHF),
        "CAD" => Some(Currency::CAD),
        "AUD" => Some(Currency::AUD),
        "NZD" => Some(Currency::NZD),
        "HKD" => Some(Currency::HKD),
        "SGD" => Some(Currency::SGD),
        _ => None,
    }
}

fn is_overnight_rfr_tokens(tokens: &[String]) -> bool {
    tokens.iter().any(|t| {
        matches!(
            t.as_str(),
            // USD
            "SOFR" | "EFFR" | "FEDFUNDS" | "FEDFUND" | "FF"
            // GBP
            | "SONIA"
            // EUR
            | "ESTR" | "EST" | "EONIA"
            // JPY
            | "TONA" | "TONAR"
            // CHF
            | "SARON"
            // CAD
            | "CORRA"
            // SGD / HKD / AUD (common tags)
            | "SORA" | "HONIA" | "AONIA" | "BBSWON"
            // Generic
            | "OIS"
        )
    })
}

fn default_term_payment_frequency(tenor: Option<Tenor>) -> Tenor {
    // If the index itself is a term index like 3M, align payment frequency with index tenor.
    // Otherwise, retain a conservative quarterly default for legacy/unknown indices.
    tenor.unwrap_or_else(Tenor::quarterly)
}

fn default_index_day_count(currency: Currency, tokens: &[String], kind: RateIndexKind) -> DayCount {
    // Keep these defaults simple and explicit. The long-term design should come from an
    // index registry (per-index conventions), but this centralizes current behavior.
    match kind {
        RateIndexKind::OvernightRfr => {
            if tokens.iter().any(|t| t == "SONIA") {
                return DayCount::Act365F;
            }
            if tokens.iter().any(|t| t == "TONA" || t == "TONAR") {
                return DayCount::Act365F;
            }
            // Currency-style fallbacks
            match currency {
                Currency::GBP
                | Currency::JPY
                | Currency::CAD
                | Currency::AUD
                | Currency::NZD
                | Currency::SGD
                | Currency::HKD => DayCount::Act365F,
                _ => DayCount::Act360,
            }
        }
        RateIndexKind::Term | RateIndexKind::Unknown => match currency {
            Currency::GBP | Currency::JPY | Currency::AUD => DayCount::Act365F,
            _ => DayCount::Act360,
        },
    }
}

fn default_ois_compounding(currency: Currency, tokens: &[String]) -> FloatingLegCompounding {
    if tokens.iter().any(|t| t == "SONIA") {
        return FloatingLegCompounding::sonia();
    }
    if tokens.iter().any(|t| t == "ESTR" || t == "EST") {
        return FloatingLegCompounding::estr();
    }
    if tokens.iter().any(|t| t == "TONA" || t == "TONAR") {
        return FloatingLegCompounding::tona();
    }
    // USD overnight indices are not all the same:
    // - SOFR OIS uses a 2-business-day lookback (ARRC)
    // - Fed Funds / EFFR-style OIS typically uses no lookback
    if tokens.iter().any(|t| t == "SOFR") {
        return FloatingLegCompounding::sofr();
    }
    if tokens
        .iter()
        .any(|t| t == "EFFR" || t == "FEDFUNDS" || t == "FEDFUND" || t == "FF")
    {
        return FloatingLegCompounding::fedfunds();
    }

    // Currency fallback for generic ids like "USD-OIS".
    match currency {
        Currency::GBP => FloatingLegCompounding::sonia(),
        Currency::EUR => FloatingLegCompounding::estr(),
        Currency::JPY => FloatingLegCompounding::tona(),
        // For USD, avoid assuming SOFR lookback unless it is explicitly tagged.
        Currency::USD => FloatingLegCompounding::fedfunds(),
        _ => FloatingLegCompounding::sofr(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_term_index_tenor() {
        let c = RateIndexConventions::for_index_with_currency(
            &IndexId::new("USD-SOFR-3M"),
            Currency::USD,
        );
        assert_eq!(c.kind, RateIndexKind::Term);
        assert_eq!(c.tenor, Some(Tenor::parse("3M").expect("tenor")));
        assert_eq!(
            c.default_payment_frequency,
            Tenor::parse("3M").expect("tenor")
        );
    }

    #[test]
    fn treats_ois_index_as_overnight_rfr_defaults() {
        let c = RateIndexConventions::for_index_with_currency(
            &IndexId::new("USD-SOFR-OIS"),
            Currency::USD,
        );
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
        let c = RateIndexConventions::for_index_with_currency(
            &IndexId::new("USD-FEDFUNDS-OIS"),
            Currency::USD,
        );
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }

    #[test]
    fn treats_generic_usd_ois_as_no_lookback_by_default() {
        let c =
            RateIndexConventions::for_index_with_currency(&IndexId::new("USD-OIS"), Currency::USD);
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }

    #[test]
    fn treats_usd_sofr_index_as_no_lookback_by_default() {
        let c =
            RateIndexConventions::for_index_with_currency(&IndexId::new("USD-SOFR"), Currency::USD);
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }
}
