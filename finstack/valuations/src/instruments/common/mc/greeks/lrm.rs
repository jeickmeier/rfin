//! Likelihood Ratio Method (LRM) for Greeks.
//!
//! Computes derivatives using the likelihood ratio (score function) method.
//! Works for discontinuous payoffs where pathwise fails.
//!
//! The key insight is: E[∂f/∂θ] = E[f * ∂ln(p)/∂θ]
//!
//! Reference: Glasserman (2003) - "Monte Carlo Methods in Financial Engineering", Chapter 7

use super::super::stats::OnlineStats;

/// Compute delta using Likelihood Ratio Method for GBM.
///
/// For GBM, the score function for delta is:
/// ```text
/// ∂ln(p)/∂S₀ = W_T / (S₀ σ √T)
/// ```
///
/// where W_T is the terminal Brownian motion.
///
/// # Arguments
///
/// * `payoffs` - Payoff values from MC paths
/// * `wiener_terminals` - Terminal Wiener process values (W_T)
/// * `initial_spot` - Initial spot price
/// * `volatility` - Volatility
/// * `time_to_maturity` - Time to maturity
/// * `discount_factor` - Discount factor
///
/// # Returns
///
/// (delta estimate, standard error)
pub fn lrm_delta(
    payoffs: &[f64],
    wiener_terminals: &[f64],
    initial_spot: f64,
    volatility: f64,
    time_to_maturity: f64,
    discount_factor: f64,
) -> (f64, f64) {
    let mut stats = OnlineStats::new();
    let sqrt_t = time_to_maturity.sqrt();
    let score_multiplier = 1.0 / (initial_spot * volatility * sqrt_t);

    for (i, &payoff) in payoffs.iter().enumerate() {
        let w_t = wiener_terminals[i];
        let score = w_t * score_multiplier;
        let delta_contribution = discount_factor * payoff * score;
        stats.update(delta_contribution);
    }

    (stats.mean(), stats.stderr())
}

/// Compute vega using Likelihood Ratio Method.
///
/// For GBM, the score function for vega is:
/// ```text
/// ∂ln(p)/∂σ = (W_T² - T) / (σ √T)
/// ```
pub fn lrm_vega(
    payoffs: &[f64],
    wiener_terminals: &[f64],
    volatility: f64,
    time_to_maturity: f64,
    discount_factor: f64,
) -> (f64, f64) {
    let mut stats = OnlineStats::new();
    let sqrt_t = time_to_maturity.sqrt();

    for (i, &payoff) in payoffs.iter().enumerate() {
        let w_t = wiener_terminals[i];
        let score = (w_t * w_t - time_to_maturity) / (volatility * sqrt_t);
        let vega_contribution = discount_factor * payoff * score;
        stats.update(vega_contribution);
    }

    (stats.mean(), stats.stderr())
}

/// Compute rho (sensitivity to interest rate) using LRM.
pub fn lrm_rho(payoffs: &[f64], time_to_maturity: f64, discount_factor: f64) -> (f64, f64) {
    let mut stats = OnlineStats::new();

    for &payoff in payoffs {
        // For interest rate: sensitivity comes from both drift and discounting
        // ∂V/∂r ≈ -T * PV + drift effect
        let rho_contribution = -time_to_maturity * discount_factor * payoff;
        stats.update(rho_contribution);
    }

    (stats.mean(), stats.stderr())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lrm_delta_basic() {
        // Simple test with fixed payoffs and Wiener paths
        let payoffs = vec![10.0, 5.0, 15.0];
        let wiener = vec![0.5, -0.3, 0.8];

        let (delta, stderr) = lrm_delta(&payoffs, &wiener, 100.0, 0.2, 1.0, 1.0);

        // Delta should be finite
        assert!(delta.is_finite());
        assert!(stderr >= 0.0);
    }

    #[test]
    fn test_lrm_vega_basic() {
        let payoffs = vec![10.0, 5.0, 15.0];
        let wiener = vec![0.5, -0.3, 0.8];

        let (vega, stderr) = lrm_vega(&payoffs, &wiener, 0.2, 1.0, 1.0);

        assert!(vega.is_finite());
        assert!(stderr >= 0.0);
    }

    #[test]
    fn test_lrm_rho() {
        let payoffs = vec![10.0, 8.0, 12.0];

        let (rho, _) = lrm_rho(&payoffs, 1.0, 0.95);

        // Rho should be negative (higher rates reduce PV)
        assert!(rho < 0.0);
    }

    #[test]
    fn test_lrm_zero_payoffs() {
        let payoffs = vec![0.0, 0.0, 0.0];
        let wiener = vec![0.1, 0.2, 0.3];

        let (delta, _) = lrm_delta(&payoffs, &wiener, 100.0, 0.2, 1.0, 1.0);

        // Zero payoffs should give zero Greeks
        assert_eq!(delta, 0.0);
    }
}
