//! Variance forecasting and volatility term structure generation.
//!
//! Provides h-step ahead variance forecasts from fitted GARCH models,
//! standard horizon term structures, and convenience functions.

use super::egarch11::Egarch11;
use super::garch::{GarchFamily, GarchFit, GarchModel};
use super::garch11::Garch11;
use super::gjr_garch11::GjrGarch11;

/// A single variance forecast at a given horizon.
#[must_use]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VarianceForecast {
    /// Forecast horizon in periods (1 = next period).
    pub horizon: usize,
    /// Forecasted conditional variance at this horizon.
    pub variance: f64,
    /// Annualized volatility: sqrt(variance * trading_days_per_year).
    pub annualized_vol: f64,
}

/// Volatility term structure from a fitted GARCH model.
#[must_use]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct VolTermStructure {
    /// Forecasts at standard horizons.
    pub forecasts: Vec<VarianceForecast>,
    /// Unconditional (long-run) annualized volatility.
    pub unconditional_vol: f64,
    /// Model used to generate the term structure.
    pub model: String,
}

/// Standard forecast horizons in trading days: 1D, 1W, 1M, 3M, 1Y.
pub const STANDARD_HORIZONS: &[usize] = &[1, 5, 21, 63, 252];

/// Forecast future conditional variances from a fitted GARCH-family model.
///
/// Dispatches on `fit.params.family`, so callers only need the fit object.
/// When `terminal_residual` is provided, the 1-step forecast uses the
/// observable last demeaned residual; otherwise it uses the iterated
/// conditional expectation from the terminal variance.
///
/// # Arguments
///
/// * `fit` - Fitted GARCH-family model.
/// * `horizons` - Forecast horizons in periods.
/// * `trading_days_per_year` - Annualization factor for volatility outputs.
/// * `terminal_residual` - Optional last demeaned residual `r_t - mu`.
///
/// # Returns
///
/// One [`VarianceForecast`] per requested horizon.
///
/// # Examples
///
/// ```rust
/// use finstack_analytics::timeseries::{
///     forecast_garch_fit, GarchFamily, GarchFit, GarchParams, InnovationDist,
/// };
///
/// let fit = GarchFit {
///     model: "GARCH(1,1)".to_string(),
///     params: GarchParams {
///         omega: 0.00001,
///         alpha: 0.10,
///         beta: 0.85,
///         gamma: None,
///         dist: InnovationDist::Gaussian,
///         family: GarchFamily::Garch11,
///         mean: 0.0,
///     },
///     std_errors: None,
///     log_likelihood: -100.0,
///     n_obs: 100,
///     n_params: 3,
///     aic: 206.0,
///     bic: 214.0,
///     hqic: 209.0,
///     conditional_variances: vec![0.0002; 100],
///     standardized_residuals: vec![0.0; 100],
///     terminal_variance: 0.0003,
///     converged: true,
///     iterations: 50,
/// };
/// let forecasts = forecast_garch_fit(&fit, &[1, 5], 252.0, Some(0.01));
/// assert_eq!(forecasts.len(), 2);
/// ```
#[must_use]
pub fn forecast_garch_fit(
    fit: &GarchFit,
    horizons: &[usize],
    trading_days_per_year: f64,
    terminal_residual: Option<f64>,
) -> Vec<VarianceForecast> {
    match fit.params.family {
        GarchFamily::Garch11 => {
            Garch11.forecast(fit, horizons, trading_days_per_year, terminal_residual)
        }
        GarchFamily::GjrGarch11 => {
            GjrGarch11.forecast(fit, horizons, trading_days_per_year, terminal_residual)
        }
        GarchFamily::Egarch11 => {
            Egarch11.forecast(fit, horizons, trading_days_per_year, terminal_residual)
        }
    }
}

/// Generate a volatility term structure from a fitted model.
///
/// Produces forecasts at the 5 standard horizons (1D, 1W, 1M, 3M, 1Y)
/// plus any custom horizons provided.
///
/// # Arguments
///
/// * `model` - GARCH-family model implementation used for forecasting.
/// * `fit` - Fitted model state.
/// * `trading_days_per_year` - Annualization factor for volatility outputs.
/// * `custom_horizons` - Optional extra horizons in periods.
///
/// # Returns
///
/// A [`VolTermStructure`] containing standard horizons plus unique custom
/// horizons, sorted ascending.
///
/// # Examples
///
/// ```no_run
/// use finstack_analytics::timeseries::{
///     vol_term_structure, Garch11, GarchModel, InnovationDist,
/// };
///
/// let returns = vec![
///     0.004, -0.006, 0.002, 0.009, -0.011, 0.003, -0.004, 0.006, -0.008,
///     0.005, 0.002, -0.003, 0.007, -0.010, 0.004, 0.006, -0.002, 0.001,
///     -0.005, 0.008,
/// ];
/// let fit = Garch11.fit(&returns, InnovationDist::Gaussian, None)?;
/// let ts = vol_term_structure(&Garch11, &fit, 252.0, Some(&[10, 126]));
/// assert!(ts.forecasts.iter().any(|f| f.horizon == 126));
/// # Ok::<(), finstack_core::Error>(())
/// ```
pub fn vol_term_structure(
    model: &dyn GarchModel,
    fit: &GarchFit,
    trading_days_per_year: f64,
    custom_horizons: Option<&[usize]>,
) -> VolTermStructure {
    let mut all_horizons: Vec<usize> = STANDARD_HORIZONS.to_vec();
    if let Some(custom) = custom_horizons {
        for &h in custom {
            if !all_horizons.contains(&h) {
                all_horizons.push(h);
            }
        }
    }
    all_horizons.sort_unstable();

    let forecasts = model.forecast(fit, &all_horizons, trading_days_per_year, None);

    let unconditional_vol = fit
        .params
        .unconditional_variance()
        .map(|uv| (uv * trading_days_per_year).sqrt())
        .unwrap_or_else(|| {
            // Fallback: use terminal variance
            (fit.terminal_variance * trading_days_per_year).sqrt()
        });

    VolTermStructure {
        forecasts,
        unconditional_vol,
        model: fit.model.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeseries::garch::{GarchFamily, GarchFit, GarchParams};
    use crate::timeseries::innovations::InnovationDist;

    fn make_test_fit() -> GarchFit {
        GarchFit {
            model: "GARCH(1,1)".to_string(),
            params: GarchParams {
                omega: 0.00001,
                alpha: 0.10,
                beta: 0.85,
                gamma: None,
                dist: InnovationDist::Gaussian,
                family: GarchFamily::Garch11,
                mean: 0.0,
            },
            std_errors: None,
            log_likelihood: -1000.0,
            n_obs: 1000,
            n_params: 3,
            aic: 2006.0,
            bic: 2020.0,
            hqic: 2010.0,
            conditional_variances: vec![0.0002; 1000],
            standardized_residuals: vec![0.0; 1000],
            terminal_variance: 0.0003,
            converged: true,
            iterations: 100,
        }
    }

    #[test]
    fn term_structure_has_standard_horizons() {
        let fit = make_test_fit();
        let ts = vol_term_structure(&Garch11, &fit, 252.0, None);

        assert_eq!(ts.forecasts.len(), STANDARD_HORIZONS.len());
        for (f, &h) in ts.forecasts.iter().zip(STANDARD_HORIZONS.iter()) {
            assert_eq!(f.horizon, h);
        }
    }

    #[test]
    fn term_structure_with_custom_horizons() {
        let fit = make_test_fit();
        let ts = vol_term_structure(&Garch11, &fit, 252.0, Some(&[10, 126]));

        // Should have standard + custom (no duplicates)
        assert!(ts.forecasts.len() >= STANDARD_HORIZONS.len() + 2);
        let horizons: Vec<usize> = ts.forecasts.iter().map(|f| f.horizon).collect();
        assert!(horizons.contains(&10));
        assert!(horizons.contains(&126));
    }

    #[test]
    fn unconditional_vol_positive() {
        let fit = make_test_fit();
        let ts = vol_term_structure(&Garch11, &fit, 252.0, None);
        assert!(ts.unconditional_vol > 0.0);
        assert!(ts.unconditional_vol.is_finite());
    }

    #[test]
    fn forecast_garch_fit_returns_empty_for_zero_horizon() {
        let fit = make_test_fit();
        let forecasts = forecast_garch_fit(&fit, &[], 252.0, Some(0.02));
        assert!(forecasts.is_empty());
    }

    #[test]
    fn forecast_garch_fit_uses_family_specific_forecast() {
        let mut fit = make_test_fit();
        fit.params.mean = 0.01;

        let forecasts = forecast_garch_fit(&fit, &[1, 3], 252.0, Some(0.02));
        assert_eq!(forecasts.len(), 2);
        assert!((forecasts[0].variance - 0.000305).abs() < 1e-12);
    }
}
