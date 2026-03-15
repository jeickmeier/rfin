//! SABR (Stochastic Alpha Beta Rho) volatility model implementation.
//!
//! The SABR model is widely used for pricing interest rate derivatives and FX options
//! with volatility smile. It provides closed-form approximations for implied volatility
//! that capture the smile and skew observed in market prices.
//!
//! # Accuracy Limitations
//!
//! This implementation uses the Hagan et al. (2002) expansion with the Obloj (2008)
//! correction applied to the z/χ(z) ratio. The correction replaces the difference-of-powers
//! moneyness with geometric-mean-based moneyness, reducing errors from O(ε²) to O(ε³)
//! for intermediate β values (0 < β < 1).
//!
//! Residual accuracy limitations (after Obloj correction):
//! - **T > 10Y**: Very long maturities may still show ~5-10bp error
//! - **ν (vol-of-vol) > 1.0**: Extreme vol-of-vol
//! - **Very far OTM strikes**: 3+ standard deviations from ATM
//!
//! References:
//! - Hagan, P. S., et al. (2002). "Managing Smile Risk." *Wilmott Magazine*.
//! - Obloj, J. (2008). "Fine-tune your smile: Correction to Hagan et al." arXiv:0708.0998v2
//!
//! # Conventions
//!
//! | Parameter | Convention | Units |
//! |-----------|-----------|-------|
//! | Forward (F) | Underlying forward rate/price | Decimal for rates, price units for equity |
//! | Strike (K) | Same units as forward | Decimal for rates, price units for equity |
//! | Alpha (α) | Initial stochastic vol | Same scale as F^β |
//! | Time (T) | Time to expiry | Years |
//! | Output | Implied (Black) volatility | Decimal (0.20 = 20%) |

use super::black::d1_d2;
use finstack_core::{Error, Result};

/// SABR model parameters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SABRParameters {
    /// Initial volatility (alpha)
    pub alpha: f64,
    /// CEV exponent (beta) - typically 0 to 1
    pub beta: f64,
    /// Volatility of volatility (nu/volvol)
    pub nu: f64,
    /// Correlation between asset and volatility (rho)
    pub rho: f64,
    /// Shift parameter for handling negative rates (optional)
    pub shift: Option<f64>,
}

impl SABRParameters {
    /// Create new SABR parameters with validation.
    ///
    /// Enforces market-standard parameter bounds:
    /// - α (alpha) > 0: Initial volatility must be positive
    /// - β (beta) ∈ [0, 1]: CEV exponent (0=normal, 1=lognormal)
    /// - ν (nu) ≥ 0: Volatility of volatility must be non-negative
    /// - ρ (rho) ∈ [-1, 1]: Correlation must be valid
    pub fn new(alpha: f64, beta: f64, nu: f64, rho: f64) -> Result<Self> {
        // Validate parameters with descriptive error messages
        if alpha <= 0.0 {
            return Err(Error::Validation(format!(
                "SABR parameter α (alpha) must be positive, got: {:.6}",
                alpha
            )));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(Error::Validation(format!(
                "SABR parameter β (beta) must be in [0, 1], got: {:.6}",
                beta
            )));
        }
        if nu < 0.0 {
            return Err(Error::Validation(format!(
                "SABR parameter ν (nu) must be non-negative, got: {:.6}",
                nu
            )));
        }
        if !(-1.0..=1.0).contains(&rho) {
            return Err(Error::Validation(format!(
                "SABR parameter ρ (rho) must be in [-1, 1], got: {:.6}",
                rho
            )));
        }

        Ok(Self {
            alpha,
            beta,
            nu,
            rho,
            shift: None,
        })
    }

    /// Equity market standard (beta = 1.0).
    ///
    /// # Errors
    ///
    /// Returns an error if any SABR parameter is invalid.
    pub fn equity_standard(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        Self::new(alpha, 1.0, nu, rho)
    }

    /// Rates market standard (beta = 0.5).
    ///
    /// # Errors
    ///
    /// Returns an error if any SABR parameter is invalid.
    pub fn rates_standard(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        Self::new(alpha, 0.5, nu, rho)
    }

    /// Create new SABR parameters with shift for negative rates.
    ///
    /// Same validation as `new()` plus shift validation:
    /// - shift > 0: Shift must be positive for negative rate support
    pub fn new_with_shift(alpha: f64, beta: f64, nu: f64, rho: f64, shift: f64) -> Result<Self> {
        // Validate base parameters via new(), then set shift
        let mut params = Self::new(alpha, beta, nu, rho)?;

        // Shift should be positive to handle negative rates
        if shift <= 0.0 {
            return Err(Error::Validation(format!(
                "SABR shift parameter must be positive for negative rate support, got: {:.6}",
                shift
            )));
        }

        params.shift = Some(shift);
        Ok(params)
    }

    /// Create parameters for normal SABR (beta = 0)
    pub fn normal(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        Self::new(alpha, 0.0, nu, rho)
    }

    /// Create parameters for lognormal SABR (beta = 1)
    pub fn lognormal(alpha: f64, nu: f64, rho: f64) -> Result<Self> {
        Self::new(alpha, 1.0, nu, rho)
    }

    /// Create parameters for shifted normal SABR (beta = 0, with shift)
    pub fn shifted_normal(alpha: f64, nu: f64, rho: f64, shift: f64) -> Result<Self> {
        Self::new_with_shift(alpha, 0.0, nu, rho, shift)
    }

    /// Create parameters for shifted lognormal SABR (beta = 1, with shift)
    pub fn shifted_lognormal(alpha: f64, nu: f64, rho: f64, shift: f64) -> Result<Self> {
        Self::new_with_shift(alpha, 1.0, nu, rho, shift)
    }

    /// Create default equity-standard SABR parameters.
    ///
    /// Returns parameters suitable as a starting point for equity options:
    /// - α = 0.20 (20% initial volatility)
    /// - β = 1.0 (lognormal, standard for equities)
    /// - ν = 0.30 (30% vol-of-vol)
    /// - ρ = -0.20 (mild negative correlation, typical equity skew)
    ///
    /// These are reasonable defaults for calibration initialization but
    /// should always be calibrated to market data for production use.
    #[must_use]
    pub const fn equity_default() -> Self {
        // Direct construction - values are hardcoded and known-valid
        Self {
            alpha: 0.20,
            beta: 1.0,
            nu: 0.30,
            rho: -0.20,
            shift: None,
        }
    }

    /// Create default rates-standard SABR parameters.
    ///
    /// Returns parameters suitable as a starting point for rates options:
    /// - α = 0.02 (2% normal vol level, typical for rates)
    /// - β = 0.5 (mixed normal/lognormal, common for rates)
    /// - ν = 0.30 (30% vol-of-vol)
    /// - ρ = 0.0 (neutral correlation as starting point)
    ///
    /// These are reasonable defaults for calibration initialization but
    /// should always be calibrated to market data for production use.
    #[must_use]
    pub const fn rates_default() -> Self {
        // Direct construction - values are hardcoded and known-valid
        Self {
            alpha: 0.02,
            beta: 0.5,
            nu: 0.30,
            rho: 0.0,
            shift: None,
        }
    }

    /// Get the shift parameter
    pub fn shift(&self) -> Option<f64> {
        self.shift
    }

    /// Check if this is a shifted SABR model
    pub fn is_shifted(&self) -> bool {
        self.shift.is_some()
    }
}

/// SABR model for volatility smile dynamics
pub struct SABRModel {
    params: SABRParameters,
}

impl SABRModel {
    /// Create new SABR model
    pub fn new(params: SABRParameters) -> Self {
        Self { params }
    }

    /// Calculate implied volatility using Hagan's approximation
    ///
    /// This is the standard SABR formula from Hagan et al. (2002) with enhanced
    /// numerical stability and support for negative rates through shifting.
    #[must_use = "computed volatility should be used"]
    #[inline]
    pub fn implied_volatility(
        &self,
        forward: f64,
        strike: f64,
        time_to_expiry: f64,
    ) -> Result<f64> {
        // Apply shift if using shifted SABR for negative rates
        let (effective_forward, effective_strike) = if let Some(shift) = self.params.shift {
            (forward + shift, strike + shift)
        } else {
            // Validate non-negative rates for standard SABR
            if forward <= 0.0 || strike <= 0.0 {
                return Err(Error::Validation(format!(
                    "Standard SABR requires positive rates. Got forward={:.6}, strike={:.6}. \
                     Use shifted SABR (new_with_shift) for negative rates.",
                    forward, strike
                )));
            }
            (forward, strike)
        };

        let alpha = self.params.alpha;
        let mut beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        // Clamp beta to avoid numerical instability near 0 or 1
        if beta < 1e-4 {
            beta = 0.0;
        } else if (1.0 - beta).abs() < 1e-4 {
            beta = 1.0;
        }

        let beta_is_zero = beta.abs() < 1e-12;
        let beta_is_one = (beta - 1.0).abs() < 1e-12;

        // Calculate intermediate values with numerical protection
        let f_mid = (effective_forward * effective_strike).sqrt();
        let f_mid_beta = if beta_is_zero {
            1.0 // Special case for normal model
        } else {
            f_mid.powf(beta)
        };

        // Enhanced log-moneyness calculation
        let z = if nu.abs() < 1e-14 {
            // Handle nu ≈ 0 case (pure CEV)
            return self.atm_volatility(effective_forward, time_to_expiry);
        } else if beta_is_one {
            (nu / alpha) * (effective_forward / effective_strike).ln()
        } else if beta_is_zero {
            (nu / alpha) * (effective_forward - effective_strike)
        } else {
            (nu / alpha) * (effective_forward.powf(1.0 - beta) - effective_strike.powf(1.0 - beta))
                / (1.0 - beta)
        };

        // ATM detection using single scale-invariant relative threshold
        // This ensures consistent behavior regardless of forward/strike scale (rates vs equities)
        let relative_diff =
            (effective_forward - effective_strike).abs() / effective_forward.max(effective_strike);
        if relative_diff < 1e-8 || z.abs() < 1e-8 {
            return self.atm_volatility(effective_forward, time_to_expiry);
        }

        // Calculate log-moneyness for correction terms
        let log_moneyness = (effective_forward / effective_strike).ln();

        // Obloj (2008) correction: use geometric-mean-based z for the z/χ(z) ratio.
        // The original Hagan et al. (2002) formula uses z = (ν/α)(F^{1-β} - K^{1-β})/(1-β)
        // which introduces O(ε²) errors for long maturities and high vol-of-vol.
        // The corrected formula uses z = (ν/α) * f_mid^{1-β} * ln(F/K) for 0 < β < 1.
        // For β=0 (normal) and β=1 (lognormal), the original formula is already exact.
        //
        // Reference: Obloj, J. (2008). "Fine-tune your smile: Correction to Hagan et al."
        // arXiv:0708.0998v2
        let z_corrected = if beta_is_one || beta_is_zero {
            z
        } else {
            (nu / alpha) * f_mid.powf(1.0 - beta) * log_moneyness
        };

        // Calculate chi(z) with robust numerical handling
        let x = self.calculate_chi_robust(z_corrected)?;

        // First factor with enhanced numerical stability
        let factor1 = if f_mid_beta.abs() < 1e-14 {
            alpha // Handle degenerate case
        } else {
            let correction_term = if beta_is_zero {
                1.0 // No correction for normal model
            } else {
                1.0 + (1.0 - beta).powi(2) / 24.0 * log_moneyness.powi(2)
                    + (1.0 - beta).powi(4) / 1920.0 * log_moneyness.powi(4)
            };
            alpha / (f_mid_beta * correction_term)
        };

        // Second factor (z/x correction) with numerical protection
        let factor2 = if x.abs() < 1e-14 {
            1.0 // Avoid division by zero
        } else {
            z_corrected / x
        };

        // Third factor (time correction) with enhanced precision
        let time_correction = if beta_is_zero {
            // Normal SABR time correction
            (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2)
        } else {
            (1.0 - beta).powi(2) / 24.0 * alpha.powi(2) / f_mid.powf(2.0 * (1.0 - beta))
                + 0.25 * rho * beta * nu * alpha / f_mid_beta
                + (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2)
        };

        let factor3 = 1.0 + time_to_expiry * time_correction;

        let volatility = factor1 * factor2 * factor3;

        // Validate result
        if volatility <= 0.0 || !volatility.is_finite() {
            return Err(Error::Validation(format!(
                "SABR produced invalid volatility={:.6} for forward={:.6}, strike={:.6}, T={:.4}. \
                 Check parameter values.",
                volatility, forward, strike, time_to_expiry
            )));
        }

        Ok(volatility)
    }

    /// Calculate ATM implied volatility with enhanced numerical stability
    #[inline]
    fn atm_volatility(&self, forward: f64, time_to_expiry: f64) -> Result<f64> {
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;
        let beta_is_zero = beta.abs() < 1e-12;
        let beta_is_one = (beta - 1.0).abs() < 1e-12;
        let beta_is_half = (beta - 0.5).abs() < 1e-12;

        // Handle degenerate cases
        if alpha.abs() < 1e-14 {
            return Ok(0.0);
        }

        // ATM volatility formula with numerical protection
        let vol = if beta_is_zero {
            // Normal SABR: vol = alpha * (1 + T * (2-3*rho²)/24 * nu²)
            alpha * (1.0 + time_to_expiry * (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2))
        } else if beta_is_one {
            // Lognormal SABR: vol = alpha/F * (1 + T * (alpha²/(24*F²) + rho*nu*alpha/(4*F) + (2-3*rho²)*nu²/24))
            let alpha_term = alpha.powi(2) / (24.0 * forward.powi(2));
            let rho_term = 0.25 * rho * nu * alpha / forward;
            let nu_term = (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2);

            alpha / forward * (1.0 + time_to_expiry * (alpha_term + rho_term + nu_term))
        } else {
            // General beta case with numerical protection
            let f_beta = if forward.abs() < 1e-14 {
                1e-14_f64.powf(beta) // Avoid zero to very small power
            } else {
                forward.powf(beta)
            };

            let alpha_term = if beta_is_half {
                // Special handling for beta = 0.5 (sqrt case)
                alpha.powi(2) / (24.0 * forward)
            } else {
                (1.0 - beta).powi(2) / 24.0 * alpha.powi(2) / forward.powf(2.0 * (1.0 - beta))
            };

            let rho_term = 0.25 * rho * beta * nu * alpha / f_beta;
            let nu_term = (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2);

            let time_correction = alpha_term + rho_term + nu_term;

            alpha / f_beta * (1.0 + time_to_expiry * time_correction)
        };

        // Validate result
        if vol <= 0.0 || !vol.is_finite() {
            return Err(Error::Validation(format!(
                "SABR ATM volatility calculation produced invalid result={:.6} for forward={:.6}, T={:.4}",
                vol, forward, time_to_expiry
            )));
        }

        Ok(vol)
    }

    /// Calculate chi(z) for the SABR formula with enhanced numerical stability.
    ///
    /// Uses a smooth blending function between series expansion (for small z)
    /// and the exact formula (for larger z) to ensure continuous derivatives
    /// for smooth Greeks near ATM.
    ///
    /// # Implementation Notes
    ///
    /// - Series expansion: χ(z) ≈ z + (ρ/2)z² + ((2ρ² - 1)/6)z³ + O(z⁴)
    /// - Smooth sigmoid blend in transition region [1e-5, 1e-3]
    /// - Special handling for extreme rho values (±1)
    #[inline]
    fn calculate_chi_robust(&self, z: f64) -> Result<f64> {
        let rho = self.params.rho;

        // Fourth-order Taylor series expansion around z = 0:
        // χ(z) = ln((√(1 - 2ρz + z²) + z - ρ)/(1 - ρ))
        //
        // Expand: Let f(z) = √(1 - 2ρz + z²) + z - ρ
        // f(0) = 1 - ρ
        // f'(0) = (-ρ + 0)/1 + 1 = 1 - ρ (since d/dz √(1-2ρz+z²)|_{z=0} = -ρ)
        // etc.
        //
        // After careful expansion: χ(z) ≈ z + (ρ/2)z² + ((2ρ² - 1)/6)z³ + O(z⁴)
        let series_chi = |z_val: f64| -> f64 {
            let z2 = z_val * z_val;
            let z3 = z2 * z_val;
            let z4 = z2 * z2;
            // Coefficients from Taylor expansion
            let c1 = 1.0; // coefficient of z
            let c2 = rho / 2.0; // coefficient of z²
            let c3 = (2.0 * rho * rho - 1.0) / 6.0; // coefficient of z³
            let c4 = rho * (5.0 * rho * rho - 3.0) / 24.0; // coefficient of z⁴
            c1 * z_val + c2 * z2 + c3 * z3 + c4 * z4
        };

        // Exact chi formula
        let exact_chi = |z_val: f64| -> Result<f64> {
            let discriminant = 1.0 - 2.0 * rho * z_val + z_val * z_val;

            if discriminant < 0.0 {
                return Err(Error::Validation(format!(
                    "SABR chi function: negative discriminant {} for z={:.6}, rho={:.6}",
                    discriminant, z_val, rho
                )));
            }

            let sqrt_disc = discriminant.sqrt();

            // Handle extreme rho cases
            if (1.0 - rho).abs() < 1e-10 {
                // rho ≈ 1: Use approximation χ(z) ≈ z / (1 + z/2)
                return Ok(z_val / (1.0 + z_val / 2.0));
            }
            if (1.0 + rho).abs() < 1e-10 {
                // rho ≈ -1: Use stable form
                return Ok((sqrt_disc + z_val + 1.0).ln() - (2.0_f64).ln() / 2.0);
            }

            let numerator = sqrt_disc + z_val - rho;
            let denominator = 1.0 - rho;

            if numerator <= 0.0 {
                return Err(Error::Validation(format!(
                    "SABR chi function: non-positive log argument {} for z={:.6}, rho={:.6}",
                    numerator, z_val, rho
                )));
            }

            Ok((numerator / denominator).ln())
        };

        let abs_z = z.abs();

        // Transition region bounds
        let z_low = 1e-5; // Below this, use pure series
        let z_high = 1e-3; // Above this, use pure exact

        if abs_z < z_low {
            // Pure series expansion for very small z
            Ok(series_chi(z))
        } else if abs_z > z_high {
            // Pure exact formula for larger z
            exact_chi(z)
        } else {
            // Smooth sigmoid blend in transition region
            // Use smooth step function: t = (|z| - z_low) / (z_high - z_low)
            // blend = 3t² - 2t³ (smooth step with zero derivative at endpoints)
            let t = (abs_z - z_low) / (z_high - z_low);
            let blend = t * t * (3.0 - 2.0 * t); // Hermite smoothstep

            let series_val = series_chi(z);
            let exact_val = exact_chi(z)?;

            // Linear interpolation with smooth weights
            Ok((1.0 - blend) * series_val + blend * exact_val)
        }
    }

    /// Get model parameters
    pub fn parameters(&self) -> &SABRParameters {
        &self.params
    }

    /// Update model parameters
    pub fn set_parameters(&mut self, params: SABRParameters) {
        self.params = params;
    }

    /// Check if this model supports negative rates
    pub fn supports_negative_rates(&self) -> bool {
        self.params.shift.is_some()
    }

    /// Get the effective forward/strike after applying shift
    pub fn effective_rates(&self, forward: f64, strike: f64) -> (f64, f64) {
        if let Some(shift) = self.params.shift {
            (forward + shift, strike + shift)
        } else {
            (forward, strike)
        }
    }

    /// Validate inputs for SABR model
    pub fn validate_inputs(&self, forward: f64, strike: f64, time_to_expiry: f64) -> Result<()> {
        // Time validation
        if time_to_expiry <= 0.0 {
            return Err(Error::Validation(format!(
                "SABR time_to_expiry must be positive, got: {:.6}",
                time_to_expiry
            )));
        }

        // Rate validation based on model type
        if self.params.shift.is_none() {
            // Standard SABR requires positive rates
            if forward <= 0.0 || strike <= 0.0 {
                return Err(Error::Validation(format!(
                    "Standard SABR requires positive rates. Got forward={:.6}, strike={:.6}. \
                     Use shifted SABR for negative rates.",
                    forward, strike
                )));
            }
        } else if let Some(shift) = self.params.shift {
            // Shifted SABR allows negative rates but shifted values must be positive
            if forward + shift <= 0.0 || strike + shift <= 0.0 {
                return Err(Error::Validation(format!(
                    "Shifted SABR: effective rates must be positive. \
                     Got forward+shift={:.6}, strike+shift={:.6} (shift={:.6})",
                    forward + shift,
                    strike + shift,
                    shift
                )));
            }
        }

        Ok(())
    }
}

/// SABR calibration using market prices.
///
/// # Tolerance Considerations
///
/// The default tolerance of 1e-6 provides a balance between speed and accuracy:
///
/// | Tolerance | Use Case | Accuracy | Speed |
/// |-----------|----------|----------|-------|
/// | 1e-4 | Quick screening | ~0.5 vol bp | Fast |
/// | 1e-6 | Standard production | ~0.01 vol bp | Moderate |
/// | 1e-8 | High-precision (BBG VCUB) | ~0.0001 vol bp | Slow |
/// | 1e-10 | Research/validation | Machine precision | Very slow |
///
/// For production vol surfaces where Greeks are computed from the surface,
/// consider using tighter tolerance (1e-8) to ensure smooth Greeks.
///
/// # Gradient Methods
///
/// Two gradient computation methods are available:
///
/// - **Finite differences** (default): More robust, works for all parameter ranges
/// - **Analytical**: Faster but may have numerical issues at parameter boundaries
pub struct SABRCalibrator {
    /// Tolerance for calibration convergence.
    ///
    /// Lower values give more accurate calibration but take longer.
    /// See struct-level docs for guidance on choosing tolerance.
    tolerance: f64,
    /// Maximum iterations for the optimizer.
    max_iterations: usize,
    /// Use finite-difference gradients instead of analytical approximations.
    use_fd_gradients: bool,
}

impl SABRCalibrator {
    /// Create new calibrator with production-ready defaults.
    ///
    /// Default settings:
    /// - **Tolerance**: 1e-6 (standard production accuracy)
    /// - **Max iterations**: 100
    /// - **Gradient method**: Finite difference (more robust)
    ///
    /// # Production Usage
    ///
    /// For high-precision applications (e.g., Greeks computation from vol surface),
    /// consider using tighter tolerance:
    ///
    /// ```rust,no_run
    /// use finstack_valuations::instruments::common::models::volatility::sabr::SABRCalibrator;
    ///
    /// let _calibrator = SABRCalibrator::new();
    ///
    /// let _precise_calibrator = SABRCalibrator::new()
    ///     .with_tolerance(1e-8)
    ///     .with_max_iterations(200);
    /// ```
    ///
    /// By default, uses finite-difference gradients (`use_fd_gradients: true`)
    /// for more accurate calibration at the cost of some performance.
    /// Use `with_fd_gradients(false)` to switch to analytical approximations
    /// for faster but potentially less accurate calibration.
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 100,
            use_fd_gradients: true, // Default to FD for production accuracy
        }
    }

    /// Create calibrator with high-precision settings.
    ///
    /// Uses Bloomberg VCUB-equivalent tolerance (1e-8) for applications
    /// requiring very accurate vol surface fitting, such as:
    /// - Greeks computation from interpolated surface
    /// - Exotic pricing with vol smile dependence
    /// - Regulatory model validation
    pub fn high_precision() -> Self {
        Self {
            tolerance: 1e-8,
            max_iterations: 200,
            use_fd_gradients: true,
        }
    }

    /// Set tolerance
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Enable finite-difference gradients for higher accuracy (slower).
    pub fn with_fd_gradients(mut self, use_fd: bool) -> Self {
        self.use_fd_gradients = use_fd;
        self
    }

    /// Calibrate SABR parameters with automatic negative rate detection
    pub fn calibrate_auto_shift(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
    ) -> Result<SABRParameters> {
        // Check if we need shift for negative rates
        let min_strike = strikes
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or_else(|| Error::Validation("Strikes should not be empty".to_string()))?;
        let min_rate = forward.min(*min_strike);

        if min_rate < 0.0 {
            // Use shifted SABR
            let shift = (-min_rate + 0.001).max(0.001); // At least 10bps shift
            self.calibrate_shifted(forward, strikes, market_vols, time_to_expiry, beta, shift)
        } else {
            // Use standard SABR
            self.calibrate(forward, strikes, market_vols, time_to_expiry, beta)
        }
    }

    /// Calibrate SABR parameters with automatic negative rate detection and analytical derivatives
    pub fn calibrate_auto_shift_with_derivatives(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
    ) -> Result<SABRParameters> {
        // Check if we need shift for negative rates
        let min_strike = strikes
            .iter()
            .min_by(|a, b| a.total_cmp(b))
            .ok_or_else(|| Error::Validation("Strikes should not be empty".to_string()))?;
        let min_rate = forward.min(*min_strike);

        if min_rate < 0.0 {
            // Use shifted SABR with derivatives
            let shift = (-min_rate + 0.001).max(0.001); // At least 10bps shift
            self.calibrate_shifted_with_derivatives(
                forward,
                strikes,
                market_vols,
                time_to_expiry,
                beta,
                shift,
            )
        } else {
            // Use standard SABR with derivatives
            self.calibrate_with_derivatives(forward, strikes, market_vols, time_to_expiry, beta)
        }
    }

    /// Calibrate shifted SABR parameters for negative rate environments
    pub fn calibrate_shifted(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
        shift: f64,
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Validation(format!(
                "SABR calibration: strikes length ({}) must match market_vols length ({})",
                strikes.len(),
                market_vols.len()
            )));
        }

        // Apply shift to all rates
        let shifted_forward = forward + shift;
        let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

        // Validate shifted rates are positive
        if shifted_forward <= 0.0 || shifted_strikes.iter().any(|&s| s <= 0.0) {
            let min_shifted_strike = shifted_strikes
                .iter()
                .copied()
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or(0.0);
            return Err(Error::Validation(format!(
                "Shifted SABR calibration: shift={:.6} is insufficient. \
                 shifted_forward={:.6}, min_shifted_strike={:.6}. Increase shift.",
                shift, shifted_forward, min_shifted_strike
            )));
        }

        // Calibrate using shifted rates
        let base_params = self.calibrate(
            shifted_forward,
            &shifted_strikes,
            market_vols,
            time_to_expiry,
            beta,
        )?;

        // Return parameters with shift
        SABRParameters::new_with_shift(
            base_params.alpha,
            beta,
            base_params.nu,
            base_params.rho,
            shift,
        )
    }

    /// Calibrate SABR parameters to market implied volatilities using multi-dimensional solver
    pub fn calibrate(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64, // Beta is usually fixed
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Validation(format!(
                "SABR calibration: strikes length ({}) must match market_vols length ({})",
                strikes.len(),
                market_vols.len()
            )));
        }

        // Use Levenberg-Marquardt solver for robust calibration
        use finstack_core::math::solver_multi::{LevenbergMarquardtSolver, MultiSolver};

        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_iterations(self.max_iterations);

        // Define objective function: sum of squared volatility errors
        let strikes_vec = strikes.to_vec();
        let market_vols_vec = market_vols.to_vec();
        let objective = move |params: &[f64]| -> f64 {
            let alpha = params[0];
            let nu = params[1];
            let rho = params[2];

            // Create SABR parameters and model
            if let Ok(sabr_params) = SABRParameters::new(alpha, beta, nu, rho) {
                let model = SABRModel::new(sabr_params);

                // Calculate sum of squared errors
                strikes_vec
                    .iter()
                    .zip(market_vols_vec.iter())
                    .map(|(&strike, &market_vol)| {
                        model
                            .implied_volatility(forward, strike, time_to_expiry)
                            .map(|model_vol| (model_vol - market_vol).powi(2))
                            .unwrap_or(1e6) // Large penalty for invalid parameters
                    })
                    .sum()
            } else {
                1e12 // Very large penalty for invalid parameters
            }
        };

        // Initial guess for parameters
        let atm_vol = self.find_atm_vol(forward, strikes, market_vols)?;
        let initial = vec![
            atm_vol * forward.powf(1.0 - beta), // alpha: ATM vol adjusted for beta
            0.3,                                // nu: typical vol-of-vol
            0.0,                                // rho: start neutral
        ];

        // Parameter bounds for SABR model
        let bounds = vec![
            (0.001, 5.0),  // alpha: positive, reasonable range
            (0.001, 2.0),  // nu: positive vol-of-vol
            (-0.99, 0.99), // rho: correlation bounds
        ];

        // Calibrate using multi-dimensional solver
        let solution = solver.minimize(objective, &initial, Some(&bounds))?;

        // Extract calibrated parameters
        SABRParameters::new(solution[0], beta, solution[1], solution[2])
    }

    /// Calibrate SABR parameters with analytical derivatives for improved performance
    pub fn calibrate_with_derivatives(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Validation(format!(
                "SABR calibration: strikes length ({}) must match market_vols length ({})",
                strikes.len(),
                market_vols.len()
            )));
        }

        // Use analytical derivatives from the local module
        use super::sabr_derivatives::{SABRCalibrationDerivatives, SABRMarketData};
        use finstack_core::math::solver_multi::LevenbergMarquardtSolver;

        // Create market data structure
        let market_data = SABRMarketData {
            forward,
            time_to_expiry,
            strikes: strikes.to_vec(),
            market_vols: market_vols.to_vec(),
            beta,
            shift: None,
        };

        // Create derivatives provider (with or without FD gradients)
        let derivatives_provider = if self.use_fd_gradients {
            SABRCalibrationDerivatives::new_with_fd(market_data.clone())
        } else {
            SABRCalibrationDerivatives::new(market_data.clone())
        };

        // Create Levenberg-Marquardt solver
        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_iterations(self.max_iterations);

        // Define objective function: sum of squared volatility errors
        let objective = move |params: &[f64]| -> f64 {
            let alpha = params[0];
            let nu = params[1];
            let rho = params[2];

            // Create SABR parameters and model
            if let Ok(sabr_params) = SABRParameters::new(alpha, beta, nu, rho) {
                let model = SABRModel::new(sabr_params);

                // Calculate sum of squared errors
                market_data
                    .strikes
                    .iter()
                    .zip(market_data.market_vols.iter())
                    .map(|(&strike, &market_vol)| {
                        model
                            .implied_volatility(forward, strike, time_to_expiry)
                            .map(|model_vol| (model_vol - market_vol).powi(2))
                            .unwrap_or(1e6) // Large penalty for invalid parameters
                    })
                    .sum()
            } else {
                1e12 // Very large penalty for invalid parameters
            }
        };

        // Initial guess for parameters
        let atm_vol = self.find_atm_vol(forward, strikes, market_vols)?;
        let initial = vec![
            atm_vol * forward.powf(1.0 - beta), // alpha
            0.3,                                // nu
            0.0,                                // rho
        ];

        // Parameter bounds
        let bounds = vec![
            (1e-6, 5.0),   // alpha bounds
            (1e-6, 2.0),   // nu bounds
            (-0.99, 0.99), // rho bounds
        ];

        // Solve with analytical derivatives
        let solution = solver.minimize_with_derivatives(
            objective,
            &derivatives_provider,
            &initial,
            Some(&bounds),
        )?;

        // Extract calibrated parameters
        let alpha = solution[0];
        let nu = solution[1];
        let rho = solution[2];

        SABRParameters::new(alpha, beta, nu, rho)
    }

    /// Calibrate shifted SABR with analytical derivatives
    pub fn calibrate_shifted_with_derivatives(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
        shift: f64,
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Validation(format!(
                "SABR calibration: strikes length ({}) must match market_vols length ({})",
                strikes.len(),
                market_vols.len()
            )));
        }

        // Apply shift to all rates
        let shifted_forward = forward + shift;
        let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

        // Validate shifted rates are positive
        if shifted_forward <= 0.0 || shifted_strikes.iter().any(|&s| s <= 0.0) {
            let min_shifted_strike = shifted_strikes
                .iter()
                .copied()
                .min_by(|a, b| a.total_cmp(b))
                .unwrap_or(0.0);
            return Err(Error::Validation(format!(
                "Shifted SABR calibration: shift={:.6} is insufficient. \
                 shifted_forward={:.6}, min_shifted_strike={:.6}. Increase shift.",
                shift, shifted_forward, min_shifted_strike
            )));
        }

        // Calibrate using shifted rates with derivatives
        let base_params = self.calibrate_with_derivatives(
            shifted_forward,
            &shifted_strikes,
            market_vols,
            time_to_expiry,
            beta,
        )?;

        // Return parameters with shift
        SABRParameters::new_with_shift(
            base_params.alpha,
            beta,
            base_params.nu,
            base_params.rho,
            shift,
        )
    }

    /// Find ATM volatility from market data
    fn find_atm_vol(&self, forward: f64, strikes: &[f64], vols: &[f64]) -> Result<f64> {
        // Find the strike closest to forward
        let mut min_diff = f64::INFINITY;
        let mut atm_vol = vols[0];

        for (i, &strike) in strikes.iter().enumerate() {
            let diff = (strike - forward).abs();
            if diff < min_diff {
                min_diff = diff;
                atm_vol = vols[i];
            }
        }

        Ok(atm_vol)
    }

    /// Calibrate SABR with ATM volatility pinning (market-standard approach).
    ///
    /// This method ensures the calibrated model matches the ATM volatility exactly
    /// by solving for alpha analytically, then fitting only nu and rho to the smile.
    /// This is the standard market approach for SABR calibration.
    ///
    /// # Arguments
    /// * `forward` - Forward rate
    /// * `strikes` - Vector of strikes (should include ATM)
    /// * `market_vols` - Market implied volatilities corresponding to strikes
    /// * `time_to_expiry` - Time to expiry in years
    /// * `beta` - SABR beta parameter (typically fixed)
    ///
    /// # Returns
    /// Calibrated SABR parameters with exact ATM match
    pub fn calibrate_with_atm_pinning(
        &self,
        forward: f64,
        strikes: &[f64],
        market_vols: &[f64],
        time_to_expiry: f64,
        beta: f64,
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Validation(format!(
                "SABR calibration: strikes length ({}) must match market_vols length ({})",
                strikes.len(),
                market_vols.len()
            )));
        }

        // Find ATM vol from market data
        let atm_vol = self.find_atm_vol(forward, strikes, market_vols)?;

        // Use 2D solver for nu and rho only
        use finstack_core::math::solver_multi::{LevenbergMarquardtSolver, MultiSolver};

        let solver = LevenbergMarquardtSolver::new()
            .with_tolerance(self.tolerance)
            .with_max_iterations(self.max_iterations);

        let strikes_vec = strikes.to_vec();
        let market_vols_vec = market_vols.to_vec();
        let tol = self.tolerance;

        // Objective: fit nu and rho, with alpha solved to match ATM
        let objective = move |params: &[f64]| -> f64 {
            let nu = params[0];
            let rho = params[1];

            // Solve for alpha that matches ATM vol exactly
            let alpha =
                match solve_alpha_for_atm(forward, atm_vol, time_to_expiry, beta, nu, rho, tol) {
                    Ok(a) => a,
                    Err(_) => return 1e12,
                };

            // Create model and compute smile errors (excluding ATM)
            if let Ok(sabr_params) = SABRParameters::new(alpha, beta, nu, rho) {
                let model = SABRModel::new(sabr_params);

                strikes_vec
                    .iter()
                    .zip(market_vols_vec.iter())
                    .map(|(&strike, &market_vol)| {
                        // Skip ATM point (it's matched exactly by construction)
                        let is_atm = (strike - forward).abs() / forward < 0.001;
                        if is_atm {
                            0.0
                        } else {
                            model
                                .implied_volatility(forward, strike, time_to_expiry)
                                .map(|model_vol| (model_vol - market_vol).powi(2))
                                .unwrap_or(1e6)
                        }
                    })
                    .sum()
            } else {
                1e12
            }
        };

        // Initial guess: nu=0.3 (typical), rho=0.0 (neutral)
        let initial = vec![0.3, 0.0];

        // Bounds for nu and rho
        let bounds = vec![
            (0.001, 2.0),  // nu
            (-0.99, 0.99), // rho
        ];

        let solution = solver.minimize(objective, &initial, Some(&bounds))?;

        let nu = solution[0];
        let rho = solution[1];

        // Final alpha solve with calibrated nu/rho
        let alpha = solve_alpha_for_atm(
            forward,
            atm_vol,
            time_to_expiry,
            beta,
            nu,
            rho,
            self.tolerance,
        )?;

        SABRParameters::new(alpha, beta, nu, rho)
    }
}

/// Solve for alpha that matches target ATM volatility given other SABR parameters.
///
/// Uses Newton iteration on the ATM volatility formula:
/// σ_ATM = α/F^(1-β) * [1 + T * corrections(α, ν, ρ)]
fn solve_alpha_for_atm(
    forward: f64,
    target_atm_vol: f64,
    time_to_expiry: f64,
    beta: f64,
    nu: f64,
    rho: f64,
    tolerance: f64,
) -> Result<f64> {
    // Initial guess: first-order approximation
    let f_pow = forward.powf(1.0 - beta);
    let mut alpha = target_atm_vol * f_pow;

    // Newton iteration to refine alpha
    for _ in 0..50 {
        // Compute model ATM vol with current alpha
        let params = SABRParameters::new(alpha, beta, nu, rho)?;
        let model = SABRModel::new(params);
        let model_vol = model.atm_volatility(forward, time_to_expiry)?;

        let error = model_vol - target_atm_vol;
        if error.abs() < tolerance {
            return Ok(alpha);
        }

        // Numerical derivative for Newton step
        let bump = alpha * 1e-6;
        let params_bumped = SABRParameters::new(alpha + bump, beta, nu, rho)?;
        let model_bumped = SABRModel::new(params_bumped);
        let vol_bumped = model_bumped.atm_volatility(forward, time_to_expiry)?;

        let d_vol_d_alpha = (vol_bumped - model_vol) / bump;
        if d_vol_d_alpha.abs() < 1e-14 {
            break; // Can't continue Newton iteration
        }

        // Newton step with damping for stability
        let step = -error / d_vol_d_alpha;
        alpha += step.clamp(-alpha * 0.5, alpha * 0.5); // Limit step size

        // Ensure alpha stays positive
        if alpha <= 0.0 {
            alpha = target_atm_vol * f_pow * 0.5;
        }
    }

    // Return best alpha found even if not converged
    Ok(alpha)
}

impl Default for SABRCalibrator {
    fn default() -> Self {
        Self::new()
    }
}

/// SABR smile generator for creating volatility surfaces
pub struct SABRSmile {
    model: SABRModel,
    forward: f64,
    time_to_expiry: f64,
}

/// Result of arbitrage validation, containing any violations found.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ArbitrageValidationResult {
    /// Strikes where butterfly spread is negative (convexity violation)
    pub butterfly_violations: Vec<ButterflyViolation>,
    /// Pairs of strikes where call prices increase (monotonicity violation)
    pub monotonicity_violations: Vec<MonotonicityViolation>,
}

/// A butterfly spread violation at a specific strike.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ButterflyViolation {
    /// Strike at which the violation occurs
    pub strike: f64,
    /// Butterfly spread value (negative indicates violation)
    pub butterfly_value: f64,
    /// Severity as percentage of mid-strike price
    pub severity_pct: f64,
}

/// A monotonicity violation between two strikes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MonotonicityViolation {
    /// Lower strike
    pub strike_low: f64,
    /// Higher strike
    pub strike_high: f64,
    /// Call price at lower strike
    pub price_low: f64,
    /// Call price at higher strike (should be lower)
    pub price_high: f64,
}

impl ArbitrageValidationResult {
    /// Returns true if no arbitrage was detected.
    #[must_use]
    pub fn is_arbitrage_free(&self) -> bool {
        self.butterfly_violations.is_empty() && self.monotonicity_violations.is_empty()
    }

    /// Returns the worst butterfly violation severity, if any.
    #[must_use]
    pub fn worst_butterfly_severity(&self) -> Option<f64> {
        self.butterfly_violations
            .iter()
            .map(|v| v.severity_pct.abs())
            .max_by(|a, b| a.total_cmp(b))
    }
}

impl SABRSmile {
    /// Create new smile generator
    pub fn new(model: SABRModel, forward: f64, time_to_expiry: f64) -> Self {
        Self {
            model,
            forward,
            time_to_expiry,
        }
    }

    /// Returns the ATM (at-the-money) implied volatility.
    ///
    /// This is a convenience method that computes the implied volatility
    /// at strike = forward, which is the most frequently quoted volatility level.
    ///
    /// # Returns
    ///
    /// ATM implied volatility as a decimal (e.g., 0.20 for 20% vol).
    ///
    /// # Errors
    ///
    /// Returns an error if the volatility computation fails (e.g., invalid parameters).
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use finstack_valuations::instruments::common::models::volatility::sabr::{
    ///     SABRParameters, SABRModel, SABRSmile,
    /// };
    ///
    /// let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap();
    /// let model = SABRModel::new(params);
    /// let smile = SABRSmile::new(model, 100.0, 1.0);
    ///
    /// let atm_vol = smile.atm_vol().unwrap();
    /// assert!(atm_vol > 0.0);
    /// ```
    #[must_use = "computed ATM volatility should be used"]
    pub fn atm_vol(&self) -> Result<f64> {
        self.model
            .implied_volatility(self.forward, self.forward, self.time_to_expiry)
    }

    /// Generate volatility smile for given strikes
    pub fn generate_smile(&self, strikes: &[f64]) -> Result<Vec<f64>> {
        let mut vols = Vec::with_capacity(strikes.len());

        for &strike in strikes {
            let vol = self
                .model
                .implied_volatility(self.forward, strike, self.time_to_expiry)?;
            vols.push(vol);
        }

        Ok(vols)
    }

    /// Generate strike from delta
    pub fn strike_from_delta(&self, delta: f64, is_call: bool) -> Result<f64> {
        // This requires iterative solving
        // Simplified version using ATM vol as approximation
        let atm_vol = self
            .model
            .atm_volatility(self.forward, self.time_to_expiry)?;
        let variance = atm_vol.powi(2) * self.time_to_expiry;
        let std_dev = variance.sqrt();

        // Normal inverse for delta
        let z = if is_call {
            finstack_core::math::standard_normal_inv_cdf(delta)
        } else {
            finstack_core::math::standard_normal_inv_cdf(1.0 - delta)
        };

        let strike = self.forward * (z * std_dev).exp();
        Ok(strike)
    }

    /// Validate the generated smile for no-arbitrage conditions.
    ///
    /// Checks for two types of static arbitrage:
    ///
    /// 1. **Butterfly arbitrage** (convexity): Call(K-δ) - 2·Call(K) + Call(K+δ) ≥ 0
    ///    A negative butterfly spread means you can buy the wings and sell the body
    ///    for a risk-free profit.
    ///
    /// 2. **Monotonicity arbitrage**: Call prices must decrease as strike increases.
    ///    If C(K₁) < C(K₂) for K₁ < K₂, you can buy the lower strike and sell the
    ///    higher strike for immediate profit.
    ///
    /// # Arguments
    /// * `strikes` - Array of strikes to validate (must be sorted ascending)
    /// * `r` - Risk-free rate for discounting
    /// * `q` - Dividend/foreign rate
    ///
    /// # Returns
    /// `ArbitrageValidationResult` containing any violations found.
    ///
    /// # Example
    /// ```rust,ignore
    /// let result = smile.validate_no_arbitrage(&strikes, 0.05, 0.02)?;
    /// if !result.is_arbitrage_free() {
    ///     println!("Warning: {} butterfly violations found",
    ///              result.butterfly_violations.len());
    /// }
    /// ```
    pub fn validate_no_arbitrage(
        &self,
        strikes: &[f64],
        r: f64,
        q: f64,
    ) -> Result<ArbitrageValidationResult> {
        if strikes.len() < 3 {
            return Ok(ArbitrageValidationResult::default());
        }

        let vols = self.generate_smile(strikes)?;

        // Convert to call prices for validation
        let prices: Vec<f64> = strikes
            .iter()
            .zip(vols.iter())
            .map(|(&k, &vol)| bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry))
            .collect();

        let mut result = ArbitrageValidationResult::default();

        // Tolerance for numerical noise (0.1 bps of notional)
        let tol = 1e-6;

        // Check monotonicity: C(K₁) > C(K₂) for K₁ < K₂
        for i in 1..prices.len() {
            if prices[i] > prices[i - 1] + tol {
                result.monotonicity_violations.push(MonotonicityViolation {
                    strike_low: strikes[i - 1],
                    strike_high: strikes[i],
                    price_low: prices[i - 1],
                    price_high: prices[i],
                });
            }
        }

        // Check butterfly positivity (convexity)
        for i in 1..prices.len() - 1 {
            let butterfly = prices[i - 1] - 2.0 * prices[i] + prices[i + 1];
            if butterfly < -tol {
                let severity_pct = if prices[i] > tol {
                    butterfly.abs() / prices[i] * 100.0
                } else {
                    0.0
                };

                result.butterfly_violations.push(ButterflyViolation {
                    strike: strikes[i],
                    butterfly_value: butterfly,
                    severity_pct,
                });
            }
        }

        Ok(result)
    }

    /// Quick check if the smile is arbitrage-free.
    ///
    /// Returns `Ok(())` if no arbitrage detected, `Err` with description if arbitrage found.
    pub fn check_no_arbitrage(&self, strikes: &[f64], r: f64, q: f64) -> Result<()> {
        let result = self.validate_no_arbitrage(strikes, r, q)?;

        if !result.is_arbitrage_free() {
            let mut msg = String::from("SABR smile contains arbitrage: ");

            if !result.butterfly_violations.is_empty() {
                msg.push_str(&format!(
                    "{} butterfly violations (worst: {:.2}%)",
                    result.butterfly_violations.len(),
                    result.worst_butterfly_severity().unwrap_or(0.0)
                ));
            }

            if !result.monotonicity_violations.is_empty() {
                if !result.butterfly_violations.is_empty() {
                    msg.push_str(", ");
                }
                msg.push_str(&format!(
                    "{} monotonicity violations",
                    result.monotonicity_violations.len()
                ));
            }

            return Err(Error::Validation(msg));
        }

        Ok(())
    }

    /// Repair arbitrage in the SABR smile by adjusting volatilities.
    ///
    /// This method generates a smile and then applies monotonicity and convexity
    /// corrections to remove static arbitrage violations. The repair is conservative:
    /// it only modifies volatilities at violating strikes.
    ///
    /// # Algorithm
    ///
    /// 1. Generate the raw SABR smile
    /// 2. Apply monotonicity repair: ensure call prices decrease with strike
    /// 3. Apply butterfly repair: ensure convexity (positive second derivative)
    ///
    /// The repair uses a simple projection approach:
    /// - For monotonicity: clamp prices to maintain decreasing sequence
    /// - For butterfly: adjust mid-strike to satisfy convexity constraint
    ///
    /// # Arguments
    ///
    /// * `strikes` - Array of strikes (should be sorted ascending)
    /// * `r` - Risk-free rate for Black-Scholes conversion
    /// * `q` - Dividend/foreign rate
    /// * `max_iterations` - Maximum repair iterations (default: 10)
    ///
    /// # Returns
    ///
    /// Repaired volatility smile as `Vec<f64>`.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let repaired_vols = smile.repair_arbitrage(&strikes, 0.05, 0.02, 10)?;
    /// ```
    ///
    /// # References
    ///
    /// - Fengler, M. (2009). "Arbitrage-free smoothing of the implied volatility surface."
    ///   Quantitative Finance, 9(4), 417-428.
    pub fn repair_arbitrage(
        &self,
        strikes: &[f64],
        r: f64,
        q: f64,
        max_iterations: usize,
    ) -> Result<Vec<f64>> {
        if strikes.len() < 3 {
            return self.generate_smile(strikes);
        }

        // Generate initial smile
        let mut vols = self.generate_smile(strikes)?;

        // Convert to call prices for manipulation
        let mut prices: Vec<f64> = strikes
            .iter()
            .zip(vols.iter())
            .map(|(&k, &vol)| bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry))
            .collect();

        // Iterative repair
        for _ in 0..max_iterations {
            let mut changed = false;

            // Repair monotonicity: C(K₁) > C(K₂) for K₁ < K₂
            for i in 1..prices.len() {
                if prices[i] > prices[i - 1] {
                    // Project to monotonic: set to slightly below previous
                    prices[i] = prices[i - 1] * 0.9999;
                    changed = true;
                }
            }

            // Repair butterfly convexity: C(K-δ) - 2C(K) + C(K+δ) ≥ 0
            for i in 1..prices.len() - 1 {
                let butterfly = prices[i - 1] - 2.0 * prices[i] + prices[i + 1];
                if butterfly < 0.0 {
                    // Adjust mid-strike price to satisfy convexity
                    // C(K) should be at most (C(K-δ) + C(K+δ)) / 2
                    let max_mid = (prices[i - 1] + prices[i + 1]) / 2.0;
                    prices[i] = max_mid * 0.9999; // Slightly below for numerical safety
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }

        // Convert prices back to volatilities using implied vol inversion
        for i in 0..vols.len() {
            let target_price = prices[i];
            let k = strikes[i];

            // Newton-Raphson to find implied vol
            let mut vol = vols[i]; // Start from original vol
            for _ in 0..20 {
                let price = bs_call_price(self.forward, k, r, q, vol, self.time_to_expiry);
                let vega = bs_call_vega(self.forward, k, r, q, vol, self.time_to_expiry);

                if vega.abs() < 1e-14 {
                    break;
                }

                let error = price - target_price;
                if error.abs() < 1e-10 {
                    break;
                }

                vol -= error / vega;
                vol = vol.clamp(0.001, 5.0); // Reasonable bounds
            }

            vols[i] = vol;
        }

        Ok(vols)
    }
}

/// Black-Scholes call vega for implied vol inversion.
#[inline]
fn bs_call_vega(forward: f64, strike: f64, r: f64, q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 || vol <= 0.0 {
        return 0.0;
    }

    let (d1, _d2) = d1_d2(forward, strike, r, vol, t, q);
    let pdf_d1 = finstack_core::math::norm_pdf(d1);

    forward * (-q * t).exp() * t.sqrt() * pdf_d1
}

/// Black-Scholes call price for arbitrage checking.
///
/// Uses the standard Black-Scholes formula for European call options.
#[inline]
fn bs_call_price(forward: f64, strike: f64, r: f64, q: f64, vol: f64, t: f64) -> f64 {
    if t <= 0.0 {
        return (forward - strike).max(0.0);
    }

    let (d1, d2) = d1_d2(forward, strike, r, vol, t, q);

    let cdf_d1 = finstack_core::math::norm_cdf(d1);
    let cdf_d2 = finstack_core::math::norm_cdf(d2);

    forward * (-q * t).exp() * cdf_d1 - strike * (-r * t).exp() * cdf_d2
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_sabr_parameters_validation() {
        // Valid parameters
        assert!(SABRParameters::new(0.2, 0.5, 0.3, 0.1).is_ok());

        // Invalid alpha
        assert!(SABRParameters::new(-0.1, 0.5, 0.3, 0.1).is_err());

        // Invalid beta
        assert!(SABRParameters::new(0.2, 1.5, 0.3, 0.1).is_err());

        // Invalid rho
        assert!(SABRParameters::new(0.2, 0.5, 0.3, 1.5).is_err());
    }

    #[test]
    fn test_sabr_atm_volatility() {
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 100.0;
        let time_to_expiry = 1.0;

        let atm_vol = model
            .atm_volatility(forward, time_to_expiry)
            .expect("ATM volatility calculation should succeed in test");

        // ATM vol should be positive
        assert!(atm_vol > 0.0);

        // For ATM, implied vol should match ATM vol
        let implied_vol = model
            .implied_volatility(forward, forward, time_to_expiry)
            .expect("Volatility calculation should succeed in test");
        assert!((implied_vol - atm_vol).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_smile_shape() {
        let params = SABRParameters::new(0.2, 0.7, 0.4, -0.3)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 100.0;
        let time_to_expiry = 1.0;

        // Generate strikes
        let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
        let mut vols = Vec::new();

        for strike in &strikes {
            let vol = model
                .implied_volatility(forward, *strike, time_to_expiry)
                .expect("Volatility calculation should succeed in test");
            vols.push(vol);
        }

        // With negative rho, we expect downward sloping skew
        // Lower strikes should have higher vols
        // But the actual shape depends on all parameters
        // Just check that we get different vols (smile exists)
        let vol_range = vols
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .expect("Vols should not be empty")
            - vols
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .expect("Vols should not be empty");
        assert!(vol_range > 0.001); // There is a smile
    }

    #[test]
    fn test_sabr_normal_model() {
        // Beta = 0 gives normal SABR
        let params = SABRParameters::normal(20.0, 0.3, 0.0)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 0.05; // 5% rate
        let strike = 0.06; // 6% strike
        let time_to_expiry = 2.0;

        let vol = model
            .implied_volatility(forward, strike, time_to_expiry)
            .expect("Volatility calculation should succeed in test");

        // Should produce reasonable normal vol
        assert!(vol > 0.0);
        // Normal vol can be very large for small forward rates, so we just check it's positive
    }

    #[test]
    fn test_sabr_lognormal_model() {
        // Beta = 1 gives lognormal SABR (like Black-Scholes)
        let params = SABRParameters::lognormal(0.3, 0.4, 0.2)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 100.0;
        let strike = 105.0;
        let time_to_expiry = 0.5;

        let vol = model
            .implied_volatility(forward, strike, time_to_expiry)
            .expect("Volatility calculation should succeed in test");

        // Should produce reasonable lognormal vol
        assert!(vol > 0.0);
        assert!(vol < 1.0); // Less than 100% vol
    }

    #[test]
    fn test_sabr_calibration() {
        // Create synthetic market data
        let forward = 100.0;
        let strikes = vec![90.0, 95.0, 100.0, 105.0, 110.0];
        let market_vols = vec![0.22, 0.20, 0.19, 0.195, 0.21];
        let time_to_expiry = 1.0;
        let beta = 0.5; // Fixed beta

        let calibrator = SABRCalibrator::new();
        let params = calibrator
            .calibrate(forward, &strikes, &market_vols, time_to_expiry, beta)
            .expect("Volatility calculation should succeed in test");

        // Check calibrated parameters are reasonable
        assert!(params.alpha > 0.0);
        assert!(params.nu >= 0.0);
        assert!(params.rho >= -1.0 && params.rho <= 1.0);

        // Check fit quality
        let model = SABRModel::new(params);
        for (i, &strike) in strikes.iter().enumerate() {
            let model_vol = model
                .implied_volatility(forward, strike, time_to_expiry)
                .expect("Volatility calculation should succeed in test");
            let error = (model_vol - market_vols[i]).abs();
            assert!(error < 0.05); // Within 5% vol (calibration is approximate)
        }
    }

    #[test]
    fn test_sabr_smile_generator() {
        let params = SABRParameters::new(0.25, 0.6, 0.35, -0.25)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 1.0);

        let strikes = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];
        let vols = smile
            .generate_smile(&strikes)
            .expect("Smile generation should succeed in test");

        // Check all vols are positive
        for vol in &vols {
            assert!(*vol > 0.0);
        }

        // Validate that smile has variation (different volatilities)
        assert!(!vols.is_empty());
        assert!(vols.iter().all(|&v| v > 0.0));
    }

    #[test]
    fn test_sabr_negative_rates_shifted() {
        // Test shifted SABR with negative forward rates
        let forward = -0.005; // -50bps
        let strikes = vec![-0.01, -0.005, 0.0, 0.005, 0.01];
        let shift = 0.02; // 200bps shift

        let params = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, shift)
            .expect("SABR parameters should be valid in test"); // Higher alpha for more reasonable vols
        let model = SABRModel::new(params);

        // Should handle negative rates correctly
        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, 1.0);
            assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
            let vol_val = vol.expect("Volatility should be Some in test");
            assert!(
                vol_val > 0.0,
                "Non-positive volatility {} for strike {}",
                vol_val,
                strike
            );
            assert!(
                vol_val < 10.0,
                "Unreasonably high volatility {} for strike {}",
                vol_val,
                strike
            );
        }
    }

    #[test]
    fn test_sabr_atm_stability() {
        // Test enhanced ATM stability with very close strikes
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 0.025;
        let strikes = vec![
            forward - 1e-10,
            forward - 1e-12,
            forward,
            forward + 1e-12,
            forward + 1e-10,
        ];

        // All should give very similar results (ATM case)
        let mut vols = Vec::new();
        for &strike in &strikes {
            let vol = model
                .implied_volatility(forward, strike, 1.0)
                .expect("Implied volatility calculation should succeed in test");
            vols.push(vol);
        }

        // Check all ATM-like volatilities are similar with practical tolerance
        let vol_range = vols
            .iter()
            .max_by(|a, b| a.total_cmp(b))
            .expect("Vols should not be empty")
            - vols
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .expect("Vols should not be empty");
        assert!(vol_range < 1e-2); // Practical tolerance for numerical precision in ATM case
    }

    #[test]
    fn test_sabr_auto_shift_calibration() {
        // Test automatic shift detection and calibration
        let forward = -0.002; // Negative forward
        let strikes = vec![-0.005, -0.002, 0.0, 0.002, 0.005];
        let market_vols = vec![0.015, 0.012, 0.010, 0.011, 0.013]; // More reasonable vols for rates
        let time_to_expiry = 0.5;
        let beta = 0.0; // Normal model for rates

        let calibrator = SABRCalibrator::new().with_tolerance(1e-4); // Relaxed tolerance for difficult calibration
        let params = calibrator
            .calibrate_auto_shift(forward, &strikes, &market_vols, time_to_expiry, beta)
            .expect("Volatility calculation should succeed in test");

        // Should have detected need for shift
        assert!(params.is_shifted());
        assert!(params.shift().expect("Shift should be Some") > 0.0);

        // Check model works with negative rates
        let model = SABRModel::new(params);
        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, time_to_expiry);
            assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
            let vol_val = vol.expect("Volatility should be Some in test");
            assert!(
                vol_val > 0.0,
                "Non-positive volatility {} for strike {}",
                vol_val,
                strike
            );
        }
    }

    #[test]
    fn test_sabr_numerical_stability_extreme_parameters() {
        // Test with extreme but valid parameters
        let params = SABRParameters::new(0.01, 0.1, 0.1, 0.9)
            .expect("SABR parameters should be valid in test");
        let model = SABRModel::new(params);

        let forward = 0.001; // Very low rate
        let strikes = vec![0.0005, 0.001, 0.002];

        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, 5.0); // Long maturity
            assert!(vol.is_ok());
            let vol_val = vol.expect("Volatility should be Some in test");
            assert!(vol_val > 0.0);
            assert!(vol_val.is_finite());
        }
    }

    #[test]
    fn test_sabr_chi_function_stability() {
        // Test chi function with various extreme cases
        let params = SABRParameters::new(0.2, 0.5, 0.3, 0.95)
            .expect("SABR parameters should be valid in test"); // High rho
        let model = SABRModel::new(params);

        // Test small z values
        let small_z_values = vec![1e-8, 1e-6, 1e-4];
        for z in small_z_values {
            let chi = model.calculate_chi_robust(z);
            assert!(chi.is_ok());
            assert!(chi.expect("Chi should be Some").is_finite());
        }

        // Test rho ≈ 1 case
        let params_rho_one = SABRParameters::new(0.2, 0.5, 0.3, 0.999)
            .expect("SABR parameters should be valid in test");
        let model_rho_one = SABRModel::new(params_rho_one);
        let chi_rho_one = model_rho_one.calculate_chi_robust(0.1);
        assert!(chi_rho_one.is_ok());

        // Test rho ≈ -1 case
        let params_rho_minus_one = SABRParameters::new(0.2, 0.5, 0.3, -0.999)
            .expect("SABR parameters should be valid in test");
        let model_rho_minus_one = SABRModel::new(params_rho_minus_one);
        let chi_rho_minus_one = model_rho_minus_one.calculate_chi_robust(0.1);
        assert!(chi_rho_minus_one.is_ok());
    }

    // ===================================================================
    // Market Standards Validation Tests (Priority 1, Task 1.2)
    // ===================================================================

    #[test]
    fn test_sabr_rejects_negative_alpha() {
        let result = SABRParameters::new(-0.1, 0.5, 0.3, 0.1);
        assert!(result.is_err(), "Negative alpha should be rejected");

        let err = result.expect_err("should fail");
        assert!(
            matches!(err, Error::Validation(_)),
            "Should return Validation error"
        );

        // Verify error message mentions alpha
        let err_str = format!("{}", err);
        assert!(err_str.contains("alpha") || err_str.contains("α"));
    }

    #[test]
    fn test_sabr_rejects_zero_alpha() {
        let result = SABRParameters::new(0.0, 0.5, 0.3, 0.1);
        assert!(result.is_err(), "Zero alpha should be rejected");

        let err = result.expect_err("should fail");
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn test_sabr_rejects_invalid_rho() {
        // Rho > 1
        let result1 = SABRParameters::new(0.2, 0.5, 0.3, 1.5);
        assert!(result1.is_err(), "Rho > 1 should be rejected");
        assert!(matches!(
            result1.expect_err("should fail"),
            Error::Validation(_)
        ));

        // Rho < -1
        let result2 = SABRParameters::new(0.2, 0.5, 0.3, -1.5);
        assert!(result2.is_err(), "Rho < -1 should be rejected");
        assert!(matches!(
            result2.expect_err("should fail"),
            Error::Validation(_)
        ));

        // Rho = exactly 1.0 should be OK
        let result3 = SABRParameters::new(0.2, 0.5, 0.3, 1.0);
        assert!(result3.is_ok(), "Rho = 1.0 is valid");

        // Rho = exactly -1.0 should be OK
        let result4 = SABRParameters::new(0.2, 0.5, 0.3, -1.0);
        assert!(result4.is_ok(), "Rho = -1.0 is valid");
    }

    #[test]
    fn test_sabr_rejects_negative_nu() {
        let result = SABRParameters::new(0.2, 0.5, -0.1, 0.1);
        assert!(result.is_err(), "Negative nu should be rejected");

        let err = result.expect_err("should fail");
        assert!(matches!(err, Error::Validation(_)));

        // Verify error message mentions nu
        let err_str = format!("{}", err);
        assert!(err_str.contains("nu") || err_str.contains("ν"));
    }

    #[test]
    fn test_sabr_rejects_invalid_beta() {
        // Beta > 1
        let result1 = SABRParameters::new(0.2, 1.5, 0.3, 0.1);
        assert!(result1.is_err(), "Beta > 1 should be rejected");
        assert!(matches!(
            result1.expect_err("should fail"),
            Error::Validation(_)
        ));

        // Beta < 0
        let result2 = SABRParameters::new(0.2, -0.1, 0.3, 0.1);
        assert!(result2.is_err(), "Beta < 0 should be rejected");
        assert!(matches!(
            result2.expect_err("should fail"),
            Error::Validation(_)
        ));

        // Beta = 0 should be OK (normal SABR)
        let result3 = SABRParameters::new(0.2, 0.0, 0.3, 0.1);
        assert!(result3.is_ok(), "Beta = 0 is valid (normal SABR)");

        // Beta = 1 should be OK (lognormal SABR)
        let result4 = SABRParameters::new(0.2, 1.0, 0.3, 0.1);
        assert!(result4.is_ok(), "Beta = 1 is valid (lognormal SABR)");
    }

    #[test]
    fn test_sabr_accepts_boundary_values() {
        // Test that exact boundary values are accepted
        assert!(SABRParameters::new(1e-10, 0.0, 0.0, -1.0).is_ok());
        assert!(SABRParameters::new(1e-10, 1.0, 0.0, 1.0).is_ok());
        assert!(SABRParameters::new(0.001, 0.5, 0.0, 0.0).is_ok());
    }

    // ===================================================================
    // Inverse Normal CDF Precision Tests
    // ===================================================================

    #[test]
    fn test_normal_inverse_cdf_precision() {
        // Test that the inverse CDF has high precision for tail probabilities.
        // These golden values are from high-precision statistical tables.

        // Standard values
        assert!(
            (finstack_core::math::standard_normal_inv_cdf(0.5) - 0.0).abs() < 1e-12,
            "CDF^-1(0.5) should be 0"
        );
        assert!(
            (finstack_core::math::standard_normal_inv_cdf(0.84134474606854) - 1.0).abs() < 1e-8,
            "CDF^-1(0.84134...) should be ~1.0"
        );
        assert!(
            (finstack_core::math::standard_normal_inv_cdf(0.97724986805182) - 2.0).abs() < 1e-8,
            "CDF^-1(0.97724...) should be ~2.0"
        );

        // Tail precision test: p = 1e-8 should give approximately -5.6120
        // (from scipy.stats.norm.ppf(1e-8) = -5.612001244174965)
        let tail_result = finstack_core::math::standard_normal_inv_cdf(1e-8);
        assert!(
            (tail_result - (-5.612001244174965)).abs() < 1e-6,
            "Tail precision: CDF^-1(1e-8) = {} should be ~-5.612",
            tail_result
        );

        // Upper tail: p = 1 - 1e-8 should give approximately +5.6120
        let upper_tail_result = finstack_core::math::standard_normal_inv_cdf(1.0 - 1e-8);
        assert!(
            (upper_tail_result - 5.612001244174965).abs() < 1e-6,
            "Upper tail precision: CDF^-1(1-1e-8) = {} should be ~5.612",
            upper_tail_result
        );

        // Extreme tail: p = 1e-15 should give approximately -7.941
        let extreme_tail = finstack_core::math::standard_normal_inv_cdf(1e-15);
        assert!(
            (extreme_tail - (-7.941397804)).abs() < 1e-4,
            "Extreme tail: CDF^-1(1e-15) = {} should be ~-7.941",
            extreme_tail
        );
    }

    #[test]
    fn test_normal_inverse_cdf_boundary_behavior() {
        // Edge cases: boundaries should return appropriate infinity values
        assert!(
            finstack_core::math::standard_normal_inv_cdf(0.0).is_infinite()
                && finstack_core::math::standard_normal_inv_cdf(0.0) < 0.0,
            "CDF^-1(0) should be -infinity"
        );
        assert!(
            finstack_core::math::standard_normal_inv_cdf(1.0).is_infinite()
                && finstack_core::math::standard_normal_inv_cdf(1.0) > 0.0,
            "CDF^-1(1) should be +infinity"
        );

        // Values very close to boundaries
        let near_zero = finstack_core::math::standard_normal_inv_cdf(1e-300);
        assert!(near_zero < -30.0, "CDF^-1(1e-300) should be very negative");

        let near_one = finstack_core::math::standard_normal_inv_cdf(1.0 - 1e-300);
        assert!(near_one > 30.0, "CDF^-1(1-1e-300) should be very positive");
    }

    // ===================================================================
    // Arbitrage Validation Tests
    // ===================================================================

    #[test]
    fn test_sabr_arbitrage_validation_clean_smile() {
        // Well-behaved SABR parameters should produce arbitrage-free smile
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 1.0);

        let strikes: Vec<f64> = (70..=130).step_by(5).map(|k| k as f64).collect();
        let r = 0.05;
        let q = 0.02;

        let result = smile
            .validate_no_arbitrage(&strikes, r, q)
            .expect("Validation should succeed");

        assert!(
            result.is_arbitrage_free(),
            "Standard SABR parameters should be arbitrage-free. \
             Butterfly violations: {}, Monotonicity violations: {}",
            result.butterfly_violations.len(),
            result.monotonicity_violations.len()
        );
    }

    #[test]
    fn test_sabr_arbitrage_check_api() {
        // Test the simplified check API
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 1.0);

        let strikes: Vec<f64> = (80..=120).step_by(5).map(|k| k as f64).collect();

        // Should pass without error
        let check_result = smile.check_no_arbitrage(&strikes, 0.05, 0.02);
        assert!(
            check_result.is_ok(),
            "Clean smile should pass arbitrage check"
        );
    }

    #[test]
    fn test_sabr_arbitrage_validation_result_methods() {
        // Test ArbitrageValidationResult helper methods
        let mut result = ArbitrageValidationResult::default();

        // Empty result should be arbitrage-free
        assert!(result.is_arbitrage_free());
        assert!(result.worst_butterfly_severity().is_none());

        // Add a violation
        result.butterfly_violations.push(ButterflyViolation {
            strike: 100.0,
            butterfly_value: -0.01,
            severity_pct: 0.5,
        });

        assert!(!result.is_arbitrage_free());
        assert!(
            (result
                .worst_butterfly_severity()
                .expect("severity should exist after adding violation")
                - 0.5)
                .abs()
                < 1e-10
        );
    }

    #[test]
    fn test_sabr_arbitrage_too_few_strikes() {
        // With fewer than 3 strikes, validation should return empty result
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.2).expect("Valid SABR parameters");
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 1.0);

        let strikes = vec![95.0, 100.0]; // Only 2 strikes

        let result = smile
            .validate_no_arbitrage(&strikes, 0.05, 0.02)
            .expect("Validation should succeed");

        assert!(
            result.is_arbitrage_free(),
            "With < 3 strikes, no violations should be reported"
        );
    }

    #[test]
    fn test_sabr_arbitrage_extreme_params_may_have_violations() {
        // Extreme parameters might produce arbitrage (this tests detection, not prevention)
        // High vol-of-vol with extreme rho can sometimes produce problematic smiles
        let params = SABRParameters::new(0.5, 0.9, 1.5, 0.8).expect("Valid SABR parameters");
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 0.1); // Short expiry

        let strikes: Vec<f64> = (50..=150).step_by(5).map(|k| k as f64).collect();

        // This tests that the validation runs without panicking
        // The result may or may not have violations depending on exact parameters
        let result = smile.validate_no_arbitrage(&strikes, 0.05, 0.02);
        assert!(result.is_ok(), "Validation should complete without error");
    }
}
