//! Theta metric for `CdsOption`.

use crate::instruments::cds_option::CdsOption;
use crate::metrics::{MetricCalculator, MetricContext, MetricId};
use finstack_core::{Result, F};

/// Theta calculator for credit options on CDS spreads.
pub struct ThetaCalculator;

impl MetricCalculator for ThetaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let option: &CdsOption = context.instrument_as()?;
        let t = option.day_count.year_fraction(
            context.as_of,
            option.expiry,
            finstack_core::dates::DayCountCtx::default(),
        )?;

        if t <= 0.0 {
            return Ok(0.0);
        }

        // Risk-free rate proxy from discount curve at expiry
        let disc = context
            .curves
            .get_ref::<finstack_core::market_data::term_structures::discount_curve::DiscountCurve>(
            option.disc_id.clone(),
        )?;
        let r = disc.zero(t);

        // Forward spread in bp
        let hazard_curve = context
            .curves
            .get::<finstack_core::market_data::term_structures::hazard_curve::HazardCurve>(
            option.credit_id.clone(),
        )?;
        let current_tenor = option.day_count.year_fraction(
            context.as_of,
            option.cds_maturity,
            finstack_core::dates::DayCountCtx::default(),
        )?;
        let fwd_bp = if current_tenor > 0.0 {
            use finstack_core::market_data::term_structures::hazard_curve::ParInterp;
            hazard_curve.quoted_spread_bp(current_tenor, ParInterp::Linear)
        } else {
            option.strike_spread_bp
        };

        let sigma = if let Some(v) = option.pricing_overrides.implied_volatility {
            v
        } else {
            context
                .curves
                .surface(option.vol_id)?
                .value_clamped(t, option.strike_spread_bp)
        };

        let pricer = crate::instruments::cds_option::pricing::engine::CdsOptionPricer::default();
        let theta = pricer.theta(option, fwd_bp, r, sigma, t);
        Ok(theta * option.notional.amount())
    }

    fn dependencies(&self) -> &[MetricId] {
        &[]
    }
}
