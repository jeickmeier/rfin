//! Credit instrument quote types for hazard curve and correlation calibration.
//!
//! Credit instrument quotes for hazard curve and correlation calibration.

use super::conventions::InstrumentConventions;
use finstack_core::dates::Date;
use finstack_core::prelude::*;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

/// Credit instrument quotes for hazard curve and correlation calibration.
#[cfg_attr(feature = "ts_export", derive(TS))]
#[cfg_attr(feature = "ts_export", ts(export))]
#[cfg_attr(feature = "ts_export", ts(rename_all = "snake_case"))]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CreditQuote {
    /// CDS par spread quote
    CDS {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Par spread in basis points
        spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// CDS upfront quote
    CDSUpfront {
        /// Reference entity
        entity: String,
        /// CDS maturity
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread in basis points
        running_spread_bp: f64,
        /// Recovery rate assumption
        recovery_rate: f64,
        /// Currency
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        currency: Currency,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
    /// CDS Tranche quote
    CDSTranche {
        /// Index name
        index: String,
        /// Attachment point (%)
        attachment: f64,
        /// Detachment point (%)
        detachment: f64,
        /// Maturity date
        #[cfg_attr(feature = "ts_export", ts(type = "string"))]
        maturity: Date,
        /// Upfront payment (% of notional)
        upfront_pct: f64,
        /// Running spread (bps)
        running_spread_bp: f64,
        /// Per-instrument conventions
        #[serde(default, skip_serializing_if = "InstrumentConventions::is_empty")]
        conventions: InstrumentConventions,
    },
}

impl CreditQuote {
    /// Get per-instrument conventions for this quote.
    pub fn conventions(&self) -> &InstrumentConventions {
        match self {
            CreditQuote::CDS { conventions, .. } => conventions,
            CreditQuote::CDSUpfront { conventions, .. } => conventions,
            CreditQuote::CDSTranche { conventions, .. } => conventions,
        }
    }

    /// Get maturity date for this quote if applicable.
    pub fn maturity_date(&self) -> Option<Date> {
        match self {
            CreditQuote::CDS { maturity, .. } => Some(*maturity),
            CreditQuote::CDSUpfront { maturity, .. } => Some(*maturity),
            CreditQuote::CDSTranche { maturity, .. } => Some(*maturity),
        }
    }

    /// Create a new quote with the credit spread bumped by a **decimal rate** amount.
    ///
    /// The `rate_bump` parameter is specified in decimal terms (e.g., `0.0001`
    /// for 1 basis point). Credit quotes are commonly expressed in basis points,
    /// so this bump is applied as:
    ///
    /// - CDS: `spread_bp += rate_bump * 10_000`
    /// - CDSUpfront / CDSTranche: `running_spread_bp += rate_bump * 10_000`
    pub fn bump_spread_decimal(&self, rate_bump: f64) -> Self {
        let bump_bp = rate_bump * 10_000.0;
        match self {
            CreditQuote::CDS {
                entity,
                maturity,
                spread_bp,
                recovery_rate,
                currency,
                conventions,
            } => CreditQuote::CDS {
                entity: entity.clone(),
                maturity: *maturity,
                spread_bp: spread_bp + bump_bp,
                recovery_rate: *recovery_rate,
                currency: *currency,
                conventions: conventions.clone(),
            },
            CreditQuote::CDSUpfront {
                entity,
                maturity,
                upfront_pct,
                running_spread_bp,
                recovery_rate,
                currency,
                conventions,
            } => CreditQuote::CDSUpfront {
                entity: entity.clone(),
                maturity: *maturity,
                upfront_pct: *upfront_pct,
                running_spread_bp: running_spread_bp + bump_bp,
                recovery_rate: *recovery_rate,
                currency: *currency,
                conventions: conventions.clone(),
            },
            CreditQuote::CDSTranche {
                index,
                attachment,
                detachment,
                maturity,
                upfront_pct,
                running_spread_bp,
                conventions,
            } => CreditQuote::CDSTranche {
                index: index.clone(),
                attachment: *attachment,
                detachment: *detachment,
                maturity: *maturity,
                upfront_pct: *upfront_pct,
                running_spread_bp: running_spread_bp + bump_bp,
                conventions: conventions.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::dates::Date;
    use time::Month;

    const BP: f64 = 0.0001;

    fn date(year: i32, month: Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).expect("valid date")
    }

    #[test]
    fn bump_spread_decimal_adjusts_spreads_in_bp() {
        let cds = CreditQuote::CDS {
            entity: "ABC".to_string(),
            maturity: date(2030, Month::January, 1),
            spread_bp: 100.0,
            recovery_rate: 0.4,
            currency: Currency::USD,
            conventions: InstrumentConventions::default(),
        };
        let bumped = cds.bump_spread_decimal(BP);
        match bumped {
            CreditQuote::CDS { spread_bp, .. } => assert!((spread_bp - 101.0).abs() < 1e-12),
            _ => panic!("unexpected variant"),
        }

        let upfront = CreditQuote::CDSUpfront {
            entity: "ABC".to_string(),
            maturity: date(2030, Month::January, 1),
            upfront_pct: 0.02,
            running_spread_bp: 500.0,
            recovery_rate: 0.4,
            currency: Currency::USD,
            conventions: InstrumentConventions::default(),
        };
        let bumped = upfront.bump_spread_decimal(BP);
        match bumped {
            CreditQuote::CDSUpfront {
                running_spread_bp,
                upfront_pct,
                ..
            } => {
                assert!((running_spread_bp - 501.0).abs() < 1e-12);
                assert!((upfront_pct - 0.02).abs() < 1e-12);
            }
            _ => panic!("unexpected variant"),
        }

        let tranche = CreditQuote::CDSTranche {
            index: "CDX".to_string(),
            attachment: 0.03,
            detachment: 0.07,
            maturity: date(2030, Month::January, 1),
            upfront_pct: 0.01,
            running_spread_bp: 1000.0,
            conventions: InstrumentConventions::default(),
        };
        let bumped = tranche.bump_spread_decimal(BP);
        match bumped {
            CreditQuote::CDSTranche {
                running_spread_bp, ..
            } => assert!((running_spread_bp - 1001.0).abs() < 1e-12),
            _ => panic!("unexpected variant"),
        }
    }
}
