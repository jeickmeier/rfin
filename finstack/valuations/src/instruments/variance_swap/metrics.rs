//! Metrics calculators for variance swaps.

use finstack_core::{F, Result};
use crate::{
    metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry},
    instruments::traits::Priceable,
};
use super::types::VarianceSwap;

/// Calculate variance notional.
pub struct VarianceNotionalCalculator;

impl MetricCalculator for VarianceNotionalCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        Ok(swap.notional.amount())
    }
}

/// Calculate vega (sensitivity to 1% change in volatility).
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        
        // Vega for variance swap = 2 * Notional * σ * ∂σ
        // Where ∂σ = 0.01 for 1% vol move
        // σ is the current implied volatility
        
        // Try to get current implied vol
        let current_vol = if let Ok(scalar) = context.curves.price(format!("{}_IMPL_VOL", swap.underlying_id)) {
            match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(vol) => *vol,
                finstack_core::market_data::scalars::MarketScalar::Price(price) => price.amount(),
            }
        } else {
            // Use strike vol as approximation
            swap.strike_variance.sqrt()
        };
        
        // Vega per 1% vol move
        let vega = 2.0 * swap.notional.amount() * current_vol * 0.01;
        
        // Apply side
        Ok(vega * swap.side.sign())
    }
}

/// Calculate variance vega (sensitivity to 1 point change in variance).
pub struct VarianceVegaCalculator;

impl MetricCalculator for VarianceVegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        // Variance vega is simply the notional for a variance swap
        Ok(swap.notional.amount() * swap.side.sign())
    }
}

/// Calculate the current realized variance to date.
pub struct RealizedVarianceCalculator;

impl MetricCalculator for RealizedVarianceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;
        
        if as_of < swap.start_date {
            // Not started yet
            return Ok(0.0);
        }
        
        // Get historical prices and calculate realized variance
        // In a real implementation, this would fetch actual price data
        if let Ok(_scalar) = context.curves.price(&swap.underlying_id) {
            // For now, return a placeholder
            // In practice, would fetch full price history
            Ok(swap.strike_variance * 0.95) // Example: slightly below strike
        } else {
            Ok(0.0)
        }
    }
}

/// Calculate the expected variance (blend of realized and forward).
pub struct ExpectedVarianceCalculator;

impl MetricCalculator for ExpectedVarianceCalculator {
    fn dependencies(&self) -> &[MetricId] {
        &[] // TODO: Add RealizedVariance when metric ID is added
    }
    
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;
        
        if as_of >= swap.maturity {
            // Use realized variance
            // TODO: Get from metric when RealizedVariance is added
            return Ok(swap.strike_variance * 0.95);
        }
        
        if as_of < swap.start_date {
            // Use forward variance (implied)
            if let Ok(scalar) = context.curves.price(format!("{}_IMPL_VOL", swap.underlying_id)) {
                let vol = match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                    finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
                };
                return Ok(vol * vol);
            }
            return Ok(swap.strike_variance);
        }
        
        // Blend realized and forward
        // TODO: Get realized from metric when RealizedVariance is added
        let realized = swap.strike_variance * 0.95;
        let forward = if let Ok(scalar) = context.curves.price(format!("{}_IMPL_VOL", swap.underlying_id)) {
            let vol = match scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(v) => *v,
                finstack_core::market_data::scalars::MarketScalar::Price(p) => p.amount(),
            };
            vol * vol
        } else {
            swap.strike_variance
        };
        
        let weight = swap.time_elapsed_fraction(as_of);
        Ok(realized * weight + forward * (1.0 - weight))
    }
}

/// Calculate DV01 (sensitivity to 1bp move in interest rates).
pub struct Dv01Calculator;

impl MetricCalculator for Dv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;
        
        if as_of >= swap.maturity {
            // No interest rate sensitivity after maturity
            return Ok(0.0);
        }
        
        // Get current PV
        let pv = swap.value(&context.curves, as_of)?;
        
        // Time to maturity
        let ttm = swap.day_count.year_fraction(as_of, swap.maturity, Default::default())?;
        
        // DV01 approximation: PV * ttm * 0.0001
        Ok(pv.amount().abs() * ttm * 0.0001)
    }
}

/// Calculate strike in volatility terms.
pub struct StrikeVolCalculator;

impl MetricCalculator for StrikeVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        Ok(swap.strike_variance.sqrt())
    }
}

/// Calculate time to maturity in years.
pub struct TimeToMaturityCalculator;

impl MetricCalculator for TimeToMaturityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;
        
        if as_of >= swap.maturity {
            return Ok(0.0);
        }
        
        swap.day_count.year_fraction(as_of, swap.maturity, Default::default())
    }
}

/// Register variance swap metrics with the registry.
pub fn register_variance_swap_metrics(registry: &mut MetricRegistry) {
    use std::sync::Arc;
    
    // Note: These metric IDs would need to be added to the MetricId enum
    // For now, using existing ones as placeholders
    registry.register_metric(
        MetricId::Vega,
        Arc::new(VegaCalculator),
        &["VarianceSwap"],
    );
    registry.register_metric(
        MetricId::Dv01,
        Arc::new(Dv01Calculator),
        &["VarianceSwap"],
    );
    // Additional metrics would be registered once their IDs are added
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::variance_swap::VarianceSwap;
    use finstack_core::dates::{DayCount, Frequency};
    use finstack_core::math::stats::RealizedVarMethod;
    use crate::instruments::variance_swap::PayReceive;
    use finstack_core::{
        currency::Currency,
        dates::Date,
        market_data::context::MarketContext,
        money::Money,
    };
    use std::sync::Arc;

    fn test_date(year: i32, month: u8, day: u8) -> Date {
        Date::from_calendar_date(year, time::Month::try_from(month).unwrap(), day).unwrap()
    }

    #[test]
    fn test_variance_notional_calculator() {
        let swap = VarianceSwap::builder()
            .id("VAR_TEST".into())
            .underlying_id("SPX".to_string())
            .notional(Money::new(100_000.0, Currency::USD))
            .strike_variance(0.20 * 0.20)
            .start_date(test_date(2025, 1, 1))
            .maturity(test_date(2026, 1, 1))
            .observation_freq(Frequency::daily())
            .realized_var_method(RealizedVarMethod::CloseToClose)
            .side(PayReceive::Receive)
            .disc_id("USD_OIS".into())
            .day_count(DayCount::Act365F)
            .attributes(crate::instruments::traits::Attributes::new())
            .build()
            .unwrap();

        let market_context = MarketContext::new();
        let mut metric_context = MetricContext::new(
            Arc::new(swap),
            Arc::new(market_context),
            test_date(2025, 6, 1),
            Money::new(0.0, Currency::USD),
        );

        let calculator = VarianceNotionalCalculator;
        let result = calculator.calculate(&mut metric_context).unwrap();
        assert_eq!(result, 100_000.0);
    }

    #[test]
    fn test_strike_vol_calculator() {
        let swap = VarianceSwap::builder()
            .id("VAR_TEST".into())
            .underlying_id("SPX".to_string())
            .notional(Money::new(100_000.0, Currency::USD))
            .strike_variance(0.20 * 0.20)
            .start_date(test_date(2025, 1, 1))
            .maturity(test_date(2026, 1, 1))
            .observation_freq(Frequency::daily())
            .realized_var_method(RealizedVarMethod::CloseToClose)
            .side(PayReceive::Receive)
            .disc_id("USD_OIS".into())
            .day_count(DayCount::Act365F)
            .attributes(crate::instruments::traits::Attributes::new())
            .build()
            .unwrap();

        let market_context = MarketContext::new();
        let mut metric_context = MetricContext::new(
            Arc::new(swap),
            Arc::new(market_context),
            test_date(2025, 6, 1),
            Money::new(0.0, Currency::USD),
        );

        let calculator = StrikeVolCalculator;
        let result = calculator.calculate(&mut metric_context).unwrap();
        assert!((result - 0.20).abs() < 1e-10);
    }
}
