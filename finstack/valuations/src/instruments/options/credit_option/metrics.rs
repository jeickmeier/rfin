//! Credit option specific metrics calculators

use crate::instruments::options::credit_option::CreditOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId, MetricRegistry};
use finstack_core::{Result, F};
use std::sync::Arc;

/// Delta calculator for credit options
pub struct DeltaCalculator;

impl MetricCalculator for DeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CreditOption = context.instrument_as()?;
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        let hazard_curve = context.curves.hazard(option.credit_id)?;
        let current_tenor = option
            .day_count
            .year_fraction(context.as_of, option.cds_maturity)?;
        let forward_spread_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            option.strike_spread_bp
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context
                .curves
                .vol_surface(option.vol_id)?
                .value_clamped(time_to_expiry, option.strike_spread_bp)
        };

        let delta = option.delta(forward_spread_bp, sigma, time_to_expiry);
        // Scale by notional (risk per unit spread basis)
        Ok(delta * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Gamma calculator for credit options
pub struct GammaCalculator;

impl MetricCalculator for GammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CreditOption = context.instrument_as()?;
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        let hazard_curve = context.curves.hazard(option.credit_id)?;
        let current_tenor = option
            .day_count
            .year_fraction(context.as_of, option.cds_maturity)?;
        let forward_spread_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            option.strike_spread_bp
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context
                .curves
                .vol_surface(option.vol_id)?
                .value_clamped(time_to_expiry, option.strike_spread_bp)
        };

        let gamma = option.gamma(forward_spread_bp, sigma, time_to_expiry);
        Ok(gamma * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Vega calculator for credit options
pub struct VegaCalculator;

impl MetricCalculator for VegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CreditOption = context.instrument_as()?;
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        let hazard_curve = context.curves.hazard(option.credit_id)?;
        let current_tenor = option
            .day_count
            .year_fraction(context.as_of, option.cds_maturity)?;
        let forward_spread_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            option.strike_spread_bp
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context
                .curves
                .vol_surface(option.vol_id)?
                .value_clamped(time_to_expiry, option.strike_spread_bp)
        };

        let vega = option.vega(forward_spread_bp, sigma, time_to_expiry);
        Ok(vega * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Theta calculator for credit options
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CreditOption = context.instrument_as()?;
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        let disc_curve = context.curves.discount(option.disc_id)?;
        let r = disc_curve.zero(time_to_expiry);

        let hazard_curve = context.curves.hazard(option.credit_id)?;
        let current_tenor = option
            .day_count
            .year_fraction(context.as_of, option.cds_maturity)?;
        let forward_spread_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            option.strike_spread_bp
        };

        let sigma = if let Some(impl_vol) = option.implied_vol {
            impl_vol
        } else {
            context
                .curves
                .vol_surface(option.vol_id)?
                .value_clamped(time_to_expiry, option.strike_spread_bp)
        };

        let theta = option.theta(forward_spread_bp, r, sigma, time_to_expiry);
        Ok(theta * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Rho calculator for credit options
pub struct RhoCalculator;

impl MetricCalculator for RhoCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CreditOption = context.instrument_as()?;
        let time_to_expiry = option
            .day_count
            .year_fraction(context.as_of, option.expiry)?;

        if time_to_expiry <= 0.0 {
            return Ok(0.0);
        }

        // Black-76 property: dPrice/dr = -t * Price (holding forward/spread, vol constant)
        // Report rho per 1% change in rates, matching equity option convention.
        let base_price = context.base_value.amount();
        Ok(-0.01 * time_to_expiry * base_price)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Implied Volatility calculator for credit options
pub struct ImpliedVolCalculator;

impl MetricCalculator for ImpliedVolCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let _option: &CreditOption = context.instrument_as()?;
        Ok(0.0)
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}

/// Register credit option metrics with the registry
pub fn register_credit_option_metrics(registry: &mut MetricRegistry) {
    registry.register_metric(
        MetricId::Delta,
        Arc::new(DeltaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GammaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(MetricId::Vega, Arc::new(VegaCalculator), &["CreditOption"]);

    registry.register_metric(
        MetricId::Theta,
        Arc::new(ThetaCalculator),
        &["CreditOption"],
    );

    registry.register_metric(MetricId::Rho, Arc::new(RhoCalculator), &["CreditOption"]);

    registry.register_metric(
        MetricId::ImpliedVol,
        Arc::new(ImpliedVolCalculator),
        &["CreditOption"],
    );
}
