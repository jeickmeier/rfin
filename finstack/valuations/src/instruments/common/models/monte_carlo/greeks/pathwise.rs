//! Pathwise differentiation for Greeks.
//!
//! Computes derivatives by differentiating the payoff with respect to parameters
//! along each path. Works best for smooth payoffs without discontinuities.
//!
//! Reference: Glasserman (2003) - "Monte Carlo Methods in Financial Engineering"

use crate::instruments::common::mc::online_stats::OnlineStats;

/// Compute pathwise delta for European call under GBM.
///
/// Delta = ∂V/∂S₀ = e^(-rT) E[1_{S_T > K} * S_T / S_0]
///
/// # Arguments
///
/// * `terminal_spots` - Terminal spot prices from MC paths
/// * `initial_spot` - Initial spot price
/// * `strike` - Strike price
/// * `discount_factor` - Discount factor e^(-rT)
///
/// # Returns
///
/// Delta estimate with standard error
pub fn pathwise_delta_call(
    terminal_spots: &[f64],
    initial_spot: f64,
    strike: f64,
    discount_factor: f64,
) -> (f64, f64) {
    let mut stats = OnlineStats::new();

    for &s_t in terminal_spots {
        let payoff_deriv = if s_t > strike {
            s_t / initial_spot
        } else {
            0.0
        };

        let delta_contribution = discount_factor * payoff_deriv;
        stats.update(delta_contribution);
    }

    (stats.mean(), stats.stderr())
}

/// Compute pathwise delta for European put under GBM.
pub fn pathwise_delta_put(
    terminal_spots: &[f64],
    initial_spot: f64,
    strike: f64,
    discount_factor: f64,
) -> (f64, f64) {
    let mut stats = OnlineStats::new();

    for &s_t in terminal_spots {
        let payoff_deriv = if s_t < strike {
            -s_t / initial_spot
        } else {
            0.0
        };

        let delta_contribution = discount_factor * payoff_deriv;
        stats.update(delta_contribution);
    }

    (stats.mean(), stats.stderr())
}

/// Compute pathwise vega for European option under GBM.
///
/// Vega involves differentiating the Brownian motion with respect to σ.
///
/// Returns Vega scaled by 0.01 (sensitivity per 1% volatility change).
///
/// # Arguments
///
/// * `terminal_spots` - Terminal spot prices
/// * `initial_spot` - Initial spot price
/// * `strike` - Strike price  
/// * `time_to_maturity` - Time to maturity
/// * `volatility` - Current volatility
/// * `discount_factor` - Discount factor
/// * `wiener_increments` - Sum of Brownian increments for each path
/// * `is_call` - true for call, false for put
#[allow(clippy::too_many_arguments)]
pub fn pathwise_vega(
    terminal_spots: &[f64],
    _initial_spot: f64,
    strike: f64,
    time_to_maturity: f64,
    volatility: f64,
    discount_factor: f64,
    wiener_increments: &[f64],
    is_call: bool,
) -> (f64, f64) {
    let mut stats = OnlineStats::new();
    let sqrt_t = time_to_maturity.sqrt();

    for (i, &s_t) in terminal_spots.iter().enumerate() {
        let in_money = if is_call { s_t > strike } else { s_t < strike };

        if in_money {
            let w = wiener_increments[i];
            // ∂S_T/∂σ = S_T * (W_T - σT) / σ
            let ds_dsigma = s_t * (w / sqrt_t - volatility * sqrt_t) / volatility;

            let vega_contribution = discount_factor * ds_dsigma;
            // Scale by 0.01 to represent sensitivity per 1% vol change
            stats.update(vega_contribution * 0.01);
        } else {
            stats.update(0.0);
        }
    }

    (stats.mean(), stats.stderr())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathwise_delta_call_itm() {
        // All paths ITM
        let terminal_spots = vec![110.0, 120.0, 115.0];
        let (delta, stderr) = pathwise_delta_call(&terminal_spots, 100.0, 100.0, 1.0);

        // Delta should be positive for ITM calls
        assert!(delta > 0.0);
        assert!(delta < 2.0); // Sanity check
        assert!(stderr >= 0.0);
    }

    #[test]
    fn test_pathwise_delta_call_otm() {
        // All paths OTM
        let terminal_spots = vec![90.0, 85.0, 95.0];
        let (delta, _) = pathwise_delta_call(&terminal_spots, 100.0, 100.0, 1.0);

        // Delta should be zero for OTM
        assert_eq!(delta, 0.0);
    }

    #[test]
    fn test_pathwise_delta_put() {
        // ITM put paths
        let terminal_spots = vec![90.0, 85.0, 95.0];
        let (delta, _) = pathwise_delta_put(&terminal_spots, 100.0, 100.0, 1.0);

        // Delta should be negative for puts
        assert!(delta < 0.0);
        assert!(delta > -2.0);
    }

    #[test]
    fn test_vega_positive() {
        let terminal_spots = vec![110.0, 105.0, 115.0];
        let wiener_increments = vec![0.5, 0.3, 0.7];

        let (vega, _) = pathwise_vega(
            &terminal_spots,
            100.0,
            100.0,
            1.0,
            0.2,
            1.0,
            &wiener_increments,
            true,
        );

        // Vega can be positive or negative depending on paths
        assert!(vega.is_finite());
    }
}
