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
        .map(|(&var, &pnl)| {
            if pnl < var {
                Breach::Hit
            } else {
                Breach::Miss
            }
        })
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
/// * `breaches` - Slice of breach indicators from `classify_breaches()`.
/// * `confidence` - VaR confidence level (e.g. 0.99).
///
/// # Returns
///
/// `KupiecResult` with test statistic, p-value, and breach counts.
/// Returns a degenerate result with `NaN` p-value for empty input.
///
/// # References
///
/// - Kupiec (1995): see docs/REFERENCES.md#kupiec1995VaRBacktest
#[must_use]
pub fn kupiec_test(breaches: &[Breach], confidence: f64) -> KupiecResult {
    let t = breaches.len();
    if t == 0 {
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

    let x = breaches.iter().filter(|b| **b == Breach::Hit).count();
    let alpha = 1.0 - confidence;
    let t_f = t as f64;
    let x_f = x as f64;
    let tx = t_f - x_f;

    // Log-likelihood under H0 (alpha is the expected breach rate)
    let ll_h0 = tx * (1.0 - alpha).ln() + x_f * alpha.ln();

    // Log-likelihood under H1 (observed breach rate x/T)
    let p_hat = x_f / t_f;
    let ll_h1 = if x == 0 {
        tx * 1.0_f64.ln() // = 0.0
    } else if x == t {
        x_f * 1.0_f64.ln() // = 0.0
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
        total_observations: t,
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
    let kupiec = kupiec_test(breaches, confidence);

    // Build transition matrix from consecutive observations
    let (n00, n01, n10, n11) = build_transition_matrix(breaches);
    let transition_counts = [n00, n01, n10, n11];

    // Compute independence LR
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

    // Unconditional transition probability
    let pi = (n01 + n11) as f64 / t_adj;

    // Log-likelihood under H0 (independence): same pi for both rows
    let ll_ind_h0 = markov_ll(n00, n01, pi) + markov_ll(n10, n11, pi);

    // Log-likelihood under H1: row-specific transition probabilities
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
/// * `breaches` - Slice of breach indicators.
/// * `confidence` - VaR confidence level (e.g. 0.99).
/// * `window_size` - Evaluation window (e.g. 250).
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
pub fn traffic_light(
    breaches: &[Breach],
    confidence: f64,
    window_size: usize,
) -> TrafficLightResult {
    // Use the last `window_size` observations, or all if fewer
    let window = if breaches.len() > window_size {
        &breaches[breaches.len() - window_size..]
    } else {
        breaches
    };

    let exceptions = window.iter().filter(|b| **b == Breach::Hit).count();

    // Standard Basel thresholds for 250-day / 99% calibration.
    // For non-standard calibrations, scale proportionally.
    let expected = window.len() as f64 * (1.0 - confidence);
    let (green_max, yellow_max) = if window.len() == 250 && confidence == 0.99 {
        (4, 9)
    } else {
        // Scale: green threshold at ~2x expected, red at ~4x expected
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
        window_size: window.len(),
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

#[cfg(test)]
#[allow(clippy::expect_used)]
mod unit_tests {
    use super::*;

    // ---------------------------------------------------------------
    // classify_breaches
    // ---------------------------------------------------------------

    #[test]
    fn classify_breaches_basic() {
        let var = [-0.02, -0.02, -0.02];
        let pnl = [-0.01, -0.03, -0.02];
        let b = classify_breaches(&var, &pnl);
        assert_eq!(b, vec![Breach::Miss, Breach::Hit, Breach::Miss]);
    }

    #[test]
    fn classify_breaches_empty() {
        assert!(classify_breaches(&[], &[]).is_empty());
    }

    #[test]
    fn classify_breaches_length_mismatch() {
        assert!(classify_breaches(&[-0.02], &[-0.01, -0.03]).is_empty());
    }

    #[test]
    fn classify_breaches_all_hits() {
        let var = [-0.01; 5];
        let pnl = [-0.05; 5];
        let b = classify_breaches(&var, &pnl);
        assert!(b.iter().all(|&x| x == Breach::Hit));
    }

    #[test]
    fn classify_breaches_all_misses() {
        let var = [-0.05; 5];
        let pnl = [-0.01; 5];
        let b = classify_breaches(&var, &pnl);
        assert!(b.iter().all(|&x| x == Breach::Miss));
    }

    // ---------------------------------------------------------------
    // kupiec_test
    // ---------------------------------------------------------------

    #[test]
    fn kupiec_empty() {
        let result = kupiec_test(&[], 0.99);
        assert!(result.lr_statistic.is_nan());
        assert!(result.p_value.is_nan());
        assert_eq!(result.breach_count, 0);
        assert!(!result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_exact_expected_rate() {
        // 1000 observations, 10 breaches at 99% VaR => alpha=0.01, expected=10
        let mut breaches = vec![Breach::Miss; 1000];
        // Place 10 hits uniformly
        for i in 0..10 {
            breaches[i * 100] = Breach::Hit;
        }
        let result = kupiec_test(&breaches, 0.99);
        assert_eq!(result.breach_count, 10);
        assert_eq!(result.total_observations, 1000);
        assert!((result.observed_rate - 0.01).abs() < 1e-10);
        assert!((result.expected_count - 10.0).abs() < 1e-10);
        // LR should be near 0, p-value near 1
        assert!(result.lr_statistic.abs() < 1e-10);
        assert!(result.p_value > 0.95);
        assert!(!result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_too_many_breaches_rejects() {
        // 1000 observations, 50 breaches at 99% VaR => 5% rate vs 1% expected
        let mut breaches = vec![Breach::Miss; 1000];
        for i in 0..50 {
            breaches[i * 20] = Breach::Hit;
        }
        let result = kupiec_test(&breaches, 0.99);
        assert_eq!(result.breach_count, 50);
        assert!(result.lr_statistic > 10.0);
        assert!(result.p_value < 0.01);
        assert!(result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_zero_breaches() {
        let breaches = vec![Breach::Miss; 250];
        let result = kupiec_test(&breaches, 0.99);
        assert_eq!(result.breach_count, 0);
        assert_eq!(result.observed_rate, 0.0);
        // With zero breaches at 99%, the model is conservative but LR is not huge
        assert!(result.lr_statistic.is_finite());
    }

    #[test]
    fn kupiec_all_breaches() {
        let breaches = vec![Breach::Hit; 100];
        let result = kupiec_test(&breaches, 0.99);
        assert_eq!(result.breach_count, 100);
        assert!((result.observed_rate - 1.0).abs() < 1e-10);
        assert!(result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_single_observation() {
        let result = kupiec_test(&[Breach::Miss], 0.99);
        assert_eq!(result.total_observations, 1);
        assert_eq!(result.breach_count, 0);
    }

    // ---------------------------------------------------------------
    // christoffersen_test
    // ---------------------------------------------------------------

    #[test]
    fn christoffersen_transition_matrix_hand_check() {
        // Sequence: [Miss, Miss, Hit, Hit, Miss, Hit]
        // Transitions: (M,M),(M,H),(H,H),(H,M),(M,H)
        // Expected: n00=1, n01=2, n10=1, n11=1
        let breaches = [
            Breach::Miss,
            Breach::Miss,
            Breach::Hit,
            Breach::Hit,
            Breach::Miss,
            Breach::Hit,
        ];
        let result = christoffersen_test(&breaches, 0.99);
        assert_eq!(result.transition_counts, [1, 2, 1, 1]);
    }

    #[test]
    fn christoffersen_clustered_breaches_rejects_independence() {
        // 1000 observations, 10 consecutive breaches (clustered)
        let mut breaches = vec![Breach::Miss; 1000];
        for i in 100..110 {
            breaches[i] = Breach::Hit;
        }
        let result = christoffersen_test(&breaches, 0.99);
        // LR_ind should be significantly large due to clustering
        assert!(result.lr_ind > 5.0, "lr_ind={}, expected large", result.lr_ind);
        assert!(result.p_value_ind < 0.05);
    }

    #[test]
    fn christoffersen_scattered_breaches_accepts_independence() {
        // 1000 observations, 10 scattered breaches (independent)
        let mut breaches = vec![Breach::Miss; 1000];
        for i in 0..10 {
            breaches[i * 100] = Breach::Hit;
        }
        let result = christoffersen_test(&breaches, 0.99);
        // LR_ind should be small for scattered breaches
        assert!(
            result.p_value_ind > 0.05,
            "p_value_ind={}, expected > 0.05",
            result.p_value_ind
        );
    }

    #[test]
    fn christoffersen_empty() {
        let result = christoffersen_test(&[], 0.99);
        assert!(result.lr_uc.is_nan());
        assert!(result.lr_ind.is_nan());
        assert!(result.lr_cc.is_nan());
    }

    #[test]
    fn christoffersen_single_element() {
        // Single element => no transitions => NaN for independence
        let result = christoffersen_test(&[Breach::Hit], 0.99);
        assert!(result.lr_ind.is_nan());
    }

    #[test]
    fn christoffersen_joint_is_sum() {
        let mut breaches = vec![Breach::Miss; 500];
        for i in 0..5 {
            breaches[i * 100] = Breach::Hit;
        }
        let result = christoffersen_test(&breaches, 0.99);
        if result.lr_uc.is_finite() && result.lr_ind.is_finite() {
            assert!(
                (result.lr_cc - (result.lr_uc + result.lr_ind)).abs() < 1e-10,
                "LR_cc should equal LR_uc + LR_ind"
            );
        }
    }

    // ---------------------------------------------------------------
    // traffic_light
    // ---------------------------------------------------------------

    #[test]
    fn traffic_light_green_boundary() {
        // 4 exceptions in 250-day window at 99% => Green
        let mut breaches = vec![Breach::Miss; 250];
        for i in 0..4 {
            breaches[i * 60] = Breach::Hit;
        }
        let result = traffic_light(&breaches, 0.99, 250);
        assert_eq!(result.zone, TrafficLightZone::Green);
        assert_eq!(result.exceptions, 4);
        assert!((result.capital_multiplier - 3.0).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_yellow_boundary_low() {
        // 5 exceptions in 250-day window at 99% => Yellow
        let mut breaches = vec![Breach::Miss; 250];
        for i in 0..5 {
            breaches[i * 50] = Breach::Hit;
        }
        let result = traffic_light(&breaches, 0.99, 250);
        assert_eq!(result.zone, TrafficLightZone::Yellow);
        assert_eq!(result.exceptions, 5);
        assert!((result.capital_multiplier - 3.4).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_yellow_boundary_high() {
        // 9 exceptions in 250-day window at 99% => Yellow
        let mut breaches = vec![Breach::Miss; 250];
        for i in 0..9 {
            breaches[i * 27] = Breach::Hit;
        }
        let result = traffic_light(&breaches, 0.99, 250);
        assert_eq!(result.zone, TrafficLightZone::Yellow);
        assert_eq!(result.exceptions, 9);
        assert!((result.capital_multiplier - 3.85).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_red_boundary() {
        // 10 exceptions in 250-day window at 99% => Red
        let mut breaches = vec![Breach::Miss; 250];
        for i in 0..10 {
            breaches[i * 25] = Breach::Hit;
        }
        let result = traffic_light(&breaches, 0.99, 250);
        assert_eq!(result.zone, TrafficLightZone::Red);
        assert_eq!(result.exceptions, 10);
        assert!((result.capital_multiplier - 4.0).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_zero_exceptions() {
        let breaches = vec![Breach::Miss; 250];
        let result = traffic_light(&breaches, 0.99, 250);
        assert_eq!(result.zone, TrafficLightZone::Green);
        assert_eq!(result.exceptions, 0);
    }

    #[test]
    fn traffic_light_uses_last_window() {
        // 300 observations, window=250 => uses last 250
        // Put hits only in the first 50 (outside window)
        let mut breaches = vec![Breach::Miss; 300];
        for i in 0..20 {
            breaches[i] = Breach::Hit;
        }
        let result = traffic_light(&breaches, 0.99, 250);
        // Only the last 250 matter; hits are in first 50, so overlap
        // is breaches[50..300], which has the hits at indices 0..20
        // Since 20 < 50 and window starts at 300-250=50, no hits in window
        assert_eq!(result.exceptions, 0);
        assert_eq!(result.zone, TrafficLightZone::Green);
    }

    #[test]
    fn traffic_light_capital_multipliers_yellow_range() {
        // Verify all yellow multipliers: 5->3.4, 6->3.5, 7->3.65, 8->3.75, 9->3.85
        let expected_multipliers = [(5, 3.4), (6, 3.5), (7, 3.65), (8, 3.75), (9, 3.85)];
        for (exc, mult) in &expected_multipliers {
            let mut breaches = vec![Breach::Miss; 250];
            for i in 0..*exc {
                breaches[i] = Breach::Hit;
            }
            let result = traffic_light(&breaches, 0.99, 250);
            assert_eq!(result.zone, TrafficLightZone::Yellow);
            assert!(
                (result.capital_multiplier - mult).abs() < 1e-10,
                "exceptions={}, expected mult={}, got={}",
                exc,
                mult,
                result.capital_multiplier
            );
        }
    }

    // ---------------------------------------------------------------
    // pnl_explanation
    // ---------------------------------------------------------------

    #[test]
    fn pnl_explanation_constant_diff() {
        // Hypothetical = [1.0, 2.0, 3.0], risk_theoretical = [0.5, 1.5, 2.5]
        // Diff = [0.5, 0.5, 0.5] (constant)
        // VaR = [-1.0, -1.0, -1.0]
        // Explanation ratio = mean(0.5 / -1.0) = -0.5
        // MAD = mean(|0.5|) = 0.5
        // Std = 0.0 (constant diff)
        let hyp = [1.0, 2.0, 3.0];
        let rtp = [0.5, 1.5, 2.5];
        let var = [-1.0, -1.0, -1.0];
        let result = pnl_explanation(&hyp, &rtp, &var);
        assert_eq!(result.n, 3);
        assert!((result.explanation_ratio - (-0.5)).abs() < 1e-10);
        assert!((result.mean_abs_unexplained - 0.5).abs() < 1e-10);
        assert!(result.std_unexplained.abs() < 1e-10);
    }

    #[test]
    fn pnl_explanation_empty() {
        let result = pnl_explanation(&[], &[], &[]);
        assert!(result.explanation_ratio.is_nan());
        assert_eq!(result.n, 0);
    }

    #[test]
    fn pnl_explanation_mismatched_lengths() {
        let result = pnl_explanation(&[1.0, 2.0], &[1.0], &[1.0]);
        assert!(result.explanation_ratio.is_nan());
        assert_eq!(result.n, 0);
    }

    #[test]
    fn pnl_explanation_variable_diff() {
        // Diff = [1.0, -1.0], VaR = [-2.0, -2.0]
        // ratio_sum = (1.0/-2.0) + (-1.0/-2.0) = -0.5 + 0.5 = 0.0
        // MAD = (1.0 + 1.0) / 2 = 1.0
        // mean_unexplained = 0.0
        // var = [(1-0)^2 + (-1-0)^2] / 1 = 2.0
        // std = sqrt(2) ~= 1.414
        let hyp = [2.0, 1.0];
        let rtp = [1.0, 2.0];
        let var = [-2.0, -2.0];
        let result = pnl_explanation(&hyp, &rtp, &var);
        assert_eq!(result.n, 2);
        assert!(result.explanation_ratio.abs() < 1e-10);
        assert!((result.mean_abs_unexplained - 1.0).abs() < 1e-10);
        assert!((result.std_unexplained - 2.0_f64.sqrt()).abs() < 1e-10);
    }

    // ---------------------------------------------------------------
    // Numerical stability
    // ---------------------------------------------------------------

    #[test]
    fn kupiec_large_sample() {
        // 100,000 observations with expected breach rate
        let n = 100_000;
        let mut breaches = vec![Breach::Miss; n];
        // Place ~1000 breaches (1% at 99% VaR)
        for i in 0..1000 {
            breaches[i * 100] = Breach::Hit;
        }
        let result = kupiec_test(&breaches, 0.99);
        assert!(result.lr_statistic.is_finite());
        assert!(result.p_value.is_finite());
        assert!(result.p_value > 0.05);
    }

    #[test]
    fn christoffersen_large_sample() {
        let n = 100_000;
        let mut breaches = vec![Breach::Miss; n];
        for i in 0..1000 {
            breaches[i * 100] = Breach::Hit;
        }
        let result = christoffersen_test(&breaches, 0.99);
        assert!(result.lr_ind.is_finite());
        assert!(result.lr_cc.is_finite());
    }
}
