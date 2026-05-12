//! Stable public-API invariants that should survive internal refactors.

use finstack_analytics::risk_metrics::{expected_shortfall, tail_ratio, value_at_risk};
use finstack_core::dates::{Date, Duration, Month};
use finstack_core::market_data::term_structures::{PriceCurve, VolatilityIndexCurve};
use serde::Deserialize;

const TOL: f64 = 1e-12;

mod tail_risk_api {
    use super::*;

    fn large_data() -> Vec<f64> {
        (0..201).map(|i| (i as f64 - 100.0) / 100.0).collect()
    }

    #[test]
    fn es_le_var() {
        let data = large_data();
        let confidence = 0.95;
        let var = value_at_risk(&data, confidence);
        let es = expected_shortfall(&data, confidence);
        assert!(es <= var + TOL, "ES must be <= VaR: es={es}, var={var}");
    }

    #[test]
    fn empty_input_consistency() {
        let empty: Vec<f64> = vec![];
        assert_eq!(value_at_risk(&empty, 0.95), 0.0);
        assert_eq!(expected_shortfall(&empty, 0.95), 0.0);
        assert_eq!(tail_ratio(&empty, 0.95), 0.0);
    }
}

mod api_invariants_fixture {
    use super::*;
    use finstack_analytics::{benchmark, risk_metrics};

    const API_INVARIANTS_FIXTURE: &str = include_str!("fixtures/api_invariants_data.json");

    #[derive(Deserialize)]
    struct Fixture {
        returns: Vec<f64>,
        benchmark: Vec<f64>,
        factors: Vec<Vec<f64>>,
        dates: Vec<Date>,
        expected: Expected,
    }

    #[derive(Deserialize)]
    struct Expected {
        cagr_factor: f64,
        sharpe: f64,
        sortino: f64,
        value_at_risk: f64,
        expected_shortfall: f64,
        rolling_greeks: ExpectedRollingGreeks,
        multi_factor_greeks: ExpectedMultiFactorGreeks,
    }

    #[derive(Deserialize)]
    struct ExpectedRollingGreeks {
        alphas: Vec<f64>,
        betas: Vec<f64>,
    }

    #[derive(Deserialize)]
    struct ExpectedMultiFactorGreeks {
        alpha: f64,
        betas: Vec<f64>,
        r_squared: f64,
        adjusted_r_squared: f64,
        residual_vol: f64,
    }

    fn fixture() -> Fixture {
        serde_json::from_str(API_INVARIANTS_FIXTURE).expect("API invariants fixture should parse")
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "actual={actual}, expected={expected}"
        );
    }

    fn assert_vec_close(actual: &[f64], expected: &[f64]) {
        assert_eq!(actual.len(), expected.len());
        for (&actual, &expected) in actual.iter().zip(expected.iter()) {
            assert_close(actual, expected);
        }
    }

    #[test]
    fn rust_core_matches_api_invariants_fixture() {
        let fixture = fixture();
        let expected = &fixture.expected;

        assert_close(
            risk_metrics::cagr(&fixture.returns, risk_metrics::CagrBasis::factor(252.0))
                .expect("valid fixture CAGR"),
            expected.cagr_factor,
        );
        assert_close(risk_metrics::sharpe(0.12, 0.18, 0.02), expected.sharpe);
        assert_close(
            risk_metrics::sortino(&fixture.returns, true, 252.0, 0.0),
            expected.sortino,
        );
        assert_close(
            risk_metrics::value_at_risk(&fixture.returns, 0.95),
            expected.value_at_risk,
        );
        assert_close(
            risk_metrics::expected_shortfall(&fixture.returns, 0.95),
            expected.expected_shortfall,
        );

        let rolling = benchmark::rolling_greeks(
            &fixture.returns,
            &fixture.benchmark,
            &fixture.dates,
            5,
            252.0,
        );
        assert_vec_close(&rolling.alphas, &expected.rolling_greeks.alphas);
        assert_vec_close(&rolling.betas, &expected.rolling_greeks.betas);

        let factor_refs: Vec<&[f64]> = fixture.factors.iter().map(Vec::as_slice).collect();
        let multi = benchmark::multi_factor_greeks(&fixture.returns, &factor_refs, 252.0)
            .expect("valid fixture multi-factor regression");
        assert_close(multi.alpha, expected.multi_factor_greeks.alpha);
        assert_vec_close(&multi.betas, &expected.multi_factor_greeks.betas);
        assert_close(multi.r_squared, expected.multi_factor_greeks.r_squared);
        assert_close(
            multi.adjusted_r_squared,
            expected.multi_factor_greeks.adjusted_r_squared,
        );
        assert_close(
            multi.residual_vol,
            expected.multi_factor_greeks.residual_vol,
        );
    }
}

mod scalar_curve_invariants {
    use super::*;
    use finstack_core::math::interp::InterpStyle;

    fn base_date() -> Date {
        Date::from_calendar_date(2025, Month::January, 1).unwrap()
    }

    fn price_curve() -> PriceCurve {
        PriceCurve::builder("WTI")
            .base_date(base_date())
            .spot_price(80.0)
            .knots([
                (0.0, 80.0),
                (0.25, 81.0),
                (0.5, 82.0),
                (1.0, 83.0),
                (2.0, 84.0),
            ])
            .interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    fn vol_curve() -> VolatilityIndexCurve {
        VolatilityIndexCurve::builder("VIX")
            .base_date(base_date())
            .spot_level(20.0)
            .knots([
                (0.0, 20.0),
                (0.25, 21.0),
                (0.5, 22.0),
                (1.0, 23.0),
                (2.0, 24.0),
            ])
            .interp(InterpStyle::Linear)
            .build()
            .unwrap()
    }

    #[test]
    fn parallel_bump_shifts_all_knots_by_bump_amount() {
        let pc = price_curve();
        let vc = vol_curve();
        let bump = 2.5;

        let bumped_pc = pc.with_parallel_bump(bump).unwrap();
        let bumped_vc = vc.with_parallel_bump(bump).unwrap();

        assert!((bumped_pc.spot_price() - (pc.spot_price() + bump)).abs() < TOL);
        assert!((bumped_vc.spot_level() - (vc.spot_level() + bump)).abs() < TOL);

        for (orig, bumped) in pc.prices().iter().zip(bumped_pc.prices().iter()) {
            assert!((bumped - orig - bump).abs() < TOL);
        }
        for (orig, bumped) in vc.levels().iter().zip(bumped_vc.levels().iter()) {
            assert!((bumped - orig - bump).abs() < TOL);
        }
    }

    #[test]
    fn parallel_bump_zero_is_identity() {
        let pc = price_curve();
        let vc = vol_curve();

        let bumped_pc = pc.with_parallel_bump(0.0).unwrap();
        let bumped_vc = vc.with_parallel_bump(0.0).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0] {
            assert!((bumped_pc.price(t) - pc.price(t)).abs() < TOL);
            assert!((bumped_vc.forward_level(t) - vc.forward_level(t)).abs() < TOL);
        }
    }

    #[test]
    fn parallel_bumps_are_additive() {
        let pc = price_curve();
        let vc = vol_curve();

        let double_bump_pc = pc
            .with_parallel_bump(1.0)
            .unwrap()
            .with_parallel_bump(2.0)
            .unwrap();
        let single_bump_pc = pc.with_parallel_bump(3.0).unwrap();
        let double_bump_vc = vc
            .with_parallel_bump(1.0)
            .unwrap()
            .with_parallel_bump(2.0)
            .unwrap();
        let single_bump_vc = vc.with_parallel_bump(3.0).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0] {
            assert!((double_bump_pc.price(t) - single_bump_pc.price(t)).abs() < TOL);
            assert!(
                (double_bump_vc.forward_level(t) - single_bump_vc.forward_level(t)).abs() < TOL
            );
        }
    }

    #[test]
    fn triangular_key_rate_bump_does_not_shift_spot() {
        let pc = price_curve();
        let vc = vol_curve();

        let bumped_pc = pc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, 3.0)
            .unwrap();
        let bumped_vc = vc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, 3.0)
            .unwrap();

        assert!((bumped_pc.spot_price() - pc.spot_price()).abs() < TOL);
        assert!((bumped_vc.spot_level() - vc.spot_level()).abs() < TOL);
    }

    #[test]
    fn triangular_key_rate_bump_full_at_peak() {
        let pc = price_curve();
        let vc = vol_curve();

        let bumped_pc = pc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, 3.0)
            .unwrap();
        let bumped_vc = vc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, 3.0)
            .unwrap();

        let idx_pc = pc
            .knots()
            .iter()
            .position(|&t| (t - 0.5).abs() < 1e-9)
            .unwrap();
        let idx_vc = vc
            .knots()
            .iter()
            .position(|&t| (t - 0.5).abs() < 1e-9)
            .unwrap();

        assert!((bumped_pc.prices()[idx_pc] - pc.prices()[idx_pc] - 3.0).abs() < TOL);
        assert!((bumped_vc.levels()[idx_vc] - vc.levels()[idx_vc] - 3.0).abs() < TOL);
    }

    #[test]
    fn roll_forward_advances_base_date_and_new_spot() {
        let pc = price_curve();
        let vc = vol_curve();
        let days = 90_i64;

        let rolled_pc = pc.roll_forward(days).unwrap();
        let rolled_vc = vc.roll_forward(days).unwrap();

        let expected_base = base_date() + Duration::days(days);
        assert_eq!(rolled_pc.base_date(), expected_base);
        assert_eq!(rolled_vc.base_date(), expected_base);

        let dt_years = {
            use finstack_core::dates::{DayCount, DayCountContext};
            DayCount::Act365F
                .year_fraction(base_date(), expected_base, DayCountContext::default())
                .unwrap()
        };

        assert!((rolled_pc.spot_price() - pc.price(dt_years)).abs() < TOL);
        assert!((rolled_vc.spot_level() - vc.forward_level(dt_years)).abs() < TOL);
    }

    #[test]
    fn roll_forward_preserves_far_tenor_levels() {
        let rolled_pc = price_curve().roll_forward(30).unwrap();
        let rolled_vc = vol_curve().roll_forward(30).unwrap();

        assert!(rolled_pc.price(1.0) > 0.0);
        assert!(rolled_vc.forward_level(1.0) > 0.0);
    }

    #[test]
    fn price_curve_serde_roundtrip() {
        let pc = price_curve();
        let restored: PriceCurve =
            serde_json::from_str(&serde_json::to_string(&pc).unwrap()).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0, 2.0] {
            assert!((pc.price(t) - restored.price(t)).abs() < TOL);
        }
    }

    #[test]
    fn vol_curve_serde_roundtrip() {
        let vc = vol_curve();
        let restored: VolatilityIndexCurve =
            serde_json::from_str(&serde_json::to_string(&vc).unwrap()).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0, 2.0] {
            assert!((vc.forward_level(t) - restored.forward_level(t)).abs() < TOL);
        }
    }

    #[test]
    fn spot_inference_consistency_across_curve_types() {
        let pc_inferred = PriceCurve::builder("WTI-INF")
            .base_date(base_date())
            .knots([(0.0, 75.0), (0.25, 76.0), (0.5, 77.0), (1.0, 78.0)])
            .build()
            .unwrap();
        let vc_inferred = VolatilityIndexCurve::builder("VIX-INF")
            .base_date(base_date())
            .knots([(0.0, 18.0), (0.25, 19.0), (0.5, 20.0), (1.0, 21.0)])
            .build()
            .unwrap();

        assert!((pc_inferred.spot_price() - 75.0).abs() < TOL);
        assert!((vc_inferred.spot_level() - 18.0).abs() < TOL);
        assert!((pc_inferred.price(0.0) - pc_inferred.spot_price()).abs() < TOL);
        assert!((vc_inferred.forward_level(0.0) - vc_inferred.spot_level()).abs() < TOL);
    }
}
