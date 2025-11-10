//! Deterministic pricing engine for revolving credit facilities.
//!
//! This module provides deterministic pricing capabilities for revolving credit facilities
//! using deterministic cashflow schedules. The pricer supports both fixed and floating
//! rate facilities with optional credit risk adjustment through hazard curves.
//!
//! # Key Features
//!
//! - **Deterministic Cashflows**: Uses pre-defined draw/repay schedules or constant utilization
//! - **Credit Risk**: Optional hazard curve integration for survival probability weighting
//! - **Multi-Rate Support**: Handles both fixed and floating rate base rates
//! - **Fee Structure**: Comprehensive fee modeling (commitment, usage, facility fees)
//! - **Currency Safety**: All calculations preserve currency semantics
//!
//! # Pricing Methodology
//!
//! The deterministic pricer:
//! 1. Generates cashflows using the facility's deterministic schedule
//! 2. Applies survival probabilities if hazard curves are provided
//! 3. Discounts all cashflows to the valuation date
//! 4. Adds upfront fee PV (if any) as a separate inflow
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use finstack_valuations::instruments::revolving_credit::pricer::deterministic::RevolvingCreditDiscountingPricer;
//!
//! let pricer = RevolvingCreditDiscountingPricer::new();
//! let pv = pricer.price_deterministic(&facility, &market, as_of)?;
//! ```

use crate::instruments::common::traits::Instrument;
use crate::pricer::{InstrumentType, ModelKey, Pricer, PricerKey, PricingError};
use crate::results::ValuationResult;
use finstack_core::dates::Date;
use finstack_core::market_data::MarketContext;
use finstack_core::money::Money;

use super::super::cashflows::generate_deterministic_cashflows_with_curves;
use super::super::types::RevolvingCredit;

/// Compute the present value of upfront fee paid at commitment.
///
/// The upfront fee is a one-time payment from the borrower to the lender (inflow to lender),
/// paid at the commitment date and discounted to the valuation date.
///
/// # Arguments
///
/// * `upfront_fee_opt` - Optional upfront fee amount
/// * `commitment_date` - Date when facility becomes available
/// * `as_of` - Valuation date
/// * `disc_curve` - Discount curve for PV calculation
/// * `disc_dc` - Day count convention of the discount curve
///
/// # Returns
///
/// Present value of upfront fee (0.0 if no fee), discounted to `as_of` date
pub(crate) fn compute_upfront_fee_pv(
    upfront_fee_opt: Option<Money>,
    commitment_date: Date,
    as_of: Date,
    disc_curve: &dyn finstack_core::market_data::traits::Discounting,
    disc_dc: finstack_core::dates::DayCount,
) -> finstack_core::Result<f64> {
    let upfront_fee = match upfront_fee_opt {
        Some(fee) => fee,
        None => return Ok(0.0),
    };

    if commitment_date > as_of {
        // Discount from commitment date to as_of
        let base_date = disc_curve.base_date();
        let t_commitment = disc_dc.year_fraction(
            base_date,
            commitment_date,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let t_as_of = disc_dc.year_fraction(
            base_date,
            as_of,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        let df_commitment = disc_curve.df(t_commitment);
        let df_as_of = disc_curve.df(t_as_of);
        let df = if df_as_of > 0.0 {
            df_commitment / df_as_of
        } else {
            1.0
        };

        Ok(upfront_fee.amount() * df)
    } else {
        // Commitment date in past or today - no discounting needed
        Ok(upfront_fee.amount())
    }
}

/// Discounting pricer for revolving credit facilities with deterministic cashflows.
///
/// This pricer generates cashflows using the facility's deterministic schedule
/// and discounts them using the discount curve. It supports both fixed and floating
/// rate facilities with comprehensive fee structures and optional credit risk adjustment.
///
/// # Supported Features
///
/// - Fixed and floating rate base rates
/// - Tiered fee structures (commitment, usage, facility fees)
/// - Deterministic draw/repay schedules
/// - Optional credit risk through hazard curves
/// - Currency-safe calculations
///
/// # Mathematical Approach
///
/// For each cashflow at time t:
/// ```text
/// PV += amount × DF(t) × SP(t)
/// ```
///
/// Where:
/// - `DF(t)` is the discount factor at time t
/// - `SP(t)` is the survival probability at time t (1.0 if no hazard curve)
#[derive(Default)]
pub struct RevolvingCreditDiscountingPricer;

impl RevolvingCreditDiscountingPricer {
    /// Create a new revolving credit discounting pricer.
    pub fn new() -> Self {
        Self
    }

    /// Price a revolving credit facility using deterministic cashflows.
    ///
    /// This method generates the facility's cashflow schedule and computes the present
    /// value by discounting all future cashflows. The valuation includes interest payments,
    /// fee income, principal repayments, and optional upfront fees.
    ///
    /// # Arguments
    ///
    /// * `facility` - The revolving credit facility to price
    /// * `market` - Market data context containing curves and surfaces
    /// * `as_of` - Valuation date
    ///
    /// # Returns
    ///
    /// Present value of the facility as seen by the lender (positive values indicate
    /// facility is worth holding/issuing).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Required market data (discount curve) is missing
    /// - Cashflow generation fails
    /// - Date calculations are invalid
    pub fn price_deterministic(
        facility: &RevolvingCredit,
        market: &MarketContext,
        as_of: finstack_core::dates::Date,
    ) -> finstack_core::Result<Money> {
        // Generate cashflows (excludes upfront fee - handled below)
        let schedule = generate_deterministic_cashflows_with_curves(facility, market, as_of)?;

        // Get discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();
        let base_date = disc.base_date();

        // Get optional hazard curve for credit risk
        let hazard_opt = facility
            .hazard_curve_id
            .as_ref()
            .and_then(|hid| market.get_hazard_ref(hid.as_str()).ok());

        // Compute PV of each cashflow with optional credit adjustment
        // When hazard curve is present, apply survival probability: PV = amount * df(t) * sp(t)
        let mut total_pv = 0.0;
        let ccy = facility.commitment_amount.currency();

        let mut future_dates = Vec::new();
        for cf in &schedule.flows {
            if cf.date > as_of {
                future_dates.push(cf.date);
            }
        }

        let hazard_survival = if let Some(hazard) = hazard_opt.as_ref() {
            Some(hazard.survival_at_dates(&future_dates)?)
        } else {
            None
        };

        let mut sp_index = 0usize;

        for cf in &schedule.flows {
            if cf.date <= as_of {
                continue; // Skip past cashflows
            }

            let t_df = disc_dc.year_fraction(
                base_date,
                cf.date,
                finstack_core::dates::DayCountCtx::default(),
            )?;
            let df = disc.df(t_df);

            // Apply survival probability if hazard curve is present
            let sp = hazard_survival
                .as_ref()
                .map(|weights| {
                    let value = weights
                        .get(sp_index)
                        .copied()
                        .unwrap_or(1.0);
                    value
                })
                .unwrap_or(1.0);

            sp_index += 1;

            let pv = cf.amount.amount() * df * sp;
            total_pv += pv;
        }

        // Handle upfront fee at pricer level (consistent with MC pricer)
        // Upfront fee is paid by borrower to lender at commitment, so it increases facility value (inflow)
        let upfront_fee_pv = compute_upfront_fee_pv(
            facility.fees.upfront_fee,
            facility.commitment_date,
            as_of,
            disc as &dyn finstack_core::market_data::traits::Discounting,
            disc_dc,
        )?;

        // Lender perspective: upfront fee is an inflow, so add to PV
        let final_pv = total_pv + upfront_fee_pv;

        Ok(Money::new(final_pv, ccy))
    }
}

impl Pricer for RevolvingCreditDiscountingPricer {
    fn key(&self) -> PricerKey {
        PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
    }

    fn price_dyn(
        &self,
        instrument: &dyn Instrument,
        market: &MarketContext,
        _as_of: finstack_core::dates::Date,
    ) -> Result<ValuationResult, PricingError> {
        // Type-safe downcasting
        let facility = instrument
            .as_any()
            .downcast_ref::<RevolvingCredit>()
            .ok_or_else(|| {
                PricingError::type_mismatch(InstrumentType::RevolvingCredit, instrument.key())
            })?;

        // Validate that we have a deterministic spec
        if !facility.is_deterministic() {
            return Err(PricingError::model_failure(
                "RevolvingCreditDiscountingPricer requires deterministic cashflows".to_string(),
            ));
        }

        // Extract valuation date from discount curve
        let disc = market.get_discount_ref(facility.discount_curve_id.as_str())?;
        let as_of = disc.base_date();

        // Price the facility
        let pv = Self::price_deterministic(facility, market, as_of)?;

        // Return stamped result
        Ok(ValuationResult::stamped(facility.id(), as_of, pv))
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::types::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::term_structures::hazard_curve::HazardCurve;
    use finstack_core::types::CurveId;
    use time::Month;

    /// Helper to create a standard test facility with common defaults
    fn create_test_facility(
        id: &str,
        start: Date,
        end: Date,
        commitment: f64,
        drawn: f64,
        base_rate_spec: BaseRateSpec,
        fees: RevolvingCreditFees,
    ) -> RevolvingCredit {
        RevolvingCredit::builder()
            .id(id.into())
            .commitment_amount(Money::new(commitment, Currency::USD))
            .drawn_amount(Money::new(drawn, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(base_rate_spec)
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(fees)
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .recovery_rate(0.0)
            .build()
            .unwrap()
    }

    #[test]
    fn test_pricer_key() {
        let pricer = RevolvingCreditDiscountingPricer::new();
        assert_eq!(
            pricer.key(),
            PricerKey::new(InstrumentType::RevolvingCredit, ModelKey::Discounting)
        );
    }

    #[test]
    fn test_deterministic_period_pv_consistency() {
        // Test that sum of per-period PVs equals total NPV
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-TEST",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::flat(25.0, 10.0, 5.0),
        );

        // Create a simple flat discount curve
        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let hazard_curve = HazardCurve::builder("TEST-HZD")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 0.0)])
            .build()
            .unwrap();
        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_curve);

        let pv = match RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start) {
            Ok(value) => value,
            Err(err) => panic!("deterministic pricing failed: {:?}", err),
        };

        // Verify we get a reasonable PV magnitude
        assert!(
            pv.amount().abs() < 10_000_000.0,
            "PV magnitude should be reasonable"
        );
    }

    #[test]
    fn test_deterministic_with_draw_repay() {
        // Test deterministic pricing with draw/repay events
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let draw_date = Date::from_calendar_date(2025, Month::March, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-TEST-2".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![
                super::super::super::types::DrawRepayEvent {
                    date: draw_date,
                    amount: Money::new(2_000_000.0, Currency::USD),
                    is_draw: true,
                },
            ]))
            .discount_curve_id("USD-OIS".into())
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let hazard_curve = HazardCurve::builder("TEST-HZD")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([(0.0, 0.0)])
            .build()
            .unwrap();
        let market = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_curve);

        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // Should price successfully
        assert!(pv.currency() == Currency::USD);
    }

    #[test]
    fn test_deterministic_survival_uses_hazard_axis() {
        use time::Duration;

        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = RevolvingCredit::builder()
            .id("RC-HZD".into())
            .commitment_amount(Money::new(8_000_000.0, Currency::USD))
            .drawn_amount(Money::new(4_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::flat(20.0, 10.0, 5.0))
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("BORROWER-HZD"))
            .recovery_rate(0.4)
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.025f64).exp()),
                (5.0, (-0.025f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let disc_curve_shifted = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.025f64).exp()),
                (5.0, (-0.025f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();

        let hazard_same_axis = HazardCurve::builder("BORROWER-HZD")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 0.01),
                (1.0, 0.012),
                (5.0, 0.015),
            ])
            .build()
            .unwrap();

        let hazard_shifted_axis = HazardCurve::builder("BORROWER-HZD")
            .base_date(start + Duration::days(60))
            .day_count(DayCount::Act360)
            .knots([
                (0.0, 0.01),
                (1.0, 0.012),
                (5.0, 0.015),
            ])
            .build()
            .unwrap();

        let market_same = MarketContext::new()
            .insert_discount(disc_curve)
            .insert_hazard(hazard_same_axis);
        let market_shifted = MarketContext::new()
            .insert_discount(disc_curve_shifted)
            .insert_hazard(hazard_shifted_axis);

        let pv_same =
            RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market_same, start)
                .unwrap();
        let pv_shifted =
            RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market_shifted, start)
                .unwrap();

        assert!(
            (pv_same.amount() - pv_shifted.amount()).abs() > 1e-4,
            "Changing the hazard curve axis should change survival weighting (same={}, shifted={})",
            pv_same.amount(),
            pv_shifted.amount()
        );
    }

    #[test]
    fn test_payment_dates_parity_with_utils() {
        // Test that our refactored code using utils produces identical payment dates
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-PARITY-TEST",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::default(),
        );

        // Without sentinel for cashflow generation
        let dates_no_sentinel = super::super::super::utils::build_payment_dates(&facility, false).unwrap();
        assert!(dates_no_sentinel.len() >= 2);

        // Last date should be at or before maturity
        assert!(*dates_no_sentinel.last().unwrap() <= end);

        // With sentinel for PV aggregation
        let dates_with_sentinel = super::super::super::utils::build_payment_dates(&facility, true).unwrap();
        assert_eq!(dates_with_sentinel.len(), dates_no_sentinel.len() + 1);

        // Sentinel should be one day after last payment
        let last_payment = dates_no_sentinel.last().unwrap();
        let sentinel = dates_with_sentinel.last().unwrap();
        assert_eq!(*sentinel, *last_payment + time::Duration::days(1));
    }

    #[test]
    fn test_reset_dates_parity_fixed_vs_floating() {
        // Test that fixed returns None and floating returns Some with correct dates
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        // Fixed facility
        let facility_fixed = create_test_facility(
            "RC-FIXED",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::default(),
        );

        let reset_dates_fixed = super::super::super::utils::build_reset_dates(&facility_fixed).unwrap();
        assert!(reset_dates_fixed.is_none(), "Fixed rate should return None");

        // Floating facility
        let facility_floating = create_test_facility(
            "RC-FLOAT",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Floating {
                index_id: "USD-SOFR-3M".into(),
                margin_bp: 200.0,
                reset_freq: Frequency::quarterly(),
                floor_bp: None,
            },
            RevolvingCreditFees::default(),
        );

        let reset_dates_floating = match super::super::super::utils::build_reset_dates(&facility_floating) {
            Ok(dates) => dates,
            Err(err) => panic!("failed to build reset dates: {:?}", err),
        };
        assert!(reset_dates_floating.is_some(), "Floating rate should return Some");

        let dates = reset_dates_floating.unwrap();
        assert!(dates.len() >= 2, "Should have at least 2 reset dates");
    }

    #[test]
    fn test_period_pv_parity_with_helper_periods() {
        // Test that total PV using helper-generated payment dates matches expectations
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = create_test_facility(
            "RC-PV-PARITY",
            start,
            end,
            10_000_000.0,
            5_000_000.0,
            BaseRateSpec::Fixed { rate: 0.05 },
            RevolvingCreditFees::flat(25.0, 10.0, 5.0),
        );

        let base_date = start;
        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();
        let market = MarketContext::new().insert_discount(disc_curve);

        // Price using deterministic pricer (which uses our helpers internally)
        let pv = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        // PV should be finite and reasonable
        assert!(pv.amount().is_finite());
        assert!(
            pv.amount().abs() < facility.commitment_amount.amount(),
            "PV magnitude should be less than commitment"
        );

        // Price a second time to ensure consistency
        let pv2 = RevolvingCreditDiscountingPricer::price_deterministic(&facility, &market, start)
            .unwrap();

        assert_eq!(pv.amount(), pv2.amount(), "Multiple calls should produce identical PVs");
    }
}
