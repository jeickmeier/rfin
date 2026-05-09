//! CDS-Option-specific DV01 calculator.
//!
//! CDS-option IR DV01 is a swap-curve quote sensitivity: bump the stored swap
//! curve market quotes, rebuild the discount curve, and reprice. Direct
//! discount-factor bumps are intentionally rejected so the reported value has a
//! single market convention.

use crate::calibration::bumps::rates::bump_discount_curve_from_rate_calibration;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

const MIN_BUMP_BP: f64 = 1e-10;

/// CDS option DV01 calculator with par-spread hazard re-bootstrap when
/// possible (Bloomberg CDSO convention).
pub(crate) struct CdsOptionDv01Calculator;

impl CdsOptionDv01Calculator {
    fn price_at_rate_bump(
        option: &CDSOption,
        context: &MetricContext,
        bump_bp: f64,
    ) -> Result<f64> {
        let mut bumped_market: MarketContext = context.curves.as_ref().clone();
        let base_discount = context
            .curves
            .get_discount(option.discount_curve_id.as_str())?;
        let calibration = base_discount.rate_calibration().ok_or_else(|| {
            finstack_core::Error::Validation(format!(
                "CDS option '{}' IR DV01 requires swap-curve quote calibration metadata for discount curve '{}'",
                option.id,
                option.discount_curve_id.as_str()
            ))
        })?;
        let bumped_discount = bump_discount_curve_from_rate_calibration(
            base_discount.as_ref(),
            calibration,
            context.curves.as_ref(),
            &BumpRequest::Parallel(bump_bp),
        )?;
        bumped_market = bumped_market.insert(bumped_discount);

        context.reprice_raw(&bumped_market, context.as_of)
    }
}

impl MetricCalculator for CdsOptionDv01Calculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let option: &CDSOption = context.instrument_as()?;
        let defaults =
            sens_config::from_context_or_default(context.config(), context.get_metric_overrides())?;
        let bump_bp = defaults.rate_bump_bp;
        if bump_bp.abs() <= MIN_BUMP_BP {
            return Ok(0.0);
        }

        // Bloomberg CDSO IR DV01 is a swap-curve quote sensitivity with the
        // hazard curve held fixed. The screen convention reports the symmetric
        // ±1bp quote-shock PV change in bond sign, rather than the half-width
        // central-difference slope used by generic DV01 helpers.
        let pv_up = Self::price_at_rate_bump(option, context, bump_bp)?;
        let pv_down = Self::price_at_rate_bump(option, context, -bump_bp)?;

        // Sign convention: Bloomberg reports IR DV01 as the value INCREASE
        // for a 1bp DOWNWARD parallel rate shift. For an option (or any
        // instrument that gains value when rates decrease), this is
        // POSITIVE. Our central difference `(pv_up - pv_down) / (2 × bp)`
        // is the slope ∂V/∂r per +1bp; multiplying by −1 gives the
        // Bloomberg-displayed bond-convention DV01.
        Ok(-(pv_up - pv_down) / bump_bp)
    }
}
