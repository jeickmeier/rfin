//! Numerically stable summation algorithms.
//!
//! Implements compensated summation methods to minimize floating-point
//! rounding errors when summing large sequences. Critical for maintaining
//! deterministic results in financial calculations.
//!
//! # Algorithms
//!
//! - [`kahan_sum`]: Compensated summation with error tracking (best for same-sign values)
//! - [`neumaier_sum`]: Improved compensated summation (best for mixed-sign values)
//! - [`NeumaierAccumulator`]: Incremental accumulator for streaming summation
//!
//! # Choosing an Algorithm
//!
//! For most use cases, prefer [`neumaier_sum`] or [`NeumaierAccumulator`] as they
//! handle both same-sign and mixed-sign values correctly.
//!
//! # References
//!
//! - Kahan, W. (1965). "Further Remarks on Reducing Truncation Errors."
//!   *Communications of the ACM*, 8(1), 40.
//! - Neumaier, A. (1974). "Rundungsfehleranalyse einiger Verfahren zur Summation
//!   endlicher Summen." *Zeitschrift für Angewandte Mathematik und Mechanik*, 54(1), 39-51.
//! - Higham, N. J. (1993). "The Accuracy of Floating Point Summation."
//!   *SIAM Journal on Scientific Computing*, 14(4), 783-799.

/// Kahan compensated summation – improves numerical stability while preserving a fixed iteration order.
///
/// Best for sequences where all values have the same sign. For mixed-sign values,
/// prefer [`neumaier_sum`] which handles magnitude differences better.
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
/// **Recommended for financial calculations with mixed-sign cashflows.**
///
/// # Example
///
/// ```rust
/// use finstack_core::math::neumaier_sum;
///
/// // Mixed-sign values where naive summation loses precision
/// let values = [1e16, 1.0, -1e16];
/// let sum = neumaier_sum(values.iter().copied());
/// assert!((sum - 1.0).abs() < 1e-10);
/// ```
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

/// Incremental Neumaier compensated summation.
///
/// Useful when you want stable accumulation without allocating a temporary
/// `Vec<f64>` of terms (e.g., PV loops over cashflows). Handles both same-sign
/// and mixed-sign values correctly.
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
