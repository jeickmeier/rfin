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
