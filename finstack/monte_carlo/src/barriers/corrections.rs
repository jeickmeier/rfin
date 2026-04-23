//! Barrier adjustment corrections.
//!
//! Implements the Broadie–Glasserman–Kou (1997) continuity correction which
//! adjusts the barrier level to reduce discretization bias when a continuous
//! barrier is monitored discretely. Gobet & Miri (2001) later generalized the
//! same shift to local-volatility models.

/// Continuity-correction coefficient `β = −ζ(1/2)/√(2π)`.
///
/// Under continuous monitoring with equal time steps, shifting the barrier by
/// `β · σ · √Δt` toward the spot makes the discretely-monitored price match
/// the continuous price up to `o(1/√n)` error in the number of monitoring
/// points.
///
/// The adjusted barrier is `B' = B · exp(±β · σ · √Δt)`, where the sign
/// shifts the barrier toward spot.
///
/// # Numerical value
///
/// Using `ζ(1/2) = -1.4603545088095868…` and `√(2π) = 2.5066282746310002…`
/// gives `β ≈ 0.5825971579390106`, i.e. full f64 precision. Previously this
/// constant was rounded to 4 decimal digits (`0.5826`), introducing a
/// systematic bias of a few parts per 10⁴ in every barrier shift. The extra
/// digits cost nothing at runtime and align the implementation with the
/// published formula.
///
/// References:
/// - Broadie, Glasserman & Kou (1997). "A Continuity Correction for Discrete
///   Barrier Options." *Mathematical Finance*, 7(4), 325–349.
/// - Gobet & Miri (2001). "Weak approximation of averaged diffusion
///   processes" (extension to local-volatility models; same leading
///   coefficient β).
pub const GOBET_MIRI_BETA: f64 = 0.582_597_157_939_010_6;

/// Apply the Broadie–Glasserman–Kou (1997) / Gobet–Miri barrier shift.
///
/// Adjusts the barrier level to reduce discretization bias when monitoring is
/// discrete. Named for historical reasons; the leading coefficient is from
/// Broadie–Glasserman–Kou.
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
/// - Down barrier: B' = B * exp(+β * σ * √Δt)  (shift up toward spot)
/// - Up barrier: B' = B * exp(-β * σ * √Δt)  (shift down toward spot)
///
/// The shift moves the barrier *toward* spot to compensate for the fact
/// that discrete monitoring misses some barrier crossings between steps.
pub fn gobet_miri_adjusted_barrier(
    barrier: f64,
    sigma: f64,
    dt: f64,
    is_down_barrier: bool,
) -> f64 {
    let shift = GOBET_MIRI_BETA * sigma * dt.sqrt();

    if is_down_barrier {
        barrier * shift.exp()
    } else {
        barrier * (-shift).exp()
    }
}

/// Alternative barrier adjustment using the "half-step" method.
///
/// This simpler method shifts the barrier by approximately 0.5 * σ * √Δt.
#[must_use]
pub fn half_step_adjusted_barrier(barrier: f64, sigma: f64, dt: f64, is_down_barrier: bool) -> f64 {
    let shift = 0.5 * sigma * dt.sqrt();

    if is_down_barrier {
        barrier * shift.exp()
    } else {
        barrier * (-shift).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gobet_miri_down_barrier() {
        let barrier = 100.0;
        let sigma = 0.2;
        let dt = 1.0 / 252.0; // Daily monitoring

        let adjusted = gobet_miri_adjusted_barrier(barrier, sigma, dt, true);

        // Down barrier shifts UP toward spot
        assert!(adjusted > barrier);

        // Shift should be small for small dt
        assert!((adjusted / barrier - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_gobet_miri_up_barrier() {
        let barrier = 100.0;
        let sigma = 0.2;
        let dt = 1.0 / 252.0;

        let adjusted = gobet_miri_adjusted_barrier(barrier, sigma, dt, false);

        // Up barrier shifts DOWN toward spot
        assert!(adjusted < barrier);
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

        // Both should shift up (toward spot) for down barrier
        assert!(gm_down > barrier);
        assert!(hs_down > barrier);

        // Should be similar magnitude (GobetMiri beta ~0.58, half-step=0.5)
        assert!((gm_down - hs_down).abs() < 0.2);
    }

    #[test]
    fn test_adjustment_scales_with_dt() {
        let barrier = 100.0;
        let sigma = 0.2;

        let adj_small_dt = gobet_miri_adjusted_barrier(barrier, sigma, 0.01, true);
        let adj_large_dt = gobet_miri_adjusted_barrier(barrier, sigma, 0.1, true);

        // Larger dt should give larger adjustment (further from original barrier)
        assert!(adj_large_dt - barrier > adj_small_dt - barrier);
    }
}
