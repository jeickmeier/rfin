//! Polars expression plugins for analytics metrics.
//!
//! Each `#[polars_expr]` function is exported as a C ABI symbol that Polars
//! discovers at runtime via `register_plugin_function` on the Python side.
//! All computation delegates to `finstack_core::analytics` — no logic is
//! duplicated.

use finstack_core::analytics::{benchmark, drawdown, returns, risk_metrics};
use finstack_core::dates::PeriodKind;
use polars::prelude::*;
use pyo3_polars::derive::polars_expr;
use serde::Deserialize;

// ── Helpers ──

fn parse_ann_factor(freq: &str) -> PolarsResult<f64> {
    freq.parse::<PeriodKind>()
        .map(|pk| pk.annualization_factor())
        .map_err(|_| {
            polars_err!(
                InvalidOperation: "unknown frequency '{}'; expected one of: daily, weekly, monthly, quarterly, semiannual, annual",
                freq
            )
        })
}

fn series_to_f64_vec(s: &Series) -> PolarsResult<Vec<f64>> {
    let ca = s.f64()?;
    if ca.null_count() > 0 {
        polars_bail!(
            InvalidOperation: "expression plugin input contains {} null value(s); filter or fill nulls before calling",
            ca.null_count()
        );
    }
    Ok(ca.into_no_null_iter().collect())
}

// ── Kwargs structs ──

#[derive(Deserialize)]
struct FreqKwargs {
    freq: String,
    risk_free: f64,
}

#[derive(Deserialize)]
struct FreqOnlyKwargs {
    freq: String,
}

#[derive(Deserialize)]
struct ConfidenceKwargs {
    confidence: f64,
}

#[derive(Deserialize)]
struct ConfidenceFreqKwargs {
    confidence: f64,
    freq: String,
    risk_free: f64,
}

#[derive(Deserialize)]
struct ThresholdKwargs {
    threshold: f64,
}

#[derive(Deserialize)]
struct WindowFreqKwargs {
    window: usize,
    freq: String,
    risk_free: f64,
}

#[derive(Deserialize)]
struct WindowFreqOnlyKwargs {
    window: usize,
    freq: String,
}

#[derive(Deserialize)]
struct AnnFreqKwargs {
    freq: String,
    annualize: bool,
}

#[derive(Deserialize)]
struct BaseKwargs {
    base: f64,
}

#[derive(Deserialize)]
struct MSquaredKwargs {
    freq: String,
    risk_free: f64,
}

// ── Tier 1: Scalar risk metrics (single-series aggregations) ──

#[polars_expr(output_type=Float64)]
fn expr_sharpe(inputs: &[Series], kwargs: FreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let m = risk_metrics::mean_return(&data, true, ann);
    let v = risk_metrics::volatility(&data, true, ann);
    let result = risk_metrics::sharpe(m, v, kwargs.risk_free);
    Ok(Series::new("sharpe".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_sortino(inputs: &[Series], kwargs: FreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::sortino(&data, true, ann);
    Ok(Series::new("sortino".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_volatility(inputs: &[Series], kwargs: AnnFreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::volatility(&data, kwargs.annualize, ann);
    Ok(Series::new("volatility".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_mean_return(inputs: &[Series], kwargs: AnnFreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::mean_return(&data, kwargs.annualize, ann);
    Ok(Series::new("mean_return".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_cagr(inputs: &[Series], kwargs: FreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::cagr_from_periods(&data, ann);
    Ok(Series::new("cagr".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_calmar(inputs: &[Series], kwargs: FreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let cagr_val = risk_metrics::cagr_from_periods(&data, ann);
    let dd = drawdown::to_drawdown_series(&data);
    let max_dd = dd.iter().copied().fold(0.0_f64, f64::min);
    let result = risk_metrics::calmar(cagr_val, max_dd);
    Ok(Series::new("calmar".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_max_drawdown(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let dd = drawdown::to_drawdown_series(&data);
    let result = dd.iter().copied().fold(0.0_f64, f64::min);
    Ok(Series::new("max_drawdown".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_geometric_mean(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::geometric_mean(&data);
    Ok(Series::new("geometric_mean".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_downside_deviation(inputs: &[Series], kwargs: AnnFreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::downside_deviation(&data, 0.0, kwargs.annualize, ann);
    Ok(Series::new("downside_deviation".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_skewness(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::skewness(&data);
    Ok(Series::new("skewness".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_kurtosis(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::kurtosis(&data);
    Ok(Series::new("kurtosis".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_value_at_risk(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::value_at_risk(&data, kwargs.confidence, None);
    Ok(Series::new("value_at_risk".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_expected_shortfall(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::expected_shortfall(&data, kwargs.confidence, None);
    Ok(Series::new("expected_shortfall".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_parametric_var(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::parametric_var(&data, kwargs.confidence, None);
    Ok(Series::new("parametric_var".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_cornish_fisher_var(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::cornish_fisher_var(&data, kwargs.confidence, None);
    Ok(Series::new("cornish_fisher_var".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_ulcer_index(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let dd = drawdown::to_drawdown_series(&data);
    let result = risk_metrics::ulcer_index(&dd);
    Ok(Series::new("ulcer_index".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_pain_index(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let dd = drawdown::to_drawdown_series(&data);
    let result = risk_metrics::pain_index(&dd);
    Ok(Series::new("pain_index".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_omega_ratio(inputs: &[Series], kwargs: ThresholdKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::omega_ratio(&data, kwargs.threshold);
    Ok(Series::new("omega_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_gain_to_pain(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::gain_to_pain(&data);
    Ok(Series::new("gain_to_pain".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_tail_ratio(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::tail_ratio(&data, kwargs.confidence);
    Ok(Series::new("tail_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_outlier_win_ratio(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::outlier_win_ratio(&data, kwargs.confidence);
    Ok(Series::new("outlier_win_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_outlier_loss_ratio(inputs: &[Series], kwargs: ConfidenceKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::outlier_loss_ratio(&data, kwargs.confidence);
    Ok(Series::new("outlier_loss_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_risk_of_ruin(inputs: &[Series], kwargs: FreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::risk_of_ruin_from_returns(&data);
    Ok(Series::new("risk_of_ruin".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_recovery_factor(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = risk_metrics::recovery_factor_from_returns(&data);
    Ok(Series::new("recovery_factor".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_martin_ratio(inputs: &[Series], kwargs: FreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::martin_ratio_from_returns(&data, ann);
    Ok(Series::new("martin_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_sterling_ratio(inputs: &[Series], kwargs: FreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::sterling_ratio_from_returns(&data, ann, kwargs.risk_free);
    Ok(Series::new("sterling_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_burke_ratio(inputs: &[Series], kwargs: FreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let cagr_val = risk_metrics::cagr_from_periods(&data, ann);
    let dd = drawdown::to_drawdown_series(&data);
    let result = risk_metrics::burke_ratio(cagr_val, &dd, kwargs.risk_free);
    Ok(Series::new("burke_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_pain_ratio(inputs: &[Series], kwargs: FreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::pain_ratio_from_returns(&data, ann, kwargs.risk_free);
    Ok(Series::new("pain_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_modified_sharpe(inputs: &[Series], kwargs: ConfidenceFreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::modified_sharpe(&data, kwargs.risk_free, kwargs.confidence, ann);
    Ok(Series::new("modified_sharpe".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_m_squared(inputs: &[Series], kwargs: MSquaredKwargs) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = risk_metrics::m_squared_from_returns(&portfolio, &bench, ann, kwargs.risk_free);
    Ok(Series::new("m_squared".into(), &[result]))
}

// ── Tier 2: Series transforms (series in, series out) ──

#[polars_expr(output_type=Float64)]
fn expr_simple_returns(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = returns::simple_returns(&data);
    Ok(Series::new("simple_returns".into(), result.as_slice()))
}

#[polars_expr(output_type=Float64)]
fn expr_cumulative_returns(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = returns::comp_sum(&data);
    Ok(Series::new("cumulative_returns".into(), result.as_slice()))
}

#[polars_expr(output_type=Float64)]
fn expr_drawdown_series(inputs: &[Series]) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = drawdown::to_drawdown_series(&data);
    Ok(Series::new("drawdown_series".into(), result.as_slice()))
}

#[polars_expr(output_type=Float64)]
fn expr_rebase(inputs: &[Series], kwargs: BaseKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let result = returns::rebase(&data, kwargs.base);
    Ok(Series::new("rebase".into(), result.as_slice()))
}

// ── Tier 3: Two-input benchmark metrics ──

#[polars_expr(output_type=Float64)]
fn expr_tracking_error(inputs: &[Series], kwargs: AnnFreqKwargs) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = benchmark::tracking_error(&portfolio, &bench, kwargs.annualize, ann);
    Ok(Series::new("tracking_error".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_information_ratio(inputs: &[Series], kwargs: AnnFreqKwargs) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let result = benchmark::information_ratio(&portfolio, &bench, kwargs.annualize, ann);
    Ok(Series::new("information_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_r_squared(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::r_squared(&portfolio, &bench);
    Ok(Series::new("r_squared".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_beta(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::calc_beta(&portfolio, &bench);
    Ok(Series::new("beta".into(), &[result.beta]))
}

#[polars_expr(output_type=Float64)]
fn expr_up_capture(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::up_capture(&portfolio, &bench);
    Ok(Series::new("up_capture".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_down_capture(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::down_capture(&portfolio, &bench);
    Ok(Series::new("down_capture".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_capture_ratio(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::capture_ratio(&portfolio, &bench);
    Ok(Series::new("capture_ratio".into(), &[result]))
}

#[polars_expr(output_type=Float64)]
fn expr_batting_average(inputs: &[Series]) -> PolarsResult<Series> {
    let portfolio = series_to_f64_vec(&inputs[0])?;
    let bench = series_to_f64_vec(&inputs[1])?;
    let result = benchmark::batting_average(&portfolio, &bench);
    Ok(Series::new("batting_average".into(), &[result]))
}

// ── Tier 4: Rolling metrics (series in, series out) ──

#[polars_expr(output_type=Float64)]
fn expr_rolling_sharpe(inputs: &[Series], kwargs: WindowFreqKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let values = risk_metrics::rolling_sharpe_values(&data, kwargs.window, ann, kwargs.risk_free);
    if values.is_empty() {
        return Ok(Series::new_empty(
            "rolling_sharpe".into(),
            &DataType::Float64,
        ));
    }
    Ok(Series::new("rolling_sharpe".into(), values.as_slice()))
}

#[polars_expr(output_type=Float64)]
fn expr_rolling_sortino(inputs: &[Series], kwargs: WindowFreqOnlyKwargs) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let values = risk_metrics::rolling_sortino_values(&data, kwargs.window, ann);
    if values.is_empty() {
        return Ok(Series::new_empty(
            "rolling_sortino".into(),
            &DataType::Float64,
        ));
    }
    Ok(Series::new("rolling_sortino".into(), values.as_slice()))
}

#[polars_expr(output_type=Float64)]
fn expr_rolling_volatility(
    inputs: &[Series],
    kwargs: WindowFreqOnlyKwargs,
) -> PolarsResult<Series> {
    let data = series_to_f64_vec(&inputs[0])?;
    let ann = parse_ann_factor(&kwargs.freq)?;
    let values = risk_metrics::rolling_volatility_values(&data, kwargs.window, ann);
    if values.is_empty() {
        return Ok(Series::new_empty(
            "rolling_volatility".into(),
            &DataType::Float64,
        ));
    }
    Ok(Series::new("rolling_volatility".into(), values.as_slice()))
}
