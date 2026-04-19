use super::matrix::SensitivityMatrix;
use crate::instruments::Instrument;
use finstack_core::dates::Date;
use finstack_core::factor_model::FactorDefinition;
use finstack_core::market_data::context::MarketContext;
use finstack_core::Result;

/// Engine for computing per-position, per-factor sensitivities.
pub trait FactorSensitivityEngine: Send + Sync {
    /// Compute a sensitivity matrix for `positions` against `factors`.
    fn compute_sensitivities(
        &self,
        positions: &[(String, &dyn Instrument, f64)],
        factors: &[FactorDefinition],
        market: &MarketContext,
        as_of: Date,
    ) -> Result<SensitivityMatrix>;
}
