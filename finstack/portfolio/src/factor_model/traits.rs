//! Trait abstractions for portfolio factor risk decomposition engines.

use super::types::RiskDecomposition;
use finstack_core::factor_model::{FactorCovarianceMatrix, RiskMeasure};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

/// Common interface for engines that decompose portfolio risk into factor contributions.
pub trait RiskDecomposer: Send + Sync {
    /// Decompose the requested portfolio risk measure into factor and residual components.
    ///
    /// `sensitivities` must already reflect any position sizing or weighting applied by the
    /// upstream sensitivity engine. Implementations should therefore treat each row as aligned
    /// with `sensitivities.position_ids()` and must not re-apply portfolio weights.
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        measure: &RiskMeasure,
    ) -> finstack_core::Result<RiskDecomposition>;
}
