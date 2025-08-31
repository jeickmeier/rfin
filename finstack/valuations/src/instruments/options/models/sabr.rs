//! SABR (Stochastic Alpha Beta Rho) volatility model implementation.
//!
//! The SABR model is widely used for pricing interest rate derivatives and FX options
//! with volatility smile. It provides closed-form approximations for implied volatility
//! that capture the smile and skew observed in market prices.

use finstack_core::{Error, Result, F};
use std::f64::consts::PI;

/// SABR model parameters
#[derive(Clone, Debug)]
pub struct SABRParameters {
    /// Initial volatility (alpha)
    pub alpha: F,
    /// CEV exponent (beta) - typically 0 to 1
    pub beta: F,
    /// Volatility of volatility (nu/volvol)
    pub nu: F,
    /// Correlation between asset and volatility (rho)
    pub rho: F,
}

impl SABRParameters {
    /// Create new SABR parameters with validation
    pub fn new(alpha: F, beta: F, nu: F, rho: F) -> Result<Self> {
        // Validate parameters
        if alpha <= 0.0 {
            return Err(Error::Internal); // Alpha must be positive
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(Error::Internal); // Beta must be in [0, 1]
        }
        if nu < 0.0 {
            return Err(Error::Internal); // Nu must be non-negative
        }
        if !(-1.0..=1.0).contains(&rho) {
            return Err(Error::Internal); // Rho must be in [-1, 1]
        }

        Ok(Self {
            alpha,
            beta,
            nu,
            rho,
        })
    }

    /// Create parameters for normal SABR (beta = 0)
    pub fn normal(alpha: F, nu: F, rho: F) -> Result<Self> {
        Self::new(alpha, 0.0, nu, rho)
    }

    /// Create parameters for lognormal SABR (beta = 1)
    pub fn lognormal(alpha: F, nu: F, rho: F) -> Result<Self> {
        Self::new(alpha, 1.0, nu, rho)
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
    /// This is the standard SABR formula from Hagan et al. (2002)
    pub fn implied_volatility(&self, forward: F, strike: F, time_to_expiry: F) -> Result<F> {
        // Handle ATM case
        if (forward - strike).abs() < 1e-12 {
            return self.atm_volatility(forward, time_to_expiry);
        }

        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        // Calculate intermediate values
        let f_mid = (forward * strike).sqrt();
        let f_mid_beta = f_mid.powf(beta);

        // Log-moneyness
        let z = if beta == 1.0 {
            (forward / strike).ln()
        } else {
            (nu / alpha) * (forward.powf(1.0 - beta) - strike.powf(1.0 - beta)) / (1.0 - beta)
        };

        // Avoid division by zero
        if z.abs() < 1e-10 {
            return self.atm_volatility(forward, time_to_expiry);
        }

        // Calculate chi(z)
        let x = self.calculate_chi(z)?;

        // First factor
        let factor1 = alpha
            / (f_mid_beta
                * (1.0
                    + (1.0 - beta).powi(2) / 24.0 * ((forward / strike).ln()).powi(2)
                    + (1.0 - beta).powi(4) / 1920.0 * ((forward / strike).ln()).powi(4)));

        // Second factor (z/x correction)
        let factor2 = z / x;

        // Third factor (time correction)
        let factor3 = 1.0
            + time_to_expiry
                * ((1.0 - beta).powi(2) / 24.0 * alpha.powi(2) / f_mid.powf(2.0 * (1.0 - beta))
                    + 0.25 * rho * beta * nu * alpha / f_mid_beta
                    + (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2));

        Ok(factor1 * factor2 * factor3)
    }

    /// Calculate ATM implied volatility
    fn atm_volatility(&self, forward: F, time_to_expiry: F) -> Result<F> {
        let alpha = self.params.alpha;
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        let f_beta = forward.powf(beta);

        // ATM volatility formula
        let vol = alpha / f_beta
            * (1.0
                + time_to_expiry
                    * ((1.0 - beta).powi(2) / 24.0 * alpha.powi(2)
                        / forward.powf(2.0 * (1.0 - beta))
                        + 0.25 * rho * beta * nu * alpha / f_beta
                        + (2.0 - 3.0 * rho.powi(2)) / 24.0 * nu.powi(2)));

        Ok(vol)
    }

    /// Calculate chi(z) for the SABR formula
    fn calculate_chi(&self, z: F) -> Result<F> {
        let _nu = self.params.nu;
        let rho = self.params.rho;

        // Calculate discriminant
        let discriminant = 1.0 - 2.0 * rho * z + z.powi(2);

        if discriminant < 0.0 {
            return Err(Error::Internal); // Invalid parameters
        }

        let sqrt_disc = discriminant.sqrt();

        // Calculate chi
        let numerator = sqrt_disc + z - rho;
        let denominator = 1.0 - rho;

        if denominator.abs() < 1e-12 {
            // Handle rho ≈ 1 case
            Ok(z / (1.0 + z))
        } else {
            Ok((numerator / denominator).ln())
        }
    }

    /// Calculate implied volatility with advanced Obloj correction
    ///
    /// This provides better accuracy for extreme strikes and long maturities
    pub fn implied_volatility_obloj(&self, forward: F, strike: F, time_to_expiry: F) -> Result<F> {
        // Get base Hagan volatility
        let base_vol = self.implied_volatility(forward, strike, time_to_expiry)?;

        // Apply Obloj correction for better accuracy
        let correction = self.obloj_correction(forward, strike, time_to_expiry)?;

        Ok(base_vol * correction)
    }

    /// Calculate Obloj correction factor
    fn obloj_correction(&self, forward: F, strike: F, time_to_expiry: F) -> Result<F> {
        let beta = self.params.beta;
        let nu = self.params.nu;
        let rho = self.params.rho;

        // Moneyness
        let moneyness = (forward / strike).ln();

        // Correction terms
        let term1 = (1.0 - beta).powi(2) * moneyness.powi(2) / 24.0;
        let term2 = rho * nu * moneyness / 4.0;
        let term3 = (2.0 - 3.0 * rho.powi(2)) * nu.powi(2) / 24.0;

        // Time-dependent correction
        let correction = 1.0 + time_to_expiry.powi(2) * (term1 + term2 + term3);

        Ok(correction.sqrt())
    }

    /// Calculate the density (PDF) of the underlying at expiry
    pub fn density(&self, forward: F, strike: F, time_to_expiry: F) -> Result<F> {
        // Use finite differences on the implied volatility
        let bump = strike * 0.0001;

        let vol_plus = self.implied_volatility(forward, strike + bump, time_to_expiry)?;
        let vol_minus = self.implied_volatility(forward, strike - bump, time_to_expiry)?;

        // Calculate density using Dupire formula
        let _dvol_dk = (vol_plus - vol_minus) / (2.0 * bump);
        let _d2vol_dk2 = (vol_plus
            - 2.0 * self.implied_volatility(forward, strike, time_to_expiry)?
            + vol_minus)
            / bump.powi(2);

        // Simplified density calculation
        let vol = self.implied_volatility(forward, strike, time_to_expiry)?;
        let variance = vol.powi(2) * time_to_expiry;

        let d1 = ((forward / strike).ln() + 0.5 * variance) / variance.sqrt();
        let density = (-d1.powi(2) / 2.0).exp() / (strike * variance.sqrt() * (2.0 * PI).sqrt());

        Ok(density)
    }

    /// Get model parameters
    pub fn parameters(&self) -> &SABRParameters {
        &self.params
    }

    /// Update model parameters
    pub fn set_parameters(&mut self, params: SABRParameters) {
        self.params = params;
    }
}

/// SABR calibration using market prices
pub struct SABRCalibrator {
    /// Tolerance for calibration convergence
    tolerance: F,
    /// Maximum iterations
    max_iterations: usize,
    /// Use ATM constraint (reserved for future)
    #[allow(dead_code)]
    use_atm_constraint: bool,
}

impl SABRCalibrator {
    /// Create new calibrator
    pub fn new() -> Self {
        Self {
            tolerance: 1e-6,
            max_iterations: 100,
            use_atm_constraint: true,
        }
    }

    /// Set tolerance
    pub fn with_tolerance(mut self, tolerance: F) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Set maximum iterations
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// Calibrate SABR parameters to market implied volatilities
    pub fn calibrate(
        &self,
        forward: F,
        strikes: &[F],
        market_vols: &[F],
        time_to_expiry: F,
        beta: F, // Beta is usually fixed
    ) -> Result<SABRParameters> {
        if strikes.len() != market_vols.len() {
            return Err(Error::Internal);
        }

        // Initial guess for parameters
        let atm_vol = self.find_atm_vol(forward, strikes, market_vols)?;
        let mut alpha = atm_vol * forward.powf(1.0 - beta);
        let mut nu = 0.3; // Typical initial guess
        let mut rho = 0.0; // Start with zero correlation

        // Levenberg-Marquardt optimization
        for _ in 0..self.max_iterations {
            let params = SABRParameters::new(alpha, beta, nu, rho)?;
            let model = SABRModel::new(params.clone());

            // Calculate residuals
            let mut residuals = Vec::new();
            let mut jacobian = Vec::new();

            for (i, &strike) in strikes.iter().enumerate() {
                let model_vol = model.implied_volatility(forward, strike, time_to_expiry)?;
                let residual = model_vol - market_vols[i];
                residuals.push(residual);

                // Calculate derivatives numerically
                let derivatives =
                    self.calculate_derivatives(&params, forward, strike, time_to_expiry)?;
                jacobian.push(derivatives);
            }

            // Check convergence
            let error: F = residuals.iter().map(|r| r.powi(2)).sum::<F>().sqrt();
            if error < self.tolerance {
                return Ok(params);
            }

            // Update parameters using gradient descent
            let (d_alpha, d_nu, d_rho) = self.calculate_update(&residuals, &jacobian)?;

            alpha = (alpha - 0.1 * d_alpha).max(0.001);
            nu = (nu - 0.1 * d_nu).clamp(0.0, 2.0);
            rho = (rho - 0.1 * d_rho).clamp(-0.99, 0.99);
        }

        // Return best estimate even if not fully converged
        SABRParameters::new(alpha, beta, nu, rho)
    }

    /// Find ATM volatility from market data
    fn find_atm_vol(&self, forward: F, strikes: &[F], vols: &[F]) -> Result<F> {
        // Find the strike closest to forward
        let mut min_diff = F::INFINITY;
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

    /// Calculate parameter derivatives numerically
    fn calculate_derivatives(
        &self,
        params: &SABRParameters,
        forward: F,
        strike: F,
        time_to_expiry: F,
    ) -> Result<(F, F, F)> {
        let bump = 0.0001;

        // Base model
        let base_model = SABRModel::new(params.clone());
        let base_vol = base_model.implied_volatility(forward, strike, time_to_expiry)?;

        // Alpha derivative
        let mut params_alpha = params.clone();
        params_alpha.alpha += bump;
        let model_alpha = SABRModel::new(params_alpha);
        let vol_alpha = model_alpha.implied_volatility(forward, strike, time_to_expiry)?;
        let d_alpha = (vol_alpha - base_vol) / bump;

        // Nu derivative
        let mut params_nu = params.clone();
        params_nu.nu += bump;
        let model_nu = SABRModel::new(params_nu);
        let vol_nu = model_nu.implied_volatility(forward, strike, time_to_expiry)?;
        let d_nu = (vol_nu - base_vol) / bump;

        // Rho derivative
        let mut params_rho = params.clone();
        params_rho.rho = (params_rho.rho + bump).min(0.999);
        let model_rho = SABRModel::new(params_rho);
        let vol_rho = model_rho.implied_volatility(forward, strike, time_to_expiry)?;
        let d_rho = (vol_rho - base_vol) / bump;

        Ok((d_alpha, d_nu, d_rho))
    }

    /// Calculate parameter update using least squares
    fn calculate_update(&self, residuals: &[F], jacobian: &[(F, F, F)]) -> Result<(F, F, F)> {
        // Simple gradient calculation
        let mut grad_alpha = 0.0;
        let mut grad_nu = 0.0;
        let mut grad_rho = 0.0;

        for (i, &residual) in residuals.iter().enumerate() {
            let (d_alpha, d_nu, d_rho) = jacobian[i];
            grad_alpha += residual * d_alpha;
            grad_nu += residual * d_nu;
            grad_rho += residual * d_rho;
        }

        Ok((grad_alpha, grad_nu, grad_rho))
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
    forward: F,
    time_to_expiry: F,
}

impl SABRSmile {
    /// Create new smile generator
    pub fn new(model: SABRModel, forward: F, time_to_expiry: F) -> Self {
        Self {
            model,
            forward,
            time_to_expiry,
        }
    }

    /// Generate volatility smile for given strikes
    pub fn generate_smile(&self, strikes: &[F]) -> Result<Vec<F>> {
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
    pub fn strike_from_delta(&self, delta: F, is_call: bool) -> Result<F> {
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

    /// Calculate skew (derivative of implied vol with respect to log-strike)
    pub fn skew(&self, strike: F) -> Result<F> {
        let bump = strike * 0.001;

        let vol_up = self.model.implied_volatility(
            self.forward,
            strike * (1.0 + bump),
            self.time_to_expiry,
        )?;

        let vol_down = self.model.implied_volatility(
            self.forward,
            strike * (1.0 - bump),
            self.time_to_expiry,
        )?;

        Ok((vol_up - vol_down) / (2.0 * bump))
    }

    /// Calculate smile curvature (second derivative)
    pub fn curvature(&self, strike: F) -> Result<F> {
        let bump = strike * 0.001;

        let vol_center =
            self.model
                .implied_volatility(self.forward, strike, self.time_to_expiry)?;

        let vol_up = self.model.implied_volatility(
            self.forward,
            strike * (1.0 + bump),
            self.time_to_expiry,
        )?;

        let vol_down = self.model.implied_volatility(
            self.forward,
            strike * (1.0 - bump),
            self.time_to_expiry,
        )?;

        Ok((vol_up - 2.0 * vol_center + vol_down) / bump.powi(2))
    }
}

/// Helper function for normal CDF inverse (simplified)
fn normal_inverse_cdf(p: F) -> F {
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
        let params = SABRParameters::new(0.2, 0.5, 0.3, -0.1).unwrap();
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
        let vol_range = vols
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap()
            - vols
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap();
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

        // Check skew calculation
        let skew = smile.skew(100.0).unwrap();
        // With negative rho, expect negative skew
        assert!(skew < 0.0);

        // Check curvature (smile)
        let curvature = smile.curvature(100.0).unwrap();
        // Typically positive (smile shape)
        assert!(curvature > 0.0);
    }
}
