//! Common test fixtures and utilities for deposit tests.
//!
//! Provides reusable components to minimize duplication and ensure consistency
//! across the test suite following DRY principles.

use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::term_structures::DiscountCurve;
use finstack_core::money::Money;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::deposit::Deposit;
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use std::sync::Arc;

/// Tolerance for floating point comparisons in financial calculations.
pub const PRICE_TOLERANCE: f64 = 1e-10;
pub const RATE_TOLERANCE: f64 = 1e-12;
pub const DF_TOLERANCE: f64 = 1e-12;

/// Helper to construct dates more concisely.
pub fn date(y: i32, m: u8, d: u8) -> Date {
    Date::from_calendar_date(y, time::Month::try_from(m).unwrap(), d).unwrap()
}

/// Creates a market context with a flat discount curve.
///
/// # Arguments
/// * `base` - Base date for the curve
/// * `id` - Curve identifier
/// * `rate` - Annual discount rate (e.g., 0.02 for 2%)
pub fn ctx_with_flat_rate(base: Date, id: &str, rate: f64) -> MarketContext {
    let disc = DiscountCurve::builder(id)
        .base_date(base)
        .knots([
            (0.0, 1.0),
            (1.0, (1.0 - rate)),
            (2.0, (1.0 - rate * 2.0)),
            (5.0, (1.0 - rate * 5.0)),
        ])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

/// Creates a market context with a standard discount curve for testing.
///
/// Uses a realistic discount curve with decreasing discount factors:
/// - T=0: DF=1.0
/// - T=1: DF=0.98 (≈2% rate)
/// - T=2: DF=0.96 (≈2% rate)
pub fn ctx_with_standard_disc(base: Date, id: &str) -> MarketContext {
    let disc = DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (1.0, 0.98), (2.0, 0.96)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

/// Creates a market context with a steep discount curve for sensitivity testing.
pub fn ctx_with_steep_curve(base: Date, id: &str) -> MarketContext {
    let disc = DiscountCurve::builder(id)
        .base_date(base)
        .knots([(0.0, 1.0), (0.5, 0.95), (1.0, 0.90), (2.0, 0.80)])
        .build()
        .unwrap();
    MarketContext::new().insert_discount(disc)
}

/// Creates a standard 6-month USD deposit for testing.
///
/// - Notional: 1,000,000 USD
/// - Period: 6 months from base date
/// - Day count: Act/360
/// - Curve: USD-OIS
pub fn standard_deposit(base: Date) -> Deposit {
    Deposit::builder()
        .id(InstrumentId::new("DEP-STD"))
        .notional(Money::new(1_000_000.0, Currency::USD))
        .start(base)
        .end(date(
            base.year(),
            (base.month() as u8 + 6).min(12),
            base.day(),
        ))
        .day_count(DayCount::Act360)
        .quote_rate_opt(Some(0.0))
        .discount_curve_id(CurveId::new("USD-OIS"))
        .build()
        .unwrap()
}

/// Creates a deposit with custom parameters.
pub struct DepositBuilder {
    id: String,
    notional: Money,
    start: Date,
    end: Date,
    day_count: DayCount,
    quote_rate: Option<f64>,
    discount_curve_id: String,
}

impl DepositBuilder {
    pub fn new(base: Date) -> Self {
        Self {
            id: "DEP-TEST".to_string(),
            notional: Money::new(1_000_000.0, Currency::USD),
            start: base,
            end: date(base.year(), (base.month() as u8 + 6).min(12), base.day()),
            day_count: DayCount::Act360,
            quote_rate: Some(0.0),
            discount_curve_id: "USD-OIS".to_string(),
        }
    }

    pub fn id(mut self, id: &str) -> Self {
        self.id = id.to_string();
        self
    }

    pub fn notional(mut self, notional: Money) -> Self {
        self.notional = notional;
        self
    }

    pub fn start(mut self, start: Date) -> Self {
        self.start = start;
        self
    }

    pub fn end(mut self, end: Date) -> Self {
        self.end = end;
        self
    }

    pub fn day_count(mut self, day_count: DayCount) -> Self {
        self.day_count = day_count;
        self
    }

    pub fn quote_rate(mut self, rate: f64) -> Self {
        self.quote_rate = Some(rate);
        self
    }

    pub fn discount_curve_id(mut self, id: &str) -> Self {
        self.discount_curve_id = id.to_string();
        self
    }

    pub fn build(self) -> Deposit {
        let mut dep = Deposit::builder()
            .id(InstrumentId::new(&self.id))
            .notional(self.notional)
            .start(self.start)
            .end(self.end)
            .day_count(self.day_count)
            .discount_curve_id(CurveId::new(&self.discount_curve_id))
            .build()
            .unwrap();
        dep.quote_rate = self.quote_rate;
        dep
    }
}

/// Sets up a metric context for testing metric calculators.
///
/// # Returns
/// Tuple of (deposit, market context, metric context, registry)
#[allow(dead_code)]
pub fn setup_metric_context(base: Date) -> (Deposit, MarketContext, MetricContext, MetricRegistry) {
    let ctx = ctx_with_standard_disc(base, "USD-OIS");
    let dep = standard_deposit(base);
    let base_val = dep.npv(&ctx, base).unwrap();

    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> =
        Arc::new(dep.clone());
    let metric_ctx = MetricContext::new(
        instrument_arc,
        Arc::new(ctx.clone()),
        base,
        base_val,
        MetricContext::default_config(),
    );

    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::rates::deposit::register_deposit_metrics(&mut registry);

    (dep, ctx, metric_ctx, registry)
}

/// Computes a specific metric for a deposit.
pub fn compute_metric(
    deposit: &Deposit,
    ctx: &MarketContext,
    base: Date,
    metric_id: MetricId,
) -> f64 {
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::rates::deposit::register_deposit_metrics(&mut registry);

    let base_val = deposit.npv(ctx, base).unwrap();
    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> =
        Arc::new(deposit.clone());
    let mut metric_ctx = MetricContext::new(
        instrument_arc,
        Arc::new(ctx.clone()),
        base,
        base_val,
        MetricContext::default_config(),
    );

    let results = registry
        .compute(std::slice::from_ref(&metric_id), &mut metric_ctx)
        .unwrap();
    *results.get(&metric_id).unwrap()
}

/// Computes multiple metrics for a deposit.
pub fn compute_metrics(
    deposit: &Deposit,
    ctx: &MarketContext,
    base: Date,
    metric_ids: &[MetricId],
) -> finstack_core::HashMap<MetricId, f64> {
    let mut registry = MetricRegistry::new();
    finstack_valuations::instruments::rates::deposit::register_deposit_metrics(&mut registry);

    let base_val = deposit.npv(ctx, base).unwrap();
    let instrument_arc: Arc<dyn finstack_valuations::instruments::Instrument> =
        Arc::new(deposit.clone());
    let mut metric_ctx = MetricContext::new(
        instrument_arc,
        Arc::new(ctx.clone()),
        base,
        base_val,
        MetricContext::default_config(),
    );

    registry.compute(metric_ids, &mut metric_ctx).unwrap()
}
