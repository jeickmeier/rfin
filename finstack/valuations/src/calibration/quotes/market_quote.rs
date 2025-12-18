//! Unified market quote wrapper type.
//!
//! Unified market quote that can be any instrument type.

use super::{CreditQuote, InflationQuote, RatesQuote, VolQuote};
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Unified market quote wrapper for all calibration instrument types.
///
/// This enum acts as a container for heterogeneous market data inputs
/// (rates, credit, volatility, inflation) used during the 9-step
/// calibration planning process.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum MarketQuote {
    /// Interest rate quotes (deposits, FRAs, futures, swaps).
    Rates(RatesQuote),
    /// Credit quotes (CDS, CDS upfront index).
    Credit(CreditQuote),
    /// Volatility quotes (option/swaption implied volatility).
    Vol(VolQuote),
    /// Inflation quotes (inflation swaps, CPI fixings).
    Inflation(InflationQuote),
}

impl MarketQuote {
    /// Get the underlying quote type name.
    pub fn quote_type(&self) -> &'static str {
        match self {
            MarketQuote::Rates(q) => q.get_type(),
            MarketQuote::Credit(_) => "Credit",
            MarketQuote::Vol(_) => "Vol",
            MarketQuote::Inflation(_) => "Inflation",
        }
    }

    /// Bump the quote in its natural market units.
    ///
    /// The `amount` parameter is interpreted per quote type:
    ///
    /// - Rates: decimal rate bump (e.g., `0.0001` = 1bp)
    /// - Credit: decimal-to-bp conversion (`spread_bp += amount * 10_000`)
    /// - Vol: absolute vol bump (e.g., `0.01` = +1 vol point)
    /// - Inflation: decimal rate bump (e.g., `0.0001` = 1bp)
    pub fn bump(&self, amount: f64) -> Self {
        match self {
            MarketQuote::Rates(q) => MarketQuote::Rates(q.bump_rate_decimal(amount)),
            MarketQuote::Credit(q) => MarketQuote::Credit(q.bump_spread_decimal(amount)),
            MarketQuote::Vol(q) => MarketQuote::Vol(q.bump_vol_absolute(amount)),
            MarketQuote::Inflation(q) => MarketQuote::Inflation(q.bump_rate_decimal(amount)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calibration::quotes::InstrumentConventions;
    use finstack_core::dates::Date;
    use finstack_core::types::UnderlyingId;
    use time::Month;

    const BP: f64 = 0.0001;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid date")
    }

    #[test]
    fn bump_is_not_a_silent_no_op_for_non_rates_quotes() {
        let credit = MarketQuote::Credit(CreditQuote::CDS {
            entity: "ABC".to_string(),
            maturity: date(2030, Month::January, 1),
            spread_bp: 100.0,
            recovery_rate: 0.4,
            currency: finstack_core::types::Currency::USD,
            conventions: InstrumentConventions::default(),
        });
        let bumped = credit.bump(BP);
        match bumped {
            MarketQuote::Credit(CreditQuote::CDS { spread_bp, .. }) => {
                assert!((spread_bp - 101.0).abs() < 1e-12)
            }
            _ => panic!("unexpected variant"),
        }

        let vol = MarketQuote::Vol(VolQuote::OptionVol {
            underlying: UnderlyingId::from("SPX"),
            expiry: date(2030, Month::January, 1),
            strike: 100.0,
            vol: 0.20,
            option_type: "Call".to_string(),
            conventions: InstrumentConventions::default(),
        });
        let bumped = vol.bump(0.01);
        match bumped {
            MarketQuote::Vol(VolQuote::OptionVol { vol, .. }) => {
                assert!((vol - 0.21).abs() < 1e-12)
            }
            _ => panic!("unexpected variant"),
        }
    }
}
