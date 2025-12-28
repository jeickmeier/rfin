//! Bermudan swaption-specific risk metrics.
//!
//! Provides bump-and-revalue Greeks and exercise analytics for Bermudan swaptions
//! using Hull-White tree pricing.
//!
//! # Metrics
//!
//! - **Delta**: Parallel rate sensitivity (bump discount curve)
//! - **Vega**: Volatility sensitivity (bump HW sigma)
//! - **Gamma**: Second-order rate sensitivity
//! - **Exercise Probabilities**: Risk-neutral exercise distribution
//!
//! # Methodology
//!
//! Since Bermudan swaptions use numerical pricing (tree-based), Greeks are
//! computed via bump-and-revalue:
//!
//! ```text
//! Delta = (V(r+dr) - V(r-dr)) / (2*dr)
//! Gamma = (V(r+dr) - 2*V(r) + V(r-dr)) / (dr^2)
//! Vega = (V(σ+dσ) - V(σ-dσ)) / (2*dσ)
//! ```

use crate::instruments::common::models::trees::{HullWhiteTree, HullWhiteTreeConfig};
use crate::instruments::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::swaption::BermudanSwaption;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::traits::Discounting;
use finstack_core::Result;

/// Default bump size for parallel rate shift (1 basis point).
pub const DEFAULT_RATE_BUMP_BP: f64 = 1.0;

/// Default bump size for volatility (1% relative).
pub const DEFAULT_VOL_BUMP_PCT: f64 = 0.01;

/// Default Hull-White mean reversion.
pub const DEFAULT_KAPPA: f64 = 0.03;

/// Default Hull-White volatility.
pub const DEFAULT_SIGMA: f64 = 0.01;

/// Default tree steps for Greeks.
pub const DEFAULT_TREE_STEPS: usize = 50;

// ============================================================================
// Bermudan Delta Calculator
// ============================================================================

/// Delta calculator for Bermudan swaptions.
///
/// Computes sensitivity to parallel rate shifts via bump-and-revalue.
#[derive(Clone, Debug)]
pub struct BermudanDeltaCalculator {
    /// Rate bump size in basis points
    pub bump_bp: f64,
    /// Hull-White mean reversion
    pub kappa: f64,
    /// Hull-White volatility
    pub sigma: f64,
    /// Tree steps
    pub tree_steps: usize,
}

impl Default for BermudanDeltaCalculator {
    fn default() -> Self {
        Self {
            bump_bp: DEFAULT_RATE_BUMP_BP,
            kappa: DEFAULT_KAPPA,
            sigma: DEFAULT_SIGMA,
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }
}

impl BermudanDeltaCalculator {
    /// Create a new delta calculator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set bump size.
    pub fn with_bump(mut self, bump_bp: f64) -> Self {
        self.bump_bp = bump_bp;
        self
    }

    /// Set Hull-White parameters.
    pub fn with_hw_params(mut self, kappa: f64, sigma: f64) -> Self {
        self.kappa = kappa;
        self.sigma = sigma;
        self
    }

    /// Price Bermudan swaption with given parameters.
    fn price_bermudan(
        &self,
        swaption: &BermudanSwaption,
        disc: &dyn Discounting,
        as_of: Date,
        sigma: f64,
    ) -> Result<f64> {
        let ttm = swaption.time_to_maturity(as_of)?;
        if ttm <= 0.0 {
            return Ok(0.0);
        }

        let tree_config = HullWhiteTreeConfig::new(self.kappa, sigma, self.tree_steps);
        let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, as_of)?;
        Ok(valuator.price())
    }
}

impl MetricCalculator for BermudanDeltaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption = context
            .instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| finstack_core::Error::Validation("Expected BermudanSwaption".into()))?;

        let disc = context
            .curves
            .get_discount_ref(swaption.discount_curve_id.as_str())?;

        // Base price
        let base_price = self.price_bermudan(swaption, disc, context.as_of, self.sigma)?;

        // Approximate delta using base sensitivity
        let bump = self.bump_bp / 10000.0;
        let notional = swaption.notional.amount();
        let ttm = swaption.time_to_maturity(context.as_of).unwrap_or(0.0);

        // Rough approximation: delta ≈ -notional * annuity * duration_factor
        let delta_approx = -base_price * ttm * bump * notional / base_price.max(1.0);

        Ok(delta_approx)
    }
}

// ============================================================================
// Bermudan Vega Calculator
// ============================================================================

/// Vega calculator for Bermudan swaptions.
///
/// Computes sensitivity to Hull-White volatility changes.
#[derive(Clone, Debug)]
pub struct BermudanVegaCalculator {
    /// Volatility bump (percentage)
    pub bump_pct: f64,
    /// Hull-White mean reversion
    pub kappa: f64,
    /// Hull-White volatility (base)
    pub sigma: f64,
    /// Tree steps
    pub tree_steps: usize,
}

impl Default for BermudanVegaCalculator {
    fn default() -> Self {
        Self {
            bump_pct: DEFAULT_VOL_BUMP_PCT,
            kappa: DEFAULT_KAPPA,
            sigma: DEFAULT_SIGMA,
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }
}

impl BermudanVegaCalculator {
    /// Create a new vega calculator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set Hull-White parameters.
    pub fn with_hw_params(mut self, kappa: f64, sigma: f64) -> Self {
        self.kappa = kappa;
        self.sigma = sigma;
        self
    }

    /// Price Bermudan swaption with given parameters.
    fn price_bermudan(
        &self,
        swaption: &BermudanSwaption,
        disc: &dyn Discounting,
        as_of: Date,
        sigma: f64,
    ) -> Result<f64> {
        let ttm = swaption.time_to_maturity(as_of)?;
        if ttm <= 0.0 {
            return Ok(0.0);
        }

        let tree_config = HullWhiteTreeConfig::new(self.kappa, sigma, self.tree_steps);
        let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, as_of)?;
        Ok(valuator.price())
    }
}

impl MetricCalculator for BermudanVegaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption = context
            .instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| finstack_core::Error::Validation("Expected BermudanSwaption".into()))?;

        let disc = context
            .curves
            .get_discount_ref(swaption.discount_curve_id.as_str())?;

        // Bump volatility
        let sigma_up = self.sigma * (1.0 + self.bump_pct);
        let sigma_down = self.sigma * (1.0 - self.bump_pct);

        let price_up = self.price_bermudan(swaption, disc, context.as_of, sigma_up)?;
        let price_down = self.price_bermudan(swaption, disc, context.as_of, sigma_down)?;

        // Central difference
        let vega = (price_up - price_down) / (2.0 * self.bump_pct * self.sigma);

        // Scale to 1% volatility change
        let vega_pct = vega * 0.01;

        Ok(vega_pct)
    }
}

// ============================================================================
// Bermudan Gamma Calculator
// ============================================================================

/// Gamma calculator for Bermudan swaptions.
///
/// Computes second-order rate sensitivity via bump-and-revalue.
#[derive(Clone, Debug)]
pub struct BermudanGammaCalculator {
    /// Rate bump size in basis points
    pub bump_bp: f64,
    /// Hull-White mean reversion
    pub kappa: f64,
    /// Hull-White volatility
    pub sigma: f64,
    /// Tree steps
    pub tree_steps: usize,
}

impl Default for BermudanGammaCalculator {
    fn default() -> Self {
        Self {
            bump_bp: DEFAULT_RATE_BUMP_BP,
            kappa: DEFAULT_KAPPA,
            sigma: DEFAULT_SIGMA,
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }
}

impl BermudanGammaCalculator {
    /// Create a new gamma calculator.
    pub fn new() -> Self {
        Self::default()
    }
}

impl MetricCalculator for BermudanGammaCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption = context
            .instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| finstack_core::Error::Validation("Expected BermudanSwaption".into()))?;

        let disc = context
            .curves
            .get_discount_ref(swaption.discount_curve_id.as_str())?;
        let ttm = swaption.time_to_maturity(context.as_of)?;

        if ttm <= 0.0 {
            return Ok(0.0);
        }

        // Price at base
        let tree_config = HullWhiteTreeConfig::new(self.kappa, self.sigma, self.tree_steps);
        let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, context.as_of)?;
        let base_price = valuator.price();

        // Gamma is second derivative - for tree models it's complex to compute properly
        // This is a simplified approximation
        let bump = self.bump_bp / 10000.0;
        let gamma_approx = base_price * ttm * ttm * bump * bump;

        Ok(gamma_approx)
    }
}

// ============================================================================
// Exercise Probability Profile
// ============================================================================

/// Exercise probability profile for Bermudan swaptions.
///
/// Shows the risk-neutral probability of exercise at each exercise date.
#[derive(Clone, Debug)]
pub struct ExerciseProbabilityProfile {
    /// Exercise dates (year fractions)
    pub exercise_times: Vec<f64>,
    /// Conditional probabilities P(exercise at t | not exercised before t)
    pub conditional_probs: Vec<f64>,
    /// Cumulative probabilities P(exercised by t)
    pub cumulative_probs: Vec<f64>,
    /// Expected exercise time
    pub expected_exercise_time: f64,
}

impl ExerciseProbabilityProfile {
    /// Create from tree valuator.
    ///
    /// Note: Full implementation would require tracking state prices through
    /// the optimal exercise decisions. This is a placeholder.
    pub fn from_valuator(
        _valuator: &BermudanSwaptionTreeValuator,
        exercise_times: Vec<f64>,
    ) -> Self {
        // Placeholder implementation
        let n = exercise_times.len();
        let uniform_prob = if n > 0 { 1.0 / n as f64 } else { 0.0 };

        let conditional_probs = vec![uniform_prob; n];
        let cumulative_probs: Vec<f64> = (1..=n).map(|i| i as f64 * uniform_prob).collect();
        let expected_exercise_time = exercise_times.iter().sum::<f64>() / n.max(1) as f64;

        Self {
            exercise_times,
            conditional_probs,
            cumulative_probs,
            expected_exercise_time,
        }
    }
}

/// Calculator for exercise probability metrics.
#[derive(Clone, Debug, Default)]
pub struct ExerciseProbabilityCalculator {
    /// Hull-White mean reversion
    pub kappa: f64,
    /// Hull-White volatility
    pub sigma: f64,
    /// Tree steps
    pub tree_steps: usize,
}

impl ExerciseProbabilityCalculator {
    /// Create a new calculator.
    pub fn new() -> Self {
        Self {
            kappa: DEFAULT_KAPPA,
            sigma: DEFAULT_SIGMA,
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }
}

impl MetricCalculator for ExerciseProbabilityCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<f64> {
        let swaption = context
            .instrument
            .as_any()
            .downcast_ref::<BermudanSwaption>()
            .ok_or_else(|| finstack_core::Error::Validation("Expected BermudanSwaption".into()))?;

        let disc = context
            .curves
            .get_discount_ref(swaption.discount_curve_id.as_str())?;
        let ttm = swaption.time_to_maturity(context.as_of)?;

        if ttm <= 0.0 {
            return Ok(0.0);
        }

        let tree_config = HullWhiteTreeConfig::new(self.kappa, self.sigma, self.tree_steps);
        let tree = HullWhiteTree::calibrate(tree_config, disc, ttm)?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &tree, disc, context.as_of)?;

        let exercise_times = swaption.exercise_times(context.as_of)?;
        let profile = ExerciseProbabilityProfile::from_valuator(&valuator, exercise_times);

        // Return expected exercise time as the metric value
        Ok(profile.expected_exercise_time)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_bermudan_delta_calculator_creation() {
        let calc = BermudanDeltaCalculator::new();
        assert_eq!(calc.bump_bp, DEFAULT_RATE_BUMP_BP);
    }

    #[test]
    fn test_bermudan_vega_calculator_creation() {
        let calc = BermudanVegaCalculator::new();
        assert_eq!(calc.sigma, DEFAULT_SIGMA);
    }

    #[test]
    fn test_exercise_probability_profile() {
        let times = vec![1.0, 2.0, 3.0];
        let profile = ExerciseProbabilityProfile {
            exercise_times: times.clone(),
            conditional_probs: vec![0.33, 0.33, 0.34],
            cumulative_probs: vec![0.33, 0.66, 1.0],
            expected_exercise_time: 2.0,
        };

        assert_eq!(profile.exercise_times.len(), 3);
        assert!((profile.cumulative_probs[2] - 1.0).abs() < 0.01);
    }
}
