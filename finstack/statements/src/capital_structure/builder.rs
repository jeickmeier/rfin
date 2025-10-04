//! Builder API Extensions for Capital Structure
//!
//! This module provides fluent builder methods for adding capital structure
//! instruments to a financial model.

use crate::builder::ModelBuilder;
use crate::error::Result;
use crate::types::{CapitalStructureSpec, DebtInstrumentSpec};
use finstack_core::dates::Date;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::{Bond, InterestRateSwap};

/// Helper to ensure capital structure exists and return mutable reference.
fn ensure_capital_structure<State>(builder: &mut ModelBuilder<State>) -> &mut CapitalStructureSpec {
    builder
        .capital_structure
        .get_or_insert_with(|| CapitalStructureSpec {
            debt_instruments: vec![],
            equity_instruments: vec![],
            meta: indexmap::IndexMap::new(),
        })
}

impl<State> ModelBuilder<State> {
    /// Add a bond to the capital structure.
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `notional` - Principal amount
    /// * `coupon_rate` - Annual coupon rate (e.g., 0.05 for 5%)
    /// * `issue_date` - Bond issue date
    /// * `maturity_date` - Bond maturity date
    /// * `discount_curve_id` - Discount curve ID for pricing
    ///
    /// # Example
    /// ```ignore
    /// .add_bond(
    ///     "BOND-001",
    ///     Money::new(10_000_000.0, Currency::USD),
    ///     0.05,  // 5% coupon
    ///     issue_date,
    ///     maturity_date,
    ///     "USD-OIS",
    /// )?
    /// ```
    pub fn add_bond(
        mut self,
        id: impl Into<String>,
        notional: Money,
        coupon_rate: f64,
        issue_date: Date,
        maturity_date: Date,
        discount_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str: String = id.into();

        // Create bond using valuations crate
        let bond = Bond::fixed_semiannual(
            InstrumentId::new(&id_str),
            notional,
            coupon_rate,
            issue_date,
            maturity_date,
            CurveId::new(discount_curve_id),
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&bond).map_err(|e| {
            crate::error::Error::build(format!("Failed to serialize bond '{}': {}", id_str, e))
        })?;

        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Bond {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add an interest rate swap to the capital structure.
    ///
    /// # Arguments
    /// * `id` - Unique instrument identifier
    /// * `notional` - Notional amount
    /// * `fixed_rate` - Fixed rate (e.g., 0.04 for 4%)
    /// * `start_date` - Swap start date
    /// * `maturity_date` - Swap maturity date
    /// * `discount_curve_id` - Discount curve ID
    /// * `forward_curve_id` - Forward curve ID for floating leg
    ///
    /// # Example
    /// ```ignore
    /// .add_swap(
    ///     "SWAP-001",
    ///     Money::new(5_000_000.0, Currency::USD),
    ///     0.04,  // 4% fixed rate
    ///     start_date,
    ///     maturity_date,
    ///     "USD-OIS",
    ///     "USD-SOFR-3M",
    /// )?
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn add_swap(
        mut self,
        id: impl Into<String>,
        notional: Money,
        fixed_rate: f64,
        start_date: Date,
        maturity_date: Date,
        _discount_curve_id: impl Into<String>,
        _forward_curve_id: impl Into<String>,
    ) -> Result<Self> {
        let id_str: String = id.into();

        // Create swap using valuations crate
        let swap = InterestRateSwap::usd_pay_fixed(
            InstrumentId::new(&id_str),
            notional,
            fixed_rate,
            start_date,
            maturity_date,
        );

        // Serialize to JSON
        let spec_json = serde_json::to_value(&swap)
            .map_err(|e| crate::error::Error::build(format!("Failed to serialize swap: {}", e)))?;

        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Swap {
                id: id_str,
                spec: spec_json,
            });

        Ok(self)
    }

    /// Add a generic debt instrument via JSON specification.
    ///
    /// This allows adding custom debt instruments not covered by the
    /// convenience methods (bonds, swaps).
    ///
    /// # Example
    /// ```ignore
    /// .add_custom_debt(
    ///     "TL-A",
    ///     json!({
    ///         "type": "amortizing_loan",
    ///         "notional": 25_000_000.0,
    ///         "currency": "USD",
    ///         "issue_date": "2025-01-15",
    ///         "maturity_date": "2030-01-15",
    ///         "coupon_rate": 0.06,
    ///         "frequency": "quarterly",
    ///         "amortization": {
    ///             "type": "linear",
    ///             "final_notional": 0.0
    ///         }
    ///     })
    /// )?
    /// ```
    pub fn add_custom_debt(mut self, id: impl Into<String>, spec: serde_json::Value) -> Self {
        // Add to capital structure
        ensure_capital_structure(&mut self)
            .debt_instruments
            .push(DebtInstrumentSpec::Generic {
                id: id.into(),
                spec,
            });

        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::NeedPeriods;
    use finstack_core::currency::Currency;
    use time::Month;

    #[test]
    fn test_add_bond() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

        let builder = ModelBuilder::<NeedPeriods>::new("test")
            .periods("2025Q1..2025Q2", None)
            .unwrap()
            .add_bond(
                "BOND-001",
                Money::new(1_000_000.0, Currency::USD),
                0.05,
                issue,
                maturity,
                "USD-OIS",
            )
            .unwrap();

        assert!(builder.capital_structure.is_some());
        let cs = builder.capital_structure.as_ref().unwrap();
        assert_eq!(cs.debt_instruments.len(), 1);

        match &cs.debt_instruments[0] {
            DebtInstrumentSpec::Bond { id, spec: _ } => {
                assert_eq!(id, "BOND-001");
            }
            _ => panic!("Expected Bond variant"),
        }
    }

    #[test]
    fn test_add_swap() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let builder = ModelBuilder::<NeedPeriods>::new("test")
            .periods("2025Q1..2025Q2", None)
            .unwrap()
            .add_swap(
                "SWAP-001",
                Money::new(5_000_000.0, Currency::USD),
                0.04,
                start,
                maturity,
                "USD-OIS",
                "USD-SOFR-3M",
            )
            .unwrap();

        assert!(builder.capital_structure.is_some());
        let cs = builder.capital_structure.as_ref().unwrap();
        assert_eq!(cs.debt_instruments.len(), 1);

        match &cs.debt_instruments[0] {
            DebtInstrumentSpec::Swap { id, spec: _ } => {
                assert_eq!(id, "SWAP-001");
            }
            _ => panic!("Expected Swap variant"),
        }
    }

    #[test]
    fn test_add_multiple_instruments() {
        let issue = Date::from_calendar_date(2025, Month::January, 15).unwrap();
        let maturity = Date::from_calendar_date(2030, Month::January, 15).unwrap();

        let builder = ModelBuilder::<NeedPeriods>::new("test")
            .periods("2025Q1..2025Q2", None)
            .unwrap()
            .add_bond(
                "BOND-001",
                Money::new(1_000_000.0, Currency::USD),
                0.05,
                issue,
                maturity,
                "USD-OIS",
            )
            .unwrap()
            .add_bond(
                "BOND-002",
                Money::new(2_000_000.0, Currency::USD),
                0.06,
                issue,
                maturity,
                "USD-OIS",
            )
            .unwrap();

        assert!(builder.capital_structure.is_some());
        let cs = builder.capital_structure.as_ref().unwrap();
        assert_eq!(cs.debt_instruments.len(), 2);
    }

    #[test]
    fn test_add_custom_debt() {
        let builder = ModelBuilder::<NeedPeriods>::new("test")
            .periods("2025Q1..2025Q2", None)
            .unwrap()
            .add_custom_debt(
                "TL-A",
                serde_json::json!({
                    "type": "term_loan",
                    "notional": 10_000_000.0,
                    "currency": "USD",
                }),
            );

        assert!(builder.capital_structure.is_some());
        let cs = builder.capital_structure.as_ref().unwrap();
        assert_eq!(cs.debt_instruments.len(), 1);

        match &cs.debt_instruments[0] {
            DebtInstrumentSpec::Generic { id, spec: _ } => {
                assert_eq!(id, "TL-A");
            }
            _ => panic!("Expected Generic variant"),
        }
    }
}
