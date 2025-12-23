//! Calibration handlers (targets).
//!
//! Maps API steps to domain logic execution.

use crate::calibration::api::schema::StepParams;
use crate::calibration::config::CalibrationConfig;
use crate::calibration::targets::base_correlation::BaseCorrelationBootstrapper;
use crate::calibration::targets::discount::DiscountCurveTarget;
use crate::calibration::targets::forward::ForwardCurveTarget;
use crate::calibration::targets::hazard::HazardBootstrapper;
use crate::calibration::targets::inflation::InflationBootstrapper;
use crate::calibration::targets::swaption::SwaptionVolBootstrapper;
use crate::calibration::targets::vol::VolSurfaceBootstrapper;
use crate::calibration::CalibrationReport;
use crate::market::quotes::market_quote::MarketQuote;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;


/// Execute a single calibration step.
///
/// This is the main entry point for the calibration engine to process a
/// [`CalibrationStep`]. It extracts relevant quotes, prepares the
/// target target, executes the solver (Bootstrap or Global), and
/// updates the market context with the result.
///
/// # Returns
///
/// A tuple containing:
/// - Updated [`MarketContext`] with the new curve/surface.
/// - [`CalibrationReport`] containing solver performance and diagnostics.
pub fn execute_step(
    params: &StepParams,
    quotes: &[MarketQuote],
    context: &MarketContext,
    global_config: &CalibrationConfig,
) -> Result<(MarketContext, CalibrationReport)> {
    match params {
        StepParams::Discount(p) => DiscountCurveTarget::solve(p, quotes, context, global_config),
        StepParams::Forward(p) => ForwardCurveTarget::solve(p, quotes, context, global_config),
        StepParams::Hazard(p) => HazardBootstrapper::solve(p, quotes, context, global_config),
        StepParams::Inflation(p) => InflationBootstrapper::solve(p, quotes, context, global_config),
        StepParams::VolSurface(p) => {
            let (surface, report) =
                VolSurfaceBootstrapper::solve(p, quotes, context, global_config)?;
            let mut new_context = context.clone();
            new_context.insert_surface_mut(surface);
            Ok((new_context, report))
        }
        StepParams::SwaptionVol(p) => {
            let (surface, report) =
                SwaptionVolBootstrapper::solve(p, quotes, context, global_config)?;
            let mut new_context = context.clone();
            new_context.insert_surface_mut(surface);
            Ok((new_context, report))
        }
        StepParams::BaseCorrelation(p) => {
            BaseCorrelationBootstrapper::solve(p, quotes, context, global_config)
        }
    }
}
