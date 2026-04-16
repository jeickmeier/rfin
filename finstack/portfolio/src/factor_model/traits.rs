//! Trait abstractions for portfolio factor risk decomposition engines.

use super::position_risk::{DecompositionConfig, PositionRiskDecomposition};
use super::types::RiskDecomposition;
use crate::PositionId;
use finstack_core::factor_model::{FactorCovarianceMatrix, RiskMeasure};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;

/// Common interface for engines that decompose portfolio risk into factor contributions.
pub trait RiskDecomposer: Send + Sync {
    /// Decompose the requested portfolio risk measure into factor and residual components.
    ///
    /// `sensitivities` must already reflect any position sizing or weighting applied by the
    /// upstream sensitivity engine. Implementations should therefore treat each row as aligned
    /// with `sensitivities.position_ids()` and must not re-apply portfolio weights.
    ///
    /// # Arguments
    ///
    /// * `sensitivities` - Weighted position-factor sensitivity matrix.
    /// * `covariance` - Factor covariance matrix aligned to the same factor order.
    /// * `measure` - Risk measure to decompose.
    ///
    /// # Returns
    ///
    /// Factor and residual risk decomposition in the units implied by `measure`.
    ///
    /// # Errors
    ///
    /// Implementations should return an error when factor axes are inconsistent,
    /// the covariance matrix is invalid, or the requested measure cannot be evaluated.
    fn decompose(
        &self,
        sensitivities: &SensitivityMatrix,
        covariance: &FactorCovarianceMatrix,
        measure: &RiskMeasure,
    ) -> finstack_core::Result<RiskDecomposition>;
}

/// Engine that decomposes portfolio VaR and ES into position-level contributions
/// from weights and a covariance matrix (parametric / Monte Carlo methods).
///
/// Unlike [`RiskDecomposer`] which operates at the factor level with a
/// `SensitivityMatrix`, this trait operates at the position level using
/// portfolio weights and a position-return covariance matrix.
///
/// Note: [`super::position_risk::HistoricalPositionDecomposer`] does NOT
/// implement this trait because historical simulation requires per-position
/// scenario P&L vectors rather than weights + covariance. It exposes a
/// separate `decompose_from_pnls` method instead.
pub trait PositionRiskDecomposer: Send + Sync {
    /// Decompose portfolio VaR and ES into per-position contributions.
    ///
    /// # Arguments
    ///
    /// * `weights` - Position weights as fraction of portfolio value.
    ///   Length `n_positions`. Must sum to 1.0 (or close, within tolerance).
    /// * `covariance` - Position-return covariance matrix (n x n, row-major).
    ///   Must be symmetric positive semi-definite.
    /// * `position_ids` - Position identifiers, aligned with `weights`.
    /// * `config` - Decomposition parameters.
    ///
    /// # Returns
    ///
    /// Complete position-level decomposition with VaR and ES contributions.
    ///
    /// # Errors
    ///
    /// Returns an error if dimensions are inconsistent, the covariance matrix
    /// is invalid, or the confidence level is out of bounds.
    fn decompose_positions(
        &self,
        weights: &[f64],
        covariance: &[f64],
        position_ids: &[PositionId],
        config: &DecompositionConfig,
    ) -> finstack_core::Result<PositionRiskDecomposition>;
}
