use super::common::{
    sample_base_correlation_curve, sample_base_date, sample_discount_curve, sample_forward_curve,
    sample_hazard_curve, sample_inflation_curve, sample_vol_surface,
};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::context::{BumpSpec, CurveStorage, MarketContext};
use finstack_core::market_data::dividends::DividendSchedule;
use finstack_core::market_data::scalars::{
    inflation_index::{InflationIndex, InflationInterpolation},
    MarketScalar, ScalarTimeSeries, SeriesInterpolation,
};
use finstack_core::market_data::term_structures::credit_index::CreditIndexData;
use finstack_core::money::fx::{FxConversionPolicy, FxMatrix, FxProvider};
use finstack_core::money::Money;
use finstack_core::types::CurveId;
use hashbrown::HashMap;
use std::sync::Arc;
use time::Month;

// Simple static FX provider for testing
struct StaticFxProvider;
impl FxProvider for StaticFxProvider {
    fn rate(
        &self,
        from: Currency,
        to: Currency,
        _on: Date,
        _policy: FxConversionPolicy,
    ) -> finstack_core::Result<f64> {
        if from == Currency::USD && to == Currency::EUR {
            Ok(0.9)
        } else if from == to {
            Ok(1.0)
        } else {
            Ok(1.0 / 0.9)
        }
    }
}

fn sample_fx_matrix() -> FxMatrix {
    FxMatrix::new(Arc::new(StaticFxProvider))
}

#[test]
fn curve_storage_helpers() {
    let discount = CurveStorage::Discount(Arc::new(sample_discount_curve("USD-OIS")));
    assert!(discount.is_discount());
    assert_eq!(discount.curve_type(), "Discount");

    let forward = CurveStorage::Forward(Arc::new(sample_forward_curve("USD-LIBOR")));
    assert!(forward.is_forward());
    assert_eq!(forward.curve_type(), "Forward");

    let hazard = CurveStorage::Hazard(Arc::new(sample_hazard_curve("CDX")));
    assert!(hazard.is_hazard());

    let inflation = CurveStorage::Inflation(Arc::new(sample_inflation_curve("USD-CPI")));
    assert!(inflation.is_inflation());

    let base_corr = CurveStorage::BaseCorrelation(Arc::new(sample_base_correlation_curve("CDX")));
    assert!(base_corr.is_base_correlation());
}

#[test]
fn market_context_inserts_and_retrieves_curves() {
    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_forward(sample_forward_curve("USD-LIBOR"))
        .insert_hazard(sample_hazard_curve("CDX"))
        .insert_inflation(sample_inflation_curve("USD-CPI"))
        .insert_base_correlation(sample_base_correlation_curve("CDX-BC"));

    let stats = ctx.stats();
    assert_eq!(stats.total_curves, 5);

    assert_eq!(
        ctx.get_discount("USD-OIS").unwrap().id().as_str(),
        "USD-OIS"
    );
    assert_eq!(
        ctx.get_forward("USD-LIBOR").unwrap().id().as_str(),
        "USD-LIBOR"
    );
    assert_eq!(ctx.get_hazard("CDX").unwrap().id().as_str(), "CDX");
    assert_eq!(
        ctx.get_inflation("USD-CPI").unwrap().id().as_str(),
        "USD-CPI"
    );
    assert_eq!(
        ctx.get_base_correlation("CDX-BC").unwrap().id().as_str(),
        "CDX-BC"
    );
}

#[test]
fn market_context_arc_mut_variants_share_storage() {
    let discount = Arc::new(sample_discount_curve("USD-OIS"));
    let forward = Arc::new(sample_forward_curve("USD-LIBOR"));

    let mut ctx = MarketContext::new();
    ctx.insert_discount_mut(discount.clone())
        .insert_forward_mut(forward.clone());

    // Ensure references point to same data
    assert!(Arc::ptr_eq(
        &ctx.get_discount("USD-OIS").unwrap(),
        &discount
    ));
    assert!(Arc::ptr_eq(
        &ctx.get_forward("USD-LIBOR").unwrap(),
        &forward
    ));
}

#[test]
fn market_context_manages_fx_and_scalars() {
    let vol_surface = sample_vol_surface();
    let series = ScalarTimeSeries::new(
        "CPI",
        vec![
            (sample_base_date(), 100.0),
            (
                Date::from_calendar_date(2024, Month::February, 1).unwrap(),
                101.0,
            ),
        ],
        Some(Currency::USD),
    )
    .unwrap()
    .with_interpolation(SeriesInterpolation::Linear);

    let index = InflationIndex::new(
        "US-CPI",
        vec![
            (
                Date::from_calendar_date(2024, Month::January, 31).unwrap(),
                100.0,
            ),
            (
                Date::from_calendar_date(2024, Month::February, 29).unwrap(),
                101.0,
            ),
        ],
        Currency::USD,
    )
    .unwrap()
    .with_interpolation(InflationInterpolation::Linear);

    let dividends = DividendSchedule::new("AAPL-DIVS")
        .add_cash(sample_base_date(), Money::new(1.0, Currency::USD));
    let credit_index = CreditIndexData::builder()
        .num_constituents(1)
        .index_credit_curve(Arc::new(sample_hazard_curve("CDX")))
        .base_correlation_curve(Arc::new(sample_base_correlation_curve("CDX-BC")))
        .build()
        .unwrap();

    let mut ctx = MarketContext::new()
        .insert_fx(sample_fx_matrix())
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_series(series)
        .insert_inflation_index("US-CPI", index)
        .insert_dividends(dividends)
        .insert_credit_index("CDX", credit_index);

    ctx.insert_surface_mut(vol_surface.clone());
    ctx.insert_price_mut("USD-PRIME", MarketScalar::Unitless(0.05));

    assert!(ctx.surface(vol_surface.id()).is_ok());
    match ctx.price("USD-PRIME").unwrap() {
        MarketScalar::Unitless(v) => {
            assert!((v - 0.05).abs() < 1e-12);
        }
        other => panic!("unexpected scalar variant: {:?}", other),
    }

    assert_eq!(ctx.series("CPI").unwrap().currency(), Some(Currency::USD));
    assert!(ctx.inflation_index("US-CPI").is_some());
    assert!(ctx.dividend_schedule("AAPL-DIVS").is_some());
    assert!(ctx.credit_index("CDX").is_ok());

    let ids: Vec<_> = ctx.curve_ids().map(|c| c.as_str().to_string()).collect();
    assert!(ids.contains(&"USD-OIS".to_string()));

    let counts = ctx.count_by_type();
    assert_eq!(counts.get(&"Discount"), Some(&1));
}

#[test]
fn market_context_supports_curve_bumps() {
    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_forward(sample_forward_curve("USD-LIBOR"));

    let mut bumps = HashMap::new();
    bumps.insert(
        CurveId::new("USD-OIS"),
        finstack_core::market_data::context::BumpSpec::parallel_bp(50.0),
    );

    let bumped = ctx.bump(bumps).expect("bump should succeed");
    assert!(
        ctx.get_discount("USD-OIS_bump_50bp").is_err(),
        "original context unchanged"
    );
    assert!(
        bumped.get_discount("USD-OIS_bump_50bp").is_ok(),
        "bumped curve present"
    );
}

#[test]
fn market_context_handles_additional_introspection() {
    let mut ctx = MarketContext::new();
    assert!(ctx.is_empty());

    ctx.insert_discount_mut(Arc::new(sample_discount_curve("USD-OIS")));
    assert!(!ctx.is_empty());
    assert!(ctx.curve("USD-OIS").is_some());

    let counts = ctx.count_by_type();
    assert_eq!(counts.get("Discount"), Some(&1));

    let collected: Vec<_> = ctx.curves_of_type("Discount").collect();
    assert_eq!(collected.len(), 1);

    let stats = ctx.stats();
    assert_eq!(stats.total_curves, 1);
    assert!(!stats.has_fx);
}

#[test]
fn market_context_update_and_bump_failures() {
    let base_corr = Arc::new(sample_base_correlation_curve("CDX-BC"));
    let hazard = Arc::new(sample_hazard_curve("CDX"));
    let credit_index = CreditIndexData::builder()
        .num_constituents(1)
        .index_credit_curve(hazard)
        .base_correlation_curve(base_corr.clone())
        .build()
        .unwrap();

    let mut ctx = MarketContext::new().insert_credit_index("CDX", credit_index);
    let new_curve = Arc::new(sample_base_correlation_curve("CDX-NEW"));
    assert!(ctx.update_base_correlation_curve("CDX", new_curve.clone()));
    assert_eq!(
        ctx.credit_index("CDX").unwrap().base_correlation_curve.id(),
        new_curve.id()
    );
    assert!(!ctx.update_base_correlation_curve("UNKNOWN", new_curve));

    let mut bumps = HashMap::new();
    bumps.insert(CurveId::new("MISSING"), BumpSpec::parallel_bp(10.0));
    assert!(ctx.bump(bumps).is_err());
}

#[test]
fn market_context_collateral_and_stats() {
    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .map_collateral("USD-CSA", CurveId::from("USD-OIS"));

    assert!(!ctx.is_empty());
    assert_eq!(ctx.total_objects(), 1);

    let stats = ctx.stats();
    assert_eq!(stats.total_curves, 1);
    assert_eq!(stats.collateral_mapping_count, 1);

    let collateral = ctx.collateral("USD-CSA").unwrap();
    assert!(collateral.df(0.5) < 1.0);
    let collateral_ref = ctx.collateral_ref("USD-CSA").unwrap();
    assert!(collateral_ref.df(1.0) < 1.0);
}
