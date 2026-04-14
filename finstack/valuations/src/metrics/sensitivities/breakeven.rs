//! Breakeven parameter shift calculator.
//!
//! Computes how much a valuation parameter (spread, yield, vol, correlation)
//! can move before carry + roll-down is wiped out over the configured horizon.

use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::Result;

/// Minimum absolute sensitivity value below which breakeven is undefined.
const SENSITIVITY_FLOOR: f64 = 1e-12;

/// Computes breakeven parameter shift from carry and sensitivity.
pub(crate) struct BreakevenCalculator;

impl MetricCalculator for BreakevenCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let config = context
            .metric_overrides
            .as_ref()
            .and_then(|o| o.breakeven_config)
            .ok_or_else(|| finstack_core::InputError::NotFound {
                id: "breakeven_config: set BreakevenConfig on MetricPricingOverrides".into(),
            })?;

        let carry_total = context
            .computed
            .get(&MetricId::CarryTotal)
            .copied()
            .ok_or_else(|| finstack_core::InputError::NotFound {
                id: "metric:carry_total".into(),
            })?;

        let sensitivity_id = config.target.sensitivity_metric();
        let sensitivity = context
            .computed
            .get(&sensitivity_id)
            .copied()
            .ok_or_else(|| finstack_core::InputError::NotFound {
                id: format!(
                    "metric:{}: compute {} alongside Breakeven",
                    sensitivity_id, sensitivity_id,
                ),
            })?;

        if sensitivity.abs() < SENSITIVITY_FLOOR {
            return Err(finstack_core::InputError::Invalid.into());
        }

        match config.mode {
            crate::instruments::BreakevenMode::Linear => {
                Ok(-carry_total / sensitivity)
            }
            crate::instruments::BreakevenMode::Iterative => {
                iterative_breakeven(context, carry_total, sensitivity, &config)
            }
        }
    }

    fn dependencies(&self) -> &[MetricId] {
        static DEPS: &[MetricId] = &[MetricId::CarryTotal];
        DEPS
    }
}

/// Placeholder for iterative mode — implemented in Task 4.
fn iterative_breakeven(
    _context: &MetricContext,
    _carry_total: f64,
    _sensitivity: f64,
    _config: &crate::instruments::BreakevenConfig,
) -> Result<f64> {
    Err(finstack_core::InputError::NotFound {
        id: "iterative breakeven: not yet implemented".into(),
    }
    .into())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::instruments::common_impl::traits::Instrument;
    use crate::instruments::{BreakevenConfig, BreakevenMode, BreakevenTarget};
    use crate::instruments::Bond;
    use finstack_core::config::FinstackConfig;
    use finstack_core::currency::Currency;
    use finstack_core::market_data::context::MarketContext;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use finstack_core::money::Money;
    use finstack_core::dates::DayCount;
    use std::sync::Arc;
    use time::macros::date;

    fn flat_discount_curve(id: &str, rate: f64, base_date: finstack_core::dates::Date) -> DiscountCurve {
        let knots: Vec<(f64, f64)> = (0..=20)
            .map(|i| {
                let t = i as f64 * 0.5;
                (t, (-rate * t).exp())
            })
            .collect();
        DiscountCurve::builder(id)
            .base_date(base_date)
            .day_count(DayCount::Act365F)
            .knots(knots)
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("flat discount curve")
    }

    fn context_with_carry_and_sensitivity(
        carry_total: f64,
        sensitivity: f64,
        target: BreakevenTarget,
        mode: BreakevenMode,
    ) -> MetricContext {
        let as_of = date!(2025 - 01 - 15);
        let bond = Bond::fixed(
            "TEST", Money::new(100.0, Currency::USD), 0.05,
            as_of, date!(2030 - 01 - 15), "USD-OIS",
        ).expect("bond");
        let market = MarketContext::new().insert(flat_discount_curve("USD-OIS", 0.04, as_of));
        let instrument: Arc<dyn Instrument> = Arc::new(bond);
        let base_value = instrument.value(&market, as_of).expect("pv");

        let mut ctx = MetricContext::new(
            instrument, Arc::new(market), as_of, base_value,
            Arc::new(FinstackConfig::default()),
        );
        ctx.computed.insert(MetricId::CarryTotal, carry_total);
        ctx.computed.insert(target.sensitivity_metric(), sensitivity);

        let overrides = crate::instruments::MetricPricingOverrides::default()
            .with_breakeven_config(BreakevenConfig { target, mode });
        ctx.set_metric_overrides(Some(overrides));
        ctx
    }

    #[test]
    fn test_linear_breakeven_positive_carry() {
        let mut ctx = context_with_carry_and_sensitivity(
            0.50, -0.04, BreakevenTarget::ZSpread, BreakevenMode::Linear,
        );
        let result = BreakevenCalculator.calculate(&mut ctx).expect("breakeven");
        assert!((result - 12.5).abs() < 1e-10, "got {result}");
    }

    #[test]
    fn test_linear_breakeven_negative_carry() {
        let mut ctx = context_with_carry_and_sensitivity(
            -0.30, -0.04, BreakevenTarget::ZSpread, BreakevenMode::Linear,
        );
        let result = BreakevenCalculator.calculate(&mut ctx).expect("breakeven");
        assert!((result - (-7.5)).abs() < 1e-10, "got {result}");
    }

    #[test]
    fn test_linear_breakeven_zero_sensitivity_returns_error() {
        let mut ctx = context_with_carry_and_sensitivity(
            0.50, 0.0, BreakevenTarget::ZSpread, BreakevenMode::Linear,
        );
        let result = BreakevenCalculator.calculate(&mut ctx);
        assert!(result.is_err(), "zero sensitivity should error");
    }

    #[test]
    fn test_missing_sensitivity_returns_error() {
        let as_of = date!(2025 - 01 - 15);
        let bond = Bond::fixed(
            "TEST", Money::new(100.0, Currency::USD), 0.05,
            as_of, date!(2030 - 01 - 15), "USD-OIS",
        ).expect("bond");
        let market = MarketContext::new().insert(flat_discount_curve("USD-OIS", 0.04, as_of));
        let instrument: Arc<dyn Instrument> = Arc::new(bond);
        let base_value = instrument.value(&market, as_of).expect("pv");

        let mut ctx = MetricContext::new(
            instrument, Arc::new(market), as_of, base_value,
            Arc::new(FinstackConfig::default()),
        );
        ctx.computed.insert(MetricId::CarryTotal, 0.50);
        let overrides = crate::instruments::MetricPricingOverrides::default()
            .with_breakeven_config(BreakevenConfig {
                target: BreakevenTarget::ZSpread,
                mode: BreakevenMode::Linear,
            });
        ctx.set_metric_overrides(Some(overrides));

        let result = BreakevenCalculator.calculate(&mut ctx);
        assert!(result.is_err(), "missing sensitivity should error");
    }

    #[test]
    fn test_missing_config_returns_error() {
        let as_of = date!(2025 - 01 - 15);
        let bond = Bond::fixed(
            "TEST", Money::new(100.0, Currency::USD), 0.05,
            as_of, date!(2030 - 01 - 15), "USD-OIS",
        ).expect("bond");
        let market = MarketContext::new().insert(flat_discount_curve("USD-OIS", 0.04, as_of));
        let instrument: Arc<dyn Instrument> = Arc::new(bond);
        let base_value = instrument.value(&market, as_of).expect("pv");

        let mut ctx = MetricContext::new(
            instrument, Arc::new(market), as_of, base_value,
            Arc::new(FinstackConfig::default()),
        );
        ctx.computed.insert(MetricId::CarryTotal, 0.50);
        ctx.computed.insert(MetricId::Cs01, -0.04);

        let result = BreakevenCalculator.calculate(&mut ctx);
        assert!(result.is_err(), "missing breakeven config should error");
    }

    #[test]
    fn test_linear_breakeven_ytm_target() {
        let mut ctx = context_with_carry_and_sensitivity(
            0.25, -0.05, BreakevenTarget::Ytm, BreakevenMode::Linear,
        );
        let result = BreakevenCalculator.calculate(&mut ctx).expect("breakeven");
        assert!((result - 5.0).abs() < 1e-10, "got {result}");
    }
}
