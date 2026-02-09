//! Periodized present value calculations for instruments.
//!
//! This module provides extension traits that allow any instrument implementing
//! `CashflowProvider` + `CurveDependencies` to compute present values aggregated
//! by reporting periods.
//!
//! # Overview
//!
//! The `PeriodizedPvExt` trait provides two methods:
//! - `periodized_pv`: Basic discounting with discount curve only
//! - `periodized_pv_credit_adjusted`: Optional credit adjustment via hazard curve
//!
//! These methods delegate to the instrument's `build_dated_flows` implementation
//! and leverage cashflow aggregation utilities for the actual
//! aggregation and discounting.
//!
//! # Example
//!
//! ```rust
//! use finstack_valuations::instruments::Bond;
//! use finstack_valuations::instruments::PeriodizedPvExt;
//! use finstack_core::dates::{Date, Period, PeriodId, DayCount};
//! use finstack_core::currency::Currency;
//! use finstack_core::money::Money;
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_core::market_data::term_structures::DiscountCurve;
//! use time::Month;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use finstack_core::dates::create_date;
//! let issue = create_date(2025, Month::January, 1)?;
//! let maturity = create_date(2026, Month::January, 1)?;
//!
//! // Create a simple bond
//! let bond = Bond::fixed(
//!     "BOND-001",
//!     Money::new(1_000_000.0, Currency::USD),
//!     0.05,
//!     issue,
//!     maturity,
//!     "USD-OIS",
//! )?;
//!
//! // Set up market with discount curve
//! let disc_curve = DiscountCurve::builder("USD-OIS")
//!     .base_date(issue)
//!     .knots([(0.0, 1.0), (1.0, 0.95)])
//!     .interp(finstack_core::math::interp::InterpStyle::Linear)
//!     .build()?;
//! let market = MarketContext::new().insert_discount(disc_curve);
//!
//! // Define quarterly periods
//! let periods = vec![
//!     Period {
//!         id: PeriodId::quarter(2025, 1),
//!         start: Date::from_calendar_date(2025, Month::January, 1)?,
//!         end: Date::from_calendar_date(2025, Month::April, 1)?,
//!         is_actual: true,
//!     },
//!     Period {
//!         id: PeriodId::quarter(2025, 2),
//!         start: Date::from_calendar_date(2025, Month::April, 1)?,
//!         end: Date::from_calendar_date(2025, Month::July, 1)?,
//!         is_actual: false,
//!     },
//! ];
//!
//! // Compute periodized PVs
//! let pv_by_period = bond.periodized_pv(&periods, &market, issue, DayCount::Act365F)?;
//!
//! // Access PV for Q1
//! if let Some(q1_pvs) = pv_by_period.get(&PeriodId::quarter(2025, 1)) {
//!     if let Some(usd_pv) = q1_pvs.get(&Currency::USD) {
//!         println!("Q1 PV: {}", usd_pv.amount());
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::cashflow::builder::schedule::resolve_credit_curves;
use crate::cashflow::traits::CashflowProvider;
use crate::instruments::common_impl::traits::CurveDependencies;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount, Period, PeriodId};
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use indexmap::IndexMap;

/// Extension trait providing periodized present value calculation for instruments.
///
/// Automatically implemented for any type that provides both cashflow schedules
/// (`CashflowProvider`) and curve dependencies (`CurveDependencies`).
///
/// # Design
///
/// This trait serves as a bridge between instrument-level APIs and the lower-level
/// cashflow aggregation utilities. It handles:
/// - Building the simplified cashflow schedule via `build_dated_flows`
/// - Extracting the discount curve ID from the instrument
/// - Delegating to the aggregation utilities for periodized PV calculation
///
/// # Currency Safety
///
/// All returned PVs preserve currency information. Each period maps to a
/// `Currency -> Money` sub-map, preventing accidental cross-currency aggregation.
pub trait PeriodizedPvExt: CashflowProvider + CurveDependencies {
    /// Compute present values aggregated by period using discount curve only.
    ///
    /// Groups cashflows by period and computes the present value of each cashflow
    /// discounted back to the base date. Returns a map from `PeriodId` to
    /// currency-indexed PV sums.
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `market` - Market context containing discount curves
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    ///
    /// # Errors
    /// Returns an error if the discount curve is not found in the market context
    /// or if the cashflow schedule cannot be built.
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_core::dates::{build_periods, DayCount};
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_valuations::instruments::Bond;
    /// use finstack_valuations::instruments::PeriodizedPvExt;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let base = date!(2025-01-15);
    /// let market = MarketContext::new();
    /// let quarters = build_periods("2025Q1..Q4", None)?.periods;
    ///
    /// let bond = Bond::fixed(
    ///     "BOND-1",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     0.05,
    ///     date!(2025-01-15),
    ///     date!(2030-01-15),
    ///     "USD-OIS",
    /// )?;
    ///
    /// let pv_map = bond.periodized_pv(&quarters, &market, base, DayCount::Act365F)?;
    /// # let _ = pv_map;
    /// # Ok(())
    /// # }
    /// ```
    fn periodized_pv(
        &self,
        periods: &[Period],
        market: &MarketContext,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        use crate::cashflow::traits::schedule_from_dated_flows;
        use finstack_core::dates::DayCountCtx;

        let flows = self.build_dated_flows(market, base)?;
        let schedule = schedule_from_dated_flows(flows, self.notional(), dc);

        let deps = self.curve_dependencies()?;
        let disc_curve_id = deps
            .discount_curves
            .first()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;
        let disc_arc = market.get_discount(disc_curve_id.as_str())?;

        schedule.pv_by_period_with_ctx(periods, disc_arc.as_ref(), base, dc, DayCountCtx::default())
    }

    /// Compute present values aggregated by period with optional credit adjustment.
    ///
    /// Similar to `periodized_pv`, but optionally applies credit risk adjustment via
    /// a hazard curve.
    ///
    /// # Methodology
    ///
    /// - **Interest/Fees**: Assumed zero recovery (`PV = Amount * DF * SP`)
    /// - **Principal**: Applies recovery rate `R` from hazard curve (`PV = Amount * DF * (SP + R * (1-SP))`)
    ///
    /// # Assumptions
    ///
    /// - **Independence**: Assumes independence between interest rates and credit spreads (default time).
    /// - **Recovery**: Recovery is applied only if the instrument's schedule identifies flows as Principal/Notional.
    ///
    /// # Arguments
    /// * `periods` - Period definitions with start/end boundaries
    /// * `market` - Market context containing discount and optional hazard curves
    /// * `hazard_curve_id` - Optional identifier for hazard curve (credit adjustment)
    /// * `base` - Base date for discounting (typically valuation date)
    /// * `dc` - Day count convention for year fraction calculation
    ///
    /// # Returns
    /// Map from `PeriodId` to currency-indexed PV sums. Periods with no cashflows
    /// are omitted from the result.
    ///
    /// # Errors
    /// Returns an error if the discount curve is not found, or if `hazard_curve_id`
    /// is missing or not found in the market context.
    ///
    /// # Example
    /// ```rust,no_run
    /// use finstack_core::dates::{build_periods, DayCount};
    /// use finstack_core::market_data::context::MarketContext;
    /// use finstack_core::money::Money;
    /// use finstack_core::currency::Currency;
    /// use finstack_core::types::CurveId;
    /// use finstack_valuations::instruments::Bond;
    /// use finstack_valuations::instruments::PeriodizedPvExt;
    /// use time::macros::date;
    ///
    /// # fn main() -> finstack_core::Result<()> {
    /// let base = date!(2025-01-15);
    /// let market = MarketContext::new();
    /// let quarters = build_periods("2025Q1..Q4", None)?.periods;
    /// let hazard_id = CurveId::new("USD-HY");
    ///
    /// let bond = Bond::fixed(
    ///     "BOND-1",
    ///     Money::new(1_000_000.0, Currency::USD),
    ///     0.05,
    ///     date!(2025-01-15),
    ///     date!(2030-01-15),
    ///     "USD-OIS",
    /// )?;
    ///
    /// let pv_map = bond.periodized_pv_credit_adjusted(
    ///     &quarters,
    ///     &market,
    ///     Some(&hazard_id),
    ///     base,
    ///     DayCount::Act365F,
    /// )?;
    /// # let _ = pv_map;
    /// # Ok(())
    /// # }
    /// ```
    fn periodized_pv_credit_adjusted(
        &self,
        periods: &[Period],
        market: &MarketContext,
        hazard_curve_id: Option<&CurveId>,
        base: Date,
        dc: DayCount,
    ) -> finstack_core::Result<IndexMap<PeriodId, IndexMap<Currency, Money>>> {
        let hazard_curve_id = hazard_curve_id.ok_or_else(|| {
            finstack_core::Error::Input(finstack_core::InputError::NotFound {
                id: "hazard curve id".to_string(),
            })
        })?;

        // Build full schedule (holder perspective, filtered flows) to preserve CFKind
        // This allows applying recovery rates to principal flows only.
        let schedule = self.build_full_schedule(market, base)?;

        // Resolve discount and hazard curves once to avoid duplicated logic.
        let deps = self.curve_dependencies()?;
        let disc_curve_id = deps
            .discount_curves
            .first()
            .ok_or_else(|| finstack_core::Error::from(finstack_core::InputError::Invalid))?;
        let _ = resolve_credit_curves(market, disc_curve_id, Some(hazard_curve_id))?;
        use finstack_core::dates::DayCountCtx;

        schedule.pv_by_period_with_market_and_ctx(
            periods,
            market,
            disc_curve_id,
            Some(hazard_curve_id),
            base,
            dc,
            DayCountCtx::default(),
        )
    }
}

// Blanket implementation for all types that implement CashflowProvider + CurveDependencies
impl<T> PeriodizedPvExt for T where T: CashflowProvider + CurveDependencies {}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cashflow::aggregation::DateContext;
    use crate::instruments::Bond;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCountCtx;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::market_data::term_structures::HazardCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn create_test_bond() -> Bond {
        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");

        Bond::fixed(
            "TEST-BOND",
            Money::new(1_000_000.0, Currency::USD),
            0.05, // 5% coupon
            issue,
            maturity,
            "USD-OIS",
        )
        .expect("Test bond creation should succeed")
    }

    fn create_test_market(base: Date) -> MarketContext {
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        MarketContext::new().insert_discount(disc_curve)
    }

    fn create_quarters_2025() -> Vec<Period> {
        vec![
            Period {
                id: PeriodId::quarter(2025, 1),
                start: Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date"),
                end: Date::from_calendar_date(2025, Month::April, 1).expect("Valid test date"),
                is_actual: true,
            },
            Period {
                id: PeriodId::quarter(2025, 2),
                start: Date::from_calendar_date(2025, Month::April, 1).expect("Valid test date"),
                end: Date::from_calendar_date(2025, Month::July, 1).expect("Valid test date"),
                is_actual: false,
            },
            Period {
                id: PeriodId::quarter(2025, 3),
                start: Date::from_calendar_date(2025, Month::July, 1).expect("Valid test date"),
                end: Date::from_calendar_date(2025, Month::October, 1).expect("Valid test date"),
                is_actual: false,
            },
            Period {
                id: PeriodId::quarter(2025, 4),
                start: Date::from_calendar_date(2025, Month::October, 1).expect("Valid test date"),
                end: Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date"),
                is_actual: false,
            },
            Period {
                id: PeriodId::quarter(2026, 1),
                start: Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date"),
                end: Date::from_calendar_date(2026, Month::April, 1).expect("Valid test date"),
                is_actual: false,
            },
        ]
    }

    #[test]
    fn test_periodized_pv_bond_fixed_matches_sum_npv() {
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let market = create_test_market(base);
        let periods = create_quarters_2025();

        // Compute periodized PV
        let pv_by_period = bond
            .periodized_pv(&periods, &market, base, DayCount::Act365F)
            .expect("Periodized PV calculation should succeed in test");

        // Sum all period PVs
        let mut total_pv = 0.0;
        for period_map in pv_by_period.values() {
            for money in period_map.values() {
                assert_eq!(money.currency(), Currency::USD);
                total_pv += money.amount();
            }
        }

        // Compute straight NPV for comparison
        use crate::instruments::common_impl::helpers::schedule_pv_using_curve_dc;
        let bond_disc = bond
            .curve_dependencies()
            .expect("curve_dependencies should succeed")
            .discount_curves
            .first()
            .cloned()
            .expect("Bond should declare a discount curve");
        let straight_npv = schedule_pv_using_curve_dc(&bond, &market, base, &bond_disc)
            .expect("Schedule PV calculation should succeed in test");

        // Sum of periodized PVs should match straight NPV (within rounding tolerance)
        let diff = (total_pv - straight_npv.amount()).abs();
        assert!(
            diff < 0.01, // Allow for small rounding differences
            "Periodized PV sum ({}) should match straight NPV ({}), diff: {}",
            total_pv,
            straight_npv.amount(),
            diff
        );
    }

    #[test]
    fn test_periodized_pv_bond_floating_uses_builder_rates() {
        use finstack_core::dates::Tenor;
        use finstack_core::market_data::term_structures::ForwardCurve;

        let issue = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let maturity = Date::from_calendar_date(2026, Month::January, 1).expect("Valid test date");

        // Create FRN using the new floating constructor
        let frn = Bond::floating(
            "FRN-001",
            Money::new(1_000_000.0, Currency::USD),
            "USD-SOFR",
            100, // 100 bps margin
            issue,
            maturity,
            Tenor::quarterly(),
            DayCount::Act365F,
            "USD-OIS",
        )
        .unwrap();

        // Create market with discount and forward curves
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(issue)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let fwd_curve = ForwardCurve::builder("USD-SOFR", 0.25) // 3M tenor
            .base_date(issue)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 0.03), (1.0, 0.035)]) // 3-3.5% forward rates
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_forward(fwd_curve);

        let periods = create_quarters_2025();

        // Compute periodized PV
        let pv_by_period = frn
            .periodized_pv(&periods, &market, issue, DayCount::Act365F)
            .expect("Periodized PV calculation should succeed in test");

        // Verify we got PVs for expected periods
        assert!(!pv_by_period.is_empty());

        // Verify each period has USD currency
        for period_map in pv_by_period.values() {
            assert!(period_map.contains_key(&Currency::USD));
        }

        // Sum should match straight NPV
        let mut total_pv = 0.0;
        for period_map in pv_by_period.values() {
            for money in period_map.values() {
                total_pv += money.amount();
            }
        }

        use crate::instruments::common_impl::helpers::schedule_pv_using_curve_dc;
        let straight_npv = {
            let frn_disc = frn
                .curve_dependencies()
                .expect("curve_dependencies should succeed")
                .discount_curves
                .first()
                .cloned()
                .expect("FRN should declare a discount curve");
            schedule_pv_using_curve_dc(&frn, &market, issue, &frn_disc)
        }
        .expect("Schedule PV calculation should succeed in test");

        let diff = (total_pv - straight_npv.amount()).abs();
        assert!(
            diff < 0.01, // Allow for small rounding differences
            "FRN periodized PV sum ({}) should match straight NPV ({}), diff: {}",
            total_pv,
            straight_npv.amount(),
            diff
        );
    }

    #[test]
    fn test_periodized_pv_credit_adjusted_applies_hazard() {
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        // Create market with discount and hazard curves
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let hazard_curve = HazardCurve::builder("CORP-HAZARD")
            .base_date(base)
            .recovery_rate(0.40)
            .knots([(0.0, 0.0), (1.0, 0.01)]) // 1% hazard rate at 1 year
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_curve);

        let periods = create_quarters_2025();

        // Compute credit-adjusted periodized PV
        let hazard_id = CurveId::new("CORP-HAZARD");
        let pv_with_credit = bond
            .periodized_pv_credit_adjusted(
                &periods,
                &market,
                Some(&hazard_id),
                base,
                DayCount::Act365F,
            )
            .expect("Periodized PV calculation should succeed in test");

        // Compute without credit adjustment
        let pv_no_credit = bond
            .periodized_pv(&periods, &market, base, DayCount::Act365F)
            .expect("Periodized PV calculation should succeed in test");

        // Sum both
        let mut total_with_credit = 0.0;
        for period_map in pv_with_credit.values() {
            for money in period_map.values() {
                total_with_credit += money.amount();
            }
        }

        let mut total_no_credit = 0.0;
        for period_map in pv_no_credit.values() {
            for money in period_map.values() {
                total_no_credit += money.amount();
            }
        }

        // Credit-adjusted PV should be lower (survival probability < 1)
        assert!(
            total_with_credit < total_no_credit,
            "Credit-adjusted PV ({}) should be less than non-adjusted ({})",
            total_with_credit,
            total_no_credit
        );

        // The ratio should be reasonable (survival probability effect)
        let ratio = total_with_credit / total_no_credit;
        assert!(
            ratio > 0.95 && ratio < 1.0,
            "Credit adjustment ratio should be reasonable, got {}",
            ratio
        );
    }

    #[test]
    fn test_periodized_pv_credit_adjusted_without_hazard_matches_plain() {
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let market = create_test_market(base);
        let periods = create_quarters_2025();

        let pv_credit = bond
            .periodized_pv_credit_adjusted(&periods, &market, None, base, DayCount::Act365F)
            .expect_err("Credit-adjusted PV should require a hazard curve");

        let msg = pv_credit.to_string();
        assert!(msg.contains("hazard"), "Error should mention hazard: {msg}");
    }

    #[test]
    fn test_periodized_pv_credit_adjusted_matches_detailed_engine() {
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base)
            .knots([(0.0, 1.0), (1.0, 0.95)])
            .interp(InterpStyle::Linear)
            .build()
            .expect("DiscountCurve builder should succeed with valid test data");

        let hazard_curve = HazardCurve::builder("CORP-HAZARD")
            .base_date(base)
            .recovery_rate(0.35)
            .knots([(0.0, 0.0), (1.0, 0.02)])
            .build()
            .expect("HazardCurve builder should succeed with valid test data");

        let market = MarketContext::new()
            .insert_discount(disc_curve.clone())
            .insert_hazard(hazard_curve.clone());

        let periods = create_quarters_2025();
        let hazard_id = CurveId::new("CORP-HAZARD");

        let pv_credit = bond
            .periodized_pv_credit_adjusted(
                &periods,
                &market,
                Some(&hazard_id),
                base,
                DayCount::Act365F,
            )
            .expect("Credit-adjusted PV should succeed");

        let schedule = bond
            .build_full_schedule(&market, base)
            .expect("Schedule should build");

        let disc_ref: &dyn finstack_core::market_data::traits::Discounting = &disc_curve;
        let hazard_ref: &dyn finstack_core::market_data::traits::Survival = &hazard_curve;

        let date_ctx = DateContext::new(base, DayCount::Act365F, DayCountCtx::default());
        let detailed = schedule
            .pv_by_period_with_survival_and_ctx(
                &periods,
                disc_ref,
                Some(hazard_ref),
                Some(hazard_curve.recovery_rate()),
                date_ctx,
            )
            .expect("Detailed aggregation should succeed");

        assert_eq!(pv_credit, detailed);
    }

    #[test]
    fn test_periodized_pv_empty_periods_returns_empty() {
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let market = create_test_market(base);

        let pv_by_period = bond
            .periodized_pv(&[], &market, base, DayCount::Act365F)
            .expect("Periodized PV calculation should succeed in test");

        assert!(pv_by_period.is_empty());
    }

    #[test]
    fn test_periodized_pv_preserves_currency_separation() {
        // This test would require a multi-currency instrument
        // For now, we verify that the USD bond only produces USD PVs
        let bond = create_test_bond();
        let base = Date::from_calendar_date(2025, Month::January, 1).expect("Valid test date");
        let market = create_test_market(base);
        let periods = create_quarters_2025();

        let pv_by_period = bond
            .periodized_pv(&periods, &market, base, DayCount::Act365F)
            .expect("Periodized PV calculation should succeed in test");

        // All entries should be USD only
        for period_map in pv_by_period.values() {
            assert_eq!(period_map.len(), 1);
            assert!(period_map.contains_key(&Currency::USD));
        }
    }
}
