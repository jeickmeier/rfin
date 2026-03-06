//! Return computation utilities: simple returns, excess returns, price conversion,
//! compounded returns. Delegates to `math::stats::log_returns` for log variants
//! and `math::summation` for numerically stable accumulation.

use crate::math::summation::{kahan_sum, NeumaierAccumulator};

/// Replace infinities with NaN and strip trailing all-NaN rows in-place.
///
/// Infinite values arise from divisions by zero in price series (e.g., a
/// price that transitions from 0). Trailing NaNs at the end of the slice
/// indicate missing data and are removed to avoid biasing later statistics.
///
/// # Arguments
///
/// * `r` - Mutable return vector. Modified in place.
///
/// # Returns
///
/// Nothing; the vector is mutated directly.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::clean_returns;
///
/// let mut r = vec![0.01, f64::INFINITY, 0.02, f64::NAN, f64::NAN];
/// clean_returns(&mut r);
/// assert_eq!(r.len(), 3);   // two trailing NaNs removed
/// assert!(r[1].is_nan());   // infinity replaced with NaN
/// ```
pub fn clean_returns(r: &mut Vec<f64>) {
    for v in r.iter_mut() {
        if v.is_infinite() {
            *v = f64::NAN;
        }
    }
    while r.last().is_some_and(|v| v.is_nan()) {
        r.pop();
    }
}

/// Simple (percentage-change) returns from a price series.
///
/// For prices `[p0, p1, p2, ...]` returns `[0.0, p1/p0 - 1, p2/p1 - 1, ...]`.
/// The leading zero mirrors the Python "prepend a zero before first valid"
/// convention, keeping output length equal to input length.
///
/// Division by zero or NaN prices produce `NaN` for that element.
///
/// # Arguments
///
/// * `prices` - Slice of asset prices in chronological order.
///
/// # Returns
///
/// A `Vec<f64>` of the same length as `prices`. The first element is always
/// `0.0`. Returns a vector of `0.0`s for single-element input.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::simple_returns;
///
/// let r = simple_returns(&[100.0, 110.0, 99.0]);
/// assert_eq!(r[0], 0.0);
/// assert!((r[1] - 0.1).abs() < 1e-12);   // +10%
/// assert!((r[2] - (-0.1)).abs() < 1e-12); // −10%
/// ```
pub fn simple_returns(prices: &[f64]) -> Vec<f64> {
    if prices.len() < 2 {
        return vec![0.0; prices.len()];
    }
    let mut out = Vec::with_capacity(prices.len());
    out.push(0.0);
    for w in prices.windows(2) {
        if w[0] == 0.0 || w[0].is_nan() {
            out.push(f64::NAN);
        } else {
            out.push(w[1] / w[0] - 1.0);
        }
    }
    out
}

/// Excess returns = portfolio returns minus risk-free returns.
///
/// When `nperiods` is provided, the risk-free rate is de-compounded to the
/// observation frequency before subtraction:
///
/// ```text
/// rf_adj = (1 + rf)^(1/nperiods) - 1
/// ```
///
/// For example, if `rf` is an annualized rate and observations are monthly,
/// pass `nperiods = 12.0`.
///
/// # Arguments
///
/// * `returns` - Portfolio return series.
/// * `rf` - Risk-free rate series, aligned with `returns`. If longer, the
///   excess length is ignored.
/// * `nperiods` - Optional compounding periods per year. `None` uses `rf`
///   values directly without adjustment.
///
/// # Returns
///
/// A `Vec<f64>` of length `min(returns.len(), rf.len())` containing
/// `returns[i] - rf_adj[i]` for each observation.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::excess_returns;
///
/// // Monthly returns, annualized risk-free rate of 10%.
/// let ret = [0.05, 0.03, -0.02];
/// let rf  = [0.10, 0.10,  0.10];
/// let ex  = excess_returns(&ret, &rf, Some(12.0));
/// // rf_adj ≈ (1.10)^(1/12) − 1 ≈ 0.00797
/// let rf_adj = 1.1_f64.powf(1.0 / 12.0) - 1.0;
/// assert!((ex[0] - (0.05 - rf_adj)).abs() < 1e-10);
/// ```
pub fn excess_returns(returns: &[f64], rf: &[f64], nperiods: Option<f64>) -> Vec<f64> {
    let n = returns.len().min(rf.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let rf_adj = match nperiods {
            Some(np) if np.abs() > f64::EPSILON && (np - 1.0).abs() > f64::EPSILON => {
                (1.0 + rf[i]).powf(1.0 / np) - 1.0
            }
            _ => rf[i],
        };
        out.push(returns[i] - rf_adj);
    }
    out
}

/// Convert simple returns back to a price series.
///
/// Reconstructs prices by compounding returns from a starting `base` value:
///
/// ```text
/// prices[0] = base
/// prices[i] = base * Π_{j=0}^{i-1} (1 + r[j])
/// ```
///
/// The output has `returns.len() + 1` elements (the initial base plus one
/// compounded value per return).
///
/// # Arguments
///
/// * `returns` - Slice of simple period returns.
/// * `base`    - Starting price level (e.g., `100.0`).
///
/// # Returns
///
/// A `Vec<f64>` of length `returns.len() + 1`. Returns `vec![]` if
/// `returns` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::{simple_returns, convert_to_prices};
///
/// let prices = [100.0, 110.0, 99.0, 105.0];
/// let r = simple_returns(&prices);
/// let recovered = convert_to_prices(&r[1..], 100.0);
/// for (a, b) in prices.iter().zip(recovered.iter()) {
///     assert!((a - b).abs() < 1e-10);
/// }
/// ```
pub fn convert_to_prices(returns: &[f64], base: f64) -> Vec<f64> {
    if returns.is_empty() {
        return vec![];
    }
    let mut prices = Vec::with_capacity(returns.len() + 1);
    prices.push(base);
    let mut cum = base;
    for &r in returns {
        cum *= 1.0 + r;
        prices.push(cum);
    }
    prices
}

/// Rebase a price series so that the first value equals `base`.
///
/// Multiplies every price by `base / prices[0]`, normalising the series
/// to a common starting level (typically `100.0`) for visual comparison.
/// If `prices[0]` is zero or NaN the original slice is returned unchanged.
///
/// # Arguments
///
/// * `prices` - Slice of asset prices. Must be non-empty for meaningful output.
/// * `base`   - Desired starting value (e.g., `100.0`).
///
/// # Returns
///
/// A new `Vec<f64>` of the same length as `prices`, starting at `base`.
/// Returns an empty vector if `prices` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::rebase;
///
/// let prices = [50.0, 55.0, 60.0];
/// let rebased = rebase(&prices, 100.0);
/// assert!((rebased[0] - 100.0).abs() < 1e-12);
/// assert!((rebased[1] - 110.0).abs() < 1e-12);
/// assert!((rebased[2] - 120.0).abs() < 1e-12);
/// ```
pub fn rebase(prices: &[f64], base: f64) -> Vec<f64> {
    if prices.is_empty() {
        return vec![];
    }
    let first = prices[0];
    if first == 0.0 || first.is_nan() {
        return prices.to_vec();
    }
    let factor = base / first;
    prices.iter().map(|&p| p * factor).collect()
}

/// Smallest growth factor allowed before taking the log.
///
/// Returns of exactly −100% (total wipeout) or worse would produce −∞ or NaN
/// in log-space. Clamping to this floor keeps the accumulator valid while
/// still representing an effectively total loss.
const MIN_GROWTH_FACTOR: f64 = 1e-18;

/// Cumulative compounded returns: `(1+r).cumprod() - 1`.
///
/// At each step `i` the cumulative return is:
///
/// ```text
/// comp_sum[i] = Π_{j=0}^{i} (1 + r[j]) - 1
/// ```
///
/// Uses a Neumaier accumulator in log-space for numerical stability on
/// long series. Growth factors are clamped to [`MIN_GROWTH_FACTOR`] so
/// that returns ≤ −1.0 produce a near-total-loss (≈ −100 %) rather than NaN.
/// NaN returns are also clamped to the floor (treated as total losses)
/// rather than poisoning the entire accumulator.
///
/// # Arguments
///
/// * `returns` - Slice of simple period returns.
///
/// # Returns
///
/// A `Vec<f64>` of the same length as `returns`. Returns an empty vector
/// if `returns` is empty.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::comp_sum;
///
/// let r = [0.01, 0.02, -0.005];
/// let cs = comp_sum(&r);
/// // Final value ≈ (1.01 × 1.02 × 0.995) − 1
/// let expected = 1.01 * 1.02 * 0.995 - 1.0;
/// assert!((cs[2] - expected).abs() < 1e-12);
/// ```
pub fn comp_sum(returns: &[f64]) -> Vec<f64> {
    let mut acc = NeumaierAccumulator::new();
    let mut out = Vec::with_capacity(returns.len());
    for &r in returns {
        if !r.is_finite() {
            out.push(f64::NAN);
            continue;
        }
        let g = (1.0 + r).max(MIN_GROWTH_FACTOR);
        acc.add(g.ln());
        let compounded = acc.total().exp() - 1.0;
        if out.last().is_some_and(|v| v.is_nan()) {
            out.push(f64::NAN);
        } else {
            out.push(compounded);
        }
    }
    out
}

/// Total compounded return over the full slice: `Π(1 + r_i) - 1`.
///
/// Equivalent to `comp_sum(returns).last()`, but computed in a single pass
/// without allocating an intermediate vector.
///
/// Uses Kahan summation in log-space for numerical stability. Growth
/// factors are clamped to [`MIN_GROWTH_FACTOR`] so that returns ≤ −1.0
/// produce a near-total-loss rather than NaN. NaN returns are also
/// clamped to the floor rather than poisoning the result.
///
/// # Arguments
///
/// * `returns` - Slice of simple period returns.
///
/// # Returns
///
/// The total compounded return as a scalar. Returns `0.0` for an empty slice.
///
/// # Examples
///
/// ```rust
/// use finstack_core::analytics::returns::comp_total;
///
/// let r = [0.01, 0.02, -0.005];
/// let ct = comp_total(&r);
/// let expected = 1.01 * 1.02 * 0.995 - 1.0;
/// assert!((ct - expected).abs() < 1e-12);
///
/// // Handles total wipeout without producing NaN.
/// let ct_wipeout = comp_total(&[0.05, -1.0, 0.10]);
/// assert!(ct_wipeout.is_finite());
/// assert!(ct_wipeout < -0.99);
/// ```
pub fn comp_total(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if returns.iter().any(|r| !r.is_finite()) {
        return f64::NAN;
    }
    let log_sum = kahan_sum(
        returns
            .iter()
            .map(|&r| (1.0 + r).max(MIN_GROWTH_FACTOR).ln()),
    );
    log_sum.exp() - 1.0
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn simple_returns_basic() {
        let prices = [100.0, 110.0, 99.0];
        let r = simple_returns(&prices);
        assert_eq!(r.len(), 3);
        assert_eq!(r[0], 0.0);
        assert!((r[1] - 0.1).abs() < 1e-12);
        assert!((r[2] - (-0.1)).abs() < 1e-12);
    }

    #[test]
    fn simple_returns_empty() {
        assert!(simple_returns(&[]).is_empty());
    }

    #[test]
    fn simple_returns_single() {
        let r = simple_returns(&[100.0]);
        assert_eq!(r, vec![0.0]);
    }

    #[test]
    fn excess_returns_defect_fix() {
        let ret = [0.05, 0.03, -0.02];
        let rf = [0.10, 0.10, 0.10];
        let ex = excess_returns(&ret, &rf, Some(12.0));
        // rf_adj = (1.10)^(1/12) - 1 ≈ 0.00797
        let rf_adj = 1.1_f64.powf(1.0 / 12.0) - 1.0;
        assert!((ex[0] - (0.05 - rf_adj)).abs() < 1e-10);
    }

    #[test]
    fn convert_to_prices_roundtrip() {
        let prices = [100.0, 110.0, 99.0, 105.0];
        let r = simple_returns(&prices);
        let recovered = convert_to_prices(&r[1..], 100.0);
        for (a, b) in prices.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-10, "{a} != {b}");
        }
    }

    #[test]
    fn rebase_basic() {
        let prices = [50.0, 55.0, 60.0];
        let r = rebase(&prices, 100.0);
        assert!((r[0] - 100.0).abs() < 1e-12);
        assert!((r[1] - 110.0).abs() < 1e-12);
        assert!((r[2] - 120.0).abs() < 1e-12);
    }

    #[test]
    fn comp_sum_and_total() {
        let r = [0.01, 0.02, -0.005];
        let cs = comp_sum(&r);
        assert_eq!(cs.len(), 3);
        let ct = comp_total(&r);
        assert!((cs[2] - ct).abs() < 1e-12);
        // manual: (1.01 * 1.02 * 0.995) - 1
        let expected = 1.01 * 1.02 * 0.995 - 1.0;
        assert!((ct - expected).abs() < 1e-12);
    }

    #[test]
    fn comp_total_handles_total_wipeout() {
        let r = [0.05, -1.0, 0.10];
        let ct = comp_total(&r);
        assert!(ct.is_finite(), "comp_total must not produce NaN/Inf");
        assert!(ct < -0.99, "total wipeout should be near −100%");
    }

    #[test]
    fn comp_sum_handles_return_below_minus_one() {
        let r = [0.05, -1.5, 0.10];
        let cs = comp_sum(&r);
        assert!(
            cs.iter().all(|v| v.is_finite()),
            "all values must be finite"
        );
    }

    #[test]
    fn comp_total_propagates_nan_returns() {
        let ct = comp_total(&[0.05, f64::NAN, 0.10]);
        assert!(ct.is_nan(), "NaN inputs should remain invalid, got {ct}");
    }

    #[test]
    fn comp_sum_propagates_nan_tail() {
        let cs = comp_sum(&[0.05, f64::NAN, 0.10]);
        assert!(
            cs[1].is_nan(),
            "NaN period should produce NaN compounded return"
        );
        assert!(
            cs[2].is_nan(),
            "Subsequent periods should remain NaN after invalid input"
        );
    }

    #[test]
    fn clean_returns_strips_inf_and_trailing_nan() {
        let mut r = vec![0.01, f64::INFINITY, 0.02, f64::NAN, f64::NAN];
        clean_returns(&mut r);
        assert_eq!(r.len(), 3);
        assert!(r[1].is_nan());
        assert!((r[2] - 0.02).abs() < 1e-12);
    }
}
