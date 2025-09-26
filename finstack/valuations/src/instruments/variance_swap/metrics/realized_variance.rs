//! Realized variance-to-date metric.

use super::super::types::VarianceSwap;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::math::stats::realized_variance;
use finstack_core::{Result, F};

/// Calculate the current realized variance to date.
pub struct RealizedVarianceCalculator;

impl MetricCalculator for RealizedVarianceCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let swap = context.instrument_as::<VarianceSwap>()?;
        let as_of = context.as_of;

        // No realized variance before observation period starts
        if as_of < swap.start_date {
            return Ok(0.0);
        }

        // Get observation dates up to as_of date
        let observation_dates = swap.observation_dates();
        let relevant_dates: Vec<_> = observation_dates
            .into_iter()
            .filter(|&date| date >= swap.start_date && date <= as_of)
            .collect();

        if relevant_dates.len() < 2 {
            return Ok(0.0); // Need at least 2 observations for variance
        }

        // Get price time series for the underlying
        let price_series = context.curves.series(&swap.underlying_id)?;
        
        // Extract prices for observation dates
        let mut prices = Vec::with_capacity(relevant_dates.len());
        for date in relevant_dates {
            let price = price_series.value_on(date)?;
            prices.push(price);
        }

        if prices.len() < 2 {
            return Ok(0.0);
        }

        // Calculate annualization factor based on observation frequency
        let annualization_factor = match swap.observation_freq.days() {
            Some(1) => 365.0,      // Daily observations
            Some(7) => 52.0,       // Weekly observations  
            _ => match swap.observation_freq.months() {
                Some(1) => 12.0,   // Monthly observations
                Some(3) => 4.0,    // Quarterly observations
                Some(12) => 1.0,   // Annual observations
                _ => 252.0,        // Default to business days
            }
        };

        // Calculate realized variance using the specified method
        let realized_var = realized_variance(&prices, swap.realized_var_method, annualization_factor);
        
        // Ensure non-negative result
        Ok(realized_var.max(0.0))
    }
}
