//! Rate index conventions for calibration.
//!
//! This module provides index-driven conventions resolution for rate indices,
//! allowing swaps to infer OIS-vs-term behavior from the referenced floating
//! index identifier (Bloomberg/FinCad style).

use crate::instruments::irs::FloatingLegCompounding;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::types::{Currency, IndexId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RateIndexKind {
    OvernightRfr,
    Term,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RateIndexConventions {
    pub currency: Currency,
    pub kind: RateIndexKind,
    pub tenor: Option<Tenor>,
    pub day_count: DayCount,
    pub default_payment_frequency: Tenor,
    pub default_payment_delay_days: i32,
    pub default_reset_lag_days: i32,
    pub ois_compounding: Option<FloatingLegCompounding>,
}

impl RateIndexConventions {
    pub(crate) fn for_index_with_currency(index: &IndexId, currency_hint: Currency) -> Self {
        // Registry-first resolution: prefer explicit per-index conventions for market accuracy.
        if let Some(c) = registry_conventions(index.as_str(), currency_hint) {
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
    pub(crate) fn is_overnight_rfr_index(index: &IndexId) -> bool {
        if let Some(c) = registry_conventions(index.as_str(), Currency::USD) {
            return c.kind == RateIndexKind::OvernightRfr;
        }
        let tokens = tokenize_index(index.as_str());
        let tenor = tokens.iter().find_map(|t| Tenor::parse(t).ok());
        is_overnight_rfr_tokens(&tokens) && tenor.is_none()
    }
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

/// Registry of explicit per-index conventions.
///
/// This is intentionally small and conservative: prefer being explicit for the
/// common production indices and falling back to heuristic parsing for anything
/// unknown. This avoids silently mis-classifying indices when tokens collide.
fn registry_conventions(index: &str, currency_hint: Currency) -> Option<RateIndexConventions> {
    // Normalize into ASCII-ish uppercase tokens (match tokenize_index behavior).
    let normalized = index.replace('€', "E").to_uppercase();

    // Helper: construct without allocating tokens.
    let mk = |currency: Currency,
              kind: RateIndexKind,
              tenor: Option<Tenor>,
              day_count: DayCount,
              default_payment_frequency: Tenor,
              default_payment_delay_days: i32,
              default_reset_lag_days: i32,
              ois_compounding: Option<FloatingLegCompounding>| {
        RateIndexConventions {
            currency,
            kind,
            tenor,
            day_count,
            default_payment_frequency,
            default_payment_delay_days,
            default_reset_lag_days,
            ois_compounding,
        }
    };

    // Overnight indices (explicit, registry-first).
    //
    // Notes:
    // - Payment delay conventions can vary by venue/clearer. We use the same defaults as the
    //   previous heuristic path unless we have strong, explicit conventions (USD).
    match normalized.as_str() {
        // Explicit SOFR OIS identifiers.
        "USD-SOFR-OIS" | "SOFR-OIS" => Some(mk(
            Currency::USD,
            RateIndexKind::OvernightRfr,
            None,
            DayCount::Act360,
            Tenor::annual(),
            2,
            0,
            Some(FloatingLegCompounding::sofr()),
        )),
        // Explicit overnight SOFR index (not the OIS product identifier).
        //
        // Default to the *no-lookback* variant for stability/bootstrapping. Callers who want
        // SOFR lookback conventions should use "USD-SOFR-OIS".
        "USD-SOFR" | "SOFR" => Some(mk(
            Currency::USD,
            RateIndexKind::OvernightRfr,
            None,
            DayCount::Act360,
            Tenor::annual(),
            2,
            0,
            Some(FloatingLegCompounding::fedfunds()),
        )),
        // Generic USD "OIS" is ambiguous (historically Fed Funds vs SOFR).
        // Default to the *no-lookback* variant for stability/bootstrapping; callers
        // who want SOFR lookback conventions should use "USD-SOFR-OIS".
        "USD-OIS" => Some(mk(
            Currency::USD,
            RateIndexKind::OvernightRfr,
            None,
            DayCount::Act360,
            Tenor::annual(),
            2,
            0,
            Some(FloatingLegCompounding::fedfunds()),
        )),
        "USD-FEDFUNDS-OIS" | "USD-FEDFUNDS" | "USD-FF-OIS" | "USD-EFFR-OIS" | "USD-EFFR" => Some(
            mk(
                Currency::USD,
                RateIndexKind::OvernightRfr,
                None,
                DayCount::Act360,
                Tenor::annual(),
                2,
                0,
                Some(FloatingLegCompounding::fedfunds()),
            ),
        ),
        // GBP SONIA (Act/365F, commonly 5-day lookback convention)
        "GBP-SONIA-OIS" | "GBP-SONIA" | "SONIA-OIS" => Some(mk(
            Currency::GBP,
            RateIndexKind::OvernightRfr,
            None,
            DayCount::Act365F,
            Tenor::annual(),
            2,
            0,
            Some(FloatingLegCompounding::sonia()),
        )),
        // EUR €STR (Act/360, commonly treated as 2-day lag/shift convention)
        "EUR-ESTR-OIS" | "EUR-ESTR" | "ESTR-OIS" | "EUR-EST-OIS" | "EUR-EST" | "EST-OIS" => Some(
            mk(
                Currency::EUR,
                RateIndexKind::OvernightRfr,
                None,
                DayCount::Act360,
                Tenor::annual(),
                2,
                0,
                Some(FloatingLegCompounding::estr()),
            ),
        ),
        // JPY TONA (Act/365F, commonly 2-day lag convention)
        "JPY-TONA-OIS" | "JPY-TONA" | "TONA-OIS" | "JPY-TONAR-OIS" | "JPY-TONAR" | "TONAR-OIS" => {
            Some(mk(
                Currency::JPY,
                RateIndexKind::OvernightRfr,
                None,
                DayCount::Act365F,
                Tenor::annual(),
                2,
                0,
                Some(FloatingLegCompounding::tona()),
            ))
        }
        // CHF SARON (Act/360 per common money-market style, 2-day lag default)
        "CHF-SARON-OIS" | "CHF-SARON" | "SARON-OIS" => Some(mk(
            Currency::CHF,
            RateIndexKind::OvernightRfr,
            None,
            DayCount::Act360,
            Tenor::annual(),
            2,
            0,
            Some(FloatingLegCompounding::CompoundedInArrears {
                lookback_days: 2,
                observation_shift: None,
            }),
        )),
        _ => {
            // If we can't parse a currency token at all, do not claim registry match.
            // (This prevents accidentally treating "OIS" in some unrelated name as USD OIS.)
            let _ = currency_hint;
            None
        }
    }
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
        let c = RateIndexConventions::for_index_with_currency(&IndexId::new("USD-OIS"), Currency::USD);
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }

    #[test]
    fn treats_usd_sofr_index_as_no_lookback_by_default() {
        let c = RateIndexConventions::for_index_with_currency(&IndexId::new("USD-SOFR"), Currency::USD);
        assert_eq!(c.kind, RateIndexKind::OvernightRfr);
        assert_eq!(c.default_payment_frequency, Tenor::annual());
        assert_eq!(c.default_payment_delay_days, 2);
        assert_eq!(c.default_reset_lag_days, 0);
        assert_eq!(c.ois_compounding, Some(FloatingLegCompounding::fedfunds()));
    }
}
