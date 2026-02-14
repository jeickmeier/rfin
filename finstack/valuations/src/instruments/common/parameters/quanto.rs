//! Quanto adjustment specification for cross-currency instruments.

use finstack_core::types::CurveId;

/// Quanto adjustment parameters for instruments where payoff currency differs from
/// underlying currency.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QuantoSpec {
    /// Correlation between the underlying asset and the FX rate.
    /// Must be in [-1, 1].
    pub correlation: f64,
    /// FX volatility surface ID (required for quanto vol lookup).
    pub fx_vol_surface_id: CurveId,
    /// FX spot price identifier (for proper quanto vol lookup).
    /// Falls back to ATM approximation (1.0) if not provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fx_spot_id: Option<String>,
}
