//! Test coverage for VaR backtesting metrics and orchestrators.

#[cfg(test)]
#[allow(clippy::expect_used)]
mod unit_tests {
    use crate::backtesting::{
        christoffersen_test, classify_breaches, kupiec_test, pnl_explanation, traffic_light,
        Breach, TrafficLightZone,
    };

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

    #[test]
    fn classify_breaches_at_var_boundary_is_miss() {
        let var = [-0.02, -0.03];
        let pnl = [-0.02, -0.03];
        let b = classify_breaches(&var, &pnl);
        assert_eq!(b, vec![Breach::Miss, Breach::Miss]);
    }

    // ---------------------------------------------------------------
    // kupiec_test
    // ---------------------------------------------------------------

    #[test]
    fn kupiec_empty() {
        let result = kupiec_test(0, 0, 0.99);
        assert!(result.lr_statistic.is_nan());
        assert!(result.p_value.is_nan());
        assert_eq!(result.breach_count, 0);
        assert!(!result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_exact_expected_rate() {
        // 1000 observations, 10 breaches at 99% VaR => alpha=0.01, expected=10
        let result = kupiec_test(10, 1000, 0.99);
        assert_eq!(result.breach_count, 10);
        assert_eq!(result.total_observations, 1000);
        assert!((result.observed_rate - 0.01).abs() < 1e-10);
        assert!((result.expected_count - 10.0).abs() < 1e-10);
        assert!(result.lr_statistic.abs() < 1e-10);
        assert!(result.p_value > 0.95);
        assert!(!result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_too_many_breaches_rejects() {
        // 1000 observations, 50 breaches at 99% VaR => 5% rate vs 1% expected
        let result = kupiec_test(50, 1000, 0.99);
        assert_eq!(result.breach_count, 50);
        assert!(result.lr_statistic > 10.0);
        assert!(result.p_value < 0.01);
        assert!(result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_zero_breaches() {
        let result = kupiec_test(0, 250, 0.99);
        assert_eq!(result.breach_count, 0);
        assert_eq!(result.observed_rate, 0.0);
        assert!(result.lr_statistic.is_finite());
    }

    #[test]
    fn kupiec_all_breaches() {
        let result = kupiec_test(100, 100, 0.99);
        assert_eq!(result.breach_count, 100);
        assert!((result.observed_rate - 1.0).abs() < 1e-10);
        assert!(result.reject_h0_5pct);
    }

    #[test]
    fn kupiec_single_observation() {
        let result = kupiec_test(0, 1, 0.99);
        assert_eq!(result.total_observations, 1);
        assert_eq!(result.breach_count, 0);
    }

    #[test]
    fn kupiec_invalid_confidence_returns_nan() {
        let result = kupiec_test(1, 100, 1.0);
        assert!(result.lr_statistic.is_nan());
        assert!(result.p_value.is_nan());
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
        for breach in breaches.iter_mut().take(110).skip(100) {
            *breach = Breach::Hit;
        }
        let result = christoffersen_test(&breaches, 0.99);
        // LR_ind should be significantly large due to clustering
        assert!(
            result.lr_ind > 5.0,
            "lr_ind={}, expected large",
            result.lr_ind
        );
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
        let result = traffic_light(4, 250, 0.99);
        assert_eq!(result.zone, TrafficLightZone::Green);
        assert_eq!(result.exceptions, 4);
        assert!((result.capital_multiplier - 3.0).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_yellow_boundary_low() {
        let result = traffic_light(5, 250, 0.99);
        assert_eq!(result.zone, TrafficLightZone::Yellow);
        assert_eq!(result.exceptions, 5);
        assert!((result.capital_multiplier - 3.4).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_yellow_boundary_high() {
        let result = traffic_light(9, 250, 0.99);
        assert_eq!(result.zone, TrafficLightZone::Yellow);
        assert_eq!(result.exceptions, 9);
        assert!((result.capital_multiplier - 3.85).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_red_boundary() {
        let result = traffic_light(10, 250, 0.99);
        assert_eq!(result.zone, TrafficLightZone::Red);
        assert_eq!(result.exceptions, 10);
        assert!((result.capital_multiplier - 4.0).abs() < 1e-10);
    }

    #[test]
    fn traffic_light_zero_exceptions() {
        let result = traffic_light(0, 250, 0.99);
        assert_eq!(result.zone, TrafficLightZone::Green);
        assert_eq!(result.exceptions, 0);
    }

    #[test]
    fn traffic_light_capital_multipliers_yellow_range() {
        let expected_multipliers = [(5, 3.4), (6, 3.5), (7, 3.65), (8, 3.75), (9, 3.85)];
        for (exc, mult) in &expected_multipliers {
            let result = traffic_light(*exc, 250, 0.99);
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

    #[test]
    fn pnl_explanation_reports_aggregate_ratio() {
        let hyp = [2.0, 11.0];
        let rtp = [1.0, 10.0];
        let var = [-1.0, -10.0];
        let result = pnl_explanation(&hyp, &rtp, &var);

        assert!((result.explanation_ratio - (-0.55)).abs() < 1e-10);
        assert!((result.aggregate_explanation_ratio - (-2.0 / 11.0)).abs() < 1e-10);
    }

    // ---------------------------------------------------------------
    // Numerical stability
    // ---------------------------------------------------------------

    #[test]
    fn kupiec_large_sample() {
        let result = kupiec_test(1000, 100_000, 0.99);
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
