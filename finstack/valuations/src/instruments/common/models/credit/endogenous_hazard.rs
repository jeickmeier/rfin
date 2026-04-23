//! Endogenous (leverage-dependent) hazard rate model.
//!
//! Provides a feedback loop where PIK accrual increases leverage, which in turn
//! increases the hazard rate and expected loss. Three mapping functions are
//! supported:
//!
//! - **Power law**: `lambda(L) = lambda_0 * (L / L_0)^beta`
//! - **Exponential**: `lambda(L) = lambda_0 * exp(beta * (L - L_0))`
//! - **Tabular**: Linear interpolation from empirical calibration with flat
//!   extrapolation at the edges.
//!
//! All computed hazard rates are floored at 0.0 (never negative).

use finstack_core::{InputError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Map from leverage to hazard rate.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub enum LeverageHazardMap {
    /// `lambda(t) = lambda_0 * (L(t) / L_0)^beta`
    PowerLaw {
        /// Power-law exponent (`beta`).
        exponent: f64,
    },
    /// `lambda(t) = lambda_0 * exp(beta * (L(t) - L_0))`
    Exponential {
        /// Exponential sensitivity (`beta`).
        sensitivity: f64,
    },
    /// Tabular: linear interpolation from empirical calibration.
    Tabular {
        /// Leverage breakpoints (must be sorted ascending).
        leverage_points: Vec<f64>,
        /// Corresponding hazard rates at each breakpoint.
        hazard_points: Vec<f64>,
    },
}

/// Specification for endogenous (leverage-dependent) hazard rate.
///
/// Models the relationship between a firm's leverage and its instantaneous
/// hazard rate, enabling a feedback loop where PIK accrual increases the
/// notional (and hence leverage), which drives the hazard rate higher.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct EndogenousHazardSpec {
    /// Base (reference) hazard rate `lambda_0`.
    base_hazard_rate: f64,
    /// Base (reference) leverage level `L_0`.
    base_leverage: f64,
    /// Mapping function from leverage to hazard rate.
    leverage_hazard_map: LeverageHazardMap,
}

// ---------------------------------------------------------------------------
// Implementation
// ---------------------------------------------------------------------------

impl EndogenousHazardSpec {
    // -- Convenience constructors -------------------------------------------

    /// Validate base parameters common to all parametric models.
    fn validate(base_hazard: f64, base_leverage: f64) -> Result<()> {
        if base_hazard < 0.0 {
            return Err(InputError::NegativeValue.into());
        }
        if base_leverage <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        Ok(())
    }

    /// Create a power-law endogenous hazard spec.
    ///
    /// `lambda(L) = base_hazard * (L / base_leverage)^exponent`
    ///
    /// # Errors
    ///
    /// Returns an error if `base_hazard < 0` or `base_leverage <= 0`.
    pub fn power_law(base_hazard: f64, base_leverage: f64, exponent: f64) -> Result<Self> {
        Self::validate(base_hazard, base_leverage)?;
        Ok(Self {
            base_hazard_rate: base_hazard,
            base_leverage,
            leverage_hazard_map: LeverageHazardMap::PowerLaw { exponent },
        })
    }

    /// Create an exponential endogenous hazard spec.
    ///
    /// `lambda(L) = base_hazard * exp(sensitivity * (L - base_leverage))`
    ///
    /// # Errors
    ///
    /// Returns an error if `base_hazard < 0` or `base_leverage <= 0`.
    pub fn exponential(base_hazard: f64, base_leverage: f64, sensitivity: f64) -> Result<Self> {
        Self::validate(base_hazard, base_leverage)?;
        Ok(Self {
            base_hazard_rate: base_hazard,
            base_leverage,
            leverage_hazard_map: LeverageHazardMap::Exponential { sensitivity },
        })
    }

    /// Create a tabular endogenous hazard spec from empirical calibration.
    ///
    /// Uses linear interpolation between the given points and flat
    /// extrapolation beyond the edges. `base_hazard_rate` and `base_leverage`
    /// are derived from the first tabular point.
    ///
    /// # Errors
    ///
    /// Returns an error if vectors are empty or have different lengths.
    pub fn tabular(leverage_points: Vec<f64>, hazard_points: Vec<f64>) -> Result<Self> {
        if leverage_points.is_empty() || leverage_points.len() != hazard_points.len() {
            return Err(InputError::DimensionMismatch.into());
        }
        let base_leverage = leverage_points[0];
        let base_hazard_rate = hazard_points[0];
        Ok(Self {
            base_hazard_rate,
            base_leverage,
            leverage_hazard_map: LeverageHazardMap::Tabular {
                leverage_points,
                hazard_points,
            },
        })
    }

    // -- Core computation ---------------------------------------------------

    /// Compute the hazard rate at a given leverage level.
    ///
    /// The result is always floored at 0.0 (never negative).
    pub fn hazard_at_leverage(&self, leverage: f64) -> f64 {
        let raw = match &self.leverage_hazard_map {
            LeverageHazardMap::PowerLaw { exponent } => {
                let ratio = (leverage / self.base_leverage).max(0.0);
                self.base_hazard_rate * ratio.powf(*exponent)
            }
            LeverageHazardMap::Exponential { sensitivity } => {
                self.base_hazard_rate * (*sensitivity * (leverage - self.base_leverage)).exp()
            }
            LeverageHazardMap::Tabular {
                leverage_points,
                hazard_points,
            } => tabular_interpolate(leverage_points, hazard_points, leverage),
        };
        raw.max(0.0)
    }

    /// Compute the hazard rate after PIK accrual changes the notional.
    ///
    /// Leverage is computed as `accreted_notional / asset_value`.
    pub fn hazard_after_pik_accrual(
        &self,
        _original_notional: f64,
        accreted_notional: f64,
        asset_value: f64,
    ) -> f64 {
        let leverage = accreted_notional / asset_value;
        self.hazard_at_leverage(leverage)
    }

    // -- Accessors ----------------------------------------------------------

    /// Returns the base (reference) hazard rate.
    pub fn base_hazard_rate(&self) -> f64 {
        self.base_hazard_rate
    }

    /// Returns the base (reference) leverage level.
    pub fn base_leverage(&self) -> f64 {
        self.base_leverage
    }

    /// Returns a reference to the leverage-to-hazard mapping.
    pub fn leverage_hazard_map(&self) -> &LeverageHazardMap {
        &self.leverage_hazard_map
    }
}

// ---------------------------------------------------------------------------
// Helper: tabular linear interpolation with flat extrapolation
// ---------------------------------------------------------------------------

/// Linear interpolation between tabular points with flat extrapolation at
/// the edges.
///
/// # Assumptions
///
/// - `xs` and `ys` have the same length and at least one element.
/// - `xs` is sorted in ascending order.
fn tabular_interpolate(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    assert!(
        !xs.is_empty() && xs.len() == ys.len(),
        "tabular_interpolate: xs and ys must be non-empty and equal length"
    );

    // Flat extrapolation below the first point.
    if x <= xs[0] {
        return ys[0];
    }
    // Flat extrapolation above the last point.
    if x >= xs[xs.len() - 1] {
        return ys[ys.len() - 1];
    }

    // Find the bracketing interval and interpolate.
    for i in 0..xs.len() - 1 {
        if x >= xs[i] && x <= xs[i + 1] {
            let t = (x - xs[i]) / (xs[i + 1] - xs[i]);
            return ys[i] + t * (ys[i + 1] - ys[i]);
        }
    }

    // Fallback (should not be reached for valid sorted input).
    ys[ys.len() - 1]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_law_at_base_leverage_returns_base_hazard() {
        let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5).unwrap();
        assert!((spec.hazard_at_leverage(1.5) - 0.10).abs() < 1e-10);
    }

    #[test]
    fn power_law_increases_with_leverage() {
        let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5).unwrap();
        let h_low = spec.hazard_at_leverage(1.5);
        let h_high = spec.hazard_at_leverage(2.0);
        assert!(h_high > h_low, "h_low={h_low}, h_high={h_high}");
    }

    #[test]
    fn exponential_at_base_returns_base() {
        let spec = EndogenousHazardSpec::exponential(0.10, 1.5, 5.0).unwrap();
        assert!((spec.hazard_at_leverage(1.5) - 0.10).abs() < 1e-10);
    }

    #[test]
    fn exponential_increases_with_leverage() {
        let spec = EndogenousHazardSpec::exponential(0.10, 1.5, 5.0).unwrap();
        let h_low = spec.hazard_at_leverage(1.5);
        let h_high = spec.hazard_at_leverage(2.0);
        assert!(h_high > h_low);
    }

    #[test]
    fn pik_accrual_increases_hazard() {
        let spec = EndogenousHazardSpec::power_law(0.10, 1.5, 2.5).unwrap();
        let h_before = spec.hazard_after_pik_accrual(100.0, 100.0, 66.67);
        let h_after = spec.hazard_after_pik_accrual(100.0, 120.0, 66.67);
        assert!(
            h_after > h_before,
            "PIK accrual should increase hazard: before={h_before}, after={h_after}"
        );
    }

    #[test]
    fn tabular_interpolates() {
        let spec =
            EndogenousHazardSpec::tabular(vec![1.0, 1.5, 2.0, 3.0], vec![0.02, 0.05, 0.12, 0.30])
                .unwrap();
        let h = spec.hazard_at_leverage(1.75);
        assert!(h > 0.05 && h < 0.12, "h={h}");
    }

    #[test]
    fn tabular_flat_extrapolation() {
        let spec = EndogenousHazardSpec::tabular(vec![1.0, 2.0], vec![0.05, 0.15]).unwrap();
        let h_below = spec.hazard_at_leverage(0.5);
        let h_above = spec.hazard_at_leverage(5.0);
        assert!(
            (h_below - 0.05).abs() < 1e-10,
            "Below range: flat extrapolation"
        );
        assert!(
            (h_above - 0.15).abs() < 1e-10,
            "Above range: flat extrapolation"
        );
    }

    #[test]
    fn rejects_invalid_inputs() {
        assert!(EndogenousHazardSpec::power_law(-0.10, 1.5, 2.5).is_err());
        assert!(EndogenousHazardSpec::power_law(0.10, 0.0, 2.5).is_err());
        assert!(EndogenousHazardSpec::exponential(0.10, -1.0, 5.0).is_err());
        assert!(EndogenousHazardSpec::tabular(vec![], vec![]).is_err());
        assert!(EndogenousHazardSpec::tabular(vec![1.0], vec![0.05, 0.10]).is_err());
    }
}
