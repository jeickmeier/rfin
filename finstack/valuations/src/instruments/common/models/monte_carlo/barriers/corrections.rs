//! Barrier adjustment corrections.
//!
//! Implements the Gobet-Miri continuity correction which adjusts
//! the barrier level to reduce discretization bias.

/// Gobet-Miri barrier shift coefficient.
///
/// Reference: Gobet & Miri (2001) - "Weak approximation of averaged diffusion processes"
///
/// The adjusted barrier is: B' = B * exp(-β * σ * √Δt)
/// where β ≈ 0.5826 for optimal bias reduction.
pub const GOBET_MIRI_BETA: f64 = 0.5826;

/// Apply Gobet-Miri barrier shift.
///
/// Adjusts the barrier level to reduce discretization bias when
/// monitoring is discrete.
///
/// # Arguments
///
/// * `barrier` - Original barrier level
/// * `sigma` - Volatility
/// * `dt` - Time step
/// * `is_down_barrier` - true for down barrier, false for up barrier
///
/// # Returns
///
/// Adjusted barrier level
///
/// # Formula
///
/// - Down barrier: B' = B * exp(-β * σ * √Δt)
/// - Up barrier: B' = B * exp(+β * σ * √Δt)
///
/// The shift moves the barrier *away* from the current spot to compensate
/// for the fact that discrete monitoring misses some barrier hits.
pub fn gobet_miri_adjusted_barrier(
    barrier: f64,
    sigma: f64,
    dt: f64,
    is_down_barrier: bool,
) -> f64 {
    let shift = GOBET_MIRI_BETA * sigma * dt.sqrt();

    if is_down_barrier {
        // Down barrier: shift downward (lower barrier)
        barrier * (-shift).exp()
    } else {
        // Up barrier: shift upward (raise barrier)
        barrier * shift.exp()
    }
}

/// Alternative barrier adjustment using the "half-step" method.
///
/// This simpler method shifts the barrier by approximately 0.5 * σ * √Δt.
pub fn half_step_adjusted_barrier(barrier: f64, sigma: f64, dt: f64, is_down_barrier: bool) -> f64 {
    let shift = 0.5 * sigma * dt.sqrt();

    if is_down_barrier {
        barrier * (-shift).exp()
    } else {
        barrier * shift.exp()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_gobet_miri_down_barrier() {
        let barrier = 100.0;
        let sigma = 0.2;
        let dt = 1.0 / 252.0; // Daily monitoring

        let adjusted = gobet_miri_adjusted_barrier(barrier, sigma, dt, true);

        // Should be shifted down
        assert!(adjusted < barrier);

        // Shift should be small for small dt
        assert!((adjusted / barrier - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_gobet_miri_up_barrier() {
        let barrier = 100.0;
        let sigma = 0.2;
        let dt = 1.0 / 252.0;

        let adjusted = gobet_miri_adjusted_barrier(barrier, sigma, dt, false);

        // Should be shifted up
        assert!(adjusted > barrier);
    }

    #[test]
    fn test_gobet_miri_zero_vol() {
        // With zero volatility, no adjustment
        let barrier = 100.0;
        let adjusted = gobet_miri_adjusted_barrier(barrier, 0.0, 0.01, true);
        assert_eq!(adjusted, barrier);
    }

    #[test]
    fn test_gobet_miri_vs_half_step() {
        let barrier = 100.0;
        let sigma = 0.2;
        let dt = 1.0 / 252.0;

        let gm_down = gobet_miri_adjusted_barrier(barrier, sigma, dt, true);
        let hs_down = half_step_adjusted_barrier(barrier, sigma, dt, true);

        // Both should shift in same direction
        assert!(gm_down < barrier);
        assert!(hs_down < barrier);

        // Should be similar magnitude (GobetMiri beta ~0.58, half-step=0.5)
        assert!((gm_down - hs_down).abs() < 0.2);
    }

    #[test]
    fn test_adjustment_scales_with_dt() {
        let barrier = 100.0;
        let sigma = 0.2;

        let adj_small_dt = gobet_miri_adjusted_barrier(barrier, sigma, 0.01, true);
        let adj_large_dt = gobet_miri_adjusted_barrier(barrier, sigma, 0.1, true);

        // Larger dt should give larger adjustment
        assert!(barrier - adj_large_dt > barrier - adj_small_dt);
    }
}
