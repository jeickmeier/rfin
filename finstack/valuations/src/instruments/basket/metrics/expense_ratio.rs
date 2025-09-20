//! Expense ratio metric calculator.
//!
//! Returns the configured expense ratio as a percentage.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate expense ratio as percentage
pub struct ExpenseRatioCalculator;

impl MetricCalculator for ExpenseRatioCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        Ok(basket.expense_ratio * 100.0)
    }
}
