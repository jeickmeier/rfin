//! FX dependency completeness tests.
//!
//! Ensures FX instruments can be priced with only declared curves and FX pairs.

use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::fx::{providers::SimpleFxProvider, FxMatrix};
use finstack_valuations::instruments::common::dependencies::{FxPair, InstrumentDependencies};
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::instruments::fx_forward::FxForward;
use finstack_valuations::instruments::json_loader::InstrumentJson;
use finstack_valuations::test_utils::flat_discount_with_tenor;
use std::sync::Arc;
use time::macros::date;

fn build_fx_market(deps: &InstrumentDependencies, as_of: Date, fx_pair: FxPair) -> MarketContext {
    let mut market = MarketContext::new();

    for curve_id in deps.curves.discount_curves.iter() {
        market = market.insert_discount(flat_discount_with_tenor(curve_id.as_str(), as_of, 0.02, 5.0));
    }

    let provider = Arc::new(SimpleFxProvider::new());
    provider.set_quote(fx_pair.base, fx_pair.quote, 1.1);
    let fx_matrix = FxMatrix::new(provider);
    market = market.insert_fx(fx_matrix);

    market
}

#[test]
fn test_fx_dependencies_complete() {
    let as_of = date!(2025 - 01 - 01);
    let fx_forward = FxForward::example();

    let deps =
        InstrumentDependencies::from_instrument_json(&InstrumentJson::FxForward(fx_forward.clone()));
    let fx_pair = deps
        .fx_pairs
        .first()
        .copied()
        .expect("FX forward dependencies should include FX pair");
    let market = build_fx_market(&deps, as_of, fx_pair);

    let result = fx_forward.value(&market, as_of);
    assert!(
        result.is_ok(),
        "FX forward pricing with minimal market should succeed, got: {:?}",
        result.err()
    );
}

#[test]
fn test_missing_fx_dependency_fails() {
    let as_of = date!(2025 - 01 - 01);
    let fx_forward = FxForward::example();

    let deps =
        InstrumentDependencies::from_instrument_json(&InstrumentJson::FxForward(fx_forward.clone()));

    let mut market = MarketContext::new();
    for curve_id in deps.curves.discount_curves.iter() {
        market = market.insert_discount(flat_discount_with_tenor(curve_id.as_str(), as_of, 0.02, 5.0));
    }

    let result = fx_forward.value(&market, as_of);
    assert!(
        result.is_err(),
        "FX forward pricing should fail when FX matrix is missing"
    );
}
