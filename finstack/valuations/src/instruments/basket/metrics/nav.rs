//! NAV metric calculator for Basket.
//!
//! Computes Net Asset Value per share by delegating to the basket pricer.

use crate::instruments::basket::types::Basket;
use crate::metrics::{MetricCalculator, MetricContext};
use finstack_core::{Result, F};

/// Calculate Net Asset Value per share
pub struct NavCalculator;

impl MetricCalculator for NavCalculator {
    fn calculate(&self, context: &mut MetricContext) -> Result<F> {
        let basket = context.instrument_as::<Basket>()?;
        let nav = basket.nav(&context.curves, context.as_of)?;
        Ok(nav.amount())
    }
}


