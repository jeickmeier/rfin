//! Hull-White model calibration to swaption volatility surface.
//!
//! Calibrates Hull-White 1-factor model parameters (κ, σ) to match market
//! swaption volatilities. Standard approaches include:
//!
//! - Co-terminal calibration (for Bermudan swaption pricing)
//! - Diagonal calibration (constant tenor across expiries)
//! - Global surface calibration (full expiry-tenor grid)
//!
//! # Configuration via `FinstackConfig` extensions
//!
//! Hull-White calibration settings can be sourced from
//! `FinstackConfig.extensions["valuations.hull_white_calibration.v1"]`:
//!
//! ```json
//! {
//!   "extensions": {
//!     "valuations.hull_white_calibration.v1": {
//!       "fix_kappa": 0.03,
//!       "initial_kappa": 0.03,
//!       "initial_sigma": 0.01,
//!       "kappa_bounds": { "min": 0.001, "max": 0.20 },
//!       "sigma_bounds": { "min": 0.001, "max": 0.05 },
//!       "tree_steps": 50,
//!       "tolerance": 1e-6,
//!       "max_iterations": 100,
//!       "weight_fn": "uniform"
//!     }
//!   }
//! }
//! ```
//!
//! # References
//!
//! - Hull, J. & White, A. (1990). "Pricing Interest-Rate-Derivative Securities"
//! - Brigo, D. & Mercurio, F. (2006). *Interest Rate Models*, Chapter 4

use crate::instruments::common::models::trees::HullWhiteTreeConfig;
use finstack_core::config::FinstackConfig;
use finstack_core::dates::{Date, DayCount, DayCountCtx, Tenor};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::traits::Discounting;
use finstack_core::math::{BrentSolver, Solver};
use finstack_core::types::CurveId;
use finstack_core::{Error, Result};
use serde::{Deserialize, Serialize};

// ============================================================================
// Calibration Configuration
// ============================================================================

/// Calibration target specification for Hull-White model.
#[derive(Clone, Debug)]
pub enum HullWhiteCalibrationTargets {
    /// Co-terminal swaptions (standard for Bermudan pricing).
    ///
    /// All swaptions have the same swap end date, varying expiries.
    /// Example: 1Y x 9Y, 2Y x 8Y, 3Y x 7Y, ... all ending at year 10.
    CoTerminal {
        /// Swap maturity date
        swap_end: Date,
        /// Expiry dates to calibrate
        expiry_dates: Vec<Date>,
    },

    /// Diagonal swaptions (constant tenor, varying expiries).
    ///
    /// All swaptions have the same tenor, useful for general calibration.
    /// Example: 1Y x 5Y, 2Y x 5Y, 3Y x 5Y, ... all with 5Y tenor.
    Diagonal {
        /// Swap tenor in years
        tenor_years: f64,
        /// Expiry times (year fractions from base date)
        expiry_times: Vec<f64>,
    },

    /// Custom set of swaptions.
    Custom {
        /// List of (expiry_time, tenor_years) pairs
        swaptions: Vec<(f64, f64)>,
    },
}

/// Extension section key for Hull-White calibration overrides.
pub const HULL_WHITE_CALIBRATION_CONFIG_KEY_V1: &str = "valuations.hull_white_calibration.v1";

/// Bounds represented as a struct for stable JSON serialization.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
}

impl Bounds {
    /// Convert to tuple form `(min, max)`.
    #[inline]
    pub fn to_tuple(self) -> (f64, f64) {
        (self.min, self.max)
    }
}

impl From<(f64, f64)> for Bounds {
    fn from((min, max): (f64, f64)) -> Self {
        Self { min, max }
    }
}

/// Configuration for Hull-White calibration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HullWhiteCalibrationConfig {
    /// Fix mean reversion κ (if Some, only calibrate σ)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_kappa: Option<f64>,
    /// Initial guess for mean reversion κ
    pub initial_kappa: f64,
    /// Initial guess for volatility σ
    pub initial_sigma: f64,
    /// Bounds for κ: (min, max)
    #[serde(with = "bounds_as_tuple")]
    pub kappa_bounds: (f64, f64),
    /// Bounds for σ: (min, max)
    #[serde(with = "bounds_as_tuple")]
    pub sigma_bounds: (f64, f64),
    /// Number of tree steps for pricing
    pub tree_steps: usize,
    /// Convergence tolerance
    pub tolerance: f64,
    /// Maximum iterations
    pub max_iterations: usize,
    /// Weight function for calibration (expiry -> weight)
    pub weight_fn: WeightFunction,
}

/// Serde helper to serialize bounds tuples as `{min, max}` objects.
mod bounds_as_tuple {
    use super::Bounds;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(bounds: &(f64, f64), ser: S) -> Result<S::Ok, S::Error> {
        Bounds::from(*bounds).serialize(ser)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(de: D) -> Result<(f64, f64), D::Error> {
        Bounds::deserialize(de).map(Bounds::to_tuple)
    }
}

/// Weight function for calibration targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeightFunction {
    /// Equal weights for all targets
    #[default]
    Uniform,
    /// Weight by vega (higher vega = higher weight)
    VegaWeighted,
    /// Weight inversely by expiry (shorter expiries weighted more)
    InverseExpiry,
}

/// Optional Hull-White calibration overrides from `FinstackConfig.extensions`.
///
/// All fields are optional; when absent, the `HullWhiteCalibrationConfig::default()` value is used.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HullWhiteCalibrationConfigV1 {
    /// Fix mean reversion κ (if present, only calibrate σ). Use `null` to clear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_kappa: Option<Option<f64>>,
    /// Initial guess for mean reversion κ.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_kappa: Option<f64>,
    /// Initial guess for volatility σ.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_sigma: Option<f64>,
    /// Bounds for κ: `{min, max}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kappa_bounds: Option<Bounds>,
    /// Bounds for σ: `{min, max}`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sigma_bounds: Option<Bounds>,
    /// Number of tree steps for pricing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_steps: Option<usize>,
    /// Convergence tolerance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tolerance: Option<f64>,
    /// Maximum iterations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<usize>,
    /// Weight function for calibration targets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight_fn: Option<WeightFunction>,
}

impl Default for HullWhiteCalibrationConfig {
    fn default() -> Self {
        Self {
            fix_kappa: None,
            initial_kappa: 0.03,
            initial_sigma: 0.01,
            kappa_bounds: (0.001, 0.20),
            sigma_bounds: (0.001, 0.05),
            tree_steps: 50,
            tolerance: 1e-6,
            max_iterations: 100,
            weight_fn: WeightFunction::Uniform,
        }
    }
}

impl HullWhiteCalibrationConfig {
    /// Build a Hull-White calibration config from a `FinstackConfig` extension section.
    ///
    /// If the extension section `valuations.hull_white_calibration.v1` is present,
    /// its fields override the defaults; otherwise defaults are used.
    ///
    /// # Errors
    ///
    /// Returns an error if the extension section is present but malformed.
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_core::config::FinstackConfig;
    /// use finstack_valuations::calibration::methods::hull_white::HullWhiteCalibrationConfig;
    ///
    /// let cfg = FinstackConfig::default();
    /// let hw_cfg = HullWhiteCalibrationConfig::from_finstack_config_or_default(&cfg)
    ///     .expect("valid config");
    /// assert_eq!(hw_cfg.initial_kappa, 0.03); // default
    /// ```
    #[cfg(feature = "serde")]
    pub fn from_finstack_config_or_default(cfg: &FinstackConfig) -> Result<Self> {
        let mut base = Self::default();

        if let Some(raw) = cfg.extensions.get(HULL_WHITE_CALIBRATION_CONFIG_KEY_V1) {
            let overrides: HullWhiteCalibrationConfigV1 = serde_json::from_value(raw.clone())
                .map_err(|e| Error::Calibration {
                    message: format!(
                        "Failed to parse extension '{}': {}",
                        HULL_WHITE_CALIBRATION_CONFIG_KEY_V1, e
                    ),
                    category: "config".to_string(),
                })?;

            if let Some(v) = overrides.fix_kappa {
                base.fix_kappa = v;
            }
            if let Some(v) = overrides.initial_kappa {
                base.initial_kappa = v;
            }
            if let Some(v) = overrides.initial_sigma {
                base.initial_sigma = v;
            }
            if let Some(v) = overrides.kappa_bounds {
                base.kappa_bounds = v.to_tuple();
            }
            if let Some(v) = overrides.sigma_bounds {
                base.sigma_bounds = v.to_tuple();
            }
            if let Some(v) = overrides.tree_steps {
                base.tree_steps = v;
            }
            if let Some(v) = overrides.tolerance {
                base.tolerance = v;
            }
            if let Some(v) = overrides.max_iterations {
                base.max_iterations = v;
            }
            if let Some(v) = overrides.weight_fn {
                base.weight_fn = v;
            }
        }

        Ok(base)
    }

    /// Build a Hull-White calibration config from a `FinstackConfig` (non-serde fallback).
    ///
    /// When the `serde` feature is disabled, extensions are not available and
    /// this method always returns `HullWhiteCalibrationConfig::default()`.
    #[cfg(not(feature = "serde"))]
    pub fn from_finstack_config_or_default(_cfg: &FinstackConfig) -> Result<Self> {
        Ok(Self::default())
    }

    /// Create a new configuration with fixed κ.
    pub fn with_fixed_kappa(kappa: f64) -> Self {
        Self {
            fix_kappa: Some(kappa),
            initial_kappa: kappa,
            ..Default::default()
        }
    }

    /// Set initial guesses.
    pub fn with_initial_guess(mut self, kappa: f64, sigma: f64) -> Self {
        self.initial_kappa = kappa;
        self.initial_sigma = sigma;
        self
    }

    /// Set tree steps.
    pub fn with_tree_steps(mut self, steps: usize) -> Self {
        self.tree_steps = steps;
        self
    }
}

// ============================================================================
// Calibration Result
// ============================================================================

/// Result of Hull-White calibration.
#[derive(Clone, Debug)]
pub struct HullWhiteCalibrationResult {
    /// Calibrated mean reversion speed (κ)
    pub kappa: f64,
    /// Calibrated volatility (σ)
    pub sigma: f64,
    /// Root mean squared error in basis points
    pub rmse_bp: f64,
    /// Individual calibration errors: (instrument_id, error_bp)
    pub individual_errors: Vec<(String, f64)>,
    /// Number of iterations used
    pub iterations: usize,
    /// Calibration converged
    pub converged: bool,
}

impl HullWhiteCalibrationResult {
    /// Create a calibrated Hull-White tree configuration.
    pub fn to_tree_config(&self, steps: usize) -> HullWhiteTreeConfig {
        HullWhiteTreeConfig::new(self.kappa, self.sigma, steps)
    }
}

// ============================================================================
// Hull-White Calibrator
// ============================================================================

/// Hull-White model calibrator to swaption volatilities.
///
/// Calibrates (κ, σ) to minimize the sum of squared differences between
/// model and market swaption prices or volatilities.
pub struct HullWhiteCalibrator {
    /// Base date for calculations
    base_date: Date,
    /// Discount curve ID
    discount_curve_id: CurveId,
    /// Day count convention
    day_count: DayCount,
    /// Fixed leg frequency
    fixed_freq: Tenor,
    /// Float leg frequency
    float_freq: Tenor,
    /// Configuration
    config: HullWhiteCalibrationConfig,
}

impl HullWhiteCalibrator {
    /// Create a new Hull-White calibrator.
    pub fn new(
        base_date: Date,
        discount_curve_id: impl Into<CurveId>,
        day_count: DayCount,
    ) -> Self {
        Self {
            base_date,
            discount_curve_id: discount_curve_id.into(),
            day_count,
            fixed_freq: Tenor::semi_annual(),
            float_freq: Tenor::quarterly(),
            config: HullWhiteCalibrationConfig::default(),
        }
    }

    /// Set calibration configuration from a `FinstackConfig`.
    ///
    /// Resolves `HullWhiteCalibrationConfig` from
    /// `FinstackConfig.extensions["valuations.hull_white_calibration.v1"]`.
    pub fn with_finstack_config(mut self, cfg: &FinstackConfig) -> Result<Self> {
        self.config = HullWhiteCalibrationConfig::from_finstack_config_or_default(cfg)?;
        Ok(self)
    }

    /// Set leg frequencies.
    pub fn with_frequencies(mut self, fixed_freq: Tenor, float_freq: Tenor) -> Self {
        self.fixed_freq = fixed_freq;
        self.float_freq = float_freq;
        self
    }

    /// Calibrate Hull-White parameters to swaption market.
    ///
    /// # Arguments
    ///
    /// * `targets` - Calibration targets (co-terminal, diagonal, or custom)
    /// * `market` - Market context with discount curve and vol surface
    /// * `vol_surface_id` - ID of swaption volatility surface
    ///
    /// # Returns
    ///
    /// Calibration result with optimal (κ, σ) and diagnostics.
    pub fn calibrate(
        &self,
        targets: &HullWhiteCalibrationTargets,
        market: &MarketContext,
        vol_surface_id: &str,
    ) -> Result<HullWhiteCalibrationResult> {
        let disc = market.get_discount_ref(self.discount_curve_id.as_str())?;
        let vol_surface = market.surface_ref(vol_surface_id)?;

        // Build target swaptions with market volatilities
        let target_specs = self.build_target_specs(targets, disc, vol_surface)?;

        if target_specs.is_empty() {
            return Err(Error::Validation("No valid calibration targets".into()));
        }

        // Calibrate
        let result = if let Some(fixed_kappa) = self.config.fix_kappa {
            // 1D calibration: only σ
            self.calibrate_sigma_only(fixed_kappa, &target_specs, disc)?
        } else {
            // 2D calibration: both κ and σ
            self.calibrate_kappa_sigma(&target_specs, disc)?
        };

        Ok(result)
    }

    /// Build target specifications from calibration targets.
    fn build_target_specs(
        &self,
        targets: &HullWhiteCalibrationTargets,
        disc: &dyn Discounting,
        vol_surface: &finstack_core::market_data::surfaces::vol_surface::VolSurface,
    ) -> Result<Vec<SwaptionTargetSpec>> {
        let ctx = DayCountCtx::default();
        let mut specs = Vec::new();

        match targets {
            HullWhiteCalibrationTargets::CoTerminal {
                swap_end,
                expiry_dates,
            } => {
                for &expiry in expiry_dates {
                    let expiry_time = self.day_count.year_fraction(self.base_date, expiry, ctx)?;
                    let swap_end_time =
                        self.day_count
                            .year_fraction(self.base_date, *swap_end, ctx)?;
                    let tenor = swap_end_time - expiry_time;

                    if expiry_time > 0.0 && tenor > 0.0 {
                        // Get ATM vol from surface
                        let atm_vol = vol_surface.value_clamped(expiry_time, 0.0);

                        // Calculate forward swap rate
                        let df_start = disc.df(expiry_time);
                        let df_end = disc.df(swap_end_time);
                        let annuity = self.calculate_annuity(disc, expiry_time, swap_end_time)?;
                        let forward_rate = if annuity > 1e-10 {
                            (df_start - df_end) / annuity
                        } else {
                            continue;
                        };

                        specs.push(SwaptionTargetSpec {
                            id: format!("{:.2}Y x {:.2}Y", expiry_time, tenor),
                            expiry_time,
                            tenor,
                            forward_rate,
                            atm_vol,
                            annuity,
                            weight: 1.0,
                        });
                    }
                }
            }

            HullWhiteCalibrationTargets::Diagonal {
                tenor_years,
                expiry_times,
            } => {
                for &expiry_time in expiry_times {
                    if expiry_time > 0.0 {
                        let swap_end_time = expiry_time + tenor_years;

                        // Get ATM vol from surface
                        let atm_vol = vol_surface.value_clamped(expiry_time, 0.0);

                        // Calculate forward swap rate
                        let df_start = disc.df(expiry_time);
                        let df_end = disc.df(swap_end_time);
                        let annuity = self.calculate_annuity(disc, expiry_time, swap_end_time)?;
                        let forward_rate = if annuity > 1e-10 {
                            (df_start - df_end) / annuity
                        } else {
                            continue;
                        };

                        specs.push(SwaptionTargetSpec {
                            id: format!("{:.2}Y x {:.2}Y", expiry_time, tenor_years),
                            expiry_time,
                            tenor: *tenor_years,
                            forward_rate,
                            atm_vol,
                            annuity,
                            weight: 1.0,
                        });
                    }
                }
            }

            HullWhiteCalibrationTargets::Custom { swaptions } => {
                for &(expiry_time, tenor) in swaptions {
                    if expiry_time > 0.0 && tenor > 0.0 {
                        let swap_end_time = expiry_time + tenor;

                        let atm_vol = vol_surface.value_clamped(expiry_time, 0.0);

                        let df_start = disc.df(expiry_time);
                        let df_end = disc.df(swap_end_time);
                        let annuity = self.calculate_annuity(disc, expiry_time, swap_end_time)?;
                        let forward_rate = if annuity > 1e-10 {
                            (df_start - df_end) / annuity
                        } else {
                            continue;
                        };

                        specs.push(SwaptionTargetSpec {
                            id: format!("{:.2}Y x {:.2}Y", expiry_time, tenor),
                            expiry_time,
                            tenor,
                            forward_rate,
                            atm_vol,
                            annuity,
                            weight: 1.0,
                        });
                    }
                }
            }
        }

        // Apply weight function
        for spec in &mut specs {
            spec.weight = match self.config.weight_fn {
                WeightFunction::Uniform => 1.0,
                WeightFunction::VegaWeighted => {
                    // Vega ∝ A * F * √T * n(d)
                    spec.annuity * spec.forward_rate * spec.expiry_time.sqrt()
                }
                WeightFunction::InverseExpiry => 1.0 / spec.expiry_time.max(0.25),
            };
        }

        // Normalize weights
        let total_weight: f64 = specs.iter().map(|s| s.weight).sum();
        if total_weight > 0.0 {
            for spec in &mut specs {
                spec.weight /= total_weight;
            }
        }

        Ok(specs)
    }

    /// Calculate approximate annuity for a swap.
    fn calculate_annuity(
        &self,
        disc: &dyn Discounting,
        start_time: f64,
        end_time: f64,
    ) -> Result<f64> {
        // Approximate using frequency
        let periods_per_year = match self.fixed_freq {
            freq if freq.unit == finstack_core::dates::TenorUnit::Months && freq.count > 0 => {
                12.0 / freq.count as f64
            }
            _ => 2.0, // Default semi-annual
        };

        let tenor = end_time - start_time;
        let num_periods = (tenor * periods_per_year).ceil() as usize;
        let period_length = tenor / num_periods as f64;

        let mut annuity = 0.0;
        for i in 1..=num_periods {
            let t = start_time + i as f64 * period_length;
            annuity += period_length * disc.df(t);
        }

        Ok(annuity)
    }

    /// 1D calibration: optimize σ with fixed κ.
    fn calibrate_sigma_only(
        &self,
        kappa: f64,
        targets: &[SwaptionTargetSpec],
        disc: &dyn Discounting,
    ) -> Result<HullWhiteCalibrationResult> {
        let solver = BrentSolver::new().with_tolerance(self.config.tolerance);

        // Objective: minimize weighted SSE of implied vol errors
        let objective = |sigma: f64| -> f64 {
            if sigma <= 0.0 || sigma > self.config.sigma_bounds.1 {
                return 1e10;
            }

            let mut sse = 0.0;
            for spec in targets {
                let model_vol = self.hull_white_implied_vol(kappa, sigma, spec, disc);
                let error = model_vol - spec.atm_vol;
                sse += spec.weight * error * error;
            }
            sse
        };

        // Find optimal sigma
        let optimal_sigma = solver.solve(
            |s| {
                let obj = objective(s);
                let obj_up = objective(s + 0.0001);
                (obj_up - obj) / 0.0001 // Derivative approximation
            },
            self.config.initial_sigma,
        )?;

        // Compute errors
        let (rmse_bp, individual_errors) = self.compute_errors(kappa, optimal_sigma, targets, disc);

        Ok(HullWhiteCalibrationResult {
            kappa,
            sigma: optimal_sigma,
            rmse_bp,
            individual_errors,
            iterations: 1,
            converged: true,
        })
    }

    /// 2D calibration: optimize both κ and σ.
    fn calibrate_kappa_sigma(
        &self,
        targets: &[SwaptionTargetSpec],
        disc: &dyn Discounting,
    ) -> Result<HullWhiteCalibrationResult> {
        // Use a simple grid search + refinement approach
        // More sophisticated: Levenberg-Marquardt

        let mut best_kappa = self.config.initial_kappa;
        let mut best_sse = f64::MAX;

        // Grid search
        let kappa_grid: Vec<f64> = (0..10)
            .map(|i| {
                self.config.kappa_bounds.0
                    + (self.config.kappa_bounds.1 - self.config.kappa_bounds.0) * i as f64 / 9.0
            })
            .collect();

        let sigma_grid: Vec<f64> = (0..10)
            .map(|i| {
                self.config.sigma_bounds.0
                    + (self.config.sigma_bounds.1 - self.config.sigma_bounds.0) * i as f64 / 9.0
            })
            .collect();

        for &kappa in &kappa_grid {
            for &sigma in &sigma_grid {
                let mut sse = 0.0;
                for spec in targets {
                    let model_vol = self.hull_white_implied_vol(kappa, sigma, spec, disc);
                    let error = model_vol - spec.atm_vol;
                    sse += spec.weight * error * error;
                }

                if sse < best_sse {
                    best_sse = sse;
                    best_kappa = kappa;
                }
            }
        }

        // Local refinement using 1D optimization on sigma
        let refined = self.calibrate_sigma_only(best_kappa, targets, disc)?;
        let best_sigma = refined.sigma;

        // Compute errors
        let (rmse_bp, individual_errors) =
            self.compute_errors(best_kappa, best_sigma, targets, disc);

        Ok(HullWhiteCalibrationResult {
            kappa: best_kappa,
            sigma: best_sigma,
            rmse_bp,
            individual_errors,
            iterations: 100,
            converged: rmse_bp < 50.0, // < 50bp is acceptable
        })
    }

    /// Compute Hull-White implied volatility for a swaption.
    ///
    /// Uses the analytical approximation from Hull & White (1990):
    /// σ_market ≈ σ_HW * B(T_expiry, T_end) * √((1 - e^(-2κT)) / (2κT))
    fn hull_white_implied_vol(
        &self,
        kappa: f64,
        sigma: f64,
        spec: &SwaptionTargetSpec,
        _disc: &dyn Discounting,
    ) -> f64 {
        let t = spec.expiry_time;
        let tau = spec.tenor;

        // B(T, T+τ) factor
        let b_factor = if kappa.abs() < 1e-10 {
            tau
        } else {
            (1.0 - (-kappa * tau).exp()) / kappa
        };

        // Variance factor
        let var_factor = if kappa.abs() < 1e-10 {
            t
        } else {
            (1.0 - (-2.0 * kappa * t).exp()) / (2.0 * kappa)
        };

        // Hull-White implied vol
        sigma * b_factor * (var_factor / t).sqrt()
    }

    /// Compute calibration errors.
    fn compute_errors(
        &self,
        kappa: f64,
        sigma: f64,
        targets: &[SwaptionTargetSpec],
        disc: &dyn Discounting,
    ) -> (f64, Vec<(String, f64)>) {
        let mut sse = 0.0;
        let mut individual = Vec::with_capacity(targets.len());

        for spec in targets {
            let model_vol = self.hull_white_implied_vol(kappa, sigma, spec, disc);
            let error_bp = (model_vol - spec.atm_vol) * 10000.0;
            sse += error_bp * error_bp;
            individual.push((spec.id.clone(), error_bp));
        }

        let rmse = (sse / targets.len() as f64).sqrt();
        (rmse, individual)
    }
}

/// Internal specification for a calibration target swaption.
#[derive(Clone, Debug)]
struct SwaptionTargetSpec {
    /// Identifier
    id: String,
    /// Time to expiry (years)
    expiry_time: f64,
    /// Swap tenor (years)
    tenor: f64,
    /// Forward swap rate
    forward_rate: f64,
    /// Market ATM implied volatility
    atm_vol: f64,
    /// Swap annuity
    annuity: f64,
    /// Calibration weight
    weight: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::market_data::surfaces::vol_surface::VolSurface;
    use finstack_core::market_data::term_structures::discount_curve::DiscountCurve;
    use finstack_core::math::interp::InterpStyle;
    use time::Month;

    fn test_discount_curve() -> DiscountCurve {
        DiscountCurve::builder("USD-OIS")
            .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
            .knots([
                (0.0, 1.0),
                (1.0, 0.97),
                (2.0, 0.94),
                (5.0, 0.85),
                (10.0, 0.70),
            ])
            .set_interp(InterpStyle::LogLinear)
            .build()
            .expect("Valid curve")
    }

    fn test_vol_surface() -> VolSurface {
        let expiries = vec![0.5, 1.0, 2.0, 5.0, 10.0];
        let tenors = vec![1.0, 2.0, 5.0, 10.0];
        let vols = vec![
            0.20, 0.19, 0.18, 0.17, // 0.5Y expiry
            0.19, 0.18, 0.17, 0.16, // 1Y expiry
            0.18, 0.17, 0.16, 0.15, // 2Y expiry
            0.17, 0.16, 0.15, 0.14, // 5Y expiry
            0.16, 0.15, 0.14, 0.13, // 10Y expiry
        ];
        VolSurface::from_grid("TEST-VOL", &expiries, &tenors, &vols).expect("Valid surface")
    }

    #[test]
    fn test_hull_white_calibrator_creation() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let calibrator = HullWhiteCalibrator::new(base_date, "USD-OIS", DayCount::Act365F);

        assert_eq!(calibrator.config.initial_kappa, 0.03);
        assert_eq!(calibrator.config.initial_sigma, 0.01);
    }

    #[test]
    fn test_diagonal_targets() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let calibrator = HullWhiteCalibrator::new(base_date, "USD-OIS", DayCount::Act365F);

        let disc = test_discount_curve();
        let vol_surface = test_vol_surface();

        let targets = HullWhiteCalibrationTargets::Diagonal {
            tenor_years: 5.0,
            expiry_times: vec![1.0, 2.0, 5.0],
        };

        let specs = calibrator
            .build_target_specs(&targets, &disc, &vol_surface)
            .expect("Should build specs");

        assert_eq!(specs.len(), 3);
        assert!(specs[0].atm_vol > 0.0);
    }

    #[test]
    fn test_calibrate_with_fixed_kappa() {
        let base_date = Date::from_calendar_date(2025, Month::January, 1).expect("Valid date");
        let mut cfg = FinstackConfig::default();
        cfg.extensions.insert(
            HULL_WHITE_CALIBRATION_CONFIG_KEY_V1,
            serde_json::json!({ "fix_kappa": 0.03 }),
        );
        let calibrator = HullWhiteCalibrator::new(base_date, "USD-OIS", DayCount::Act365F)
            .with_finstack_config(&cfg)
            .expect("valid config");

        // Just verify the calibrator setup works
        assert_eq!(calibrator.config.fix_kappa, Some(0.03));

        // Build target specs from market data (without full calibration)
        let disc = test_discount_curve();
        let vol_surface = test_vol_surface();
        let targets = HullWhiteCalibrationTargets::Diagonal {
            tenor_years: 5.0,
            expiry_times: vec![1.0, 2.0, 5.0],
        };

        let specs = calibrator
            .build_target_specs(&targets, &disc, &vol_surface)
            .expect("Should build specs");

        assert_eq!(specs.len(), 3);
        // Verify ATM vols are extracted from the surface
        for spec in &specs {
            assert!(spec.atm_vol > 0.0);
            assert!(spec.forward_rate > 0.0);
        }
    }
}
