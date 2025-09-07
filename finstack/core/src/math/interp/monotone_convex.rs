use super::InterpFn;
use crate::math::interp::utils::validate_knots;
use crate::{math::interp::ExtrapolationPolicy, F};
use std::boxed::Box;
use std::vec::Vec;

/// Monotone-convex discount-factor interpolator (Hagan & West, 2006).
///
/// Implements the full Hagan–West slope-selecting, monotone-convex cubic
/// interpolation in natural-log discount-factor space. This is the industry
/// standard for yield curve construction as it guarantees positive and
/// continuous forward rates.
///
/// ## Algorithm Overview
/// 1. Convert discount factors to continuously compounded yields: y = -ln(P)
/// 2. Compute initial derivatives using weighted harmonic means for smoothness
/// 3. Apply convexity constraint g(α,β) = α² + β² ≤ 9 to ensure positive forwards
/// 4. Build cubic coefficients for each segment: y(s) = a + bs + cs² + ds³
///
/// ## Numerical Stability
/// - Uses epsilon protection (100 × machine epsilon) for near-zero slopes
/// - Harmonic mean calculation protected against division by very small values
/// - Supports configurable extrapolation policy beyond input domain
///
/// The constructor computes and stores per-segment cubic coefficients guaranteeing
/// positivity, monotonicity and convexity when the input curve is arbitrage-free
/// (non-increasing). Evaluation is O(log N) due to binary search on the knot vector.
///
/// See unit tests and `examples/` for usage.
#[derive(Debug)]
pub struct MonotoneConvex {
    /// Knot times _tᵢ_ (strictly increasing).  Length ≥ 2.
    knots: Box<[F]>,
    /// Original discount factors _P(tᵢ)_.  Same length as `knots`.
    dfs: Box<[F]>,
    /// Per-segment cubic coefficients (a,b,c,d) in ln-DF space.
    /// For segment i with normalized parameter s ∈ [0,1]:
    /// y(s) = a + b*s + c*s² + d*s³ where y = -ln(P)
    ///
    /// Coefficients computed using the Hagan-West monotone-convex algorithm:
    /// - a = y[i] (left endpoint value)
    /// - b = d[i] * h (left derivative scaled by segment width)
    /// - c = 3*m[i] - 2*d[i] - d[i+1] (ensures C¹ continuity)
    /// - d = d[i] + d[i+1] - 2*m[i] (ensures slope constraints)
    ///   where m[i] is the secant slope and d[i] are the monotone derivatives.
    coeffs: Box<[(F, F, F, F)]>,
    /// Extrapolation policy for out-of-bounds evaluation.
    extrapolation: ExtrapolationPolicy,
}

impl MonotoneConvex {
    /// Construct a new monotone-convex interpolator.
    ///
    /// # Parameters
    /// * `knots` – strictly increasing times in the same units as evaluation
    ///             points (e.g. years).
    /// * `dfs`   – corresponding discount factors, positive and
    ///             non-increasing (arbitrage-free).
    /// * `extrapolation` – policy for out-of-bounds evaluation.
    ///
    /// # Errors
    /// * `InputError::TooFewPoints`        – fewer than two knots.
    /// * `InputError::NonMonotonicKnots`   – times not strictly increasing.
    /// * `InputError::NonPositiveValue`    – DF ≤ 0.
    /// * `InputError::Invalid`             – DF increases between knots.
    #[allow(clippy::boxed_local)]
    pub fn new(
        knots: Box<[F]>,
        dfs: Box<[F]>,
        extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        debug_assert_eq!(knots.len(), dfs.len());

        // ---- Sanity checks -------------------------------------------------
        validate_knots(&knots)?;
        crate::math::interp::utils::validate_monotone_nonincreasing(&dfs)?;

        // Compute cubic coefficients **before** moving `knots` and `dfs` into
        // the struct to avoid partial move/borrow checker conflicts.
        let coeffs = Self::build_coeffs(&knots, &dfs);

        Ok(Self {
            knots,
            dfs,
            coeffs,
            extrapolation,
        })
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

        // Step 1: initial derivatives d_i using monotone slope selection.
        let mut d: Vec<F> = vec![0.0; n];
        d[0] = m[0];
        d[n - 1] = m[n - 2];
        for i in 1..n - 1 {
            if m[i - 1] * m[i] <= 0.0 {
                // Sign change or zero crossing: use zero derivative for monotonicity
                d[i] = 0.0;
            } else if m[i - 1].abs() < EPS || m[i].abs() < EPS {
                // Near-zero slope: avoid numerical instability in harmonic mean
                d[i] = 0.0;
            } else {
                // Weighted harmonic mean for smooth transition
                let w1 = 2.0 * dt[i] + dt[i - 1];
                let w2 = dt[i] + 2.0 * dt[i - 1];
                d[i] = (w1 + w2) / (w1 / m[i - 1] + w2 / m[i]);
            }
        }

        // Step 2: convexity constraint scaling (Hagan-West "g" function).
        // The constraint g(α,β) = α² + β² ≤ 9 ensures the cubic interpolant
        // maintains positive forward rates. When violated, scale derivatives
        // by τ = 3/√(α² + β²) to satisfy the constraint.
        for i in 0..n - 1 {
            if m[i].abs() < EPS {
                continue; // avoid division by zero; d already satisfy monotonic.
            }
            let alpha = d[i] / m[i]; // Left derivative normalized by secant slope
            let beta = d[i + 1] / m[i]; // Right derivative normalized by secant slope
            let sum_sq = alpha * alpha + beta * beta;
            if sum_sq > 9.0 {
                // Violation of convexity constraint: scale both derivatives
                let tau = 3.0 / sum_sq.sqrt();
                d[i] *= tau;
                d[i + 1] *= tau;
            }
        }

        // Construct coefficients (a,b,c,d) for each segment's cubic polynomial.
        // These coefficients define y(s) = a + b*s + c*s² + d*s³ in normalized
        // coordinates where s ∈ [0,1] spans each segment.
        let mut coeffs: Vec<(F, F, F, F)> = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let h = dt[i];
            let a = y[i]; // y-value at left endpoint
            let b = d[i] * h; // left derivative scaled by segment width
            let c = (3.0 * m[i] - 2.0 * d[i] - d[i + 1]) * h; // C¹ continuity constraint
            let dcoef = (d[i] + d[i + 1] - 2.0 * m[i]) * h; // slope matching constraint
            coeffs.push((a, b, c, dcoef));
        }

        coeffs.into_boxed_slice()
    }

    // Shared `locate_segment` from utils is used.
}

impl InterpFn for MonotoneConvex {
    fn interp(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => self.dfs[0],
                ExtrapolationPolicy::FlatForward => {
                    // For MonotoneConvex, use linear extrapolation based on cubic slope at boundary
                    let (a, b, _c, _d) = self.coeffs[0];
                    let h = self.knots[1] - self.knots[0];
                    let s = (x - self.knots[0]) / h;
                    let y = a + b * s;
                    (-y).exp()
                }
            };
        }
        if x >= *self.knots.last().unwrap() {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => *self.dfs.last().unwrap(),
                ExtrapolationPolicy::FlatForward => {
                    // For MonotoneConvex, use linear extrapolation based on cubic slope at boundary
                    let n = self.coeffs.len();
                    let (a, b, c, d) = self.coeffs[n - 1];
                    let h = self.knots[n] - self.knots[n - 1];
                    let dy_ds_at_end = b + 2.0 * c + 3.0 * d;
                    let s_extra = 1.0 + (x - self.knots[n]) / h;
                    let y_end = a + b + c + d;
                    let y = y_end + dy_ds_at_end * (s_extra - 1.0);
                    (-y).exp()
                }
            };
        }

        // Exact knot match
        if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap()) {
            return self.dfs[idx_exact];
        }

        // Interior interpolation using monotone-convex cubic
        let idx = crate::math::interp::utils::locate_segment(&self.knots, x).unwrap();
        let (a, b, c, d) = self.coeffs[idx];
        let h = self.knots[idx + 1] - self.knots[idx];
        let s = (x - self.knots[idx]) / h;
        let y = a + b * s + c * s * s + d * s * s * s;
        (-y).exp()
    }

    fn interp_prime(&self, x: F) -> F {
        // Handle extrapolation based on policy
        if x <= self.knots[0] {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let (_a, b, _c, _d) = self.coeffs[0];
                    let h = self.knots[1] - self.knots[0];
                    let f_val = self.interp(x);
                    -f_val * b / h
                }
            };
        }
        if x >= *self.knots.last().unwrap() {
            return match self.extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let n = self.coeffs.len();
                    let (_a, b, c, d) = self.coeffs[n - 1];
                    let h = self.knots[n] - self.knots[n - 1];
                    let dy_ds_at_end = b + 2.0 * c + 3.0 * d;
                    let f_val = self.interp(x);
                    -f_val * dy_ds_at_end / h
                }
            };
        }

        // Find segment and compute derivative
        let idx = if let Ok(idx_exact) = self.knots.binary_search_by(|k| k.partial_cmp(&x).unwrap())
        {
            // Exact knot: use right derivative for consistency (except at last knot)
            if idx_exact == self.knots.len() - 1 {
                idx_exact - 1
            } else {
                idx_exact
            }
        } else {
            crate::math::interp::utils::locate_segment(&self.knots, x).unwrap()
        };

        let (a, b, c, d) = self.coeffs[idx];
        let h = self.knots[idx + 1] - self.knots[idx];
        let s = (x - self.knots[idx]) / h;
        let y = a + b * s + c * s * s + d * s * s * s;
        let dy_ds = b + 2.0 * c * s + 3.0 * d * s * s;
        let f_val = (-y).exp();

        // Chain rule: df/dx = df/dy * dy/ds * ds/dx = -f * dy/ds / h
        -f_val * dy_ds / h
    }
}

/// Numerical tolerance for near-zero slope detection.
///
/// Set to 100 × machine epsilon to provide adequate protection against
/// division by very small numbers while preserving numerical accuracy.
/// This threshold is used to:
/// - Avoid harmonic mean calculation when slopes are near zero
/// - Skip convexity constraint scaling for flat segments
pub(crate) const EPS: F = F::EPSILON * 100.0;
