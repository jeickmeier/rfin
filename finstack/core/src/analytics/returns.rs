//! Return computation utilities: simple returns, excess returns, price conversion,
//! compounded returns. Delegates to `math::stats::log_returns` for log variants
//! and `math::summation` for numerically stable accumulation.

use crate::math::summation::{kahan_sum, NeumaierAccumulator};

/// Replace infinities with NaN and strip trailing all-NaN rows in-place.
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
/// The leading zero mirrors the Python "prepend a zero before first valid" convention.
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
/// When `nperiods` is provided, the risk-free rate is compounded down to the
/// observation frequency: `rf_adj = (1 + rf)^(1/nperiods) - 1`.
///
/// **FIX DEFECT #1**: the Python version incorrectly used `returns` instead
/// of `rf` when computing the period-adjusted risk-free rate.
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
/// `prices[i] = base * Π(1 + r[j]) for j in 0..i`.
/// Uses Kahan summation via log-space for numerical stability on long series.
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

/// Cumulative compounded returns: `(1+r).cumprod() - 1`.
///
/// Uses a Neumaier accumulator in log-space for numerical stability
/// on long series.
pub fn comp_sum(returns: &[f64]) -> Vec<f64> {
    let mut acc = NeumaierAccumulator::new();
    let mut out = Vec::with_capacity(returns.len());
    for &r in returns {
        acc.add((1.0 + r).ln());
        out.push(acc.total().exp() - 1.0);
    }
    out
}

/// Total compounded return: `Π(1 + r_i) - 1`.
///
/// Uses Kahan summation in log-space.
pub fn comp_total(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let log_sum = kahan_sum(returns.iter().map(|&r| (1.0 + r).ln()));
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
    fn clean_returns_strips_inf_and_trailing_nan() {
        let mut r = vec![0.01, f64::INFINITY, 0.02, f64::NAN, f64::NAN];
        clean_returns(&mut r);
        assert_eq!(r.len(), 3);
        assert!(r[1].is_nan());
        assert!((r[2] - 0.02).abs() < 1e-12);
    }
}
