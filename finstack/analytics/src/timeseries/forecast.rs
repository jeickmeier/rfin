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
}
