//! Numerically stable summation algorithms.
//!
//! Implements compensated summation methods to minimize floating-point
//! rounding errors when summing large sequences. Critical for maintaining
//! deterministic results in financial calculations.
//!
//! # Algorithms
//!
//! - [`kahan_sum`]: Compensated summation with error tracking
//! - [`neumaier_sum`]: Improved compensated summation for mixed-sign values
//! - [`pairwise_sum`]: Divide-and-conquer approach for balanced accuracy
//! - [`stable_sum`]: Determinism-aware dispatch to appropriate method
//!
//! # References
//!
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."
//!   *Communications of the ACM*, 8(1), 40.
//! - Higham, N. J. (1993). "The Accuracy of Floating Point Summation."
//!   *SIAM Journal on Scientific Computing*, 14(4), 783-799.

/// Kahan compensated summation – improves numerical stability while preserving a fixed iteration order.
#[inline]
pub fn kahan_sum<I>(iter: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    let mut sum: f64 = 0.0;
    let mut c: f64 = 0.0; // compensation
    for x in iter {
        let y = x - c;
        let t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    sum
}

/// Neumaier compensated summation – handles both positive and negative values better than Kahan.
///
/// This algorithm improves upon Kahan summation by better handling cases where
/// the sum and the next value have similar magnitudes but opposite signs.
/// Recommended for financial calculations with mixed-sign cashflows.
///
/// # References
///
/// - Neumaier, A. (1974). "Rundungsfehleranalyse einiger Verfahren zur Summation
///   endlicher Summen." *Zeitschrift für Angewandte Mathematik und Mechanik*, 54(1), 39-51.
#[inline]
pub fn neumaier_sum<I>(iter: I) -> f64
where
    I: IntoIterator<Item = f64>,
{
    let mut sum = 0.0;
    let mut c = 0.0; // running compensation
    for x in iter {
        let t = sum + x;
        c += if sum.abs() >= x.abs() {
            (sum - t) + x
        } else {
            (x - t) + sum
        };
        sum = t;
    }
    sum + c
}

/// Incremental Kahan compensated summation.
///
/// Useful when you want stable accumulation without allocating a temporary
/// `Vec<f64>` of terms (e.g., PV loops over cashflows). Use this when all
/// values have the same sign (e.g., discounted PVs).
///
/// For mixed-sign values (e.g., cashflows with both inflows and outflows),
/// prefer [`NeumaierAccumulator`] which handles magnitude differences better.
///
/// # Example
///
/// ```rust
/// use finstack_core::math::KahanAccumulator;
///
/// // Summing many small values where naive summation accumulates error
/// let mut acc = KahanAccumulator::new();
/// for _ in 0..10_000 {
///     acc.add(0.0001);
/// }
/// // Kahan gives us precise 1.0, naive sum might drift
/// assert!((acc.total() - 1.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct KahanAccumulator {
    sum: f64,
    c: f64, // compensation term
}

impl KahanAccumulator {
    /// Create a new accumulator with zero state.
    #[inline]
    pub const fn new() -> Self {
        Self { sum: 0.0, c: 0.0 }
    }

    /// Add a value to the running total.
    #[inline]
    pub fn add(&mut self, x: f64) {
        let y = x - self.c;
        let t = self.sum + y;
        self.c = (t - self.sum) - y;
        self.sum = t;
    }

    /// Return the compensated total.
    #[inline]
    pub fn total(self) -> f64 {
        self.sum
    }

    /// Return the current sum without consuming the accumulator.
    #[inline]
    pub fn current(&self) -> f64 {
        self.sum
    }
}

/// Incremental Neumaier compensated summation.
///
/// Useful when you want stable accumulation without allocating a temporary
/// `Vec<f64>` of terms (e.g., PV loops over cashflows). This variant handles
/// mixed-sign values better than [`KahanAccumulator`].
///
/// # Example
///
/// ```rust
/// use finstack_core::math::NeumaierAccumulator;
///
/// let mut acc = NeumaierAccumulator::new();
/// for x in [1e16, 1.0, -1e16] {
///     acc.add(x);
/// }
/// // Result is more accurate than naive summation
/// assert!((acc.total() - 1.0).abs() < 1e-10);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NeumaierAccumulator {
    sum: f64,
    c: f64,
}

impl NeumaierAccumulator {
    /// Create a new accumulator with zero state.
    #[inline]
    pub const fn new() -> Self {
        Self { sum: 0.0, c: 0.0 }
    }

    /// Add a value to the running total.
    #[inline]
    pub fn add(&mut self, x: f64) {
        let t = self.sum + x;
        self.c += if self.sum.abs() >= x.abs() {
            (self.sum - t) + x
        } else {
            (x - t) + self.sum
        };
        self.sum = t;
    }

    /// Return the compensated total.
    #[inline]
    pub fn total(self) -> f64 {
        self.sum + self.c
    }

    /// Return the current sum without consuming the accumulator.
    #[inline]
    pub fn current(&self) -> f64 {
        self.sum + self.c
    }
}

/// Pairwise (divide-and-conquer) summation over a slice.
pub fn pairwise_sum(xs: &[f64]) -> f64 {
    fn recurse(slice: &[f64]) -> f64 {
        match slice.len() {
            0 => 0.0,
            1 => slice[0],
            2 => slice[0] + slice[1],
            _ => {
                let mid = slice.len() / 2;
                let left = recurse(&slice[..mid]);
                let right = recurse(&slice[mid..]);
                left + right
            }
        }
    }
    recurse(xs)
}

/// Determinism-aware sum: when the `deterministic` feature is enabled, use a
/// stable summation (pairwise). Otherwise use the standard iterator sum.
pub fn stable_sum(xs: &[f64]) -> f64 {
    // Use Neumaier summation for numerical stability by default (better for mixed-sign values)
    neumaier_sum(xs.iter().copied())
}
