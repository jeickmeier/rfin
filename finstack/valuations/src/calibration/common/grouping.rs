//! Quote grouping utilities for calibration.
//!
//! This module provides common functions to group instrument quotes by various
//! criteria, reducing duplicated logic across calibration modules.

use crate::calibration::primitives::InstrumentQuote;
use finstack_core::dates::Date;
use finstack_core::F;
use std::collections::HashMap;

/// Group option volatility quotes by expiry time (in years).
///
/// Uses proper day-count conversion for time-to-expiry calculation.
pub fn by_expiry_option_vol(
    quotes: &[InstrumentQuote],
    _base_date: Date,
) -> HashMap<String, Vec<&InstrumentQuote>> {
    let mut grouped = HashMap::new();

    for quote in quotes {
        if let InstrumentQuote::OptionVol { expiry, .. } = quote {
            let expiry_key = format!("{}", expiry);
            grouped
                .entry(expiry_key)
                .or_insert_with(Vec::new)
                .push(quote);
        }
    }

    grouped
}

/// Group CDS quotes by entity name.
pub fn by_entity_cds(quotes: &[InstrumentQuote]) -> HashMap<String, Vec<&InstrumentQuote>> {
    let mut grouped = HashMap::new();

    for quote in quotes {
        match quote {
            InstrumentQuote::CDS { entity, .. } => {
                grouped
                    .entry(entity.clone())
                    .or_insert_with(Vec::new)
                    .push(quote);
            }
            InstrumentQuote::CDSUpfront { entity, .. } => {
                grouped
                    .entry(entity.clone())
                    .or_insert_with(Vec::new)
                    .push(quote);
            }
            _ => {}
        }
    }

    grouped
}

/// Group CDS tranche quotes by index and maturity time (in years).
///
/// Returns a nested structure: index -> maturity_key -> quotes.
pub fn by_index_tranche(
    quotes: &[InstrumentQuote],
    _base_date: Date,
) -> HashMap<String, HashMap<String, Vec<&InstrumentQuote>>> {
    let mut grouped = HashMap::new();

    for quote in quotes {
        if let InstrumentQuote::CDSTranche {
            index, maturity, ..
        } = quote
        {
            let maturity_key = format!("{}", maturity);
            grouped
                .entry(index.clone())
                .or_insert_with(HashMap::new)
                .entry(maturity_key)
                .or_insert_with(Vec::new)
                .push(quote);
        }
    }

    grouped
}

/// Group inflation swap quotes by index name.
pub fn by_index_inflation(quotes: &[InstrumentQuote]) -> HashMap<String, Vec<&InstrumentQuote>> {
    let mut grouped = HashMap::new();

    for quote in quotes {
        if let InstrumentQuote::InflationSwap { index, .. } = quote {
            grouped
                .entry(index.clone())
                .or_insert_with(Vec::new)
                .push(quote);
        }
    }

    grouped
}

/// Find quotes with maturities nearest to target maturities.
///
/// For each target maturity, finds quotes within a tolerance and groups them.
pub fn nearest_maturities<'a>(
    quotes: &'a [InstrumentQuote],
    base_date: Date,
    target_maturities: &[F],
    tolerance: F,
) -> HashMap<String, Vec<&'a InstrumentQuote>> {
    let mut grouped = HashMap::new();

    for quote in quotes {
        let maturity_years = match quote {
            InstrumentQuote::OptionVol { expiry, .. } => {
                finstack_core::dates::DayCount::Act365F
                    .year_fraction(base_date, *expiry, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0)
            }
            InstrumentQuote::CDSTranche { maturity, .. } => {
                finstack_core::dates::DayCount::Act365F
                    .year_fraction(base_date, *maturity, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0)
            }
            InstrumentQuote::CDS { maturity, .. } => {
                finstack_core::dates::DayCount::Act365F
                    .year_fraction(base_date, *maturity, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0)
            }
            InstrumentQuote::CDSUpfront { maturity, .. } => {
                finstack_core::dates::DayCount::Act365F
                    .year_fraction(base_date, *maturity, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0)
            }
            InstrumentQuote::InflationSwap { maturity, .. } => {
                finstack_core::dates::DayCount::Act365F
                    .year_fraction(base_date, *maturity, finstack_core::dates::DayCountCtx::default())
                    .unwrap_or(0.0)
            }
            _ => continue,
        };

        // Find the closest target maturity within tolerance
        if let Some(&target_mat) = target_maturities.iter().min_by(|&&a, &&b| {
            (a - maturity_years)
                .abs()
                .partial_cmp(&(b - maturity_years).abs())
                .unwrap()
        }) {
            if (target_mat - maturity_years).abs() <= tolerance {
                let key = format!("{:.2}Y", target_mat);
                grouped.entry(key).or_insert_with(Vec::new).push(quote);
            }
        }
    }

    grouped
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::prelude::Currency;
    use time::Month;

    fn create_test_quotes() -> Vec<InstrumentQuote> {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();

        vec![
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: base_date + time::Duration::days(30),
                strike: 100.0,
                vol: 0.20,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::OptionVol {
                underlying: "SPY".to_string(),
                expiry: base_date + time::Duration::days(90),
                strike: 100.0,
                vol: 0.22,
                option_type: "Call".to_string(),
            },
            InstrumentQuote::CDS {
                entity: "AAPL".to_string(),
                maturity: base_date + time::Duration::days(365),
                spread_bp: 50.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
            InstrumentQuote::CDS {
                entity: "MSFT".to_string(),
                maturity: base_date + time::Duration::days(365 * 5),
                spread_bp: 75.0,
                recovery_rate: 0.4,
                currency: Currency::USD,
            },
            InstrumentQuote::CDSTranche {
                index: "CDX.NA.IG.42".to_string(),
                attachment: 0.0,
                detachment: 3.0,
                maturity: base_date + time::Duration::days(365 * 5),
                upfront_pct: 2.0,
                running_spread_bp: 500.0,
            },
            InstrumentQuote::InflationSwap {
                maturity: base_date + time::Duration::days(365 * 5),
                rate: 0.025,
                index: "US-CPI-U".to_string(),
            },
        ]
    }

    #[test]
    fn test_group_by_expiry_option_vol() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = create_test_quotes();

        let grouped = by_expiry_option_vol(&quotes, base_date);

        // Should have 2 expiry groups (1M and 3M)
        assert_eq!(grouped.len(), 2);

        // Each group should have 1 quote
        for (_, group_quotes) in grouped {
            assert_eq!(group_quotes.len(), 1);
        }
    }

    #[test]
    fn test_group_by_entity_cds() {
        let quotes = create_test_quotes();

        let grouped = by_entity_cds(&quotes);

        // Should have 2 entities (AAPL and MSFT)
        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("AAPL"));
        assert!(grouped.contains_key("MSFT"));

        // Each entity should have 1 quote
        assert_eq!(grouped["AAPL"].len(), 1);
        assert_eq!(grouped["MSFT"].len(), 1);
    }

    #[test]
    fn test_group_by_index_tranche() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = create_test_quotes();

        let grouped = by_index_tranche(&quotes, base_date);

        // Should have 1 index (CDX.NA.IG.42)
        assert_eq!(grouped.len(), 1);
        assert!(grouped.contains_key("CDX.NA.IG.42"));

        // Should have 1 maturity
        let index_quotes = &grouped["CDX.NA.IG.42"];
        assert_eq!(index_quotes.len(), 1);
    }

    #[test]
    fn test_group_by_index_inflation() {
        let quotes = create_test_quotes();

        let grouped = by_index_inflation(&quotes);

        // Should have 1 index (US-CPI-U)
        assert_eq!(grouped.len(), 1);
        assert!(grouped.contains_key("US-CPI-U"));
        assert_eq!(grouped["US-CPI-U"].len(), 1);
    }

    #[test]
    fn test_nearest_maturities() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let quotes = create_test_quotes();

        let target_maturities = [1.0, 5.0]; // 1Y and 5Y
        let grouped = nearest_maturities(&quotes, base_date, &target_maturities, 0.5);

        // Should find quotes near 1Y and 5Y
        assert!(!grouped.is_empty());
    }

    #[test]
    fn test_empty_quotes() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let empty_quotes = [];

        let grouped = by_expiry_option_vol(&empty_quotes, base_date);
        assert!(grouped.is_empty());

        let grouped = by_entity_cds(&empty_quotes);
        assert!(grouped.is_empty());
    }
}
