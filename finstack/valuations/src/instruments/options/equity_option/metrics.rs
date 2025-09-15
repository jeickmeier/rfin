//! Equity option specific metrics calculators

use crate::instruments::options::equity_option::EquityOption;
use crate::instruments::options::OptionType;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use finstack_core::market_data::traits::Discount;
use std::sync::Arc;

/// Delta calculator for equity options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &EquityOption = context.instrument_as()?;

        // Calculate time to expiry
        let time_to_expiry = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_expiry <= 0.0 {
            // Option expired - delta is 0 or 1/-1 based on moneyness
            let spot_scalar = context.curves.price(&option.spot_id)?;
            let spot = match spot_scalar {
                finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                finstack_core::market_data::scalars::MarketScalar::Price(money) => {
                    money.amount()
                }
            };

            return Ok(match option.option_type {
                OptionType::Call => {
                    if spot > option.strike.amount() {
                        1.0
                    } else {
                        0.0
                    }
                }
                OptionType::Put => {
                    if spot < option.strike.amount() {
                        -1.0
                    } else {
                        0.0
                    }
                }
            });
        }

        // Get market data
        let disc_curve = context.curves.discount_ref(&option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface_ref(&option.vol_id)?;
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

        let time_to_expiry = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern as Delta)
        let disc_curve = context.curves.discount_ref(&option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface_ref(&option.vol_id)?;
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

        let time_to_expiry = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount_ref(&option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface_ref(&option.vol_id)?;
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

        let time_to_expiry = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount_ref(&option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface_ref(&option.vol_id)?;
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

        let time_to_expiry = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Get market data (same pattern)
        let disc_curve = context.curves.discount_ref(&option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        let sigma = if let Some(impl_vol) = option.pricing_overrides.implied_volatility {
            impl_vol
        } else {
            let vol_surface = context.curves.surface(&option.vol_id)?;
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
        let option: &EquityOption = context.instrument_as()?;

        let t = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Gather market inputs
        let disc_curve = context.curves.discount(&option.disc_id)?;
        let r = disc_curve.zero(t);

        let spot_scalar = context.curves.price(&option.spot_id)?;
        let spot = match spot_scalar {
            finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
            finstack_core::market_data::scalars::MarketScalar::Price(money) => money.amount(),
        };

        let q = if let Some(div_id) = &option.div_yield_id {
            match context.curves.price(div_id.as_str()) {
                Ok(scalar) => match scalar {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(_) => 0.0,
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // Obtain market price from attributes or a market scalar id stored in attributes
        let market_price: F = if let Some(p) = option.attributes.get_meta("market_price") {
            p.parse().unwrap_or(0.0)
        } else if let Some(price_id) = option.attributes.get_meta("market_price_id") {
            match context.curves.price(price_id) {
                Ok(ms) => match ms {
                    finstack_core::market_data::scalars::MarketScalar::Unitless(val) => *val,
                    finstack_core::market_data::scalars::MarketScalar::Price(m) => m.amount(),
                },
                Err(_) => 0.0,
            }
        } else {
            0.0
        };

        // If no market price available, return 0.0 (caller should populate attributes/meta)
        if market_price <= 0.0 {
            return Ok(0.0);
        }

        // Bracketed solver (bisection with occasional Newton steps) for robustness and determinism
        let mut lo = 1e-6;
        let mut hi = 3.0;
        let tol = 1e-8;
        let max_iter = 100;

        // Helper: BS price at sigma
        let price_at = |sigma: F| -> F {
            option
                .black_scholes_price(spot, r, sigma, t, q)
                .map(|m| m.amount())
                .unwrap_or(0.0)
        };

        // Ensure bracket contains a root
        let mut f_lo = price_at(lo) - market_price;
        let mut f_hi = price_at(hi) - market_price;
        if f_lo * f_hi > 0.0 {
            // Expand hi if needed up to a cap
            let mut k = 0;
            while f_lo * f_hi > 0.0 && hi < 10.0 && k < 10 {
                hi *= 1.5;
                f_hi = price_at(hi) - market_price;
                k += 1;
            }
            if f_lo * f_hi > 0.0 {
                return Ok(0.0);
            }
        }

        let mut mid = 0.5 * (lo + hi);
        for _ in 0..max_iter {
            mid = 0.5 * (lo + hi);
            let f_mid = price_at(mid) - market_price;

            if f_mid.abs() < tol || (hi - lo) < tol {
                return Ok(mid);
            }

            // Try a guarded Newton step using closed-form vega if available
            let vega_per_1pct = option.vega(spot, r, mid, t, q) * option.contract_size; // per 1% vol
            let vega = vega_per_1pct * 100.0; // per absolute vol
            if vega.abs() > 1e-12 {
                let newton = mid - f_mid / vega;
                if newton.is_finite() && newton > lo && newton < hi {
                    mid = newton;
                    // Narrow bracket around newton step
                    let f_new = price_at(mid) - market_price;
                    if f_lo * f_new <= 0.0 {
                        hi = mid;
                        let _ = f_new; // maintain readability; hi updated
                    } else {
                        lo = mid;
                        f_lo = f_new;
                    }
                    continue;
                }
            }

            // Bisection update
            if f_lo * f_mid <= 0.0 {
                hi = mid;
                let _ = f_mid; // keep bracket, avoid unused assignment lint
            } else {
                lo = mid;
                f_lo = f_mid;
            }
        }

        Ok(mid)
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
