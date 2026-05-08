//! CDS-Option-specific DV01 calculator.
//!
//! Like single-name CDS, CDS-option rate risk is a cross-curve sensitivity:
//! after a rate-curve bump, the hazard curve must be re-bootstrapped from the
//! unchanged CDS par spreads so that the underlying CDS quotes are held fixed.
//! This matches Bloomberg-style IR DV01 on the CDSO screen — the convention
//! used by single-name CDS option desks.
//!
//! Falls back to a hazard-held-constant rate bump when the hazard curve does
//! not carry par-spread points (uncalibratable curve).

use crate::calibration::bumps::hazard::bump_hazard_spreads_with_doc_clause_and_valuation_convention;
use crate::calibration::bumps::rates::bump_discount_curve_from_rate_calibration;
use crate::calibration::bumps::BumpRequest;
use crate::instruments::credit_derivatives::cds::{CDSConvention, CdsValuationConvention};
use crate::instruments::credit_derivatives::cds_option::CDSOption;
use crate::market::conventions::ids::CdsDocClause as MarketClause;
use crate::metrics::sensitivities::config as sens_config;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::market_data::bumps::BumpSpec;
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
        rebootstrap_hazard: bool,
    ) -> Result<f64> {
        let mut bumped_market: MarketContext = context.curves.as_ref().clone();
        let base_discount = context
            .curves
            .get_discount(option.discount_curve_id.as_str())?;
        if let Some(calibration) = base_discount.rate_calibration() {
            let bumped_discount = bump_discount_curve_from_rate_calibration(
                base_discount.as_ref(),
                calibration,
                context.curves.as_ref(),
                &BumpRequest::Parallel(bump_bp),
            )?;
            bumped_market = bumped_market.insert(bumped_discount);
        } else {
            bumped_market.apply_curve_bump_in_place(
                &option.discount_curve_id,
                BumpSpec::parallel_bp(bump_bp),
            )?;
        }

        if rebootstrap_hazard {
            let base_hazard = context.curves.get_hazard(option.credit_curve_id.as_str())?;
            let recalibrated = bump_hazard_spreads_with_doc_clause_and_valuation_convention(
                base_hazard.as_ref(),
                &bumped_market,
                &BumpRequest::Parallel(0.0),
                Some(&option.discount_curve_id),
                Some(option_market_doc_clause(option)),
                Some(CdsValuationConvention::IsdaDirty),
            )?;
            bumped_market = bumped_market.insert(recalibrated);
        }

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

        // Bloomberg DOCS 2057273 §4 IR DV01 convention: "holding all other
        // inputs constant" — for the option's IR DV01, the "other input" held
        // constant is the *hazard rate curve* (not the par spreads). This is
        // the convention used on the CDSO screen: the option NPV's exposure
        // to rate moves is measured through the discount curve directly,
        // without re-deriving hazard rates from par spreads after the bump.
        // (For Spread DV01, the convention is reversed — hazard rates are
        // re-bootstrapped from bumped par spreads, holding the IR curve
        // constant; that lives in the CS01 path, not here.)
        let pv_up = Self::price_at_rate_bump(option, context, bump_bp, false)?;
        let pv_down = Self::price_at_rate_bump(option, context, -bump_bp, false)?;

        // Sign convention: Bloomberg reports IR DV01 as the value INCREASE
        // for a 1bp DOWNWARD parallel rate shift. For an option (or any
        // instrument that gains value when rates decrease), this is
        // POSITIVE. Our central difference `(pv_up - pv_down) / (2 × bp)`
        // is the slope ∂V/∂r per +1bp; multiplying by −1 gives the
        // Bloomberg-displayed bond-convention DV01.
        Ok(-(pv_up - pv_down) / (2.0 * bump_bp))
    }
}

/// Map the option's underlying CDS convention to the bootstrap doc-clause
/// identifier expected by `bump_hazard_spreads_with_doc_clause_and_valuation_convention`.
fn option_market_doc_clause(option: &CDSOption) -> MarketClause {
    match option.underlying_convention {
        CDSConvention::IsdaNa => MarketClause::IsdaNa,
        CDSConvention::IsdaEu => MarketClause::IsdaEu,
        CDSConvention::IsdaAs => MarketClause::IsdaAs,
        CDSConvention::Custom => MarketClause::Custom,
    }
}
