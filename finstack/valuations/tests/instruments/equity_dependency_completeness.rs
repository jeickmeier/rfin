//! Equity dependency completeness tests.
//!
//! Ensures instruments declaring equity dependencies can be priced with
//! only spot + vol + declared curves present.

use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::market_data::scalars::MarketScalar;
use finstack_core::money::Money;
use finstack_valuations::instruments::barrier_option::BarrierOption;
use finstack_valuations::instruments::common::dependencies::InstrumentDependencies;
use finstack_valuations::instruments::common::traits::Instrument;
use finstack_valuations::test_utils::{flat_discount_with_tenor, flat_vol_surface};
use time::macros::date;

fn build_market_from_deps(
    deps: &InstrumentDependencies,
    as_of: Date,
    spot_currency: Currency,
) -> MarketContext {
    let mut market = MarketContext::new();

    for curve_id in deps.curves.discount_curves.iter() {
        market = market.insert_discount(flat_discount_with_tenor(curve_id.as_str(), as_of, 0.02, 5.0));
    }
    for surface_id in deps.vol_surface_ids.iter() {
        market = market.insert_surface(flat_vol_surface(
            surface_id.as_str(),
            &[0.25, 0.5, 1.0],
            &[80.0, 100.0, 120.0],
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
fn test_equity_dependencies_complete() {
    let as_of = date!(2024 - 01 - 02);
    let mut option = BarrierOption::example();
    option.div_yield_id = None;

    let deps = InstrumentDependencies::from_curves_and_equity(&option);
    let market = build_market_from_deps(&deps, as_of, option.strike.currency());

    let result = option.value(&market, as_of);
    assert!(
        result.is_ok(),
        "Barrier option pricing with minimal market should succeed, got: {:?}",
        result.err()
    );
}

#[test]
fn test_missing_equity_dependency_fails() {
    let as_of = date!(2024 - 01 - 02);
    let mut option = BarrierOption::example();
    option.div_yield_id = None;

    let deps = InstrumentDependencies::from_curves_and_equity(&option);

    let mut no_vol_market = MarketContext::new();
    for curve_id in deps.curves.discount_curves.iter() {
        no_vol_market =
            no_vol_market.insert_discount(flat_discount_with_tenor(curve_id.as_str(), as_of, 0.02, 5.0));
    }
    for spot_id in deps.spot_ids.iter() {
        no_vol_market = no_vol_market.insert_price(
            spot_id,
            MarketScalar::Price(Money::new(100.0, option.strike.currency())),
        );
    }
    let result = option.value(&no_vol_market, as_of);
    assert!(
        result.is_err(),
        "Barrier option pricing should fail when vol surface is missing"
    );

    let mut no_spot_market = MarketContext::new();
    for curve_id in deps.curves.discount_curves.iter() {
        no_spot_market =
            no_spot_market.insert_discount(flat_discount_with_tenor(curve_id.as_str(), as_of, 0.02, 5.0));
    }
    for surface_id in deps.vol_surface_ids.iter() {
        no_spot_market = no_spot_market.insert_surface(flat_vol_surface(
            surface_id.as_str(),
            &[0.25, 0.5, 1.0],
            &[80.0, 100.0, 120.0],
            0.2,
        ));
    }
    let result = option.value(&no_spot_market, as_of);
    assert!(
        result.is_err(),
        "Barrier option pricing should fail when spot is missing"
    );
}
