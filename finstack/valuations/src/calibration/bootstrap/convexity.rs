//! Convexity adjustment calculations for interest rate futures.
//! 
//! Provides market-standard convexity adjustments to convert futures rates 
//! to forward rates, accounting for the daily margining of futures contracts.

use finstack_core::F;

/// Calculate convexity adjustment for interest rate futures.
///
/// The convexity adjustment accounts for the difference between futures and forwards
/// due to daily mark-to-market settlement of futures contracts.
///
/// Formula: CA = 0.5 * σ² * T₁ * T₂
/// where:
/// - σ is the rate volatility
/// - T₁ is time to futures expiry
/// - T₂ is time to rate maturity
///
/// # Arguments
/// * `time_to_expiry` - Time to futures expiry in years
/// * `time_to_maturity` - Time to underlying rate maturity in years
/// * `rate_volatility` - Annualized rate volatility (e.g., 0.01 for 1%)
///
/// # Returns
/// Convexity adjustment to add to futures rate to get forward rate
pub fn calculate_convexity_adjustment(
    time_to_expiry: F,
    time_to_maturity: F,
    rate_volatility: F,
) -> F {
    // Hull-White approximation for convexity adjustment
    0.5 * rate_volatility * rate_volatility * time_to_expiry * time_to_maturity
}

/// Calculate implied rate volatility from market data.
///
/// Uses a simplified approach based on historical volatility patterns.
/// In practice, this would be calibrated from option prices.
///
/// # Arguments
/// * `tenor_years` - Tenor of the underlying rate in years
/// * `time_to_expiry` - Time to futures expiry in years
///
/// # Returns
/// Estimated annualized rate volatility
pub fn estimate_rate_volatility(tenor_years: F, time_to_expiry: F) -> F {
    // Base volatility depends on tenor
    let base_vol = if tenor_years <= 0.25 {
        0.0080  // 80bp for 3M rates
    } else if tenor_years <= 0.5 {
        0.0075  // 75bp for 6M rates
    } else {
        0.0070  // 70bp for longer tenors
    };

    // Volatility increases with time to expiry
    let time_adjustment = (1.0 + 0.1 * time_to_expiry).min(1.5);
    
    base_vol * time_adjustment
}

/// Calculate convexity adjustment using Ho-Lee model.
///
/// More sophisticated than Hull-White for long-dated futures.
///
/// # Arguments
/// * `time_to_expiry` - Time to futures expiry in years
/// * `time_to_maturity` - Time to underlying rate maturity in years
/// * `rate_volatility` - Annualized rate volatility
/// * `mean_reversion` - Mean reversion parameter (0 for Ho-Lee)
pub fn ho_lee_convexity(
    time_to_expiry: F,
    time_to_maturity: F,
    rate_volatility: F,
    mean_reversion: F,
) -> F {
    if mean_reversion.abs() < 1e-10 {
        // Ho-Lee model (no mean reversion)
        calculate_convexity_adjustment(time_to_expiry, time_to_maturity, rate_volatility)
    } else {
        // Hull-White with mean reversion
        let exp_neg_a_t1 = (-mean_reversion * time_to_expiry).exp();
        let exp_neg_a_t2 = (-mean_reversion * time_to_maturity).exp();
        
        let b_t1_t2 = (1.0 - exp_neg_a_t2 * exp_neg_a_t1.recip()) / mean_reversion;
        let variance = rate_volatility * rate_volatility 
            * (1.0 - exp_neg_a_t1 * exp_neg_a_t1) / (2.0 * mean_reversion);
        
        0.5 * variance * b_t1_t2 * b_t1_t2
    }
}

/// Convexity adjustment parameters for different currencies.
#[derive(Clone, Debug)]
pub struct ConvexityParameters {
    /// Base rate volatility
    pub base_volatility: F,
    /// Mean reversion parameter (0 for Ho-Lee)
    pub mean_reversion: F,
    /// Use Ho-Lee model instead of simple Hull-White
    pub use_ho_lee: bool,
}

impl ConvexityParameters {
    /// USD SOFR futures parameters
    pub fn usd_sofr() -> Self {
        Self {
            base_volatility: 0.0075,
            mean_reversion: 0.03,
            use_ho_lee: false,
        }
    }

    /// EUR EURIBOR futures parameters
    pub fn eur_euribor() -> Self {
        Self {
            base_volatility: 0.0070,
            mean_reversion: 0.025,
            use_ho_lee: false,
        }
    }

    /// GBP SONIA futures parameters
    pub fn gbp_sonia() -> Self {
        Self {
            base_volatility: 0.0080,
            mean_reversion: 0.035,
            use_ho_lee: false,
        }
    }

    /// JPY TONAR futures parameters
    pub fn jpy_tonar() -> Self {
        Self {
            base_volatility: 0.0040,
            mean_reversion: 0.02,
            use_ho_lee: true,  // Use Ho-Lee for JPY due to low rates
        }
    }

    /// Calculate adjustment for given times
    pub fn calculate_adjustment(&self, time_to_expiry: F, time_to_maturity: F) -> F {
        let volatility = self.base_volatility * (1.0 + 0.1 * time_to_expiry).min(1.5);
        
        if self.use_ho_lee {
            ho_lee_convexity(time_to_expiry, time_to_maturity, volatility, self.mean_reversion)
        } else {
            calculate_convexity_adjustment(time_to_expiry, time_to_maturity, volatility)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_convexity_adjustment() {
        // 1-year future on 3-month rate with 1% volatility
        let adj = calculate_convexity_adjustment(1.0, 1.25, 0.01);
        assert!((adj - 0.0000625).abs() < 1e-8);  // 0.625 bp
    }

    #[test]
    fn test_volatility_estimation() {
        let vol_3m = estimate_rate_volatility(0.25, 1.0);
        assert!((vol_3m - 0.0088).abs() < 1e-3);  // ~88bp
        
        let vol_6m = estimate_rate_volatility(0.5, 2.0);
        assert!((vol_6m - 0.00975).abs() < 1e-3);  // ~97.5bp  
    }

    #[test]
    fn test_currency_specific_parameters() {
        let usd_params = ConvexityParameters::usd_sofr();
        let adj = usd_params.calculate_adjustment(2.0, 2.25);
        assert!(adj > 0.0 && adj < 0.001);  // Reasonable range
    }

    #[test]
    fn test_ho_lee_model() {
        // With zero mean reversion, should match simple model
        let simple = calculate_convexity_adjustment(1.0, 1.25, 0.01);
        let ho_lee = ho_lee_convexity(1.0, 1.25, 0.01, 0.0);
        assert!((simple - ho_lee).abs() < 1e-10);
        
        // With mean reversion, should be smaller
        let with_mr = ho_lee_convexity(1.0, 1.25, 0.01, 0.03);
        assert!(with_mr < simple);
    }
}
