//! Equity option specific metrics calculators

use crate::instruments::options::equity_option::EquityOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for equity options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        // Calculate time to expiry
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            // Option expired - delta is 0 or 1/-1 based on moneyness
            let spot_scalar = context.curves.market_scalar(option.spot_id)?;
            let spot = match spot_scalar {
                finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                finstack_core::market_data::primitives::MarketScalar::Price(money) => {
                    money.amount()
                }
            };

            return Ok(match option.option_type {
                super::OptionType::Call => {
                    if spot > option.strike.amount() {
                        1.0
                    } else {
                        0.0
                    }
                }
                super::OptionType::Put => {
                    if spot < option.strike.amount() {
                        -1.0
                    } else {
                        0.0
                    }
                }
            });
        }

        // Get market data
        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.market_scalar(option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = option.div_yield_id {
            match context.curves.market_scalar(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike.amount())
        };

        // Calculate delta using existing method
        Ok(option.delta(spot, r, sigma, time_to_expiry, q))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for equity options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern as Delta)
        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.market_scalar(option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = option.div_yield_id {
            match context.curves.market_scalar(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike.amount())
        };

        Ok(option.gamma(spot, r, sigma, time_to_expiry, q))
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for equity options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.market_scalar(option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = option.div_yield_id {
            match context.curves.market_scalar(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike.amount())
        };

        // Scale vega by contract size for full cash vega
        Ok(option.vega(spot, r, sigma, time_to_expiry, q) * option.contract_size)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for equity options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.market_scalar(option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = option.div_yield_id {
            match context.curves.market_scalar(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike.amount())
        };

        // Scale theta by contract size for full cash theta
        Ok(option.theta(spot, r, sigma, time_to_expiry, q) * option.contract_size)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for equity options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.market_scalar(option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::primitives::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = option.div_yield_id {
            match context.curves.market_scalar(div_id) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::primitives::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::primitives::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            let vol_surface = context.curves.vol_surface(option.vol_id)?;
            vol_surface.value_clamped(time_to_expiry, option.strike.amount())
        };

        // Scale rho by contract size for full cash rho
        Ok(option.rho(spot, r, sigma, time_to_expiry, q) * option.contract_size)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
/// Implied Volatility calculator for equity options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &EquityOption = context.instrument_as()?;
        // Requires market price and inputs; placeholder returns 0.0 until pricer wiring provides price
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register equity option metrics with the registry
pub fn register_equity_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["EquityOption"]);

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["EquityOption"],
    );

    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["EquityOption"]);

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["EquityOption"],
    );
}
