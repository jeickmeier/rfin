//! Finite-difference Greek calculators: deterministic sanity checks.
//!
//! We build a minimal deterministic instrument that satisfies the FD Greek
//! traits and has an analytic PV (quadratic in spot) so we can assert the
//! FD outputs match the closed-form derivatives.

use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::{Date, DayCount};
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::instruments::common::traits::{
    Attributes, EquityDependencies, EquityInstrumentDeps, Instrument,
};
use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
use finstack_valuations::metrics::sensitivities::fd_greeks::{
    GenericFdDelta, GenericFdGamma,
};
use finstack_valuations::metrics::{MetricContext, MetricId, MetricRegistry};
use finstack_valuations::pricer::InstrumentType;
use std::sync::Arc;
use time::macros::date;

#[derive(Clone)]
struct TestFdInstrument {
    id: String,
    expiry: Date,
    day_count: DayCount,
    spot_id: String,
    overrides: PricingOverrides,
}

impl TestFdInstrument {
    fn new(id: &str, expiry: Date, spot_id: &str) -> Self {
        Self {
            id: id.to_string(),
            expiry,
            day_count: DayCount::Act365F,
            spot_id: spot_id.to_string(),
            overrides: PricingOverrides::default(),
        }
    }
}

impl EquityDependencies for TestFdInstrument {
    fn equity_dependencies(&self) -> EquityInstrumentDeps {
        EquityInstrumentDeps::builder()
            .spot(self.spot_id.clone())
            .build()
    }
}

impl finstack_valuations::metrics::HasExpiry for TestFdInstrument {
    fn expiry(&self) -> Date {
        self.expiry
    }
}

impl finstack_valuations::metrics::HasDayCount for TestFdInstrument {
    fn day_count(&self) -> DayCount {
        self.day_count
    }
}

impl finstack_valuations::metrics::HasPricingOverrides for TestFdInstrument {
    fn pricing_overrides_mut(&mut self) -> &mut PricingOverrides {
        &mut self.overrides
    }
}

impl Instrument for TestFdInstrument {
    fn id(&self) -> &str {
        &self.id
    }

    fn key(&self) -> InstrumentType {
        InstrumentType::Equity
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn attributes(&self) -> &Attributes {
        static ATTRS: Attributes = Attributes::new();
        &ATTRS
    }

    fn attributes_mut(&mut self) -> &mut Attributes {
        // This instrument does not use attributes; return a new one.
        // For testing purposes, we don't persist mutations.
        static mut ATTRS: Attributes = Attributes::new();
        unsafe { &mut ATTRS }
    }

    fn value(&self, market: &MarketContext, as_of: Date) -> finstack_core::Result<Money> {
        if as_of >= self.expiry {
            return Ok(Money::new(0.0, Currency::USD));
        }
        let spot_scalar = market.price(self.spot_id.as_str())?;
        let spot = match spot_scalar {
            MarketScalar::Price(m) => m.amount(),
            MarketScalar::Unitless(v) => *v,
        };
        // Simple analytic PV = S^2 (currency USD)
        Ok(Money::new(spot * spot, Currency::USD))
    }

    fn price_with_metrics(
        &self,
        market: &MarketContext,
        as_of: Date,
        metrics: &[MetricId],
    ) -> finstack_core::Result<finstack_valuations::results::ValuationResult> {
        let base_value = self.value(market, as_of)?;
        finstack_valuations::instruments::common::helpers::build_with_metrics_dyn_with_config(
            Arc::from(self.clone_box()),
            Arc::new(market.clone()),
            as_of,
            base_value,
            metrics,
            Arc::new(FinstackConfig::default()),
        )
    }
}

fn registry_for_test<I: Instrument + EquityDependencies + finstack_valuations::metrics::HasExpiry + finstack_valuations::metrics::HasDayCount + finstack_valuations::metrics::HasPricingOverrides + Clone + 'static>() -> MetricRegistry {
    let mut registry = MetricRegistry::new();
    registry.register_metric(
        MetricId::Delta,
        Arc::new(GenericFdDelta::<I>::default()),
        &[InstrumentType::Equity],
    );
    registry.register_metric(
        MetricId::Gamma,
        Arc::new(GenericFdGamma::<I>::default()),
        &[InstrumentType::Equity],
    );
    registry
}

fn market_with_spot(spot_id: &str, price: f64) -> MarketContext {
    MarketContext::new().insert_price(spot_id, MarketScalar::Price(Money::new(price, Currency::USD)))
}

#[test]
fn fd_delta_matches_analytic_for_quadratic_pv() {
    let as_of = date!(2025 - 01 - 01);
    let spot = 100.0;
    let inst = TestFdInstrument::new("FD-TEST", date!(2026 - 01 - 01), "SPOT");
    let market = market_with_spot("SPOT", spot);

    let base_value = inst.value(&market, as_of).expect("base pv");
    let registry = registry_for_test::<TestFdInstrument>();
    let mut ctx = MetricContext::new(
        Arc::new(inst),
        Arc::new(market),
        as_of,
        base_value,
    );

    let result = registry
        .compute(&[MetricId::Delta], &mut ctx)
        .expect("delta");
    let delta = *result.get(&MetricId::Delta).expect("delta value");

    // For PV = S^2, dPV/dS = 2S.
    assert!((delta - 2.0 * spot).abs() < 1e-8, "delta mismatch");
}

#[test]
fn fd_gamma_matches_analytic_for_quadratic_pv() {
    let as_of = date!(2025 - 01 - 01);
    let spot = 80.0;
    let inst = TestFdInstrument::new("FD-TEST", date!(2026 - 01 - 01), "SPOT");
    let market = market_with_spot("SPOT", spot);

    let base_value = inst.value(&market, as_of).expect("base pv");
    let registry = registry_for_test::<TestFdInstrument>();
    let mut ctx = MetricContext::new(
        Arc::new(inst),
        Arc::new(market),
        as_of,
        base_value,
    );

    let result = registry
        .compute(&[MetricId::Gamma], &mut ctx)
        .expect("gamma");
    let gamma = *result.get(&MetricId::Gamma).expect("gamma value");

    // For PV = S^2, d2PV/dS2 = 2.
    assert!((gamma - 2.0).abs() < 1e-8, "gamma mismatch");
}

#[test]
fn fd_greeks_zero_when_expired() {
    let as_of = date!(2027 - 01 - 02);
    let spot = 50.0;
    let inst = TestFdInstrument::new("FD-TEST", date!(2027 - 01 - 01), "SPOT");
    let market = market_with_spot("SPOT", spot);

    let base_value = inst.value(&market, as_of).expect("base pv");
    assert_eq!(base_value.amount(), 0.0, "expired base pv should be zero");

    let registry = registry_for_test::<TestFdInstrument>();
    let mut ctx = MetricContext::new(
        Arc::new(inst),
        Arc::new(market),
        as_of,
        base_value,
    );

    let result = registry
        .compute(&[MetricId::Delta, MetricId::Gamma], &mut ctx)
        .expect("greeks");
    assert_eq!(
        *result.get(&MetricId::Delta).expect("delta"),
        0.0,
        "expired delta should be zero"
    );
    assert_eq!(
        *result.get(&MetricId::Gamma).expect("gamma"),
        0.0,
        "expired gamma should be zero"
    );
}

