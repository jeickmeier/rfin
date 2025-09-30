//! Implied volatility metric for swaptions.
//!
//! Solves for the Black implied volatility that reproduces the current PV
//! (from `context.base_value`) using the `/math` solvers. Uses a robust
//! parameterization in log-vol space and falls back to reasonable defaults
//! if inversion is not possible.

use crate::instruments::swaption::Swaption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::math::solver::{HybridSolver, Solver};
use finstack_core::prelude::Result;

/// Implied Volatility calculator for swaptions
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &Swaption = context.instrument_as()?;

        // Fetch discount curve
        let disc = context.curves.get_discount_ref(option.disc_id.as_ref())?;

        // Time to expiry from as_of
        let t = option.year_fraction(context.as_of, option.expiry, option.day_count)?;
        if t <= 0.0 {
            return Ok(0.0);
        }

        // Target price is the base PV already computed under instrument pricing
        let target_pv = context.base_value.amount();

        // Build objective in log-vol space x = ln(sigma)
        let f = |x: f64| -> f64 {
            let sigma = x.exp();
            // Use Black pricing along the same path as instrument pricing (not SABR)
            // since we are solving for the equivalent Black vol.
            match option.price_black(disc, sigma, context.as_of) {
                Ok(m) => m.amount() - target_pv,
                Err(_) => 1.0e6, // steer solver away from invalid regions
            }
        };

        // Initial guess: overrides -> SABR ATM -> surface -> 20%
        let forward = option.forward_swap_rate(disc, context.as_of)?;
        let initial_sigma = if let Some(ov) = option.pricing_overrides.implied_volatility {
            ov
        } else if let Some(sabr) = &option.sabr_params {
            let model = crate::instruments::common::models::SABRModel::new(sabr.clone());
            model
                .implied_volatility(forward, option.strike_rate, t)
                .unwrap_or(0.2)
        } else {
            context
                .curves
                .surface_ref(option.vol_id)
                .map(|s| s.value_clamped(t, option.strike_rate))
                .unwrap_or(0.2)
        };

        let eps = 1e-8;
        let x0 = (initial_sigma.max(eps)).ln();

        // Try hybrid solver (Newton with Brent fallback)
        let solver = HybridSolver::new()
            .with_tolerance(1e-10)
            .with_max_iterations(100);
        let implied_x = match solver.solve(f, x0) {
            Ok(root) => root,
            Err(_) => {
                // Fallback: sample two bounds and pick the closer one
                let x_lo = (1e-6_f64).ln();
                let x_hi = (3.0_f64).ln();
                let flo = f(x_lo).abs();
                let fhi = f(x_hi).abs();
                if flo <= fhi {
                    x_lo
                } else {
                    x_hi
                }
            }
        };

        let sigma = implied_x.exp();
        Ok(sigma)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
