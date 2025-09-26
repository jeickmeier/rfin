//! Tracking error metric calculator.
//!
//! Calculates tracking error between the basket returns and its benchmark index
//! using historical time series data from the market context.

use crate::instruments::basket::pricing::engine::BasketPricer;
use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate tracking error vs benchmark (requires benchmark data)
pub struct TrackingErrorCalculator;

impl MetricCalculator for TrackingErrorCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        
        // Need a tracking index to calculate tracking error against
        let benchmark_id = basket.tracking_index.as_ref()
            .ok_or_else(|| finstack_core::Error::Input(
                finstack_core::error::InputError::NotFound {
                    id: "No tracking index specified for basket".to_string()
                }))?;
        
        // Get benchmark time series
        let benchmark_series = context.curves.series(benchmark_id.as_str())?;
        
        // Extract data from the time series
        let state = benchmark_series.to_state()?;
        let observations = &state.observations;
        
        if observations.len() < 2 {
            return Ok(0.0); // Need at least 2 points to calculate returns
        }
        
        // Take up to the last 252 observations (1 year of daily data) or all available
        let start_idx = if observations.len() > 252 {
            observations.len() - 252
        } else {
            0
        };
        
        let relevant_obs = &observations[start_idx..];
        
        // Calculate benchmark returns
        let mut benchmark_returns = Vec::new();
        for i in 1..relevant_obs.len() {
            let prev_price = relevant_obs[i - 1].1;
            let curr_price = relevant_obs[i].1;
            let date = relevant_obs[i].0;
            
            if prev_price > 0.0 {
                let return_rate = (curr_price / prev_price - 1.0) as F;
                benchmark_returns.push((date, return_rate));
            }
        }
        
        if benchmark_returns.is_empty() {
            return Ok(0.0);
        }
        
        // Use the basket pricer to calculate tracking error
        let pricer = BasketPricer::new();
        pricer.tracking_error(basket, &context.curves, &benchmark_returns, context.as_of)
    }
}
