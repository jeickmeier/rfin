//! Unified spread sensitivity metric for revolving credit facilities.
//!
//! Provides both DV01 (discount curve sensitivity) and CS01 (credit spread sensitivity).
//! For revolving credit, these are equivalent since the discount curve incorporates credit risk.

use crate::instruments::RevolvingCredit;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::DayCountCtx;

/// Calculator for spread sensitivity (DV01/CS01).
///
/// Uses numerical differentiation: Sensitivity = (PV_down - PV_up) / 2 where
/// PV_up is computed with a +1bp spread bump and PV_down with -1bp.
///
/// This unified implementation serves both DV01 and CS01 since revolving credit
/// discounting incorporates credit risk directly in the discount curve.
#[derive(Debug, Default, Clone, Copy)]
pub struct SpreadSensitivityCalculator;

impl MetricCalculator for SpreadSensitivityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> finstack_core::Result<f64> {
        let facility: &RevolvingCredit = context.instrument_as()?;
        let as_of = context.as_of;

        // Get base curves
        let disc = context
            .curves
            .get_discount_ref(facility.discount_curve_id.as_str())?;
        let disc_dc = disc.day_count();

        // Generate cashflows
        use crate::instruments::revolving_credit::cashflow_engine::CashflowEngine;
        let engine = CashflowEngine::new(facility, Some(context.curves.as_ref()), as_of)?;
        let path_schedule = engine.generate_deterministic()?;
        let schedule = path_schedule.schedule;

        // Compute PV with spread bumps
        let bump_bp = 0.0001; // 1bp
        let mut npv_up = 0.0;
        let mut npv_down = 0.0;

        for cf in &schedule.flows {
            if cf.date <= as_of {
                continue;
            }

            let yf = disc_dc.year_fraction(disc.base_date(), cf.date, DayCountCtx::default())?;
            let df_base = disc.df(yf);

            // Apply spread bumps
            let df_up = df_base * (-bump_bp * yf).exp();
            let df_down = df_base * (bump_bp * yf).exp();

            npv_up += cf.amount.amount() * df_up;
            npv_down += cf.amount.amount() * df_down;
        }

        // Sensitivity magnitude per 1bp: use symmetric difference and return positive magnitude
        let sensitivity = ((npv_down - npv_up) / 2.0).abs();

        Ok(sensitivity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::revolving_credit::{BaseRateSpec, DrawRepaySpec, RevolvingCreditFees};
    use crate::metrics::MetricContext;
    use finstack_core::currency::Currency;
    use finstack_core::dates::{Date, DayCount, Frequency};
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::market_data::MarketContext;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use time::Month;

    #[test]
    fn test_sensitivity_positive() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2026, Month::January, 1).unwrap();

        let facility = crate::instruments::RevolvingCredit::builder()
            .id("RC-SENS".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("TEST-HZD"))
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();

        let market = MarketContext::new().insert_discount(disc_curve);

        let calculator = SpreadSensitivityCalculator;
        let mut context = MetricContext::new(
            std::sync::Arc::new(facility),
            std::sync::Arc::new(market),
            start,
            Money::new(0.0, Currency::USD), // base_value placeholder
        );

        let sensitivity = calculator.calculate(&mut context).unwrap();

        // Sensitivity should be positive
        assert!(sensitivity > 0.0, "Sensitivity should be positive");

        // For a 1-year facility with ~5M drawn at 5%, sensitivity should be reasonable
        // Rough estimate: PV01 ≈ Duration × PV × 0.0001
        // Duration ≈ 0.5 years (quarterly payments), PV ≈ few hundred k in fees/interest
        assert!(
            sensitivity < 100_000.0,
            "Sensitivity seems unreasonably high: {}",
            sensitivity
        );
    }

    #[test]
    fn test_sensitivity_increases_with_maturity() {
        let start = Date::from_calendar_date(2025, Month::January, 1).unwrap();
        let end_1y = Date::from_calendar_date(2026, Month::January, 1).unwrap();
        let end_5y = Date::from_calendar_date(2030, Month::January, 1).unwrap();

        let facility_1y = crate::instruments::RevolvingCredit::builder()
            .id("RC-1Y".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end_1y)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("TEST-HZD"))
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let facility_5y = crate::instruments::RevolvingCredit::builder()
            .id("RC-5Y".into())
            .commitment_amount(Money::new(10_000_000.0, Currency::USD))
            .drawn_amount(Money::new(5_000_000.0, Currency::USD))
            .commitment_date(start)
            .maturity_date(end_5y)
            .base_rate_spec(BaseRateSpec::Fixed { rate: 0.05 })
            .day_count(DayCount::Act360)
            .payment_frequency(Frequency::quarterly())
            .fees(RevolvingCreditFees::default())
            .draw_repay_spec(DrawRepaySpec::Deterministic(vec![]))
            .discount_curve_id("USD-OIS".into())
            .hazard_curve_id(CurveId::from("TEST-HZD"))
            .recovery_rate(0.0)
            .build()
            .unwrap();

        let disc_curve = DiscountCurve::builder("USD-OIS")
            .base_date(start)
            .day_count(DayCount::Act365F)
            .knots([
                (0.0, 1.0),
                (1.0, (-0.03f64).exp()),
                (5.0, (-0.03f64 * 5.0).exp()),
            ])
            .build()
            .unwrap();

        let market = MarketContext::new().insert_discount(disc_curve);
        let market_arc = std::sync::Arc::new(market);

        let calculator = SpreadSensitivityCalculator;

        let mut context_1y = MetricContext::new(
            std::sync::Arc::new(facility_1y),
            market_arc.clone(),
            start,
            Money::new(0.0, Currency::USD), // base_value placeholder
        );
        let sensitivity_1y = calculator.calculate(&mut context_1y).unwrap();

        let mut context_5y = MetricContext::new(
            std::sync::Arc::new(facility_5y),
            market_arc,
            start,
            Money::new(0.0, Currency::USD), // base_value placeholder
        );
        let sensitivity_5y = calculator.calculate(&mut context_5y).unwrap();

        // Longer maturity should have higher sensitivity (more interest/fee cashflows)
        assert!(
            sensitivity_5y > sensitivity_1y,
            "5Y sensitivity ({}) should be greater than 1Y sensitivity ({})",
            sensitivity_5y,
            sensitivity_1y
        );
    }
}
