mod common;

use common::*;
use finstack_core::config::FinstackConfig;
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_core::{Error, InputError};
use finstack_portfolio::types::Entity;
use finstack_portfolio::{PortfolioBuilder, Position, PositionUnit};
use finstack_valuations::instruments::common::traits::{Attributes, Instrument};
use finstack_valuations::pricer::InstrumentType;
use finstack_valuations::results::ValuationResult;
use std::any::Any;
use std::sync::Arc;

#[derive(Clone)]
struct ValueOnlyInstrument {
    id: String,
    currency: Currency,
    value: f64,
    attributes: Attributes,
}

impl ValueOnlyInstrument {
    fn new(id: &str, currency: Currency, value: f64) -> Self {
        Self {
            id: id.to_string(),
            currency,
            value,
            attributes: Attributes::new(),
        }
    }
}

impl Instrument for ValueOnlyInstrument {
    fn id(&self) -> &str {
        &self.id
    }
    fn key(&self) -> InstrumentType {
        InstrumentType::Basket
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
    fn clone_box(&self) -> Box<dyn Instrument> {
        Box::new(self.clone())
    }

    fn value(&self, _curves: &MarketContext, _as_of: Date) -> finstack_core::Result<Money> {
        Ok(Money::new(self.value, self.currency))
    }

    fn price_with_metrics(
        &self,
        _curves: &MarketContext,
        _as_of: Date,
        _metrics: &[finstack_valuations::metrics::MetricId],
    ) -> finstack_core::Result<ValuationResult> {
        Err(Error::Input(InputError::Invalid))
    }
}

#[test]
fn valuation_falls_back_when_metrics_fail() {
    let as_of = base_date();
    let inst = Arc::new(ValueOnlyInstrument::new("VO", Currency::USD, 123.45));
    let pos = Position::new("P", "E", "VO", inst, 1.0, PositionUnit::Units).unwrap();

    let portfolio = PortfolioBuilder::new("PF")
        .base_ccy(Currency::USD)
        .as_of(as_of)
        .entity(Entity::new("E"))
        .position(pos)
        .build()
        .unwrap();

    let market = market_with_usd();
    let config = FinstackConfig::default();
    let valuation = finstack_portfolio::value_portfolio(&portfolio, &market, &config).unwrap();

    let pv = valuation.get_position_value("P").unwrap();
    assert_eq!(pv.value_native.currency(), Currency::USD);
    assert!((pv.value_native.amount() - 123.45).abs() < 1e-9);
}
