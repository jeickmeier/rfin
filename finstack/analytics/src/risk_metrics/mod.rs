//! Return-based, tail-risk, and rolling analytics on simple return series.
//!
//! Start here when you already have a single `&[f64]` return slice and want a
//! scalar or rolling risk/performance metric without constructing
//! [`crate::performance::Performance`].
//!
//! Internally, the implementation is organized by return-based, tail-risk, and
//! rolling metrics. The public surface is the curated re-export list below so
//! callers have one obvious path per metric.
//!
//! Important conventions:
//! - returns are simple decimal returns, such as `0.01` for +1%
//! - annualized means scale linearly with the periods-per-year factor
//! - annualized volatility-like quantities scale with `sqrt(periods_per_year)`
//! - historical VaR and ES are reported in return space and are not time-scaled
//!
//! Drawdown-derived ratios such as Calmar, Martin, Sterling, Burke, Pain, and
//! Recovery Factor live in [`crate::drawdown`]. Benchmark-relative ratios such
//! as Treynor and M-squared live in [`crate::benchmark`].

mod return_based;
mod rolling;
mod tail_risk;

fn invalid_input<T>() -> crate::Result<T> {
    Err(crate::error::InputError::Invalid.into())
}

fn ensure_non_empty_returns(returns: &[f64]) -> crate::Result<()> {
    if returns.is_empty() {
        return invalid_input();
    }
    Ok(())
}

fn ensure_finite_returns(returns: &[f64]) -> crate::Result<()> {
    if returns.iter().any(|r| !r.is_finite()) {
        return invalid_input();
    }
    Ok(())
}

fn ensure_compoundable_returns(returns: &[f64]) -> crate::Result<()> {
    if returns.iter().any(|r| !r.is_finite() || *r < -1.0) {
        return invalid_input();
    }
    Ok(())
}

fn ensure_annualization_factor(annualize: bool, ann_factor: f64) -> crate::Result<()> {
    if invalid_annualization_factor(annualize, ann_factor) {
        return invalid_input();
    }
    Ok(())
}

fn ensure_strict_confidence(confidence: f64) -> crate::Result<()> {
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return invalid_input();
    }
    Ok(())
}

fn ensure_positive_horizon(ann_factor: Option<f64>) -> crate::Result<()> {
    if ann_factor.is_some_and(|af| !af.is_finite() || af <= 0.0) {
        return invalid_input();
    }
    Ok(())
}

pub(crate) use return_based::invalid_annualization_factor;
pub use return_based::{
    cagr, cagr_checked, downside_deviation, downside_deviation_checked, estimate_ruin,
    estimate_ruin_checked, gain_to_pain, geometric_mean, mean_return, mean_return_checked,
    modified_sharpe, omega_ratio, sharpe, sortino, sortino_checked, volatility, volatility_checked,
    AnnualizationConvention, CagrBasis, RuinDefinition, RuinEstimate, RuinModel,
};
pub use rolling::{
    rolling_sharpe, rolling_sortino, rolling_volatility, DatedSeries, RollingSharpe,
    RollingSortino, RollingVolatility,
};
pub use tail_risk::{
    cornish_fisher_var, cornish_fisher_var_checked, expected_shortfall, expected_shortfall_checked,
    kurtosis, moments4, outlier_loss_ratio, outlier_loss_ratio_checked, outlier_win_ratio,
    outlier_win_ratio_checked, parametric_var, parametric_var_checked, skewness, tail_ratio,
    tail_ratio_checked, value_at_risk, value_at_risk_checked,
};
