//! Forward curve dependency completeness tests.
//!
//! Ensures instruments declaring forward curve dependencies can be priced
//! with only the declared market data present.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::instruments::commodity::commodity_option::CommodityOption;
use finstack_valuations::instruments::common::InstrumentDependencies;
use finstack_valuations::instruments::Instrument;
use finstack_valuations::instruments::json_loader::InstrumentJson;
use finstack_valuations::test_utils::{
    flat_discount_with_tenor, flat_forward_with_tenor, flat_vol_surface,
};
use time::macros::date;

fn build_market_from_deps(
    deps: &InstrumentDependencies,
    as_of: Date,
    spot_currency: Currency,
) -> MarketContext {
    let mut market = MarketContext::new();

    for curve_id in deps.curves.discount_curves.iter() {
        market = market.insert_discount(flat_discount_with_tenor(
            curve_id.as_str(),
            as_of,
            0.02,
            5.0,
        ));
    }
    for curve_id in deps.curves.forward_curves.iter() {
        market =
            market.insert_forward(flat_forward_with_tenor(curve_id.as_str(), as_of, 0.03, 5.0));
    }
    for surface_id in deps.vol_surface_ids.iter() {
        market = market.insert_surface(flat_vol_surface(
            surface_id.as_str(),
            &[0.25, 0.5, 1.0],
            &[50.0, 75.0, 100.0],
            0.2,
        ));
    }
    for spot_id in deps.spot_ids.iter() {
        market = market.insert_price(
            spot_id,
            MarketScalar::Price(Money::new(100.0, spot_currency)),
        );
    }

    market
}

#[test]
fn test_forward_curve_dependencies_complete() {
    let as_of = date!(2025 - 01 - 01);
    let mut option = CommodityOption::example();
    option.spot_price_id = Some("WTI-SPOT".to_string());

    let deps = InstrumentDependencies::from_instrument_json(&InstrumentJson::CommodityOption(
        option.clone(),
    ));
    let market = build_market_from_deps(&deps, as_of, option.currency);

    let result = option.value(&market, as_of);
    assert!(
        result.is_ok(),
        "Commodity option pricing with minimal market should succeed, got: {:?}",
        result.err()
    );
}

#[test]
fn test_missing_forward_curve_fails() {
    let as_of = date!(2025 - 01 - 01);
    let mut option = CommodityOption::example();
    option.spot_price_id = Some("WTI-SPOT".to_string());

    let deps = InstrumentDependencies::from_instrument_json(&InstrumentJson::CommodityOption(
        option.clone(),
    ));

    let mut market = MarketContext::new();
    for curve_id in deps.curves.discount_curves.iter() {
        market = market.insert_discount(flat_discount_with_tenor(
            curve_id.as_str(),
            as_of,
            0.02,
            5.0,
        ));
    }
    for surface_id in deps.vol_surface_ids.iter() {
        market = market.insert_surface(flat_vol_surface(
            surface_id.as_str(),
            &[0.25, 0.5, 1.0],
            &[50.0, 75.0, 100.0],
            0.2,
        ));
    }
    for spot_id in deps.spot_ids.iter() {
        market = market.insert_price(
            spot_id,
            MarketScalar::Price(Money::new(100.0, option.currency)),
        );
    }

    let result = option.value(&market, as_of);
    assert!(
        result.is_err(),
        "Commodity option pricing should fail when forward curve is missing"
    );
}
