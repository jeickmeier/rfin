//! Halton-sequence multi-start scaffolding for calibration solvers.
//!
//! Extracted from `solver::global` so that model-level calibrators (Hull-White,
//! LMM, SABR, …) can reuse the same deterministic low-discrepancy perturbation
//! strategy without duplicating the logic. Lives behind `pub(crate)` because
//! only other calibration modules should consume it; external callers should
//! go through the public `SolverConfig` surface.
//!
//! # Determinism
//!
//! The Halton sequence is fully deterministic given the prime base and
//! restart index, so two runs of a calibrator with the same initial guess,
//! the same `MultiStartConfig`, and the same objective will produce bit-
//! identical perturbed starting points. No RNG is involved.
//!
//! # References
//!
//! - Halton, J. H. (1960). "On the efficiency of certain quasi-random
//!   sequences of points in evaluating multi-dimensional integrals."
//!   *Numerische Mathematik* 2(1), 84–90.
//! - Gilli, Maringer, & Schumann (2011). *Numerical Methods and Optimization
//!   in Finance*. §12.5 (global optimization), §13.4 (multi-start for
//!   calibration).

/// Configuration for multi-start optimization to escape local minima.
///
/// When enabled, the optimizer runs `num_restarts` additional solves from
/// deterministically-perturbed starting points and keeps the result with
/// the lowest weighted residual norm.
#[derive(Debug, Clone)]
pub(crate) struct MultiStartConfig {
    /// Number of additional restarts beyond the initial point.
    pub(crate) num_restarts: usize,
    /// Perturbation magnitude as a fraction of the initial parameter
    /// values, applied via `x' = x · (1 + scale · (2·h − 1))` where
    /// `h ∈ [0, 1)` is a Halton draw.
    pub(crate) perturbation_scale: f64,
}

impl Default for MultiStartConfig {
    fn default() -> Self {
        Self {
            num_restarts: 5,
            perturbation_scale: 0.5,
        }
    }
}

/// Prime bases used when assigning a Halton stream per parameter dimension.
///
/// For dimension `i` we use `BASES[i % BASES.len()]`. Ten bases is enough
/// for any realistic calibration problem in this workspace (largest seen:
/// ~8-parameter multi-factor LMM).
pub(crate) const HALTON_BASES: [usize; 10] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29];

/// Halton sequence value for index `n` with the given prime `base`.
///
/// Returns a value in `[0, 1)`. The sequence is low-discrepancy, meaning
/// the first `N` points cover `[0, 1)` more evenly than `N` independent
/// uniform samples — which is exactly what multi-start wants.
#[inline]
pub(crate) fn halton(mut n: usize, base: usize) -> f64 {
    debug_assert!(base >= 2, "Halton base must be >= 2");
    let mut result = 0.0_f64;
    let mut f = 1.0_f64 / base as f64;
    while n > 0 {
        result += f * (n % base) as f64;
        n /= base;
        f /= base as f64;
    }
    result
}

/// Produce a deterministic multiplicative perturbation of `initials` for the
/// given `restart_idx` (≥ 0). Each parameter gets a separate Halton stream
/// so that restarts do not cluster along coordinate axes.
///
/// The perturbed vector is clamped elementwise to `[lb, ub]` when those
/// bounds are supplied; dimensions beyond the bound vector's length are
/// left unclamped. Passing both as `None` means no bound enforcement.
///
/// # Panics
///
/// Does not panic. Invariants are asserted in debug builds only.
pub(crate) fn perturb_initial_guess(
    initials: &[f64],
    perturbation_scale: f64,
    restart_idx: usize,
    lb: Option<&[f64]>,
    ub: Option<&[f64]>,
) -> Vec<f64> {
    initials
        .iter()
        .enumerate()
        .map(|(i, &x)| {
            let base = HALTON_BASES[i % HALTON_BASES.len()];
            let h = halton(restart_idx + 1, base);
            let perturbation = perturbation_scale * (2.0 * h - 1.0);
            let mut v = x * (1.0 + perturbation);
            if let Some(lower) = lb {
                if i < lower.len() {
                    v = v.max(lower[i]);
                }
            }
            if let Some(upper) = ub {
                if i < upper.len() {
                    v = v.min(upper[i]);
                }
            }
            v
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Halton values for base=2 are 1/2, 1/4, 3/4, 1/8, 5/8, 3/8, 7/8, …
    #[test]
    fn halton_base_2_reference_values() {
        let expected = [0.5, 0.25, 0.75, 0.125, 0.625, 0.375, 0.875];
        for (n, &want) in (1..=expected.len()).zip(expected.iter()) {
            let got = halton(n, 2);
            assert!(
                (got - want).abs() < 1e-15,
                "halton({n}, 2) = {got}, want {want}"
            );
        }
    }

    /// Halton values for base=3 are 1/3, 2/3, 1/9, 4/9, 7/9, …
    #[test]
    fn halton_base_3_reference_values() {
        let ninth = 1.0 / 9.0;
        let expected = [
            1.0 / 3.0,
            2.0 / 3.0,
            ninth,
            1.0 / 3.0 + ninth,
            2.0 / 3.0 + ninth,
            2.0 * ninth,
        ];
        for (n, &want) in (1..=expected.len()).zip(expected.iter()) {
            let got = halton(n, 3);
            assert!(
                (got - want).abs() < 1e-15,
                "halton({n}, 3) = {got}, want {want}"
            );
        }
    }

    #[test]
    fn halton_is_in_unit_interval_for_many_bases_and_indices() {
        for &base in &HALTON_BASES {
            for n in 1..500 {
                let h = halton(n, base);
                assert!(
                    (0.0..1.0).contains(&h),
                    "halton({n}, {base}) = {h} out of [0,1)"
                );
            }
        }
    }

    #[test]
    fn halton_is_deterministic() {
        // Bit-identical outputs across two calls — the whole point of the
        // Halton multi-start approach.
        for &base in &HALTON_BASES {
            for n in 1..100 {
                let a = halton(n, base).to_bits();
                let b = halton(n, base).to_bits();
                assert_eq!(a, b);
            }
        }
    }

    #[test]
    fn perturbation_respects_lower_bound() {
        // Initial x = 0.03; with scale=0.5 and a Halton h close to 0, the
        // perturbed value is 0.03 * (1 + 0.5 * (0 - 1)) = 0.015, which
        // would be below the 0.02 floor — clamp must kick in.
        let initials = [0.03];
        let lb = [0.02];
        // restart_idx 0 → halton(1, 2) = 0.5 → perturbation = 0. Force a
        // restart index where halton(n, 2) is small: halton(3, 2) = 0.75
        // gives perturbation = +0.25. Instead use base-specific probe:
        // halton(5, 2) = 0.625 → +0.125; still above floor. Try halton(8, 2).
        //  halton(8, 2) = 0.0625 (n=8 in binary is 1000, reversed fraction
        //  0.0001 = 1/16 = 0.0625). perturbation = -0.4375 ⇒ x = 0.01687.
        //  Should clamp to 0.02.
        let perturbed = perturb_initial_guess(&initials, 0.5, 7, Some(&lb), None);
        assert!(
            perturbed[0] >= 0.02,
            "lower bound must be enforced, got {}",
            perturbed[0]
        );
    }

    #[test]
    fn perturbation_respects_upper_bound() {
        let initials = [0.03];
        let ub = [0.035];
        // Find a restart that would push the value above the bound.
        // halton(3, 2) = 0.75 → perturbation = +0.25 → x = 0.0375, clamps to 0.035.
        let perturbed = perturb_initial_guess(&initials, 0.5, 2, None, Some(&ub));
        assert!(
            perturbed[0] <= 0.035,
            "upper bound must be enforced, got {}",
            perturbed[0]
        );
    }

    #[test]
    fn perturbation_is_deterministic() {
        let initials = [0.03, 0.01, 0.20];
        let lb = [0.0, 0.0, 0.0];
        let ub = [1.0, 1.0, 1.0];
        for restart_idx in 0..10 {
            let a = perturb_initial_guess(&initials, 0.4, restart_idx, Some(&lb), Some(&ub));
            let b = perturb_initial_guess(&initials, 0.4, restart_idx, Some(&lb), Some(&ub));
            assert_eq!(a.len(), b.len());
            for (x, y) in a.iter().zip(b.iter()) {
                assert_eq!(x.to_bits(), y.to_bits());
            }
        }
    }

    #[test]
    fn perturbation_uses_different_streams_across_dimensions() {
        // With uniform initials, the perturbed values should differ per
        // dimension because each dimension uses a different prime base.
        let initials = vec![0.1_f64; 6];
        let perturbed = perturb_initial_guess(&initials, 0.5, 2, None, None);
        let first = perturbed[0];
        let all_equal = perturbed.iter().all(|&v| (v - first).abs() < 1e-18);
        assert!(
            !all_equal,
            "per-dimension Halton streams must decouple: got {perturbed:?}"
        );
    }

    #[test]
    fn multi_start_config_defaults_are_stable() {
        let cfg = MultiStartConfig::default();
        assert_eq!(cfg.num_restarts, 5);
        assert!((cfg.perturbation_scale - 0.5).abs() < 1e-15);
    }
}
