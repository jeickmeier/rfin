//! Variance forecasting and volatility term structure generation.
//!
//! Provides h-step ahead variance forecasts from fitted GARCH models,
//! standard horizon term structures, and convenience functions.

use super::garch::{GarchFit, GarchModel};

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

/// Closed-form h-step-ahead GARCH(1,1) variance forecast path.
///
/// Iterates the recurrence
///
/// ```text
/// sigma^2_{t+h} = omega + (alpha + beta) * sigma^2_{t+h-1}      (h >= 2)
/// sigma^2_{t+1} = omega + alpha * r_t^2 + beta * sigma^2_t      (h == 1)
/// ```
///
/// and returns the variance path for horizons `1..=horizon`.
#[must_use]
pub fn garch11_forecast(
    omega: f64,
    alpha: f64,
    beta: f64,
    last_variance: f64,
    last_return: f64,
    horizon: usize,
) -> Vec<f64> {
    if horizon == 0 {
        return Vec::new();
    }

    let mut out = Vec::with_capacity(horizon);
    let mut sigma2 = omega + alpha * last_return * last_return + beta * last_variance;
    out.push(sigma2.max(0.0));

    let persistence = alpha + beta;
    for _ in 1..horizon {
        sigma2 = omega + persistence * sigma2;
        out.push(sigma2.max(0.0));
    }

    out
}

/// Generate a volatility term structure from a fitted model.
///
/// Produces forecasts at the 5 standard horizons (1D, 1W, 1M, 3M, 1Y)
/// plus any custom horizons provided.
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

    let forecasts = model.forecast(fit, &all_horizons, trading_days_per_year);

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
    use crate::timeseries::garch11::Garch11;
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
    fn garch11_forecast_returns_empty_for_zero_horizon() {
        let forecasts = garch11_forecast(0.00001, 0.10, 0.85, 0.0003, 0.02, 0);
        assert!(forecasts.is_empty());
    }

    #[test]
    fn garch11_forecast_matches_closed_form_recurrence() {
        let forecasts = garch11_forecast(0.00001, 0.10, 0.85, 0.0003, 0.02, 3);
        let expected = vec![0.000305, 0.00029975, 0.0002947625];

        assert_eq!(forecasts.len(), expected.len());
        for (actual, target) in forecasts.iter().zip(expected) {
            assert!((actual - target).abs() < 1e-12);
        }
    }
}
