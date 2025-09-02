use crate::{
    market_data::{interp::InterpFn, utils::validate_knots},
    F,
};
use std::boxed::Box;
use std::vec::Vec;

/// Monotone-convex discount-factor interpolator (Hagan & West, 2006).
///
/// Implements the full Hagan–West slope-selecting, monotone-convex cubic
/// interpolation in natural-log discount-factor space.  The constructor
/// computes and stores per-segment cubic coefficients guaranteeing positivity,
/// monotonicity and convexity when the input curve is arbitrage-free
/// (non-increasing).  Evaluation is O(log N) due to binary search on the knot
/// vector.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct MonotoneConvex {
    /// Knot times _tᵢ_ (strictly increasing).  Length ≥ 2.
    knots: Box<[F]>,
    /// Original discount factors _P(tᵢ)_.  Same length as `knots`.
    dfs: Box<[F]>,
    /// Per-segment cubic coefficients (a,b,c,d) in ln-DF space.
    /// Computed using the Hagan-West monotone-convex algorithm for cubic
    /// interpolation with guaranteed positivity, monotonicity and convexity.
    coeffs: Box<[(F, F, F, F)]>,
}

impl MonotoneConvex {
    /// Construct a new monotone-convex interpolator.
    ///
    /// # Parameters
    /// * `knots` – strictly increasing times in the same units as evaluation
    ///             points (e.g. years).
    /// * `dfs`   – corresponding discount factors, positive and
    ///             non-increasing (arbitrage-free).
    ///
    /// # Errors
    /// * `InputError::TooFewPoints`        – fewer than two knots.
    /// * `InputError::NonMonotonicKnots`   – times not strictly increasing.
    /// * `InputError::NonPositiveValue`    – DF ≤ 0.
    /// * `InputError::Invalid`             – DF increases between knots.
    #[allow(clippy::boxed_local)]
    pub fn new(knots: Box<[F]>, dfs: Box<[F]>) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());

        // ---- Sanity checks -------------------------------------------------
        validate_knots(&knots)?;
        crate::market_data::utils::validate_dfs(&dfs, true)?;

        // Compute cubic coefficients **before** moving `knots` and `dfs` into
        // the struct to avoid partial move/borrow checker conflicts.
        let coeffs = Self::build_coeffs(&knots, &dfs);

        Ok(Self { knots, dfs, coeffs })
    }

    /// Compute cubic coefficients for each segment according to the
    /// Hagan–West monotone–convex algorithm.
    fn build_coeffs(knots: &[F], dfs: &[F]) -> Box<[(F, F, F, F)]> {
        let n = knots.len();
        debug_assert!(n >= 2);

        // Convert to continuously-compounded zero yields y = −ln P.
        let y: Vec<F> = dfs.iter().map(|&p| -p.ln()).collect();

        // Δt and secant slopes m_i
        let mut dt: Vec<F> = Vec::with_capacity(n - 1);
        let mut m: Vec<F> = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let h = knots[i + 1] - knots[i];
            dt.push(h);
            m.push((y[i + 1] - y[i]) / h);
        }

        // Step 1: initial derivatives d_i.
        let mut d: Vec<F> = vec![0.0; n];
        d[0] = m[0];
        d[n - 1] = m[n - 2];
        for i in 1..n - 1 {
            if m[i - 1] * m[i] <= 0.0 {
                d[i] = 0.0;
            } else {
                let w1 = 2.0 * dt[i] + dt[i - 1];
                let w2 = dt[i] + 2.0 * dt[i - 1];
                d[i] = (w1 + w2) / (w1 / m[i - 1] + w2 / m[i]);
            }
        }

        // Step 2: convexity constraint scaling.
        for i in 0..n - 1 {
            if m[i].abs() < EPS {
                continue; // avoid division by zero; d already satisfy monotonic.
            }
            let alpha = d[i] / m[i];
            let beta = d[i + 1] / m[i];
            let sum_sq = alpha * alpha + beta * beta;
            if sum_sq > 9.0 {
                let tau = 3.0 / sum_sq.sqrt();
                d[i] *= tau;
                d[i + 1] *= tau;
            }
        }

        // Construct coefficients (a,b,c,d) for each segment.
        let mut coeffs: Vec<(F, F, F, F)> = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let h = dt[i];
            let a = y[i];
            let b = d[i] * h; // already scaled by h
            let c = (3.0 * m[i] - 2.0 * d[i] - d[i + 1]) * h;
            let dcoef = (d[i] + d[i + 1] - 2.0 * m[i]) * h;
            coeffs.push((a, b, c, dcoef));
        }

        coeffs.into_boxed_slice()
    }

    // Shared `locate_segment` from utils is used.
}

impl InterpFn for MonotoneConvex {
    fn interp(&self, x: F) -> F {
        // Clamp to bounds to avoid out-of-range evaluations due to
        // small day-count or floating-point discrepancies.
        if x <= self.knots[0] {
            return self.dfs[0];
        }
        if x >= *self.knots.last().unwrap() {
            return *self.dfs.last().unwrap();
        }
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.dfs[idx_exact];
        }
        let idx = crate::market_data::utils::locate_segment(&self.knots, x).unwrap();
        let (a, b, c, d) = self.coeffs[idx];
        let h = self.knots[idx + 1] - self.knots[idx];
        let s = (x - self.knots[idx]) / h;
        let y = a + b * s + c * s * s + d * s * s * s;
        (-y).exp()
    }
}

// Use an epsilon scaled from the machine epsilon for clarity and portability.
pub(crate) const EPS: F = F::EPSILON * 100.0;
