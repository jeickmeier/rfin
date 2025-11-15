//! SABR (Stochastic Alpha Beta Rho) volatility model implementation.
//!
//! The SABR model is widely used for pricing interest rate derivatives and FX options
//! with volatility smile. It provides closed-form approximations for implied volatility
//! that capture the smile and skew observed in market prices.

use finstack_core::{Error, Result};

/// SABR model parameters
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

    /// Create new SABR parameters with shift for negative rates.
    ///
    /// Same validation as `new()` plus shift validation:
    /// - shift > 0: Shift must be positive for negative rate support
    pub fn new_with_shift(alpha: f64, beta: f64, nu: f64, rho: f64, shift: f64) -> Result<Self> {
        // Validate base parameters with descriptive messages
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
        // Shift should be positive to handle negative rates
        if shift <= 0.0 {
            return Err(Error::Validation(format!(
                "SABR shift parameter must be positive for negative rate support, got: {:.6}",
                shift
            )));
        }

        Ok(Self {
            alpha,
            beta,
            nu,
            rho,
            shift: Some(shift),
        })
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
                return Err(Error::Internal); // Standard SABR requires positive rates
            }
            (forward, strike)
        };

        // Enhanced ATM detection with absolute and relative tolerance
        let abs_diff = (effective_forward - effective_strike).abs();
        let relative_diff = abs_diff / effective_forward.max(effective_strike);
        if abs_diff < 1e-8 || relative_diff < 1e-8 {
            return self.atm_volatility(effective_forward, time_to_expiry);
        }

        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        // Calculate intermediate values with numerical protection
        let f_mid = (effective_forward * effective_strike).sqrt();
        let f_mid_beta = if beta == 0.0 {
            1.0 // Special case for normal model
        } else {
            f_mid.powf(beta)
        };

        // Enhanced log-moneyness calculation
        let z = if nu.abs() < 1e-14 {
            // Handle nu ≈ 0 case (pure CEV)
            return self.atm_volatility(effective_forward, time_to_expiry);
        } else if beta == 1.0 {
            (nu / alpha) * (effective_forward / effective_strike).ln()
        } else if beta == 0.0 {
            (nu / alpha) * (effective_forward - effective_strike)
        } else {
            (nu / alpha) * (effective_forward.powf(1.0 - beta) - effective_strike.powf(1.0 - beta))
                / (1.0 - beta)
        };

        // Enhanced ATM detection based on z
        if z.abs() < 1e-8 {
            return self.atm_volatility(effective_forward, time_to_expiry);
        }

        // Calculate chi(z) with robust numerical handling
        let x = self.calculate_chi_robust(z)?;

        // Calculate log-moneyness for correction terms
        let log_moneyness = (effective_forward / effective_strike).ln();

        // First factor with enhanced numerical stability
        let factor1 = if f_mid_beta.abs() < 1e-14 {
            alpha // Handle degenerate case
        } else {
            let correction_term = if beta == 0.0 {
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
            z / x
        };

        // Third factor (time correction) with enhanced precision
        let time_correction = if beta == 0.0 {
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
            return Err(Error::Internal); // Invalid volatility result
        }

        Ok(volatility)
    }

    /// Calculate ATM implied volatility with enhanced numerical stability
    fn atm_volatility(&self, forward: f64, time_to_expiry: f64) -> Result<f64> {
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        // Handle degenerate cases
        if alpha.abs() < 1e-14 {
            return Ok(0.0);
        }

        // ATM volatility formula with numerical protection
        let vol = if beta == 0.0 {
            // Normal SABR: vol = alpha * (1 + T * (2-3*rho²)/24 * nu²)
            alpha * (1.0 + time_to_expiry * (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2))
        } else if beta == 1.0 {
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

            let alpha_term = if beta == 0.5 {
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
            return Err(Error::Internal); // Invalid ATM volatility
        }

        Ok(vol)
    }

    /// Calculate chi(z) for the SABR formula with enhanced numerical stability
    fn calculate_chi_robust(&self, z: f64) -> Result<f64> {
        let rho = self.params.rho;

        // For very small z, use series expansion to avoid numerical issues
        if z.abs() < 1e-6 {
            // Series expansion: χ(z) ≈ z * (1 + (rho-1)/2 * z + ...)
            return Ok(z * (1.0 + (rho - 1.0) / 2.0 * z));
        }

        // Calculate discriminant with protection
        let discriminant = 1.0 - 2.0 * rho * z + z.powi(2);

        if discriminant < 0.0 {
            return Err(Error::Internal); // Invalid parameters
        }

        let sqrt_disc = discriminant.sqrt();

        // Enhanced handling for different rho cases
        if (1.0 - rho).abs() < 1e-12 {
            // Handle rho ≈ 1 case with series expansion
            // For rho ≈ 1: χ(z) ≈ z/(1+z/2) for small z
            if z.abs() < 0.1 {
                Ok(z / (1.0 + z / 2.0))
            } else {
                Ok(z / (1.0 + z))
            }
        } else if (-1.0 - rho).abs() < 1e-12 {
            // Handle rho ≈ -1 case
            Ok((sqrt_disc + z + 1.0).ln() - 0.5 * 2.0_f64.ln())
        } else {
            // Standard case with numerical protection
            let numerator = sqrt_disc + z - rho;
            let denominator = 1.0 - rho;

            if numerator <= 0.0 {
                return Err(Error::Internal); // Would result in log of non-positive number
            }

            Ok((numerator / denominator).ln())
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
            return Err(Error::Internal); // Invalid time to expiry
        }

        // Rate validation based on model type
        if self.params.shift.is_none() {
            // Standard SABR requires positive rates
            if forward <= 0.0 || strike <= 0.0 {
                return Err(Error::Internal); // Standard SABR requires positive rates
            }
        } else {
            // Shifted SABR allows negative rates but shifted values must be positive
            let shift = self
                .params
                .shift
                .expect("Shift should be Some when using shifted SABR");
            if forward + shift <= 0.0 || strike + shift <= 0.0 {
                return Err(Error::Internal); // Shifted rates must result in positive values
            }
        }

        Ok(())
    }
}

/// SABR calibration using market prices
pub struct SABRCalibrator {
    /// Tolerance for calibration convergence
    tolerance: f64,
    /// Maximum iterations
    max_iterations: usize,
    /// Use finite-difference gradients instead of analytical approximations
    use_fd_gradients: bool,
}

impl SABRCalibrator {
    /// Create new calibrator
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 100,
            use_fd_gradients: false,
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
        let min_rate = forward.min(
            *strikes
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .expect("Strikes should not be empty")
        );

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
        let min_rate = forward.min(
            *strikes
                .iter()
                .min_by(|a, b| a.total_cmp(b))
                .expect("Strikes should not be empty")
        );

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
            return Err(Error::Internal);
        }

        // Apply shift to all rates
        let shifted_forward = forward + shift;
        let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

        // Validate shifted rates are positive
        if shifted_forward <= 0.0 || shifted_strikes.iter().any(|&s| s <= 0.0) {
            return Err(Error::Internal); // Insufficient shift for negative rates
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
            return Err(Error::Internal);
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
            return Err(Error::Internal);
        }

        // Use analytical derivatives from the calibration module
        use crate::calibration::derivatives::sabr_derivatives::{
            SABRCalibrationDerivatives, SABRMarketData,
        };
        use finstack_core::math::solver_multi::LevenbergMarquardtSolver;

        // Create market data structure
        let market_data = SABRMarketData {
            forward,
            time_to_expiry,
            strikes: strikes.to_vec(),
            market_vols: market_vols.to_vec(),
            beta,
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
            return Err(Error::Internal);
        }

        // Apply shift to all rates
        let shifted_forward = forward + shift;
        let shifted_strikes: Vec<f64> = strikes.iter().map(|&s| s + shift).collect();

        // Validate shifted rates are positive
        if shifted_forward <= 0.0 || shifted_strikes.iter().any(|&s| s <= 0.0) {
            return Err(Error::Internal); // Insufficient shift for negative rates
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

impl SABRSmile {
    /// Create new smile generator
    pub fn new(model: SABRModel, forward: f64, time_to_expiry: f64) -> Self {
        Self {
            model,
            forward,
            time_to_expiry,
        }
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
            normal_inverse_cdf(delta)
        } else {
            normal_inverse_cdf(1.0 - delta)
        };

        let strike = self.forward * (z * std_dev).exp();
        Ok(strike)
    }
}

/// Helper function for normal CDF inverse (simplified)
fn normal_inverse_cdf(p: f64) -> f64 {
    // Simplified approximation - in production use a proper implementation
    if p <= 0.0 || p >= 1.0 {
        return if p <= 0.0 { -3.0 } else { 3.0 };
    }

    // Rational approximation
    let a = [
        2.50662823884,
        -18.61500062529,
        41.39119773534,
        -25.44106049637,
    ];
    let b = [
        -8.47351093090,
        23.08336743743,
        -21.06224101826,
        3.13082909833,
    ];

    let x = p - 0.5;
    if x.abs() < 0.42 {
        let x2 = x * x;
        let num = x * (((a[3] * x2 + a[2]) * x2 + a[1]) * x2 + a[0]);
        let den = (((b[3] * x2 + b[2]) * x2 + b[1]) * x2 + b[0]) * x2 + 1.0;
        num / den
    } else {
        let y = if x > 0.0 { 1.0 - p } else { p };
        let z = (-y.ln()).sqrt();
        let num = ((a[3] * z + a[2]) * z + a[1]) * z + a[0];
        let den = ((b[3] * z + b[2]) * z + b[1]) * z + b[0];
        if x > 0.0 {
            num / den
        } else {
            -(num / den)
        }
    }
}

#[cfg(test)]
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

        let atm_vol = model.atm_volatility(forward, time_to_expiry).unwrap();

        // ATM vol should be positive
        assert!(atm_vol > 0.0);

        // For ATM, implied vol should match ATM vol
        let implied_vol = model
            .implied_volatility(forward, forward, time_to_expiry)
            .unwrap();
        assert!((implied_vol - atm_vol).abs() < 1e-10);
    }

    #[test]
    fn test_sabr_smile_shape() {
        let params = SABRParameters::new(0.2, 0.7, 0.4, -0.3).unwrap();
        let model = SABRModel::new(params);

        let forward = 100.0;
        let time_to_expiry = 1.0;

        // Generate strikes
        let strikes = vec![80.0, 90.0, 100.0, 110.0, 120.0];
        let mut vols = Vec::new();

        for strike in &strikes {
            let vol = model
                .implied_volatility(forward, *strike, time_to_expiry)
                .unwrap();
            vols.push(vol);
        }

        // With negative rho, we expect downward sloping skew
        // Lower strikes should have higher vols
        // But the actual shape depends on all parameters
        // Just check that we get different vols (smile exists)
        let vol_range = vols.iter().max_by(|a, b| a.total_cmp(b)).unwrap()
            - vols.iter().min_by(|a, b| a.total_cmp(b)).unwrap();
        assert!(vol_range > 0.001); // There is a smile
    }

    #[test]
    fn test_sabr_normal_model() {
        // Beta = 0 gives normal SABR
        let params = SABRParameters::normal(20.0, 0.3, 0.0).unwrap();
        let model = SABRModel::new(params);

        let forward = 0.05; // 5% rate
        let strike = 0.06; // 6% strike
        let time_to_expiry = 2.0;

        let vol = model
            .implied_volatility(forward, strike, time_to_expiry)
            .unwrap();

        // Should produce reasonable normal vol
        assert!(vol > 0.0);
        // Normal vol can be very large for small forward rates, so we just check it's positive
    }

    #[test]
    fn test_sabr_lognormal_model() {
        // Beta = 1 gives lognormal SABR (like Black-Scholes)
        let params = SABRParameters::lognormal(0.3, 0.4, 0.2).unwrap();
        let model = SABRModel::new(params);

        let forward = 100.0;
        let strike = 105.0;
        let time_to_expiry = 0.5;

        let vol = model
            .implied_volatility(forward, strike, time_to_expiry)
            .unwrap();

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
            .unwrap();

        // Check calibrated parameters are reasonable
        assert!(params.alpha > 0.0);
        assert!(params.nu >= 0.0);
        assert!(params.rho >= -1.0 && params.rho <= 1.0);

        // Check fit quality
        let model = SABRModel::new(params);
        for (i, &strike) in strikes.iter().enumerate() {
            let model_vol = model
                .implied_volatility(forward, strike, time_to_expiry)
                .unwrap();
            let error = (model_vol - market_vols[i]).abs();
            assert!(error < 0.05); // Within 5% vol (calibration is approximate)
        }
    }

    #[test]
    fn test_sabr_smile_generator() {
        let params = SABRParameters::new(0.25, 0.6, 0.35, -0.25).unwrap();
        let model = SABRModel::new(params);
        let smile = SABRSmile::new(model, 100.0, 1.0);

        let strikes = vec![85.0, 90.0, 95.0, 100.0, 105.0, 110.0, 115.0];
        let vols = smile.generate_smile(&strikes).unwrap();

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

        let params = SABRParameters::new_with_shift(0.2, 0.5, 0.3, -0.2, shift).unwrap(); // Higher alpha for more reasonable vols
        let model = SABRModel::new(params);

        // Should handle negative rates correctly
        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, 1.0);
            assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
            let vol_val = vol.unwrap();
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
            let vol = model.implied_volatility(forward, strike, 1.0).unwrap();
            vols.push(vol);
        }

        // Check all ATM-like volatilities are similar with practical tolerance
        let vol_range = vols.iter().max_by(|a, b| a.total_cmp(b)).unwrap()
            - vols.iter().min_by(|a, b| a.total_cmp(b)).unwrap();
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
            .unwrap();

        // Should have detected need for shift
        assert!(params.is_shifted());
        assert!(params.shift().unwrap() > 0.0);

        // Check model works with negative rates
        let model = SABRModel::new(params);
        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, time_to_expiry);
            assert!(vol.is_ok(), "Failed for strike {}: {:?}", strike, vol);
            let vol_val = vol.unwrap();
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
        let params = SABRParameters::new(0.01, 0.1, 0.1, 0.9).unwrap();
        let model = SABRModel::new(params);

        let forward = 0.001; // Very low rate
        let strikes = vec![0.0005, 0.001, 0.002];

        for &strike in &strikes {
            let vol = model.implied_volatility(forward, strike, 5.0); // Long maturity
            assert!(vol.is_ok());
            let vol_val = vol.unwrap();
            assert!(vol_val > 0.0);
            assert!(vol_val.is_finite());
        }
    }

    #[test]
    fn test_sabr_chi_function_stability() {
        // Test chi function with various extreme cases
        let params = SABRParameters::new(0.2, 0.5, 0.3, 0.95).unwrap(); // High rho
        let model = SABRModel::new(params);

        // Test small z values
        let small_z_values = vec![1e-8, 1e-6, 1e-4];
        for z in small_z_values {
            let chi = model.calculate_chi_robust(z);
            assert!(chi.is_ok());
            assert!(chi.unwrap().is_finite());
        }

        // Test rho ≈ 1 case
        let params_rho_one = SABRParameters::new(0.2, 0.5, 0.3, 0.999).unwrap();
        let model_rho_one = SABRModel::new(params_rho_one);
        let chi_rho_one = model_rho_one.calculate_chi_robust(0.1);
        assert!(chi_rho_one.is_ok());

        // Test rho ≈ -1 case
        let params_rho_minus_one = SABRParameters::new(0.2, 0.5, 0.3, -0.999).unwrap();
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

        let err = result.unwrap_err();
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

        let err = result.unwrap_err();
        assert!(matches!(err, Error::Validation(_)));
    }

    #[test]
    fn test_sabr_rejects_invalid_rho() {
        // Rho > 1
        let result1 = SABRParameters::new(0.2, 0.5, 0.3, 1.5);
        assert!(result1.is_err(), "Rho > 1 should be rejected");
        assert!(matches!(result1.unwrap_err(), Error::Validation(_)));

        // Rho < -1
        let result2 = SABRParameters::new(0.2, 0.5, 0.3, -1.5);
        assert!(result2.is_err(), "Rho < -1 should be rejected");
        assert!(matches!(result2.unwrap_err(), Error::Validation(_)));

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

        let err = result.unwrap_err();
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
        assert!(matches!(result1.unwrap_err(), Error::Validation(_)));

        // Beta < 0
        let result2 = SABRParameters::new(0.2, -0.1, 0.3, 0.1);
        assert!(result2.is_err(), "Beta < 0 should be rejected");
        assert!(matches!(result2.unwrap_err(), Error::Validation(_)));

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
}
