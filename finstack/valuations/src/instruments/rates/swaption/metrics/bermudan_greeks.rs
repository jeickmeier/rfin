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
#![allow(dead_code)] // Public API items may be used by external bindings
//! Vega = (V(σ+dσ) - V(σ-dσ)) / (2*dσ)
//! ```

use crate::instruments::rates::swaption::pricing::BermudanSwaptionTreeValuator;
use crate::instruments::rates::swaption::{
    BermudanSwaption, CalibratedHullWhiteModel, HullWhiteParams,
};
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::{BumpSpec, MarketBump};
use finstack_core::market_data::traits::Discounting;
use finstack_core::types::{Bps, Percentage, Rate};
use finstack_core::Result;

/// Default bump size for parallel rate shift (1 basis point).
#[allow(dead_code)] // May be used by external bindings or tests
pub const DEFAULT_RATE_BUMP_BP: f64 = 1.0;

/// Default bump size for volatility (1% relative).
#[allow(dead_code)] // May be used by external bindings or tests
pub const DEFAULT_VOL_BUMP_PCT: f64 = 0.01;

/// Default Hull-White mean reversion.
#[allow(dead_code)] // May be used by external bindings or tests
pub const DEFAULT_KAPPA: f64 = 0.03;

/// Default Hull-White volatility.
#[allow(dead_code)] // May be used by external bindings or tests
pub const DEFAULT_SIGMA: f64 = 0.01;

/// Default tree steps for Greeks.
#[allow(dead_code)] // May be used by external bindings or tests
pub const DEFAULT_TREE_STEPS: usize = 50;

// ============================================================================
// Bermudan Delta Calculator
// ============================================================================

/// Delta calculator for Bermudan swaptions.
///
/// Computes sensitivity to parallel rate shifts via bump-and-revalue.
#[derive(Debug, Clone)]
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

    /// Set bump size using typed basis points.
    pub fn with_bump_bps(mut self, bump_bp: Bps) -> Self {
        self.bump_bp = bump_bp.as_bps() as f64;
        self
    }

    /// Set Hull-White parameters.
    ///
    /// # Panics (debug only)
    /// Panics if `sigma <= 0` or `kappa < 0`, which would produce invalid tree calibration.
    pub fn with_hw_params(mut self, kappa: f64, sigma: f64) -> Self {
        debug_assert!(sigma > 0.0, "HW sigma must be positive, got {sigma}");
        debug_assert!(kappa >= 0.0, "HW kappa must be non-negative, got {kappa}");
        self.kappa = kappa;
        self.sigma = sigma;
        self
    }

    /// Set Hull-White parameters using typed values.
    pub fn with_hw_params_rate(mut self, kappa: Rate, sigma: Percentage) -> Self {
        self.kappa = kappa.as_decimal();
        self.sigma = sigma.as_decimal();
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

        let model = CalibratedHullWhiteModel::calibrate(
            HullWhiteParams::new(self.kappa, sigma),
            self.tree_steps,
            disc,
            ttm,
        )?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &model, disc, as_of)?;
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

        let bump_bp = self.bump_bp.abs();
        let bump = bump_bp / 10000.0;
        if bump <= 0.0 {
            return Ok(0.0);
        }

        // Bump the discount curve for rate sensitivity.
        //
        // NOTE: The Hull-White tree is a single-factor short-rate model that
        // calibrates to and derives all rates (both discount and forward swap
        // rates) from the input discount curve's term structure. The forward
        // curve stored on the swaption (`forward_id`) is NOT used by the tree
        // pricer -- forward rates at each node are endogenous to the calibrated
        // short-rate dynamics.
        //
        // Consequence: this delta captures sensitivity to the discount/funding
        // curve only. If the discount and forward curves are different (e.g.,
        // OIS vs SOFR), sensitivity to the forward-discount spread is NOT
        // captured. A proper multi-curve delta would require either:
        //   (a) a two-factor tree, or
        //   (b) separate "discount delta" and "forward delta" calculators
        //       with the tree reading forward rates from the forward curve.
        let curves_up = context.curves.bump([MarketBump::Curve {
            id: swaption.discount_curve_id.clone(),
            spec: BumpSpec::parallel_bp(bump_bp),
        }])?;
        let curves_dn = context.curves.bump([MarketBump::Curve {
            id: swaption.discount_curve_id.clone(),
            spec: BumpSpec::parallel_bp(-bump_bp),
        }])?;

        let disc_up = curves_up.get_discount(swaption.discount_curve_id.as_str())?;
        let disc_dn = curves_dn.get_discount(swaption.discount_curve_id.as_str())?;

        let price_up =
            self.price_bermudan(swaption, disc_up.as_ref(), context.as_of, self.sigma)?;
        let price_dn =
            self.price_bermudan(swaption, disc_dn.as_ref(), context.as_of, self.sigma)?;

        Ok((price_up - price_dn) / (2.0 * bump))
    }
}

// ============================================================================
// Bermudan Vega Calculator
// ============================================================================

/// Vega calculator for Bermudan swaptions.
///
/// Computes sensitivity to Hull-White volatility changes.
#[derive(Debug, Clone)]
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

    /// Set volatility bump using a typed percentage.
    pub fn with_bump_pct(mut self, bump_pct: Percentage) -> Self {
        self.bump_pct = bump_pct.as_decimal();
        self
    }

    /// Set Hull-White parameters.
    ///
    /// # Panics (debug only)
    /// Panics if `sigma <= 0` or `kappa < 0`, which would produce invalid tree calibration.
    pub fn with_hw_params(mut self, kappa: f64, sigma: f64) -> Self {
        debug_assert!(sigma > 0.0, "HW sigma must be positive, got {sigma}");
        debug_assert!(kappa >= 0.0, "HW kappa must be non-negative, got {kappa}");
        self.kappa = kappa;
        self.sigma = sigma;
        self
    }

    /// Set Hull-White parameters using typed values.
    pub fn with_hw_params_rate(mut self, kappa: Rate, sigma: Percentage) -> Self {
        self.kappa = kappa.as_decimal();
        self.sigma = sigma.as_decimal();
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

        let model = CalibratedHullWhiteModel::calibrate(
            HullWhiteParams::new(self.kappa, sigma),
            self.tree_steps,
            disc,
            ttm,
        )?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &model, disc, as_of)?;
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
            .get_discount(swaption.discount_curve_id.as_str())?;

        // Bump volatility
        let sigma_up = self.sigma * (1.0 + self.bump_pct);
        let sigma_down = self.sigma * (1.0 - self.bump_pct);

        let price_up = self.price_bermudan(swaption, disc.as_ref(), context.as_of, sigma_up)?;
        let price_down = self.price_bermudan(swaption, disc.as_ref(), context.as_of, sigma_down)?;

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
#[derive(Debug, Clone)]
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

    /// Set bump size using typed basis points.
    pub fn with_bump_bps(mut self, bump_bp: Bps) -> Self {
        self.bump_bp = bump_bp.as_bps() as f64;
        self
    }

    /// Set Hull-White parameters.
    ///
    /// # Panics (debug only)
    /// Panics if `sigma <= 0` or `kappa < 0`, which would produce invalid tree calibration.
    pub fn with_hw_params(mut self, kappa: f64, sigma: f64) -> Self {
        debug_assert!(sigma > 0.0, "HW sigma must be positive, got {sigma}");
        debug_assert!(kappa >= 0.0, "HW kappa must be non-negative, got {kappa}");
        self.kappa = kappa;
        self.sigma = sigma;
        self
    }

    /// Set Hull-White parameters using typed values.
    pub fn with_hw_params_rate(mut self, kappa: Rate, sigma: Percentage) -> Self {
        self.kappa = kappa.as_decimal();
        self.sigma = sigma.as_decimal();
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

        let model = CalibratedHullWhiteModel::calibrate(
            HullWhiteParams::new(self.kappa, sigma),
            self.tree_steps,
            disc,
            ttm,
        )?;
        let valuator = BermudanSwaptionTreeValuator::new(swaption, &model, disc, as_of)?;
        Ok(valuator.price())
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
            .get_discount(swaption.discount_curve_id.as_str())?;
        let ttm = swaption.time_to_maturity(context.as_of)?;

        if ttm <= 0.0 {
            return Ok(0.0);
        }

        let bump_bp = self.bump_bp.abs();
        let bump = bump_bp / 10000.0;
        if bump <= 0.0 {
            return Ok(0.0);
        }

        let base_price = self.price_bermudan(swaption, disc.as_ref(), context.as_of, self.sigma)?;

        // Bump discount curve only -- see BermudanDeltaCalculator::calculate()
        // for documentation on single-factor HW tree limitations.
        let curves_up = context.curves.bump([MarketBump::Curve {
            id: swaption.discount_curve_id.clone(),
            spec: BumpSpec::parallel_bp(bump_bp),
        }])?;
        let curves_dn = context.curves.bump([MarketBump::Curve {
            id: swaption.discount_curve_id.clone(),
            spec: BumpSpec::parallel_bp(-bump_bp),
        }])?;

        let disc_up = curves_up.get_discount(swaption.discount_curve_id.as_str())?;
        let disc_dn = curves_dn.get_discount(swaption.discount_curve_id.as_str())?;

        let price_up =
            self.price_bermudan(swaption, disc_up.as_ref(), context.as_of, self.sigma)?;
        let price_dn =
            self.price_bermudan(swaption, disc_dn.as_ref(), context.as_of, self.sigma)?;

        Ok((price_up - 2.0 * base_price + price_dn) / (bump * bump))
    }
}

// ============================================================================
// Exercise Probability Profile
// ============================================================================

/// Exercise probability profile for Bermudan swaptions.
///
/// Shows the risk-neutral probability of exercise at each exercise date.
#[derive(Debug, Clone)]
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
    /// Create from tree valuator using actual computed exercise probabilities.
    ///
    /// Uses the risk-neutral exercise probabilities computed during backward
    /// induction in the Hull-White tree. These probabilities represent the
    /// optimal exercise strategy under the risk-neutral measure.
    ///
    /// # Arguments
    /// * `valuator` - The tree valuator that has computed the optimal exercise boundary
    /// * `exercise_times` - Exercise dates as year fractions (used for validation)
    ///
    /// # Returns
    /// An `ExerciseProbabilityProfile` with actual computed probabilities
    pub fn from_valuator(
        valuator: &BermudanSwaptionTreeValuator,
        exercise_times: Vec<f64>,
    ) -> Self {
        // Get actual exercise probabilities from the tree valuator
        let tree_probs = valuator.exercise_probabilities();

        let n = exercise_times.len();
        if n == 0 || tree_probs.is_empty() {
            return Self {
                exercise_times,
                conditional_probs: Vec::new(),
                cumulative_probs: Vec::new(),
                expected_exercise_time: 0.0,
            };
        }

        // Extract marginal probabilities (probability of exercise at each date)
        // tree_probs returns Vec<(time, probability)>
        let marginal_probs: Vec<f64> = tree_probs.iter().map(|(_, p)| *p).collect();

        // Compute conditional probabilities: P(exercise at t | survived to t)
        // conditional_prob[i] = marginal_prob[i] / (1 - cumulative_prob[i-1])
        let mut conditional_probs = Vec::with_capacity(n);
        let mut cumulative_probs = Vec::with_capacity(n);
        let mut cumulative = 0.0;

        for &marginal in &marginal_probs {
            let survival = 1.0 - cumulative;
            let conditional = if survival > 1e-10 {
                marginal / survival
            } else {
                0.0
            };
            conditional_probs.push(conditional);

            cumulative += marginal;
            // Clamp cumulative to [0, 1] to handle numerical noise
            cumulative_probs.push(cumulative.min(1.0));
        }

        // Expected exercise time = Σ t_i × P(exercise at t_i)
        // For non-exercised paths, we include remaining probability at terminal date
        let expected_exercise_time: f64 = tree_probs.iter().map(|(t, p)| t * p).sum();

        Self {
            exercise_times,
            conditional_probs,
            cumulative_probs,
            expected_exercise_time,
        }
    }
}

/// Calculator for exercise probability metrics.
#[derive(Debug, Clone, Default)]
pub struct ExerciseProbabilityCalculator {
    /// Hull-White mean reversion
    pub kappa: f64,
    /// Hull-White volatility
    pub sigma: f64,
    /// Tree steps
    pub tree_steps: usize,
}

impl ExerciseProbabilityCalculator {
    /// Create a new calculator with default (uncalibrated) parameters.
    pub fn new() -> Self {
        Self {
            kappa: DEFAULT_KAPPA,
            sigma: DEFAULT_SIGMA,
            tree_steps: DEFAULT_TREE_STEPS,
        }
    }

    /// Create a new calculator with calibrated Hull-White parameters.
    ///
    /// # Panics (debug only)
    /// Panics if `sigma <= 0` or `kappa < 0`, which would produce invalid tree calibration.
    pub fn new_with_hw(kappa: f64, sigma: f64) -> Self {
        debug_assert!(sigma > 0.0, "HW sigma must be positive, got {sigma}");
        debug_assert!(kappa >= 0.0, "HW kappa must be non-negative, got {kappa}");
        Self {
            kappa,
            sigma,
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
            .get_discount(swaption.discount_curve_id.as_str())?;
        let ttm = swaption.time_to_maturity(context.as_of)?;

        if ttm <= 0.0 {
            return Ok(0.0);
        }

        let model = CalibratedHullWhiteModel::calibrate(
            HullWhiteParams::new(self.kappa, self.sigma),
            self.tree_steps,
            disc.as_ref(),
            ttm,
        )?;
        let valuator =
            BermudanSwaptionTreeValuator::new(swaption, &model, disc.as_ref(), context.as_of)?;

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
    fn test_exercise_probability_profile_construction() {
        // Test manual construction of profile
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

    #[test]
    fn test_exercise_probability_profile_from_valuator() {
        // Integration test: verify from_valuator uses actual tree probabilities
        use crate::instruments::rates::swaption::{
            BermudanSchedule, BermudanSwaption, BermudanType, CalibratedHullWhiteModel,
            HullWhiteParams, SwaptionSettlement,
        };
        use finstack_core::currency::Currency;
        use finstack_core::dates::{DayCount, Tenor};
        use finstack_core::market_data::term_structures::DiscountCurve;
        use finstack_core::math::interp::InterpStyle;
        use finstack_core::money::Money;
        use finstack_core::types::{CurveId, InstrumentId};
        use time::Month;

        // Create test discount curve
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
            .knots([
                (0.0, 1.0),
                (0.5, 0.985),
                (1.0, 0.97),
                (2.0, 0.94),
                (5.0, 0.85),
            ])
            .interp(InterpStyle::LogLinear)
            .build()
            .expect("Valid curve");

        // Create test Bermudan swaption
        let swap_start = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let swap_end = Date::from_calendar_date(2028, Month::January, 1).expect("Valid date");
        let first_exercise = Date::from_calendar_date(2026, Month::January, 1).expect("Valid date");

        let swaption = BermudanSwaption {
            id: InstrumentId::new("TEST-BERM"),
            option_type: crate::instruments::common_impl::parameters::OptionType::Call,
            notional: Money::new(10_000_000.0, Currency::USD),
            strike_rate: 0.03,
            swap_start,
            swap_end,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            day_count: DayCount::Thirty360,
            settlement: SwaptionSettlement::Physical,
            discount_curve_id: CurveId::new("USD-OIS"),
            forward_curve_id: CurveId::new("USD-SOFR"),
            vol_surface_id: CurveId::new("USD-VOL"),
            bermudan_schedule: BermudanSchedule::co_terminal(
                first_exercise,
                swap_end,
                Tenor::semi_annual(),
            )
            .expect("valid Bermudan schedule"),
            bermudan_type: BermudanType::CoTerminal,
            calendar_id: None,
            pricing_overrides: Default::default(),
            attributes: Default::default(),
        };

        let as_of = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let ttm = swaption.time_to_maturity(as_of).expect("Valid ttm");

        let model =
            CalibratedHullWhiteModel::calibrate(HullWhiteParams::new(0.03, 0.01), 30, &curve, ttm)
                .expect("Valid model");
        let valuator = BermudanSwaptionTreeValuator::new(&swaption, &model, &curve, as_of)
            .expect("Valid valuator");

        let exercise_times = swaption
            .exercise_times(as_of)
            .expect("Valid exercise times");
        let profile = ExerciseProbabilityProfile::from_valuator(&valuator, exercise_times.clone());

        // Verify profile has correct structure
        assert_eq!(profile.exercise_times.len(), exercise_times.len());
        assert_eq!(profile.conditional_probs.len(), exercise_times.len());
        assert_eq!(profile.cumulative_probs.len(), exercise_times.len());

        // Cumulative probabilities should be non-decreasing
        for i in 1..profile.cumulative_probs.len() {
            assert!(
                profile.cumulative_probs[i] >= profile.cumulative_probs[i - 1] - 1e-10,
                "Cumulative probs should be non-decreasing"
            );
        }

        // Conditional probabilities should be in [0, 1]
        for &p in &profile.conditional_probs {
            assert!(
                (0.0..=1.0 + 1e-10).contains(&p),
                "Conditional probs should be in [0, 1]"
            );
        }

        // Expected exercise time should be reasonable
        assert!(profile.expected_exercise_time >= 0.0);
    }
}
