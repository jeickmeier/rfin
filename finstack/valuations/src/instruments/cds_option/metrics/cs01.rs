//! CDS Option CS01 metric calculator.
//!
//! CS01 measures the change in option value for a 1 basis point change in the
//! credit spread. For CDS options, this is a key risk metric as the option value
//! is highly sensitive to changes in the underlying CDS spread.
//!
//! # Market Standard
//!
//! Market-standard CS01 should bump the underlying CDS spread by 1bp and reprice.
//! This implementation uses an approximation based on delta and duration for
//! portfolio-level risk measurement. For precise CS01, consider implementing
//! finite differences with bumped hazard curves.

use crate::constants::ONE_BASIS_POINT;
use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// CS01 calculator for CDS Option instruments.
///
/// Approximates CS01 using option delta scaled by a typical CDS sensitivity.
/// For a more precise calculation, this could use finite differences with
/// bumped hazard curves, but for portfolio-level risk this approximation
/// is typically sufficient.
pub struct Cs01Calculator;

impl MetricCalculator for Cs01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let cds_option: &CdsOption = context.instrument_as()?;
        let as_of = context.as_of;

        if as_of >= cds_option.expiry {
            return Ok(0.0);
        }

        // Calculate time to expiry and CDS duration
        let time_to_expiry = cds_option
            .day_count
            .year_fraction(
                as_of,
                cds_option.expiry,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        let cds_duration = cds_option
            .day_count
            .year_fraction(
                as_of,
                cds_option.cds_maturity,
                finstack_core::dates::DayCountCtx::default(),
            )?
            .max(0.0);

        // Try to use delta if available (more precise)
        let delta_opt = context.computed.get(&MetricId::Delta).copied();

        let cs01 = if let Some(delta) = delta_opt {
            if delta != 0.0 {
                // If we have a valid delta, use it: CS01 = Delta × Notional × Duration × 1bp
                delta.abs() * cds_option.notional.amount() * cds_duration * ONE_BASIS_POINT
            } else {
                // Delta is zero, fall back to approximation
                use_approximation(
                    cds_option,
                    time_to_expiry,
                    cds_duration,
                    context.base_value.amount(),
                )
            }
        } else {
            // Delta not available, use approximation based on option value
            use_approximation(
                cds_option,
                time_to_expiry,
                cds_duration,
                context.base_value.amount(),
            )
        };

        Ok(cs01)
    }

    fn dependencies(&self) -> &[MetricId] {
        // CS01 can optionally use delta if available, but doesn't require it
        &[]
    }
}

/// Approximate CS01 when delta is not available or is zero.
///
/// Uses a simplified model: CS01 ≈ Option_Value × (Duration / Time_to_Expiry) × sensitivity_factor
fn use_approximation(
    cds_option: &CdsOption,
    time_to_expiry: f64,
    cds_duration: f64,
    option_value: f64,
) -> f64 {
    if time_to_expiry <= 0.0 {
        return 0.0;
    }

    // Rough approximation: CS01 is proportional to option value scaled by duration
    // For ATM options, CS01 ≈ 0.5 * Notional * Duration * 1bp
    // We scale by (Value/Strike) as a proxy for moneyness
    let strike_value = cds_option.notional.amount() * (cds_option.strike_spread_bp / 10_000.0);
    let moneyness_factor = if strike_value > 0.0 {
        (option_value / strike_value).clamp(0.1, 1.0)
    } else {
        0.5 // Default to ATM assumption
    };

    // CS01 approximation
    moneyness_factor * cds_option.notional.amount() * cds_duration * ONE_BASIS_POINT
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::cds_option::CdsOption;
    use crate::instruments::{ExerciseStyle, OptionType, PricingOverrides, SettlementType};
    use finstack_core::prelude::*;
    use time::macros::date;

    #[test]
    fn test_cs01_positive_for_call() {
        let option = CdsOption {
            id: "TEST_CDSOPT".into(),
            strike_spread_bp: 100.0,
            option_type: OptionType::Call,
            exercise_style: ExerciseStyle::European,
            expiry: date!(2025 - 01 - 01),
            cds_maturity: date!(2029 - 01 - 01),
            day_count: DayCount::Act360,
            notional: Money::new(10_000_000.0, Currency::USD),
            settlement: SettlementType::Cash,
            recovery_rate: 0.4,
            disc_id: "USD_OIS".into(),
            credit_id: "CORP_HAZARD".into(),
            vol_id: "CDS_VOL".into(),
            pricing_overrides: PricingOverrides::default(),
            attributes: Default::default(),
            underlying_is_index: false,
            index_factor: None,
            forward_spread_adjust_bp: 0.0,
        };

        let market = MarketContext::new();
        let as_of = date!(2024 - 01 - 01);
        let base_value = Money::new(50000.0, Currency::USD);

        let mut context = MetricContext::new(
            std::sync::Arc::new(option),
            std::sync::Arc::new(market),
            as_of,
            base_value,
        );

        // Simulate delta being computed
        context.computed.insert(MetricId::Delta, 0.5);

        let calculator = Cs01Calculator;
        let result = calculator.calculate(&mut context);

        assert!(result.is_ok());
        let cs01 = result.unwrap();

        // Should be positive and reasonable
        assert!(cs01 > 0.0, "CS01 should be positive for a long call");
        assert!(cs01 < 100_000.0, "CS01 should be reasonable in magnitude");
    }
}
