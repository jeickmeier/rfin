//! Rate index registry for multi-curve framework classification.
//!
//! Provides accurate classification of rate indices (overnight vs term rates)
//! and OIS compounding conventions. Used by calibration and pricing logic to
//! determine curve routing and swap pricing methodology.

use crate::instruments::irs::FloatingLegCompounding;
use finstack_core::prelude::*;
use finstack_core::types::IndexId;

/// Rate index family classification for multi-curve framework.
///
/// Distinguishes overnight rates (used for discounting) from term rates
/// (used for floating leg projection in non-OIS swaps).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RateIndexFamily {
    /// Overnight rates: SOFR, SONIA, €STR, TONA, etc.
    /// Used for OIS curves and collateralized discounting.
    Overnight,
    /// Term rates: 3M LIBOR, 6M EURIBOR, Term SOFR, etc.
    /// Used for floating leg projection, require separate forward curves.
    Term,
}

/// Detailed information about a rate index.
#[derive(Clone, Debug)]
pub struct RateIndexInfo {
    /// Rate index family (overnight vs term)
    pub family: RateIndexFamily,
}

/// Registry entry for a known rate index.
struct IndexEntry {
    /// Canonical tokens that identify this index (uppercase, without special chars)
    tokens: &'static [&'static str],
    /// Index metadata
    info: RateIndexInfo,
}

/// Static registry of known rate indices with market-standard conventions.
///
/// This registry provides accurate classification of rate indices for:
/// - OIS vs term rate distinction (multi-curve framework)
/// - Currency-specific settlement conventions
/// - Day count conventions per index
static INDEX_REGISTRY: &[IndexEntry] = &[
    // USD Overnight Rates
    IndexEntry {
        tokens: &["SOFR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    IndexEntry {
        tokens: &["EFFR", "FEDFUNDS", "FF"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // EUR Overnight Rates
    IndexEntry {
        tokens: &["ESTR", "ESTER", "STR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    IndexEntry {
        tokens: &["EONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // GBP Overnight Rates (T+0 settlement!)
    IndexEntry {
        tokens: &["SONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // JPY Overnight Rates
    IndexEntry {
        tokens: &["TONA", "TONAR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // CHF Overnight Rates
    IndexEntry {
        tokens: &["SARON"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // AUD Overnight Rates
    IndexEntry {
        tokens: &["AONIA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // CAD Overnight Rates
    IndexEntry {
        tokens: &["CORRA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // SGD Overnight Rates
    IndexEntry {
        tokens: &["SORA"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // Generic OIS marker (matches "USD-OIS", "EUR-OIS", etc.)
    IndexEntry {
        tokens: &["OIS"],
        info: RateIndexInfo {
            family: RateIndexFamily::Overnight,
        },
    },
    // Term Rates (explicitly NOT overnight)
    IndexEntry {
        tokens: &["LIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
        },
    },
    IndexEntry {
        tokens: &["EURIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
        },
    },
    IndexEntry {
        tokens: &["TIBOR"],
        info: RateIndexInfo {
            family: RateIndexFamily::Term,
        },
    },
];

/// Lookup rate index information from an index identifier string.
///
/// Parses the index string and matches against the registry. Returns `None`
/// if the index is not recognized (caller should treat as term rate).
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::calibration::market_standards::lookup_index_info;
///
/// let info = lookup_index_info("USD-SOFR-OIS");
/// assert!(info.is_some());
/// assert_eq!(info.unwrap().family, RateIndexFamily::Overnight);
///
/// let libor = lookup_index_info("3M-USD-LIBOR");
/// assert!(libor.is_some());
/// assert_eq!(libor.unwrap().family, RateIndexFamily::Term);
/// ```
pub fn lookup_index_info(index: &str) -> Option<RateIndexInfo> {
    // Normalize: uppercase, replace € with E, split into tokens
    let normalized = index.to_uppercase().replace('€', "E");
    let tokens: Vec<&str> = normalized
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();

    // Check each registry entry
    for entry in INDEX_REGISTRY {
        for &entry_token in entry.tokens {
            // Match if any token in the index string matches this entry
            if tokens.contains(&entry_token) {
                return Some(entry.info.clone());
            }
        }
    }

    None
}

/// Check if an index identifier represents an overnight rate.
///
/// Uses the index registry for accurate classification. Returns `false`
/// for unrecognized indices (conservative default).
pub fn is_overnight_index(index: &str) -> bool {
    lookup_index_info(index)
        .map(|info| info.family == RateIndexFamily::Overnight)
        .unwrap_or(false)
}

/// Get the OIS compounding method for a rate index.
///
/// Returns the market-standard OIS compounding method based on the index name
/// and currency. This determines how overnight rates are compounded for OIS swaps.
///
/// # Logic
///
/// 1. **Index-name driven**: Checks for specific index tokens (SONIA, ESTR, TONA, SOFR)
/// 2. **Currency fallback**: For generic indices like "USD-OIS", uses currency conventions
///
/// # Examples
///
/// ```ignore
/// use finstack_valuations::calibration::market_standards::ois_compounding_for_index;
/// use finstack_core::currency::Currency;
/// use finstack_core::types::IndexId;
///
/// let index: IndexId = "USD-SOFR".into();
/// let compounding = ois_compounding_for_index(&index, Currency::USD);
/// // Returns FloatingLegCompounding::sofr()
/// ```
pub fn ois_compounding_for_index(index: &IndexId, currency: Currency) -> FloatingLegCompounding {
    let upper = index.as_str().to_ascii_uppercase();

    // Index-name driven overrides
    if upper.contains("SONIA") {
        return FloatingLegCompounding::sonia();
    }
    if upper.contains("ESTR") || upper.contains("€STR") {
        return FloatingLegCompounding::estr();
    }
    if upper.contains("TONA") || upper.contains("TONAR") {
        return FloatingLegCompounding::tona();
    }
    if upper.contains("SOFR") {
        return FloatingLegCompounding::sofr();
    }

    // Currency fallback for generic ids like "USD-OIS"
    match currency {
        Currency::GBP => FloatingLegCompounding::sonia(),
        Currency::EUR => FloatingLegCompounding::estr(),
        Currency::JPY => FloatingLegCompounding::tona(),
        _ => FloatingLegCompounding::sofr(),
    }
}

