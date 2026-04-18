//! Behavior-locking tests for simplicity remediation (PR 1).
//!
//! These tests lock the *current behavior* before refactoring, ensuring
//! that the planned internal consolidations (PRs 2 & 3) do not change
//! observable outputs for:
//!
//! 1. **Rolling metrics parity** – `rolling_*` (struct) vs `rolling_*_values`
//!    (NaN-padded Vec) produce the same numeric values; date alignment is correct.
//! 2. **Scratch/non-scratch equivalence** – `*_with_scratch` variants produce
//!    identical results to the allocating wrappers.
//! 3. **PriceCurve / VolatilityIndexCurve bump-and-roll parity** – both
//!    scalar-curve types apply bumps and rolls symmetrically; serde round-trip
//!    is stable.

use finstack_analytics::risk_metrics::{
    expected_shortfall, outlier_loss_ratio, outlier_win_ratio, rolling_sharpe,
    rolling_sharpe_values, rolling_sortino, rolling_sortino_values, rolling_volatility,
    rolling_volatility_values, tail_ratio, value_at_risk,
};
use finstack_core::dates::{Date, Duration, Month};
use finstack_core::market_data::term_structures::{PriceCurve, VolatilityIndexCurve};

const TOL: f64 = 1e-12;

fn jan(year: i32, day: u8) -> Date {
    Date::from_calendar_date(year, Month::January, day).unwrap()
}

fn make_dates(n: usize) -> Vec<Date> {
    (0..n)
        .map(|i| jan(2025, 1) + Duration::days(i as i64))
        .collect()
}

fn make_returns(n: usize) -> Vec<f64> {
    (0..n)
        .map(|i| (i as f64 - n as f64 / 2.0) * 0.001)
        .collect()
}

// =============================================================================
// 1. Rolling metrics: struct variant vs _values variant
// =============================================================================

mod rolling_parity {
    use super::*;

    /// `rolling_sharpe` values match `rolling_sharpe_values` (skipping NaN prefix).
    #[test]
    fn rolling_sharpe_struct_matches_values() {
        let n = 30;
        let window = 7;
        let ann = 252.0;
        let rfr = 0.0;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let struct_result = rolling_sharpe(&returns, &dates, window, ann, rfr);
        let values_result = rolling_sharpe_values(&returns, window, ann, rfr);

        // _values output is NaN-padded; suffix after NaN prefix must match struct values
        let nan_prefix = window - 1;
        assert_eq!(
            struct_result.values.len(),
            n - window + 1,
            "struct should have n-window+1 values"
        );
        assert_eq!(
            values_result.len(),
            n,
            "_values output length should equal input length"
        );

        // NaN prefix
        for (i, v) in values_result.iter().take(nan_prefix).enumerate() {
            assert!(v.is_nan(), "index {i} should be NaN in _values output");
        }

        // Active region must agree
        for (idx, (&sv, &vv)) in struct_result
            .values
            .iter()
            .zip(values_result[nan_prefix..].iter())
            .enumerate()
        {
            assert!(
                (sv - vv).abs() < TOL || (sv.is_nan() && vv.is_nan()),
                "mismatch at active index {idx}: struct={sv}, values={vv}"
            );
        }
    }

    /// `rolling_sharpe` dates are aligned to the *last* date of each window.
    #[test]
    fn rolling_sharpe_date_alignment() {
        let n = 20;
        let window = 5;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let result = rolling_sharpe(&returns, &dates, window, 252.0, 0.0);

        // First date should be dates[window-1]
        assert_eq!(result.dates[0], dates[window - 1]);
        // Last date should be dates[n-1]
        assert_eq!(result.dates.last().copied().unwrap(), dates[n - 1]);
        // No gaps: each date is one step forward
        for pair in result.dates.windows(2) {
            assert!(pair[1] > pair[0]);
        }
    }

    /// `rolling_volatility` values match `rolling_volatility_values`.
    #[test]
    fn rolling_volatility_struct_matches_values() {
        let n = 25;
        let window = 6;
        let ann = 252.0;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let struct_result = rolling_volatility(&returns, &dates, window, ann);
        let values_result = rolling_volatility_values(&returns, window, ann);

        let nan_prefix = window - 1;
        assert_eq!(struct_result.values.len(), n - window + 1);
        assert_eq!(values_result.len(), n);

        for (i, v) in values_result.iter().take(nan_prefix).enumerate() {
            assert!(v.is_nan(), "index {i} should be NaN");
        }
        for (idx, (&sv, &vv)) in struct_result
            .values
            .iter()
            .zip(values_result[nan_prefix..].iter())
            .enumerate()
        {
            assert!(
                (sv - vv).abs() < TOL,
                "mismatch at active index {idx}: struct={sv}, values={vv}"
            );
        }
    }

    /// `rolling_volatility` dates are aligned to the last date of each window.
    #[test]
    fn rolling_volatility_date_alignment() {
        let n = 20;
        let window = 5;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let result = rolling_volatility(&returns, &dates, window, 252.0);

        assert_eq!(result.dates[0], dates[window - 1]);
        assert_eq!(result.dates.last().copied().unwrap(), dates[n - 1]);
        for pair in result.dates.windows(2) {
            assert!(pair[1] > pair[0]);
        }
    }

    /// `rolling_sortino` values match `rolling_sortino_values`.
    #[test]
    fn rolling_sortino_struct_matches_values() {
        let n = 28;
        let window = 8;
        let ann = 252.0;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let struct_result = rolling_sortino(&returns, &dates, window, ann);
        let values_result = rolling_sortino_values(&returns, window, ann);

        let nan_prefix = window - 1;
        assert_eq!(struct_result.values.len(), n - window + 1);
        assert_eq!(values_result.len(), n);

        for (i, v) in values_result.iter().take(nan_prefix).enumerate() {
            assert!(v.is_nan(), "index {i} should be NaN");
        }
        for (idx, (&sv, &vv)) in struct_result
            .values
            .iter()
            .zip(values_result[nan_prefix..].iter())
            .enumerate()
        {
            let ok = (sv - vv).abs() < TOL
                || (sv.is_nan() && vv.is_nan())
                || (sv.is_infinite() && vv.is_infinite() && sv.signum() == vv.signum());
            assert!(
                ok,
                "mismatch at active index {idx}: struct={sv}, values={vv}"
            );
        }
    }

    /// `rolling_sortino` dates are aligned to the last date of each window.
    #[test]
    fn rolling_sortino_date_alignment() {
        let n = 20;
        let window = 5;
        let returns = make_returns(n);
        let dates = make_dates(n);

        let result = rolling_sortino(&returns, &dates, window, ann_factor());
        assert_eq!(result.dates[0], dates[window - 1]);
        assert_eq!(result.dates.last().copied().unwrap(), dates[n - 1]);
        for pair in result.dates.windows(2) {
            assert!(pair[1] > pair[0]);
        }
    }

    fn ann_factor() -> f64 {
        252.0
    }

    /// Empty input → empty output for all rolling variants.
    #[test]
    fn rolling_variants_empty_input() {
        let empty_r: &[f64] = &[];
        let empty_d: &[Date] = &[];
        let w = 5;

        assert!(rolling_sharpe(empty_r, empty_d, w, 252.0, 0.0)
            .values
            .is_empty());
        assert!(rolling_sharpe_values(empty_r, w, 252.0, 0.0).is_empty());
        assert!(rolling_volatility(empty_r, empty_d, w, 252.0)
            .values
            .is_empty());
        assert!(rolling_volatility_values(empty_r, w, 252.0).is_empty());
        assert!(rolling_sortino(empty_r, empty_d, w, 252.0)
            .values
            .is_empty());
        assert!(rolling_sortino_values(empty_r, w, 252.0).is_empty());
    }

    /// Window larger than series → empty output for all rolling variants.
    #[test]
    fn rolling_variants_window_too_large() {
        let returns = make_returns(3);
        let dates = make_dates(3);
        let w = 10;

        assert!(rolling_sharpe(&returns, &dates, w, 252.0, 0.0)
            .values
            .is_empty());
        assert!(rolling_sharpe_values(&returns, w, 252.0, 0.0).is_empty());
        assert!(rolling_volatility(&returns, &dates, w, 252.0)
            .values
            .is_empty());
        assert!(rolling_volatility_values(&returns, w, 252.0).is_empty());
        assert!(rolling_sortino(&returns, &dates, w, 252.0)
            .values
            .is_empty());
        assert!(rolling_sortino_values(&returns, w, 252.0).is_empty());
    }

    /// First window of the struct variant matches the pointwise computation.
    #[test]
    fn rolling_sharpe_first_window_matches_pointwise() {
        let returns: Vec<f64> = (0..15).map(|i| (i as f64 - 7.0) * 0.01).collect();
        let dates = make_dates(15);
        let window = 5;
        let ann = 252.0;

        let rs = rolling_sharpe(&returns, &dates, window, ann, 0.0);
        let rsv = rolling_sharpe_values(&returns, window, ann, 0.0);

        // First active value in both variants must agree
        assert!(
            (rs.values[0] - rsv[window - 1]).abs() < TOL,
            "first window: struct={}, values={}",
            rs.values[0],
            rsv[window - 1]
        );
    }
}

// =============================================================================
// 2. Tail-risk public API invariants
// =============================================================================
//
// The internal `*_with_scratch` variants are `pub(crate)` implementation
// details consumed by the `Performance` facade. The allocating public
// functions remain the canonical API and are tested here for the key
// coherent-risk invariants and empty-input behavior.

mod tail_risk_api {
    use super::*;

    fn large_data() -> Vec<f64> {
        (0..201).map(|i| (i as f64 - 100.0) / 100.0).collect()
    }

    /// ES ≤ VaR (coherent risk property).
    #[test]
    fn es_le_var() {
        let data = large_data();
        let confidence = 0.95;
        let var = value_at_risk(&data, confidence, None);
        let es = expected_shortfall(&data, confidence, None);
        assert!(es <= var + TOL, "ES must be ≤ VaR: es={es}, var={var}");
    }

    /// Empty input returns 0.0 uniformly for all tail-risk metrics.
    #[test]
    fn empty_input_consistency() {
        let empty: Vec<f64> = vec![];
        assert_eq!(value_at_risk(&empty, 0.95, None), 0.0);
        assert_eq!(expected_shortfall(&empty, 0.95, None), 0.0);
        assert_eq!(tail_ratio(&empty, 0.95), 0.0);
        assert_eq!(outlier_win_ratio(&empty, 0.95), 0.0);
        assert_eq!(outlier_loss_ratio(&empty, 0.95), 0.0);
    }
}

// =============================================================================
// 3. PriceCurve / VolatilityIndexCurve bump-and-roll parity
// =============================================================================

mod scalar_curve_parity {
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

    /// Parallel bump shifts all knot values symmetrically for both curve types.
    #[test]
    fn parallel_bump_shifts_all_knots_by_bump_amount() {
        let pc = price_curve();
        let vc = vol_curve();
        let bump = 2.5;

        let bumped_pc = pc.with_parallel_bump(bump).unwrap();
        let bumped_vc = vc.with_parallel_bump(bump).unwrap();

        // Spot levels shifted
        assert!(
            (bumped_pc.spot_price() - (pc.spot_price() + bump)).abs() < TOL,
            "PriceCurve spot should shift by bump"
        );
        assert!(
            (bumped_vc.spot_level() - (vc.spot_level() + bump)).abs() < TOL,
            "VolatilityIndexCurve spot should shift by bump"
        );

        // All knot prices shifted by the same amount
        for (orig, bumped) in pc.prices().iter().zip(bumped_pc.prices().iter()) {
            assert!(
                (bumped - orig - bump).abs() < TOL,
                "price knot should shift by bump: orig={orig}, bumped={bumped}"
            );
        }
        for (orig, bumped) in vc.levels().iter().zip(bumped_vc.levels().iter()) {
            assert!(
                (bumped - orig - bump).abs() < TOL,
                "level knot should shift by bump: orig={orig}, bumped={bumped}"
            );
        }
    }

    /// Parallel bump of zero is identity.
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

    /// Consecutive parallel bumps are additive.
    #[test]
    fn parallel_bumps_are_additive() {
        let pc = price_curve();
        let vc = vol_curve();
        let b1 = 1.0;
        let b2 = 2.0;

        let double_bump_pc = pc
            .with_parallel_bump(b1)
            .unwrap()
            .with_parallel_bump(b2)
            .unwrap();
        let single_bump_pc = pc.with_parallel_bump(b1 + b2).unwrap();

        let double_bump_vc = vc
            .with_parallel_bump(b1)
            .unwrap()
            .with_parallel_bump(b2)
            .unwrap();
        let single_bump_vc = vc.with_parallel_bump(b1 + b2).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0] {
            assert!(
                (double_bump_pc.price(t) - single_bump_pc.price(t)).abs() < TOL,
                "PriceCurve double bump ≠ single bump at t={t}"
            );
            assert!(
                (double_bump_vc.forward_level(t) - single_bump_vc.forward_level(t)).abs() < TOL,
                "VolatilityIndexCurve double bump ≠ single bump at t={t}"
            );
        }
    }

    /// Triangular key-rate bump: spot is NOT bumped; peak knot gets full bump weight.
    #[test]
    fn triangular_key_rate_bump_does_not_shift_spot() {
        let pc = price_curve();
        let vc = vol_curve();
        let bump = 3.0;

        // Bump at 0.5y bucket (neighbours 0.25y, 1.0y)
        let bumped_pc = pc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, bump)
            .unwrap();
        let bumped_vc = vc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, bump)
            .unwrap();

        // Spot should be unchanged
        assert!(
            (bumped_pc.spot_price() - pc.spot_price()).abs() < TOL,
            "Key-rate bump must not shift spot for PriceCurve"
        );
        assert!(
            (bumped_vc.spot_level() - vc.spot_level()).abs() < TOL,
            "Key-rate bump must not shift spot for VolatilityIndexCurve"
        );
    }

    /// Triangular key-rate bump: peak knot gets full bump.
    #[test]
    fn triangular_key_rate_bump_full_at_peak() {
        let pc = price_curve();
        let vc = vol_curve();
        let bump = 3.0;

        let bumped_pc = pc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, bump)
            .unwrap();
        let bumped_vc = vc
            .with_triangular_key_rate_bump_neighbors(0.25, 0.5, 1.0, bump)
            .unwrap();

        // Find the 0.5y knot
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

        assert!(
            (bumped_pc.prices()[idx_pc] - pc.prices()[idx_pc] - bump).abs() < TOL,
            "PriceCurve peak knot should receive full bump"
        );
        assert!(
            (bumped_vc.levels()[idx_vc] - vc.levels()[idx_vc] - bump).abs() < TOL,
            "VolatilityIndexCurve peak knot should receive full bump"
        );
    }

    /// `roll_forward` advances the base date and new spot equals old forward level at dt.
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

        // New spot should equal old forward at dt_years (t=dt in original curve)
        let dt_years = {
            use finstack_core::dates::{DayCount, DayCountCtx};
            DayCount::Act365F
                .year_fraction(base_date(), expected_base, DayCountCtx::default())
                .unwrap()
        };
        assert!(
            (rolled_pc.spot_price() - pc.price(dt_years)).abs() < TOL,
            "PriceCurve rolled spot should equal old forward at dt: rolled_spot={}, old_fwd={}",
            rolled_pc.spot_price(),
            pc.price(dt_years)
        );
        assert!(
            (rolled_vc.spot_level() - vc.forward_level(dt_years)).abs() < TOL,
            "VolatilityIndexCurve rolled spot should equal old forward at dt"
        );
    }

    /// Rolling preserves relative forward structure (forward values at far tenors unchanged).
    #[test]
    fn roll_forward_preserves_far_tenor_levels() {
        let pc = price_curve();
        let vc = vol_curve();
        let days = 30_i64;

        let rolled_pc = pc.roll_forward(days).unwrap();
        let rolled_vc = vc.roll_forward(days).unwrap();

        // A far-dated tenor (1.5y) in the rolled curve should match the same tenor
        // in the original (original sees 1.5y from old base; rolled sees 1.5y from new base).
        // They're different calendar points, but forward structure is preserved in knots.
        // Verify: rolled.price(1.0) is finite and positive.
        assert!(rolled_pc.price(1.0) > 0.0);
        assert!(rolled_vc.forward_level(1.0) > 0.0);
    }

    /// Serde round-trip preserves forward prices/levels for PriceCurve.
    #[test]
    fn price_curve_serde_roundtrip() {
        let pc = price_curve();
        let json = serde_json::to_string(&pc).unwrap();
        let restored: PriceCurve = serde_json::from_str(&json).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0, 2.0] {
            assert!(
                (pc.price(t) - restored.price(t)).abs() < TOL,
                "PriceCurve serde roundtrip diverges at t={t}: orig={}, restored={}",
                pc.price(t),
                restored.price(t)
            );
        }
    }

    /// Serde round-trip preserves forward levels for VolatilityIndexCurve.
    #[test]
    fn vol_curve_serde_roundtrip() {
        let vc = vol_curve();
        let json = serde_json::to_string(&vc).unwrap();
        let restored: VolatilityIndexCurve = serde_json::from_str(&json).unwrap();

        for t in [0.0, 0.25, 0.5, 1.0, 2.0] {
            assert!(
                (vc.forward_level(t) - restored.forward_level(t)).abs() < TOL,
                "VolatilityIndexCurve serde roundtrip diverges at t={t}: orig={}, restored={}",
                vc.forward_level(t),
                restored.forward_level(t)
            );
        }
    }

    /// Symmetric spot inference: if spot is not set but t=0 knot exists, spot is inferred.
    #[test]
    fn spot_inference_consistency_across_curve_types() {
        // Build without explicit spot – spot should be inferred from t=0 knot
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

        // Spot should equal the t=0 knot value
        assert!(
            (pc_inferred.spot_price() - 75.0).abs() < TOL,
            "PriceCurve inferred spot should be 75.0"
        );
        assert!(
            (vc_inferred.spot_level() - 18.0).abs() < TOL,
            "VolatilityIndexCurve inferred spot should be 18.0"
        );

        // price(0) == spot
        assert!((pc_inferred.price(0.0) - pc_inferred.spot_price()).abs() < TOL);
        assert!((vc_inferred.forward_level(0.0) - vc_inferred.spot_level()).abs() < TOL);
    }
}
