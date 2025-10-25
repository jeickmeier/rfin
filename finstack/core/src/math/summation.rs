//! Summation helpers with determinism toggles.
//!
//! We implement these ourselves rather than using external crates to ensure:
//! - Deterministic results using our custom summation algorithm
//! - Feature-flag controlled behaviour (deterministic vs. fast)
//! - No dependencies on external crates for basic operations
//! - Consistent numerical behaviour across platforms

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
    // Use Kahan summation for numerical stability by default
    kahan_sum(xs.iter().copied())
}
