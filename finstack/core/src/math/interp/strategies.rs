//! Concrete interpolation strategy implementations.
//!
//! Provides strategy types for Linear, LogLinear, CubicHermite, and MonotoneConvex
//! interpolation, encapsulating algorithm-specific precomputed data and evaluation logic.

use super::{
    traits::InterpolationStrategy,
    types::ExtrapolationPolicy,
    utils::{locate_segment, validate_monotone_nonincreasing},
};
use std::vec::Vec;

// -----------------------------------------------------------------------------
// LinearStrategy
// -----------------------------------------------------------------------------

/// Strategy for piecewise linear interpolation on discount factors.
///
/// Simple linear interpolation between knot points. Fast and straightforward
/// but may produce negative forward rates (arbitrage) if discount factors
/// aren't carefully spaced. Prefer LogLinear or MonotoneConvex for yield curves.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LinearStrategy;

impl InterpolationStrategy for LinearStrategy {
    fn from_raw(
        _knots: &[f64],
        _values: &[f64],
        _extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        // Linear strategy has no precomputed state
        Ok(Self)
    }

    fn interp(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => values[0],
                ExtrapolationPolicy::FlatForward => {
                    let slope = segment_slope(knots, values, 0, 1);
                    let x0 = knots[0];
                    values[0] + slope * (x - x0)
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                let n = knots.len();
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => {
                        *values.last().expect("values should not be empty")
                    }
                    ExtrapolationPolicy::FlatForward => {
                        let slope = segment_slope(knots, values, n - 2, n - 1);
                        let x1 = knots[n - 1];
                        values[n - 1] + slope * (x - x1)
                    }
                };
            }
        }

        // Exact knot match
        if let Ok(idx_exact) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            return values[idx_exact];
        }

        // Interior linear interpolation
        let idx = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[idx];
        let x1 = knots[idx + 1];
        let y0 = values[idx];
        let y1 = values[idx + 1];
        let w = (x - x0) / (x1 - x0);
        y0 + w * (y1 - y0)
    }

    fn interp_prime(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => segment_slope(knots, values, 0, 1),
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                let n = knots.len();
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => 0.0,
                    ExtrapolationPolicy::FlatForward => segment_slope(knots, values, n - 2, n - 1),
                };
            }
        }

        // Interior linear interpolation derivative
        let idx = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        segment_slope(knots, values, idx, idx + 1)
    }
}

// Helper for linear slope calculation
#[inline]
fn segment_slope(knots: &[f64], values: &[f64], left_index: usize, right_index: usize) -> f64 {
    let x0 = knots[left_index];
    let x1 = knots[right_index];
    let y0 = values[left_index];
    let y1 = values[right_index];
    (y1 - y0) / (x1 - x0)
}

// -----------------------------------------------------------------------------
// LogLinearStrategy
// -----------------------------------------------------------------------------

/// Strategy for log-linear interpolation of discount factors.
///
/// Performs linear interpolation on ln(DF), equivalent to piecewise-constant
/// zero rates. Guarantees positive forward rates and is commonly used for
/// government bond curves and simple yield curve construction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LogLinearStrategy {
    /// Precomputed log(values) for efficient evaluation.
    log_values: Box<[f64]>,
}

impl InterpolationStrategy for LogLinearStrategy {
    fn from_raw(
        _knots: &[f64],
        values: &[f64],
        _extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        // Precompute log(values) for efficiency
        let log_values: Vec<f64> = values.iter().map(|v| v.ln()).collect();
        Ok(Self {
            log_values: log_values.into_boxed_slice(),
        })
    }

    fn interp(
        &self,
        x: f64,
        knots: &[f64],
        _values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => self.log_values[0].exp(),
                ExtrapolationPolicy::FlatForward => {
                    let slope = log_segment_slope(&self.log_values, knots, 0, 1);
                    let extrapolated_log = self.log_values[0] + slope * (x - knots[0]);
                    extrapolated_log.exp()
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                let n = knots.len();
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => {
                        (*self.log_values.last().expect("log_values should not be empty")).exp()
                    }
                    ExtrapolationPolicy::FlatForward => {
                        let slope = log_segment_slope(&self.log_values, knots, n - 2, n - 1);
                        let extrapolated_log =
                            self.log_values[n - 1] + slope * (x - knots[n - 1]);
                        extrapolated_log.exp()
                    }
                };
            }
        }

        // Exact knot match
        if let Ok(idx_exact) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            return self.log_values[idx_exact].exp();
        }

        // Interior interpolation
        let idx = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[idx];
        let x1 = knots[idx + 1];
        let y0 = self.log_values[idx];
        let y1 = self.log_values[idx + 1];
        let w = (x - x0) / (x1 - x0);
        (y0 + w * (y1 - y0)).exp()
    }

    fn interp_prime(
        &self,
        x: f64,
        knots: &[f64],
        _values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // For log-linear interpolation: f(x) = exp(y0 + w*(y1-y0)) where w = (x-x0)/(x1-x0)
        // The derivative is: df/dx = f(x) * (y1-y0)/(x1-x0)

        // At boundaries, handle based on extrapolation policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let slope = log_segment_slope(&self.log_values, knots, 0, 1);
                    let f_val = self.interp(x, knots, &[], extrapolation);
                    f_val * slope
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                let n = knots.len();
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => 0.0,
                    ExtrapolationPolicy::FlatForward => {
                        let slope = log_segment_slope(&self.log_values, knots, n - 2, n - 1);
                        let f_val = self.interp(x, knots, &[], extrapolation);
                        f_val * slope
                    }
                };
            }
        }

        // Get the interpolated value and log-linear slope
        let f_val = self.interp(x, knots, &[], extrapolation);
        let idx = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let slope = log_segment_slope(&self.log_values, knots, idx, idx + 1);

        // Derivative: f(x) * (slope in log space)
        f_val * slope
    }
}

impl LogLinearStrategy {
    /// Access the log values (for serialization).
    pub fn log_values(&self) -> &[f64] {
        &self.log_values
    }
}

// Helper for log-linear slope calculation
#[inline]
fn log_segment_slope(log_values: &[f64], knots: &[f64], left_index: usize, right_index: usize) -> f64 {
    let x0 = knots[left_index];
    let x1 = knots[right_index];
    let y0 = log_values[left_index];
    let y1 = log_values[right_index];
    (y1 - y0) / (x1 - x0)
}

// -----------------------------------------------------------------------------
// CubicHermiteStrategy
// -----------------------------------------------------------------------------

/// Strategy for monotone cubic Hermite interpolation (PCHIP).
///
/// Implements the Piecewise Cubic Hermite Interpolating Polynomial with
/// Fritsch-Carlson slope selection. Preserves monotonicity of input data,
/// ensuring no spurious oscillations. Requires monotone input discount factors.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CubicHermiteStrategy {
    /// First-derivative values at each knot (PCHIP slopes).
    ms: Box<[f64]>,
}

impl InterpolationStrategy for CubicHermiteStrategy {
    fn from_raw(
        knots: &[f64],
        values: &[f64],
        _extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        // Pre-compute monotone slopes (PCHIP / Fritsch-Carlson)
        let ms = compute_monotone_slopes(knots, values);
        Ok(Self { ms })
    }

    fn interp(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => values[0],
                ExtrapolationPolicy::FlatForward => {
                    let x0 = knots[0];
                    let slope = self.ms[0];
                    let dx = x - x0;
                    values[0] + slope * dx
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => {
                        *values.last().expect("values should not be empty")
                    }
                    ExtrapolationPolicy::FlatForward => {
                        let n = knots.len();
                        let x_last = knots[n - 1];
                        let slope = self.ms[n - 1];
                        let dx = x - x_last;
                        values[n - 1] + slope * dx
                    }
                };
            }
        }

        // Fast-path: exact knot value → short-circuit
        if let Ok(idx) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            return values[idx];
        }

        // Interior interpolation using cubic Hermite
        let i = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[i];
        let x1 = knots[i + 1];
        let h = x1 - x0;
        // Normalised coordinate t ∈ (0,1)
        let t = (x - x0) / h;
        let t2 = t * t;
        let t3 = t2 * t;

        // Basis functions
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;

        // Values and slopes
        let f0 = values[i];
        let f1 = values[i + 1];
        let m0 = self.ms[i];
        let m1 = self.ms[i + 1];

        // Cubic Hermite formula
        h00 * f0 + h10 * h * m0 + h01 * f1 + h11 * h * m1
    }

    fn interp_prime(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => self.ms[0],
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => 0.0,
                    ExtrapolationPolicy::FlatForward => self.ms[self.ms.len() - 1],
                };
            }
        }

        // For exact knot values, return the precomputed slope
        if let Ok(idx) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            return self.ms[idx];
        }

        let i = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[i];
        let x1 = knots[i + 1];
        let h = x1 - x0;
        // Normalised coordinate t ∈ (0,1)
        let t = (x - x0) / h;
        let t2 = t * t;

        // Derivative of basis functions w.r.t. t
        let h00_prime = 6.0 * t2 - 6.0 * t;
        let h10_prime = 3.0 * t2 - 4.0 * t + 1.0;
        let h01_prime = -6.0 * t2 + 6.0 * t;
        let h11_prime = 3.0 * t2 - 2.0 * t;

        // Values and slopes
        let f0 = values[i];
        let f1 = values[i + 1];
        let m0 = self.ms[i];
        let m1 = self.ms[i + 1];

        // Derivative w.r.t. t
        let df_dt = h00_prime * f0 + h10_prime * h * m0 + h01_prime * f1 + h11_prime * h * m1;

        // Convert to derivative w.r.t. x using chain rule: df/dx = (df/dt) * (dt/dx) = (df/dt) / h
        df_dt / h
    }
}

impl CubicHermiteStrategy {
    /// Access the slopes (for serialization or inspection).
    pub fn slopes(&self) -> &[f64] {
        &self.ms
    }
}

/// Compute shape-preserving slopes using the Fritsch-Carlson monotone scheme
/// (a.k.a. PCHIP slopes).
#[inline]
fn compute_monotone_slopes(xs: &[f64], ys: &[f64]) -> Box<[f64]> {
    let n = xs.len();
    debug_assert_eq!(n, ys.len());

    // When only two points are available we fall back to linear
    if n == 2 {
        let slope = (ys[1] - ys[0]) / (xs[1] - xs[0]);
        return vec![slope, slope].into_boxed_slice();
    }

    let mut ms = vec![0.0; n];

    // Compute intervals in a single iterator pass
    let (h, delta): (Vec<f64>, Vec<f64>) = xs
        .windows(2)
        .zip(ys.windows(2))
        .map(|(xw, yw)| {
            let hi = xw[1] - xw[0];
            let di = (yw[1] - yw[0]) / hi;
            (hi, di)
        })
        .unzip();

    // Interior points
    for i in 1..n - 1 {
        if delta[i - 1] == 0.0 || delta[i] == 0.0 || delta[i - 1].signum() != delta[i].signum() {
            ms[i] = 0.0;
        } else {
            let w1 = 2.0 * h[i] + h[i - 1];
            let w2 = h[i] + 2.0 * h[i - 1];
            ms[i] = (w1 + w2) / (w1 / delta[i - 1] + w2 / delta[i]);
        }
    }

    // Endpoints (monotone one-sided estimates)
    // m0
    ms[0] = ((2.0 * h[0] + h[1]) * delta[0] - h[0] * delta[1]) / (h[0] + h[1]);
    if ms[0].signum() != delta[0].signum() {
        ms[0] = 0.0;
    } else if delta[0].signum() != delta[1].signum() && ms[0].abs() > 3.0 * delta[0].abs() {
        ms[0] = 3.0 * delta[0];
    }

    // m_{n-1}
    let last = n - 1;
    ms[last] =
        ((2.0 * h[last - 1] + h[last - 2]) * delta[last - 1] - h[last - 1] * delta[last - 2])
            / (h[last - 1] + h[last - 2]);
    if ms[last].signum() != delta[last - 1].signum() {
        ms[last] = 0.0;
    } else if delta[last - 2].signum() != delta[last - 1].signum()
        && ms[last].abs() > 3.0 * delta[last - 1].abs()
    {
        ms[last] = 3.0 * delta[last - 1];
    }

    ms.into_boxed_slice()
}

// -----------------------------------------------------------------------------
// MonotoneConvexStrategy
// -----------------------------------------------------------------------------

/// Default epsilon for near-zero slope detection in MonotoneConvex.
const DEFAULT_EPSILON: f64 = 100.0 * f64::EPSILON;

/// Strategy for monotone-convex discount-factor interpolation (Hagan & West, 2006).
///
/// Implements the full Hagan–West slope-selecting, monotone-convex cubic
/// interpolation in natural-log discount-factor space. This is the industry
/// standard for yield curve construction as it guarantees positive and
/// continuous forward rates.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MonotoneConvexStrategy {
    /// Per-segment cubic coefficients (a,b,c,d) in ln-DF space.
    coeffs: Box<[(f64, f64, f64, f64)]>,
    /// Numerical tolerance for near-zero slope detection.
    epsilon: f64,
}

impl InterpolationStrategy for MonotoneConvexStrategy {
    fn from_raw(
        knots: &[f64],
        values: &[f64],
        _extrapolation: ExtrapolationPolicy,
    ) -> crate::Result<Self> {
        // Validate monotone non-increasing (arbitrage-free)
        validate_monotone_nonincreasing(values)?;
        
        // Build cubic coefficients with default epsilon
        let epsilon = DEFAULT_EPSILON;
        let coeffs = Self::build_coeffs(knots, values, epsilon);
        
        Ok(Self { coeffs, epsilon })
    }

    fn interp(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => values[0],
                ExtrapolationPolicy::FlatForward => {
                    let (a, b, _c, _d) = self.coeffs[0];
                    let h = knots[1] - knots[0];
                    let s = (x - knots[0]) / h;
                    let y = a + b * s;
                    (-y).exp()
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => {
                        *values.last().expect("values should not be empty")
                    }
                    ExtrapolationPolicy::FlatForward => {
                        let n = self.coeffs.len();
                        let (a, b, c, d) = self.coeffs[n - 1];
                        let h = knots[n] - knots[n - 1];
                        let dy_ds_at_end = b + 2.0 * c + 3.0 * d;
                        let s_extra = 1.0 + (x - knots[n]) / h;
                        let y_end = a + b + c + d;
                        let y = y_end + dy_ds_at_end * (s_extra - 1.0);
                        (-y).exp()
                    }
                };
            }
        }

        // Exact knot match
        if let Ok(idx_exact) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            return values[idx_exact];
        }

        // Interior interpolation
        let i = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[i];
        let x1 = knots[i + 1];
        let h = x1 - x0;
        let s = (x - x0) / h;
        let s2 = s * s;
        let s3 = s2 * s;

        let (a, b, c, d) = self.coeffs[i];
        let y = a + b * s + c * s2 + d * s3;
        (-y).exp()
    }

    fn interp_prime(
        &self,
        x: f64,
        knots: &[f64],
        values: &[f64],
        extrapolation: ExtrapolationPolicy,
    ) -> f64 {
        // Handle extrapolation based on policy
        if x <= knots[0] {
            return match extrapolation {
                ExtrapolationPolicy::FlatZero => 0.0,
                ExtrapolationPolicy::FlatForward => {
                    let (a, b, _c, _d) = self.coeffs[0];
                    let h = knots[1] - knots[0];
                    let s = (x - knots[0]) / h;
                    let y = a + b * s;
                    let dy_ds = b;
                    let p = (-y).exp();
                    -p * dy_ds / h
                }
            };
        }
        if let Some(&last_knot) = knots.last() {
            if x >= last_knot {
                return match extrapolation {
                    ExtrapolationPolicy::FlatZero => 0.0,
                    ExtrapolationPolicy::FlatForward => {
                        let n = self.coeffs.len();
                        let (a, b, c, d) = self.coeffs[n - 1];
                        let h = knots[n] - knots[n - 1];
                        let s_extra = 1.0 + (x - knots[n]) / h;
                        let y_end = a + b + c + d;
                        let dy_ds_at_end = b + 2.0 * c + 3.0 * d;
                        let y = y_end + dy_ds_at_end * (s_extra - 1.0);
                        let p = (-y).exp();
                        -p * dy_ds_at_end / h
                    }
                };
            }
        }

        // For exact knot values, compute derivative using coefficients
        if let Ok(idx) = knots.binary_search_by(|k| {
            k.partial_cmp(&x)
                .expect("f64 comparison should always be comparable")
        }) {
            let p = values[idx];
            if idx == 0 {
                let (_a, b, _c, _d) = self.coeffs[0];
                let h = knots[1] - knots[0];
                let dy_ds = b;
                return -p * dy_ds / h;
            } else {
                let (_a, b, c, d) = self.coeffs[idx - 1];
                let h = knots[idx] - knots[idx - 1];
                let dy_ds = b + 2.0 * c + 3.0 * d;
                return -p * dy_ds / h;
            }
        }

        let i = locate_segment(knots, x).expect("Segment location should succeed for valid x");
        let x0 = knots[i];
        let x1 = knots[i + 1];
        let h = x1 - x0;
        let s = (x - x0) / h;
        let s2 = s * s;

        let (a, b, c, d) = self.coeffs[i];
        let y = a + b * s + c * s2 + d * s * s2;
        let dy_ds = b + 2.0 * c * s + 3.0 * d * s2;
        let p = (-y).exp();
        -p * dy_ds / h
    }
}

impl MonotoneConvexStrategy {
    /// Construct with custom epsilon for near-zero slope detection.
    pub fn with_epsilon(
        knots: &[f64],
        values: &[f64],
        epsilon: f64,
    ) -> crate::Result<Self> {
        use crate::error::InputError;
        
        // Validate epsilon is reasonable
        if epsilon <= 0.0 || epsilon > 1e-6 {
            return Err(InputError::Invalid.into());
        }
        
        // Validate monotone non-increasing
        validate_monotone_nonincreasing(values)?;
        
        let coeffs = Self::build_coeffs(knots, values, epsilon);
        Ok(Self { coeffs, epsilon })
    }

    /// Compute cubic coefficients for each segment according to the
    /// Hagan–West monotone–convex algorithm.
    fn build_coeffs(knots: &[f64], dfs: &[f64], epsilon: f64) -> Box<[(f64, f64, f64, f64)]> {
        let n = knots.len();
        debug_assert!(n >= 2);

        // Convert to continuously-compounded zero yields y = −ln P
        let y: Vec<f64> = dfs.iter().map(|&p| -p.ln()).collect();

        // Δt and secant slopes m_i
        let mut dt: Vec<f64> = Vec::with_capacity(n - 1);
        let mut m: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let h = knots[i + 1] - knots[i];
            dt.push(h);
            m.push((y[i + 1] - y[i]) / h);
        }

        // Step 1: initial derivatives d_i using monotone slope selection
        let mut d: Vec<f64> = vec![0.0; n];
        d[0] = m[0];
        d[n - 1] = m[n - 2];
        for i in 1..n - 1 {
            if m[i - 1] * m[i] <= 0.0 || m[i - 1].abs() < epsilon || m[i].abs() < epsilon {
                // Sign change, zero crossing, or near-zero slope: use zero derivative for monotonicity
                d[i] = 0.0;
            } else {
                // Weighted harmonic mean for smooth transition
                let w1 = 2.0 * dt[i] + dt[i - 1];
                let w2 = dt[i] + 2.0 * dt[i - 1];
                d[i] = (w1 + w2) / (w1 / m[i - 1] + w2 / m[i]);
            }
        }

        // Step 2: convexity constraint scaling
        for i in 0..n - 1 {
            if m[i].abs() < epsilon {
                continue;
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

        // Construct coefficients (a,b,c,d) for each segment's cubic polynomial
        let mut coeffs: Vec<(f64, f64, f64, f64)> = Vec::with_capacity(n - 1);
        for i in 0..n - 1 {
            let h = dt[i];
            let a = y[i];
            let b = d[i] * h;
            let c = (3.0 * m[i] - 2.0 * d[i] - d[i + 1]) * h;
            let dcoef = (d[i] + d[i + 1] - 2.0 * m[i]) * h;
            coeffs.push((a, b, c, dcoef));
        }

        coeffs.into_boxed_slice()
    }

    /// Get the epsilon value used for near-zero slope detection.
    pub fn epsilon(&self) -> f64 {
        self.epsilon
    }

    /// Access the coefficients (for serialization or inspection).
    pub fn coeffs(&self) -> &[(f64, f64, f64, f64)] {
        &self.coeffs
    }
}

