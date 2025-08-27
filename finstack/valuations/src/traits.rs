#![allow(missing_docs)]

use finstack_core::prelude::*;
use finstack_core::market_data::multicurve::CurveSet;

/// Currency-preserving schedule as a list of dated `Money` amounts.
pub type DatedFlows = Vec<(Date, Money)>;

/// Build cashflow schedules and provide currency-safe aggregation hooks.
pub trait CashflowProvider: Send + Sync {
    /// Build complete dated cashflow schedule as `(date, amount)` pairs.
    fn build_schedule(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<DatedFlows>;
}

/// Priceable instruments produce a `ValuationResult` at `as_of` using curves.
pub trait Priceable: Send + Sync {
    fn price(&self, curves: &CurveSet, as_of: Date) -> finstack_core::Result<super::pricing::result::ValuationResult>;
}


