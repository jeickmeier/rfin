//! Basket total value metric calculator.
//!
//! Computes the total basket value (gross, not per share) by delegating to the
//! basket pricer.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate total basket value (before per-share division)
pub struct BasketValueCalculator;

impl MetricCalculator for BasketValueCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        let value = basket.basket_value(&context.curves, context.as_of)?;
        Ok(value.amount())
    }
}


