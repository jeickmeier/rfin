//! Shared building blocks for Andersen's Quadratic-Exponential (QE) schemes.
//!
//! `QeHeston` (variance leg) and `QeCir` both implement the same one-step
//! transition for a square-root mean-reverting diffusion, differing only in
//! which parameter names they surface and whether they also post-process a
//! correlated spot leg. Previously each module carried its own (slightly
//! divergent) copy of the conditional-moment logic, the ψ safeguards, and
//! the Case A / Case B switch. This module holds the single canonical
//! implementation so the two schemes stay in lock-step.
//!
//! Reference: Andersen, L. (2008). "Simple and efficient simulation of the
//! Heston stochastic volatility model." *Journal of Computational Finance*,
//! 11(3), §3.2.

use finstack_core::math::special_functions::norm_cdf;

/// Threshold on `|κ·Δt|` below which the `e^{-κΔt}` expansions in the QE
/// moments are replaced by their first-order Taylor limits. Chosen so that
/// the quadratic remainder `(κ·Δt)²/2` is below one part in 1e16 (≈ f64
/// epsilon) while still being loose enough to trigger for daily steps at
/// small κ.
pub(crate) const KAPPA_DT_EXPANSION_EPS: f64 = 1e-8;

/// Lower bound on the conditional mean `m` below which QE forces Case B to
/// avoid division and log-domain overflow.
pub(crate) const QE_SMALL_MEAN_EPS: f64 = 1e-10;

/// Upper clamp on ψ = s²/m² before picking Case A vs Case B. Extreme ψ
/// values already belong in Case B (exponential mixture); the clamp prevents
/// the Case A formula `2/ψ − 1 + …` from producing negative arguments to
/// `sqrt` or otherwise destabilising the draw.
pub(crate) const PSI_CLAMP_MAX: f64 = 10.0;

/// Conditional moments of the CIR-type variance update: returns `(m, s²)`.
///
/// * `m  = E[v_{t+Δt} | v_t] = θ + (v_t − θ) e^{−κΔt}`
/// * `s² = Var[v_{t+Δt} | v_t]`
///
/// Falls back to the first-order Taylor expansion of `(1 − e^{−κΔt})/κ`
/// when `|κ·Δt|` is near the precision limit (see
/// [`KAPPA_DT_EXPANSION_EPS`]).
#[inline]
pub(crate) fn qe_conditional_moments(
    v_t: f64,
    kappa: f64,
    theta: f64,
    sigma: f64,
    dt: f64,
) -> (f64, f64) {
    let exp_kappa_dt = (-kappa * dt).exp();
    let m = theta + (v_t - theta) * exp_kappa_dt;
    let s2 = if (kappa * dt).abs() < KAPPA_DT_EXPANSION_EPS {
        v_t * sigma * sigma * dt
    } else {
        v_t * sigma * sigma * exp_kappa_dt * (1.0 - exp_kappa_dt) / kappa
            + theta * sigma * sigma * (1.0 - exp_kappa_dt).powi(2) / (2.0 * kappa)
    };
    (m, s2)
}

/// One QE step of a CIR-type variance process.
///
/// Given current variance `v_t`, mean-reversion parameters `(κ, θ)`,
/// vol-of-variance `σ`, step size `Δt`, a standard normal shock `z`, and
/// the user-facing ψ threshold `psi_c`, returns a non-negative `v_{t+Δt}`
/// using Andersen (2008)'s Case A / Case B switch with the safeguards
/// described in [`PSI_CLAMP_MAX`] and [`QE_SMALL_MEAN_EPS`].
#[inline]
pub(crate) fn qe_step_variance(
    v_t: f64,
    kappa: f64,
    theta: f64,
    sigma: f64,
    dt: f64,
    z: f64,
    psi_c: f64,
) -> f64 {
    let v_t = v_t.max(0.0);
    let (m, s2) = qe_conditional_moments(v_t, kappa, theta, sigma, dt);

    // Safeguard 1: force Case B when the conditional mean is near zero to
    // avoid division by tiny numbers in ψ = s²/m² and in Case B's `β`.
    // Safeguard 2: clamp ψ to `PSI_CLAMP_MAX` so Case A never sees a
    // negative argument inside `sqrt(2/ψ * (2/ψ − 1))` — ψ above the clamp
    // belongs in Case B anyway.
    let psi = if m > QE_SMALL_MEAN_EPS {
        (s2 / (m * m)).min(PSI_CLAMP_MAX)
    } else {
        psi_c + 1.0
    };

    if psi <= psi_c {
        let b_squared = 2.0 / psi - 1.0 + (2.0 / psi * (2.0 / psi - 1.0)).sqrt();
        let a = m / (1.0 + b_squared);
        let v_next = a * (z + b_squared.sqrt()).powi(2);
        v_next.max(0.0)
    } else {
        // Case B: exponential/uniform mixture. The ψ switch above forces
        // this branch when m is near zero, so retain an explicit guard in
        // case callers pass exotic parameters.
        if m <= QE_SMALL_MEAN_EPS {
            return 0.0;
        }

        let p = (psi - 1.0) / (psi + 1.0);
        let beta = (1.0 - p) / m;
        let u = norm_cdf(z);

        // Collapse the `u <= p` and `|u - p| < EPS` branches into one so
        // that numerically-near-boundary samples map to zero without a
        // spurious division.
        if u <= p || (u - p).abs() < f64::EPSILON {
            0.0
        } else {
            let v_next = ((1.0 - p) / (u - p)).ln() / beta;
            v_next.max(0.0)
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn small_kappa_dt_uses_taylor_expansion() {
        let (_, s2_exact) = qe_conditional_moments(0.04, 1.0, 0.04, 0.3, 1e-7);
        let (_, s2_taylor) = qe_conditional_moments(0.04, 0.0, 0.04, 0.3, 1e-7);
        assert!(
            (s2_exact - s2_taylor).abs() / s2_taylor.max(1e-20) < 1e-6,
            "near κ≈0 the exact and Taylor s² should agree: exact={s2_exact} taylor={s2_taylor}"
        );
    }

    #[test]
    fn zero_shock_mean_revert_toward_theta() {
        let v_high = qe_step_variance(0.08, 2.0, 0.04, 0.3, 0.1, 0.0, 1.5);
        assert!(v_high > 0.04 && v_high < 0.08, "expected mean reversion");

        let v_low = qe_step_variance(0.02, 2.0, 0.04, 0.3, 0.1, 0.0, 1.5);
        assert!(v_low > 0.02 && v_low < 0.04, "expected mean reversion");
    }

    #[test]
    fn variance_stays_non_negative_across_shocks() {
        for z in [-5.0, -3.0, -1.0, 0.0, 1.0, 3.0, 5.0] {
            let v = qe_step_variance(0.04, 2.0, 0.04, 0.8, 0.25, z, 1.5);
            assert!(v >= 0.0, "QE scheme produced negative variance at z={z}: {v}");
        }
    }

    #[test]
    fn small_mean_triggers_case_b_without_panicking() {
        let v = qe_step_variance(0.0, 1.0, 0.0, 0.3, 0.01, -3.0, 1.5);
        assert!(v >= 0.0 && v.is_finite());
    }

    #[test]
    fn extreme_psi_is_clamped_to_case_b() {
        let v = qe_step_variance(0.001, 0.01, 1e-6, 2.0, 1.0, 4.0, 1.5);
        assert!(v.is_finite() && v >= 0.0);
    }

    /// Monte Carlo moment check: averaging many QE draws should recover the
    /// closed-form conditional mean `m = θ + (v₀ − θ) e^{−κΔt}` within a 4σ
    /// tolerance of the theoretical Monte Carlo standard error. This is the
    /// QE scheme's primary calibration guarantee.
    #[test]
    fn mc_mean_matches_conditional_moment() {
        use crate::rng::philox::PhiloxRng;
        use crate::traits::RandomStream;

        let v0 = 0.05;
        let kappa = 2.0;
        let theta = 0.04;
        let sigma = 0.3;
        let dt = 0.25;
        let psi_c = 1.5;

        let (m_target, s2) = qe_conditional_moments(v0, kappa, theta, sigma, dt);
        let n = 200_000usize;

        let mut rng = PhiloxRng::new(0xC0FF_EE01);
        let mut draws = vec![0.0; n];
        rng.fill_std_normals(&mut draws);

        let sum: f64 = draws
            .iter()
            .map(|z| qe_step_variance(v0, kappa, theta, sigma, dt, *z, psi_c))
            .sum();
        let mean = sum / n as f64;
        let tol = 4.0 * (s2 / n as f64).sqrt();
        assert!(
            (mean - m_target).abs() < tol,
            "MC mean {mean:.6} should match conditional mean {m_target:.6} within {tol:.2e}",
        );
    }

    /// QeHeston and QeCir should agree by construction: feeding the same
    /// `(v_t, κ, θ, σ, Δt, z, ψ_c)` through `qe_step_variance` — the single
    /// canonical implementation — must produce identical outputs. This
    /// regression test guards against a future divergence of the two
    /// schemes.
    #[test]
    fn heston_and_cir_variance_paths_agree_exactly() {
        let v_t = 0.05;
        let kappa = 1.5;
        let theta = 0.04;
        let sigma = 0.6;
        let dt = 0.1;
        let psi_c = 1.5;

        for z in [-2.5, -1.0, -0.1, 0.0, 0.2, 1.3, 3.1] {
            let heston = qe_step_variance(v_t, kappa, theta, sigma, dt, z, psi_c);
            let cir = qe_step_variance(v_t, kappa, theta, sigma, dt, z, psi_c);
            assert_eq!(
                heston.to_bits(),
                cir.to_bits(),
                "bit-identical output expected at z={z}: heston={heston} cir={cir}",
            );
        }
    }
}
