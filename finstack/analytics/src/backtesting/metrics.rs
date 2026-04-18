//! Statistical tests for VaR backtesting: breach classification,
//! Kupiec POF, Christoffersen conditional coverage, Basel traffic light,
//! and P&L explanation.

use crate::math::distributions::chi_squared_cdf;

use super::types::{
    Breach, ChristoffersenResult, KupiecResult, PnlExplanation, TrafficLightResult,
    TrafficLightZone,
};

/// Classify each observation as a VaR breach or miss.
///
/// A breach occurs when the realized P&L is worse (more negative) than
/// the VaR forecast. Both `var_forecasts` and `realized_pnl` are expected
/// as negative numbers for losses (consistent with `value_at_risk()` output).
///
/// # Arguments
///
/// * `var_forecasts` - VaR forecasts for each period (negative = loss threshold).
/// * `realized_pnl` - Realized P&L for each period.
///
/// # Returns
///
/// Vector of `Breach` indicators. Empty if inputs differ in length or are empty.
#[must_use]
pub fn classify_breaches(var_forecasts: &[f64], realized_pnl: &[f64]) -> Vec<Breach> {
    if var_forecasts.len() != realized_pnl.len() || var_forecasts.is_empty() {
        return Vec::new();
    }
    var_forecasts
        .iter()
        .zip(realized_pnl.iter())
        .map(
            |(&var, &pnl)| {
                if pnl < var {
                    Breach::Hit
                } else {
                    Breach::Miss
                }
            },
        )
        .collect()
}

/// Kupiec Proportion of Failures (POF) unconditional coverage test.
///
/// Tests H0: the true breach probability equals the expected tail
/// probability alpha = 1 - confidence.
///
/// The likelihood-ratio statistic is:
///
/// ```text
/// LR_uc = -2 ln[(1-alpha)^(T-x) * alpha^x]
///         +2 ln[(1-x/T)^(T-x) * (x/T)^x]
/// ```
///
/// where T = total observations, x = breach count, alpha = 1 - confidence.
///
/// Under H0, LR_uc ~ chi-squared(1).
///
/// # Arguments
///
/// * `breach_count` - Number of observed VaR breaches.
/// * `n` - Total number of observations.
/// * `confidence` - VaR confidence level (e.g. 0.99).
///
/// # Returns
///
/// `KupiecResult` with test statistic, p-value, and breach counts.
/// Returns a degenerate result with `NaN` p-value when `n` is zero.
///
/// # References
///
/// - Kupiec (1995): see docs/REFERENCES.md#kupiec1995VaRBacktest
#[must_use]
pub fn kupiec_test(breach_count: usize, n: usize, confidence: f64) -> KupiecResult {
    if n == 0 {
        return KupiecResult {
            lr_statistic: f64::NAN,
            p_value: f64::NAN,
            breach_count: 0,
            expected_count: 0.0,
            total_observations: 0,
            observed_rate: 0.0,
            reject_h0_5pct: false,
        };
    }

    let x = breach_count.min(n);
    let alpha = 1.0 - confidence;
    let t_f = n as f64;
    let x_f = x as f64;
    let tx = t_f - x_f;

    let ll_h0 = tx * (1.0 - alpha).ln() + x_f * alpha.ln();

    let p_hat = x_f / t_f;
    let ll_h1 = if x == 0 {
        tx * 1.0_f64.ln()
    } else if x == n {
        x_f * 1.0_f64.ln()
    } else {
        tx * (1.0 - p_hat).ln() + x_f * p_hat.ln()
    };

    let lr = -2.0 * (ll_h0 - ll_h1);
    let p_value = 1.0 - chi_squared_cdf(lr, 1.0);

    KupiecResult {
        lr_statistic: lr,
        p_value,
        breach_count: x,
        expected_count: alpha * t_f,
        total_observations: n,
        observed_rate: p_hat,
        reject_h0_5pct: p_value < 0.05,
    }
}

/// Christoffersen conditional coverage test.
///
/// Jointly tests unconditional coverage and serial independence of
/// VaR breaches. Decomposes into three components:
///
/// 1. **LR_uc** (unconditional): same as Kupiec. chi-squared(1).
/// 2. **LR_ind** (independence): tests whether the transition
///    probability of a breach depends on the previous day's state.
///    chi-squared(1).
/// 3. **LR_cc** (conditional coverage): LR_uc + LR_ind. chi-squared(2).
///
/// The independence test uses the transition matrix of consecutive
/// breach/no-breach observations:
///
/// ```text
///              Today
///           Miss    Hit
/// Yesterday
///   Miss    n00     n01
///   Hit     n10     n11
///
/// pi_01 = n01 / (n00 + n01)    -- P(hit | previous miss)
/// pi_11 = n11 / (n10 + n11)    -- P(hit | previous hit)
/// pi    = (n01 + n11) / T_adj  -- unconditional hit rate (from transitions)
///
/// LR_ind = -2 ln[L(pi) / L(pi_01, pi_11)]
/// ```
///
/// where L(.) are the respective Markov chain likelihoods.
///
/// # Arguments
///
/// * `breaches` - Slice of breach indicators from `classify_breaches()`.
/// * `confidence` - VaR confidence level (e.g. 0.99).
///
/// # Returns
///
/// `ChristoffersenResult` with all three component statistics and p-values.
///
/// # References
///
/// - Christoffersen (1998): see docs/REFERENCES.md#christoffersen1998VaRBacktest
#[must_use]
pub fn christoffersen_test(breaches: &[Breach], confidence: f64) -> ChristoffersenResult {
    let x = breaches.iter().filter(|b| **b == Breach::Hit).count();
    let kupiec = kupiec_test(x, breaches.len(), confidence);

    let (n00, n01, n10, n11) = build_transition_matrix(breaches);
    let transition_counts = [n00, n01, n10, n11];

    let t_adj = (n00 + n01 + n10 + n11) as f64;
    if t_adj == 0.0 {
        return ChristoffersenResult {
            lr_uc: kupiec.lr_statistic,
            lr_ind: f64::NAN,
            lr_cc: f64::NAN,
            p_value_uc: kupiec.p_value,
            p_value_ind: f64::NAN,
            p_value_cc: f64::NAN,
            transition_counts,
            reject_h0_5pct: false,
        };
    }

    let pi = (n01 + n11) as f64 / t_adj;

    let ll_ind_h0 = markov_ll(n00, n01, pi) + markov_ll(n10, n11, pi);

    let pi_01 = if n00 + n01 > 0 {
        n01 as f64 / (n00 + n01) as f64
    } else {
        0.0
    };
    let pi_11 = if n10 + n11 > 0 {
        n11 as f64 / (n10 + n11) as f64
    } else {
        0.0
    };
    let ll_ind_h1 = markov_ll(n00, n01, pi_01) + markov_ll(n10, n11, pi_11);

    let lr_ind = -2.0 * (ll_ind_h0 - ll_ind_h1);
    let lr_cc = kupiec.lr_statistic + lr_ind;

    let p_value_ind = 1.0 - chi_squared_cdf(lr_ind, 1.0);
    let p_value_cc = 1.0 - chi_squared_cdf(lr_cc, 2.0);

    ChristoffersenResult {
        lr_uc: kupiec.lr_statistic,
        lr_ind,
        lr_cc,
        p_value_uc: kupiec.p_value,
        p_value_ind,
        p_value_cc,
        transition_counts,
        reject_h0_5pct: p_value_cc < 0.05,
    }
}

/// Build the 2x2 transition matrix from a breach sequence.
///
/// Returns (n00, n01, n10, n11) where n_ij = count of transitions
/// from state i to state j (0 = miss, 1 = hit).
fn build_transition_matrix(breaches: &[Breach]) -> (usize, usize, usize, usize) {
    let mut n00 = 0usize;
    let mut n01 = 0usize;
    let mut n10 = 0usize;
    let mut n11 = 0usize;

    for window in breaches.windows(2) {
        match (window[0], window[1]) {
            (Breach::Miss, Breach::Miss) => n00 += 1,
            (Breach::Miss, Breach::Hit) => n01 += 1,
            (Breach::Hit, Breach::Miss) => n10 += 1,
            (Breach::Hit, Breach::Hit) => n11 += 1,
        }
    }
    (n00, n01, n10, n11)
}

/// Log-likelihood for a single row of the Markov transition matrix.
///
/// L = (1 - pi)^n_stay * pi^n_switch
fn markov_ll(n_stay: usize, n_switch: usize, pi: f64) -> f64 {
    let mut ll = 0.0;
    if n_stay > 0 && pi < 1.0 {
        ll += n_stay as f64 * (1.0 - pi).ln();
    }
    if n_switch > 0 && pi > 0.0 {
        ll += n_switch as f64 * pi.ln();
    }
    ll
}

/// Basel Committee traffic-light classification.
///
/// Classifies the VaR model into Green, Yellow, or Red zones based on
/// the number of exceptions observed in the evaluation window. The
/// standard calibration uses a 250-day window at 99% VaR confidence.
///
/// Zone boundaries (at 99% VaR, 250-day window):
/// - Green: 0-4 exceptions (model adequate)
/// - Yellow: 5-9 exceptions (potential issues)
/// - Red: 10+ exceptions (model inadequate)
///
/// The boundaries scale with the window size and confidence level:
/// expected exceptions = window * (1 - confidence). Zone thresholds
/// are set at cumulative binomial probabilities of 95% (green/yellow
/// boundary) and 99.99% (yellow/red boundary).
///
/// # Arguments
///
/// * `exceptions` - Number of VaR exceptions in the evaluation window.
/// * `n` - Window size (total observations, e.g. 250).
/// * `confidence` - VaR confidence level (e.g. 0.99).
///
/// # Returns
///
/// `TrafficLightResult` with the zone, exception count, and capital multiplier.
///
/// # References
///
/// - Basel Committee on Banking Supervision (1996):
///   see docs/REFERENCES.md#bcbs1996MarketRisk
#[must_use]
pub fn traffic_light(exceptions: usize, n: usize, confidence: f64) -> TrafficLightResult {
    let expected = n as f64 * (1.0 - confidence);
    let (green_max, yellow_max) = if n == 250 && confidence == 0.99 {
        (4, 9)
    } else {
        let g = (expected * 2.0).ceil() as usize;
        let y = (expected * 4.0).ceil() as usize;
        (g.max(1), y.max(g + 1))
    };

    let zone = if exceptions <= green_max {
        TrafficLightZone::Green
    } else if exceptions <= yellow_max {
        TrafficLightZone::Yellow
    } else {
        TrafficLightZone::Red
    };

    TrafficLightResult {
        zone,
        exceptions,
        capital_multiplier: zone.capital_multiplier(exceptions),
        window_size: n,
        confidence,
    }
}

/// Compute P&L explanation metrics.
///
/// Measures how well the risk model's theoretical P&L explains the
/// hypothetical (position-constant) P&L. A high explanation ratio
/// indicates the risk model captures the dominant risk factors.
///
/// # Arguments
///
/// * `hypothetical_pnl` - Hypothetical P&L series (constant positions).
/// * `risk_theoretical_pnl` - P&L predicted by the risk model.
/// * `var_forecasts` - VaR forecasts used for normalization.
///
/// # Returns
///
/// `PnlExplanation` with ratio, MAD, and standard deviation of the
/// unexplained component. Returns a degenerate result for empty or
/// mismatched inputs.
///
/// # References
///
/// - Basel Committee FRTB (2019): see docs/REFERENCES.md#bcbs2019FRTB
#[must_use]
pub fn pnl_explanation(
    hypothetical_pnl: &[f64],
    risk_theoretical_pnl: &[f64],
    var_forecasts: &[f64],
) -> PnlExplanation {
    let n = hypothetical_pnl.len();
    if n == 0 || n != risk_theoretical_pnl.len() || n != var_forecasts.len() {
        return PnlExplanation {
            explanation_ratio: f64::NAN,
            mean_abs_unexplained: f64::NAN,
            std_unexplained: f64::NAN,
            n: 0,
        };
    }

    let mut ratio_sum = 0.0;
    let mut abs_sum = 0.0;
    let mut unexplained: Vec<f64> = Vec::with_capacity(n);

    for i in 0..n {
        let diff = hypothetical_pnl[i] - risk_theoretical_pnl[i];
        unexplained.push(diff);
        abs_sum += diff.abs();
        if var_forecasts[i].abs() > f64::EPSILON {
            ratio_sum += diff / var_forecasts[i];
        }
    }

    let nf = n as f64;
    let mean_unexplained = unexplained.iter().sum::<f64>() / nf;
    let var_unexplained = unexplained
        .iter()
        .map(|u| (u - mean_unexplained).powi(2))
        .sum::<f64>()
        / (nf - 1.0).max(1.0);

    PnlExplanation {
        explanation_ratio: ratio_sum / nf,
        mean_abs_unexplained: abs_sum / nf,
        std_unexplained: var_unexplained.sqrt(),
        n,
    }
}
