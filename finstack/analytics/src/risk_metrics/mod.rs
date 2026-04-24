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

/// Convert a sentinel-returning metric value into a typed error at a
/// boundary where invalid input should surface explicitly.
///
/// The risk metrics in this module return sentinel values (`NaN`, `0.0`,
/// or `±∞`) for invalid inputs because that matches the pipeline ergonomics
/// expected by DataFrame- and array-based callers. Applications that need
/// strict error propagation can wrap the result:
///
/// ```rust
/// use finstack_analytics::risk_metrics::{require_finite, value_at_risk};
///
/// let returns = [0.01, 0.02, -0.01, 0.03];
/// let var = require_finite(value_at_risk(&returns, 0.99))?;
/// # Ok::<(), finstack_core::Error>(())
/// ```
///
/// # Errors
///
/// Returns [`finstack_core::error::InputError::Invalid`] if `value` is not
/// finite (i.e. it is `NaN`, `+∞`, or `-∞`).
pub fn require_finite(value: f64) -> crate::Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(crate::error::InputError::Invalid.into())
    }
}

pub(crate) use return_based::invalid_annualization_factor;
pub use return_based::{
    cagr, downside_deviation, estimate_ruin, gain_to_pain, geometric_mean, mean_return,
    modified_sharpe, omega_ratio, sharpe, sortino, volatility, AnnualizationConvention, CagrBasis,
    RuinDefinition, RuinEstimate, RuinModel,
};
pub use rolling::{
    rolling_sharpe, rolling_sortino, rolling_volatility, DatedSeries, RollingSharpe,
    RollingSortino, RollingVolatility,
};
pub use tail_risk::{
    cornish_fisher_var, expected_shortfall, kurtosis, moments4, outlier_loss_ratio,
    outlier_win_ratio, parametric_var, skewness, tail_ratio, value_at_risk,
};
