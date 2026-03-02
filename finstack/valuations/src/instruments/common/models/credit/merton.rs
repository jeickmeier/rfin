//! Merton structural credit model with distance-to-default and default probability.
//!
//! Implements the Merton (1974) model and its Black-Cox (1976) first-passage
//! extension for estimating firm default probability from balance-sheet data.
//!
//! # References
//!
//! - Merton, R. C. (1974). "On the Pricing of Corporate Debt: The Risk
//!   Structure of Interest Rates." *Journal of Finance*, 29(2), 449-470.
//! - Black, F. & Cox, J. C. (1976). "Valuing Corporate Securities: Some
//!   Effects of Bond Indenture Provisions." *Journal of Finance*, 31(2), 351-367.
//!
//! # Examples
//!
//! ```
//! use finstack_valuations::instruments::common::models::credit::MertonModel;
//!
//! let model = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
//! let dd = model.distance_to_default(1.0);
//! let pd = model.default_probability(1.0);
//! let spread = model.implied_spread(5.0, 0.40);
//! ```

use finstack_core::math::norm_cdf;
use finstack_core::{InputError, Result};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Asset dynamics specification for the Merton model.
///
/// Controls the stochastic process assumed for the firm's asset value.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AssetDynamics {
    /// Standard geometric Brownian motion (lognormal diffusion).
    GeometricBrownian,
    /// Jump-diffusion process (Merton 1976) with Poisson jumps.
    JumpDiffusion {
        /// Poisson jump arrival intensity (jumps per year).
        jump_intensity: f64,
        /// Mean log-jump size.
        jump_mean: f64,
        /// Volatility of log-jump size.
        jump_vol: f64,
    },
    /// CreditGrades model extension with uncertain recovery barrier.
    CreditGrades {
        /// Uncertainty in the default barrier level.
        barrier_uncertainty: f64,
        /// Mean recovery rate at default.
        mean_recovery: f64,
    },
}

/// Barrier monitoring type for default determination.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum BarrierType {
    /// Default only assessed at maturity (classic Merton).
    Terminal,
    /// Continuous barrier monitoring (Black-Cox extension).
    FirstPassage {
        /// Growth rate of the default barrier over time.
        barrier_growth_rate: f64,
    },
}

/// Merton structural credit model.
///
/// Models a firm's equity as a call option on its assets, where default
/// occurs when asset value falls below the debt barrier.
///
/// # Fields
///
/// - `asset_value` (V_0): Current market value of the firm's assets.
/// - `asset_vol` (sigma_V): Annualized volatility of asset returns.
/// - `debt_barrier` (B): Face value of debt / default point.
/// - `risk_free_rate` (r): Continuous risk-free rate.
/// - `payout_rate` (q): Continuous dividend / payout yield on assets.
/// - `barrier_type`: Terminal or first-passage barrier monitoring.
/// - `dynamics`: Asset return dynamics specification.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MertonModel {
    asset_value: f64,
    asset_vol: f64,
    debt_barrier: f64,
    risk_free_rate: f64,
    payout_rate: f64,
    barrier_type: BarrierType,
    dynamics: AssetDynamics,
}

impl MertonModel {
    /// Create a new Merton model with GBM dynamics and terminal barrier.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be >= 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value <= 0` or
    /// `debt_barrier <= 0`, and [`InputError::NegativeValue`] if `asset_vol < 0`.
    pub fn new(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
    ) -> Result<Self> {
        Self::new_with_dynamics(
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            0.0,
            BarrierType::Terminal,
            AssetDynamics::GeometricBrownian,
        )
    }

    /// Create a new Merton model with full parameterisation.
    ///
    /// # Arguments
    ///
    /// * `asset_value` - Current asset value V_0 (must be > 0)
    /// * `asset_vol` - Asset volatility sigma_V (must be >= 0)
    /// * `debt_barrier` - Debt face value B (must be > 0)
    /// * `risk_free_rate` - Risk-free rate r
    /// * `payout_rate` - Dividend / payout yield q
    /// * `barrier_type` - Terminal or first-passage
    /// * `dynamics` - Asset return dynamics
    ///
    /// # Errors
    ///
    /// Returns [`InputError::NonPositiveValue`] if `asset_value <= 0` or
    /// `debt_barrier <= 0`, and [`InputError::NegativeValue`] if `asset_vol < 0`.
    pub fn new_with_dynamics(
        asset_value: f64,
        asset_vol: f64,
        debt_barrier: f64,
        risk_free_rate: f64,
        payout_rate: f64,
        barrier_type: BarrierType,
        dynamics: AssetDynamics,
    ) -> Result<Self> {
        if asset_value <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        if asset_vol < 0.0 {
            return Err(InputError::NegativeValue.into());
        }
        if debt_barrier <= 0.0 {
            return Err(InputError::NonPositiveValue.into());
        }
        Ok(Self {
            asset_value,
            asset_vol,
            debt_barrier,
            risk_free_rate,
            payout_rate,
            barrier_type,
            dynamics,
        })
    }

    /// Distance-to-default over the given horizon.
    ///
    /// DD = (ln(V/B) + (r - q - sigma^2/2) * T) / (sigma * sqrt(T))
    ///
    /// A higher DD indicates a lower probability of default.
    #[inline]
    pub fn distance_to_default(&self, horizon: f64) -> f64 {
        let sigma = self.asset_vol;
        let mu = self.risk_free_rate - self.payout_rate - 0.5 * sigma * sigma;
        let sqrt_t = horizon.sqrt();
        ((self.asset_value / self.debt_barrier).ln() + mu * horizon) / (sigma * sqrt_t)
    }

    /// Default probability over the given horizon.
    ///
    /// - **Terminal barrier**: PD = N(-DD) (Merton 1974).
    /// - **First-passage barrier**: Black-Cox (1976) closed-form with
    ///   exponentially growing barrier at rate `g`:
    ///
    ///   Let mu = r - q - sigma^2/2, H = B * exp(g * T).
    ///   d_plus  = (ln(V/H) + mu * T) / (sigma * sqrt(T))
    ///   d_minus = (ln(V/H) - mu * T) / (sigma * sqrt(T))
    ///   PD = N(-d_plus) + (V/H)^(-2*mu/sigma^2) * N(d_minus)
    pub fn default_probability(&self, horizon: f64) -> f64 {
        match self.barrier_type {
            BarrierType::Terminal => {
                let dd = self.distance_to_default(horizon);
                norm_cdf(-dd)
            }
            BarrierType::FirstPassage {
                barrier_growth_rate,
            } => {
                let sigma = self.asset_vol;
                let mu = self.risk_free_rate - self.payout_rate - 0.5 * sigma * sigma;
                let sqrt_t = horizon.sqrt();
                let sigma_sqrt_t = sigma * sqrt_t;

                // Barrier at horizon: H = B * exp(g * T)
                let h = self.debt_barrier * (barrier_growth_rate * horizon).exp();
                let log_v_h = (self.asset_value / h).ln();

                let d_plus = (log_v_h + mu * horizon) / sigma_sqrt_t;
                let d_minus = (log_v_h - mu * horizon) / sigma_sqrt_t;

                // (V/H)^(-2*mu/sigma^2)
                let exponent = -2.0 * mu / (sigma * sigma);
                let ratio_term = (self.asset_value / h).powf(exponent);

                norm_cdf(-d_plus) + ratio_term * norm_cdf(d_minus)
            }
        }
    }

    /// Implied credit spread from default probability and recovery rate.
    ///
    /// s = -ln(1 - PD * (1 - R)) / T
    ///
    /// where PD is the default probability over horizon T and R is the
    /// assumed recovery rate (fraction of face value recovered at default).
    #[inline]
    pub fn implied_spread(&self, horizon: f64, recovery: f64) -> f64 {
        let pd = self.default_probability(horizon);
        let lgd = 1.0 - recovery;
        -(1.0 - pd * lgd).ln() / horizon
    }

    // -----------------------------------------------------------------------
    // Accessors
    // -----------------------------------------------------------------------

    /// Current asset value V_0.
    #[inline]
    pub fn asset_value(&self) -> f64 {
        self.asset_value
    }

    /// Asset volatility sigma_V.
    #[inline]
    pub fn asset_vol(&self) -> f64 {
        self.asset_vol
    }

    /// Debt barrier B.
    #[inline]
    pub fn debt_barrier(&self) -> f64 {
        self.debt_barrier
    }

    /// Risk-free rate r.
    #[inline]
    pub fn risk_free_rate(&self) -> f64 {
        self.risk_free_rate
    }

    /// Payout rate q (dividend yield).
    #[inline]
    pub fn payout_rate(&self) -> f64 {
        self.payout_rate
    }

    /// Barrier monitoring type.
    #[inline]
    pub fn barrier_type(&self) -> &BarrierType {
        &self.barrier_type
    }

    /// Asset dynamics specification.
    #[inline]
    pub fn dynamics(&self) -> &AssetDynamics {
        &self.dynamics
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn dd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let dd = m.distance_to_default(1.0);
        // DD = (ln(100/80) + (0.05 - 0 - 0.02)*1) / (0.2*1) = (0.22314 + 0.03) / 0.2 = 1.2657
        assert!((dd - 1.2657).abs() < 0.01, "DD={dd}");
    }

    #[test]
    fn pd_textbook_values() {
        let m = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        // PD = N(-1.2657) ~ 0.1028
        assert!((pd - 0.1028).abs() < 0.01, "PD={pd}");
    }

    #[test]
    fn zero_vol_means_no_default_when_solvent() {
        let m = MertonModel::new(100.0, 1e-10, 80.0, 0.05).unwrap();
        let pd = m.default_probability(1.0);
        assert!(pd < 1e-6, "Zero vol, solvent -> PD~0, got {pd}");
    }

    #[test]
    fn pd_increases_with_vol() {
        let m_low = MertonModel::new(100.0, 0.10, 80.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.40, 80.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn pd_increases_with_leverage() {
        let m_low = MertonModel::new(100.0, 0.20, 60.0, 0.05).unwrap();
        let m_high = MertonModel::new(100.0, 0.20, 95.0, 0.05).unwrap();
        assert!(m_high.default_probability(1.0) > m_low.default_probability(1.0));
    }

    #[test]
    fn first_passage_pd_higher_than_terminal() {
        let m_term = MertonModel::new(100.0, 0.20, 80.0, 0.05).unwrap();
        let m_fp = MertonModel::new_with_dynamics(
            100.0,
            0.20,
            80.0,
            0.05,
            0.0,
            BarrierType::FirstPassage {
                barrier_growth_rate: 0.05,
            },
            AssetDynamics::GeometricBrownian,
        )
        .unwrap();
        assert!(
            m_fp.default_probability(5.0) > m_term.default_probability(5.0),
            "First-passage PD should be higher than terminal PD"
        );
    }

    #[test]
    fn implied_spread_positive_for_risky_firm() {
        let m = MertonModel::new(100.0, 0.25, 80.0, 0.04).unwrap();
        let spread = m.implied_spread(5.0, 0.40);
        assert!(spread > 0.0, "Spread should be positive");
        assert!(spread < 0.20, "Spread should be reasonable, got {spread}");
    }

    #[test]
    fn new_rejects_invalid_inputs() {
        assert!(MertonModel::new(0.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(-1.0, 0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, -0.20, 80.0, 0.05).is_err());
        assert!(MertonModel::new(100.0, 0.20, 0.0, 0.05).is_err());
    }
}
