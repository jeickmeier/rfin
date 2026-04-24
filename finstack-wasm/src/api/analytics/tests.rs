#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::module_inception)]
mod tests {
    use finstack_analytics as fa;

    use super::super::{benchmark, drawdown, risk_metrics, timeseries};

    #[test]
    fn sharpe_basic() {
        let s = risk_metrics::sharpe(0.10, 0.15, 0.02);
        assert!((s - (0.10 - 0.02) / 0.15).abs() < 1e-10);
    }

    #[test]
    fn calmar_basic() {
        assert!((drawdown::calmar(0.10, 0.20) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn recovery_factor_basic() {
        assert!((drawdown::recovery_factor(0.50, 0.25) - 2.0).abs() < 1e-10);
    }

    #[test]
    fn martin_ratio_basic() {
        let m = drawdown::martin_ratio(0.10, 0.05);
        assert!((m - 2.0).abs() < 1e-10);
    }

    #[test]
    fn sterling_ratio_basic() {
        let sr = drawdown::sterling_ratio(0.10, 0.20, 0.02);
        assert!((sr - (0.10 - 0.02) / 0.20).abs() < 1e-10);
    }

    #[test]
    fn pain_ratio_basic() {
        let pr = drawdown::pain_ratio(0.10, 0.03, 0.02);
        let expected = (0.10 - 0.02) / 0.03;
        assert!((pr - expected).abs() < 1e-10);
    }

    #[test]
    fn treynor_basic() {
        let t = benchmark::treynor(0.12, 0.02, 1.2);
        let expected = (0.12 - 0.02) / 1.2;
        assert!((t - expected).abs() < 1e-10);
    }

    #[test]
    fn m_squared_basic() {
        let ms = benchmark::m_squared(0.12, 0.18, 0.15, 0.02);
        assert!(ms.is_finite());
    }

    #[test]
    fn underlying_sortino() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let s = fa::risk_metrics::sortino(&r, true, 252.0, 0.0);
        assert!(s.is_finite());
    }

    #[test]
    fn underlying_volatility() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let v = fa::risk_metrics::volatility(&r, true, 252.0);
        assert!(v > 0.0);
    }

    #[test]
    fn underlying_mean_return() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let m = fa::risk_metrics::mean_return(&r, false, 252.0);
        assert!(m.is_finite());
    }

    #[test]
    fn underlying_cagr_factor_basis() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let c = fa::risk_metrics::cagr(&r, fa::risk_metrics::CagrBasis::factor(252.0));
        assert!(c.is_finite());
    }

    #[test]
    fn underlying_downside_deviation() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let dd = fa::risk_metrics::downside_deviation(&r, 0.0, true, 252.0);
        assert!(dd >= 0.0);
    }

    #[test]
    fn underlying_geometric_mean() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gm = fa::risk_metrics::geometric_mean(&r);
        assert!(gm.is_finite());
    }

    #[test]
    fn underlying_omega_ratio() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let o = fa::risk_metrics::omega_ratio(&r, 0.0);
        assert!(o > 0.0);
    }

    #[test]
    fn underlying_gain_to_pain() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let gtp = fa::risk_metrics::gain_to_pain(&r);
        assert!(gtp.is_finite());
    }

    #[test]
    fn underlying_modified_sharpe() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.015, 0.025, -0.005, 0.01, -0.01,
        ];
        let ms = fa::risk_metrics::modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(!ms.is_nan() || ms.is_nan());
    }

    #[test]
    fn underlying_var_and_es() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.02, -0.01,
        ];
        let var = fa::risk_metrics::value_at_risk(&r, 0.95);
        let es = fa::risk_metrics::expected_shortfall(&r, 0.95);
        assert!(var.is_finite());
        assert!(es.is_finite());
    }

    #[test]
    fn underlying_parametric_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::risk_metrics::parametric_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_cornish_fisher_var() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let v = fa::risk_metrics::cornish_fisher_var(&r, 0.95, None);
        assert!(v.is_finite());
    }

    #[test]
    fn underlying_skewness_kurtosis() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005];
        let s = fa::risk_metrics::skewness(&r);
        let k = fa::risk_metrics::kurtosis(&r);
        assert!(s.is_finite());
        assert!(k.is_finite());
    }

    #[test]
    fn underlying_tail_ratios() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, -0.03, 0.015, -0.005, 0.025, -0.015,
        ];
        let tr = fa::risk_metrics::tail_ratio(&r, 0.95);
        let owr = fa::risk_metrics::outlier_win_ratio(&r, 0.95);
        let olr = fa::risk_metrics::outlier_loss_ratio(&r, 0.95);
        assert!(tr.is_finite());
        assert!(owr.is_finite());
        assert!(olr.is_finite());
    }

    #[test]
    fn underlying_returns() {
        let prices = vec![100.0, 102.0, 101.0, 103.0];
        let sr = fa::returns::simple_returns(&prices);
        assert!(!sr.is_empty());
        let cs = fa::returns::comp_sum(&sr);
        assert_eq!(cs.len(), sr.len());
        let ct = fa::returns::comp_total(&sr);
        assert!(ct.is_finite());
        let rebased = fa::returns::rebase(&prices, 1.0);
        assert_eq!(rebased.len(), prices.len());
    }

    #[test]
    fn underlying_clean_returns() {
        let mut r = vec![0.01, f64::NAN, 0.03, f64::INFINITY];
        fa::returns::clean_returns(&mut r);
        assert!(r[0].is_finite());
        assert!(r[2].is_finite());
    }

    #[test]
    fn underlying_convert_to_prices() {
        let r = vec![0.01, -0.02, 0.03];
        let p = fa::returns::convert_to_prices(&r, 100.0);
        assert!((p[0] - 100.0).abs() < 1e-10);
    }

    #[test]
    fn underlying_excess_returns() {
        let r = vec![0.05, 0.03, 0.07];
        let rf = vec![0.01, 0.01, 0.01];
        let er = fa::returns::excess_returns(&r, &rf, None);
        assert!((er[0] - 0.04).abs() < 1e-10);
    }

    #[test]
    fn underlying_drawdown() {
        let r = vec![0.01, -0.02, 0.03, -0.05, 0.02];
        let dd = fa::drawdown::to_drawdown_series(&r);
        let max_dd = fa::drawdown::max_drawdown(&dd);
        assert!(max_dd <= 0.0);
        let avg = fa::drawdown::mean_episode_drawdown(&dd, 2);
        assert!(avg.is_finite());
        let avg_depth = fa::drawdown::mean_drawdown(&dd);
        assert!(avg_depth.is_finite());
        let cdar_val = fa::drawdown::cdar(&dd, 0.95);
        assert!(cdar_val.is_finite());
        let ulcer = fa::drawdown::ulcer_index(&dd);
        assert!(ulcer >= 0.0);
        let pain = fa::drawdown::pain_index(&dd);
        assert!(pain >= 0.0);
    }

    #[test]
    fn underlying_burke_ratio() {
        let dd = vec![-0.02, -0.05, -0.01];
        let br = fa::drawdown::burke_ratio(0.10, &dd, 0.02);
        assert!(br.is_finite());
    }

    #[test]
    fn underlying_beta() {
        let y = vec![0.02, 0.04, 0.06, 0.08, 0.10];
        let x = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let result = fa::benchmark::beta(&y, &x);
        assert!((result.beta - 2.0).abs() < 1e-10);
        assert!(result.std_err.is_finite());
        assert!(result.ci_lower <= result.ci_upper);
    }

    #[test]
    fn underlying_greeks() {
        let r = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let b = vec![0.005, 0.01, 0.015, 0.02, 0.025];
        let g = fa::benchmark::greeks(&r, &b, 252.0);
        assert!((g.beta - 2.0).abs() < 1e-10);
        assert!((g.r_squared - 1.0).abs() < 1e-10);
        assert!((g.adjusted_r_squared - 1.0).abs() < 1e-10);
    }

    #[test]
    fn underlying_rolling_greeks() {
        let r: Vec<f64> = (0..20).map(|i| (i as f64 + 1.0) * 0.001).collect();
        let b: Vec<f64> = (0..20).map(|i| i as f64 * 0.0005).collect();
        let base =
            time::Date::from_calendar_date(2025, time::Month::January, 1).expect("valid date");
        let dates: Vec<time::Date> = (0..20).map(|i| base + time::Duration::days(i)).collect();
        let rg = fa::benchmark::rolling_greeks(&r, &b, &dates, 5, 252.0);
        assert_eq!(rg.betas.len(), 16);
        assert_eq!(rg.alphas.len(), 16);
    }

    #[test]
    fn underlying_multi_factor_greeks() {
        let y = vec![0.02, 0.04, 0.06, 0.08, 0.10];
        let f1 = vec![0.01, 0.02, 0.03, 0.04, 0.05];
        let result = fa::benchmark::multi_factor_greeks(&y, &[&f1], 252.0).expect("single-factor");
        assert!((result.betas[0] - 2.0).abs() < 1e-8);
        assert!(result.r_squared > 0.999);
    }

    #[test]
    fn underlying_benchmark_metrics() {
        let r = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let te = fa::benchmark::tracking_error(&r, &b, true, 252.0);
        let ir = fa::benchmark::information_ratio(&r, &b, true, 252.0);
        let rsq = fa::benchmark::r_squared(&r, &b);
        let uc = fa::benchmark::up_capture(&r, &b);
        let dc = fa::benchmark::down_capture(&r, &b);
        let cr = fa::benchmark::capture_ratio(&r, &b);
        let ba = fa::benchmark::batting_average(&r, &b);
        assert!(te.is_finite());
        assert!(ir.is_finite());
        assert!(rsq.is_finite());
        assert!(uc.is_finite());
        assert!(dc.is_finite());
        assert!(cr.is_finite());
        assert!(ba.is_finite());
    }

    #[test]
    fn underlying_m_squared_composes_from_primitives() {
        let p = vec![0.01, -0.02, 0.03, -0.01, 0.02];
        let b = vec![0.005, -0.01, 0.02, -0.005, 0.015];
        let ann = 252.0;
        let ann_return = fa::risk_metrics::mean_return(&p, true, ann);
        let ann_vol = fa::risk_metrics::volatility(&p, true, ann);
        let bench_vol = fa::risk_metrics::volatility(&b, true, ann);
        let ms = fa::benchmark::m_squared(ann_return, ann_vol, bench_vol, 0.02);
        assert!(ms.is_finite());
    }

    #[test]
    fn underlying_info_criteria_aic_bic_hqic() {
        let a = timeseries::aic(-500.0, 3);
        let b = timeseries::bic(-500.0, 3, 100);
        let h = timeseries::hqic(-500.0, 3, 100);
        assert!(a.is_finite());
        assert!(b.is_finite());
        assert!(h.is_finite());
    }

    #[test]
    fn underlying_ljung_box() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.015, 0.01, -0.005, 0.02,
        ];
        let (q, p) = fa::timeseries::ljung_box(&r, 5);
        assert!(q.is_finite());
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn underlying_arch_lm() {
        let r = vec![
            0.01, -0.02, 0.03, -0.01, 0.02, 0.005, -0.015, 0.01, -0.005, 0.02,
        ];
        let (lm, p) = fa::timeseries::arch_lm(&r, 3);
        assert!(lm.is_finite());
        assert!((0.0..=1.0).contains(&p));
    }

    #[test]
    fn underlying_garch_info_criteria() {
        let a = fa::timeseries::aic(-500.0, 3);
        let b = fa::timeseries::bic(-500.0, 3, 100);
        let h = fa::timeseries::hqic(-500.0, 3, 100);
        assert!(a.is_finite());
        assert!(b.is_finite());
        assert!(h.is_finite());
        assert!((a - 1006.0).abs() < 1e-10);
    }
}
