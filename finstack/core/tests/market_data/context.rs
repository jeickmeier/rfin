use super::test_helpers::{
    sample_base_correlation_curve, sample_base_date, sample_discount_curve, sample_forward_curve,
    sample_hazard_curve, sample_inflation_curve, sample_vol_surface,
};
use finstack_core::currency::Currency;
use finstack_core::dates::Date;
use finstack_core::market_data::bumps::BumpType;
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
    ctx.insert_mut(discount.clone()).insert_mut(forward.clone());

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

    // Get the original discount factor for verification
    let orig_disc = ctx.get_discount_ref("USD-OIS").unwrap();
    let orig_df_5y = orig_disc.df(5.0);

    let bumped = ctx.bump(bumps).expect("bump should succeed");

    // Original context is unchanged
    let orig_disc_after = ctx.get_discount_ref("USD-OIS").unwrap();
    assert_eq!(
        orig_disc_after.df(5.0),
        orig_df_5y,
        "original context unchanged"
    );

    // Bumped context has the curve under the same ID, but with bumped values
    let bumped_disc = bumped.get_discount_ref("USD-OIS").unwrap();
    assert_ne!(
        bumped_disc.df(5.0),
        orig_df_5y,
        "bumped curve has different values"
    );
    assert!(
        bumped_disc.df(5.0) < orig_df_5y,
        "bumped curve has lower discount factors (higher rates)"
    );
}

#[test]
fn market_context_bumps_surfaces_and_scalars() {
    let surface = sample_vol_surface();
    let price = MarketScalar::Price(Money::new(100.0, Currency::USD));
    let series = ScalarTimeSeries::new(
        "TS",
        vec![
            (sample_base_date(), 10.0),
            (sample_base_date() + time::Duration::days(30), 12.0),
        ],
        None,
    )
    .unwrap();

    let ctx = MarketContext::new()
        .insert_surface_arc(surface.clone())
        .insert_price("EQ-SPOT", price.clone())
        .insert_series(series.clone());

    let original_vol = surface
        .value_checked(0.5, 1.0)
        .expect("vol lookup should succeed in test");
    let original_price = match ctx.price("EQ-SPOT").unwrap() {
        MarketScalar::Price(m) => m.amount(),
        _ => panic!("unexpected scalar variant"),
    };
    let original_series = ctx
        .series("TS")
        .unwrap()
        .value_on(sample_base_date())
        .unwrap();

    let mut bumps = HashMap::new();
    bumps.insert(CurveId::from("EQ-VOL"), BumpSpec::multiplier(1.10));
    bumps.insert(
        CurveId::from("EQ-SPOT"),
        BumpSpec {
            mode: finstack_core::market_data::context::BumpMode::Multiplicative,
            units: finstack_core::market_data::context::BumpUnits::Factor,
            value: 1.05,
            bump_type: BumpType::Parallel,
        },
    );
    bumps.insert(
        CurveId::from("TS"),
        BumpSpec {
            mode: finstack_core::market_data::context::BumpMode::Additive,
            units: finstack_core::market_data::context::BumpUnits::Percent,
            value: 10.0,
            bump_type: BumpType::Parallel,
        },
    );

    let bumped = ctx.bump(bumps).expect("bump should succeed");

    let bumped_vol = bumped
        .surface("EQ-VOL")
        .unwrap()
        .value_checked(0.5, 1.0)
        .unwrap();
    assert!(bumped_vol > original_vol);

    let bumped_price = match bumped.price("EQ-SPOT").unwrap() {
        MarketScalar::Price(m) => m.amount(),
        _ => panic!("unexpected scalar variant"),
    };
    assert!((bumped_price - original_price * 1.05).abs() < 1e-9);

    let bumped_series_value = bumped
        .series("TS")
        .unwrap()
        .value_on(sample_base_date())
        .unwrap();
    assert!((bumped_series_value - (original_series + 0.10)).abs() < 1e-9);
}

#[test]
fn market_context_handles_additional_introspection() {
    let mut ctx = MarketContext::new();
    assert!(ctx.is_empty());

    ctx.insert_mut(Arc::new(sample_discount_curve("USD-OIS")));
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

#[test]
fn market_context_getters_type_mismatch_and_not_found() {
    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_forward(sample_forward_curve("USD-LIBOR"))
        .insert_surface_arc(sample_vol_surface())
        .insert_price("EQ-SPOT", MarketScalar::Unitless(1.0));

    // Not found cases
    assert!(ctx.get_discount("MISSING").is_err());
    assert!(ctx.get_forward("MISSING").is_err());
    assert!(ctx.surface("MISSING").is_err());
    assert!(ctx.price("MISSING").is_err());
    assert!(ctx.series("MISSING").is_err());
    assert!(ctx.credit_index("MISSING").is_err());
    assert!(ctx.inflation_index("MISSING").is_none());
    assert!(ctx.inflation_index_ref("MISSING").is_none());
    assert!(ctx.dividend_schedule("MISSING").is_none());
    assert!(ctx.dividend_schedule_ref("MISSING").is_none());

    // Type mismatch cases (ensure the "mismatch" branches are exercised)
    assert!(ctx.get_forward("USD-OIS").is_err());
    assert!(ctx.get_discount("USD-LIBOR").is_err());
    assert!(ctx.get_forward_ref("USD-OIS").is_err());
    assert!(ctx.get_discount_ref("USD-LIBOR").is_err());
}

#[test]
fn market_context_surface_and_dividends_arc_variants_preserve_identity() {
    let surface = sample_vol_surface();
    let dividends = Arc::new(
        DividendSchedule::new("AAPL-DIVS")
            .add_cash(sample_base_date(), Money::new(1.0, Currency::USD)),
    );

    let mut ctx = MarketContext::new()
        .insert_surface_arc(surface.clone())
        .insert_dividends_arc(dividends.clone());

    // insert_*_mut variants should also work and preserve the Arc when used.
    ctx.insert_surface_mut(surface.clone());
    ctx.insert_dividends_arc_mut(dividends.clone());

    let got_surface = ctx.surface("EQ-VOL").unwrap();
    assert!(Arc::ptr_eq(&got_surface, &surface));

    let got_divs = ctx.dividend_schedule("AAPL-DIVS").unwrap();
    assert!(Arc::ptr_eq(&got_divs, &dividends));
}

#[test]
fn market_context_collateral_error_paths() {
    // Missing mapping
    let ctx = MarketContext::new().insert_discount(sample_discount_curve("USD-OIS"));
    assert!(ctx.collateral("MISSING-CSA").is_err());
    assert!(ctx.collateral_ref("MISSING-CSA").is_err());

    // Mapping exists but curve is missing
    let ctx = MarketContext::new().map_collateral("USD-CSA", CurveId::from("USD-OIS"));
    assert!(ctx.collateral("USD-CSA").is_err());
    assert!(ctx.collateral_ref("USD-CSA").is_err());
}

#[test]
fn market_context_roll_forward_preserves_ids_and_clones_non_curves() {
    let surface = sample_vol_surface();
    let mut ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_forward(sample_forward_curve("USD-LIBOR"))
        .insert_hazard(sample_hazard_curve("CDX"))
        .insert_inflation(sample_inflation_curve("USD-CPI"))
        .insert_base_correlation(sample_base_correlation_curve("CDX-BC"))
        .insert_surface_arc(surface.clone())
        .insert_price("EQ-SPOT", MarketScalar::Unitless(1.0))
        .map_collateral("USD-CSA", CurveId::from("USD-OIS"));

    // Ensure we have an FX matrix too (roll should preserve it)
    ctx = ctx.insert_fx(sample_fx_matrix());

    let before_base = ctx.get_discount_ref("USD-OIS").unwrap().base_date();
    let rolled = ctx.roll_forward(30).expect("roll_forward should succeed");

    // Original unchanged
    assert_eq!(
        ctx.get_discount_ref("USD-OIS").unwrap().base_date(),
        before_base
    );

    // IDs preserved and base date advanced in rolled context
    assert_eq!(
        rolled.get_discount_ref("USD-OIS").unwrap().id().as_str(),
        "USD-OIS"
    );
    assert_ne!(
        rolled.get_discount_ref("USD-OIS").unwrap().base_date(),
        before_base
    );

    // Base correlation curves are passed through without "rolling" logic (still available, same id)
    assert_eq!(
        rolled
            .get_base_correlation_ref("CDX-BC")
            .unwrap()
            .id()
            .as_str(),
        "CDX-BC"
    );

    // Non-curve data is preserved
    assert!(rolled.fx.is_some());
    assert!(rolled.surface("EQ-VOL").is_ok());
    assert!(rolled.price("EQ-SPOT").is_ok());
    assert!(rolled.collateral("USD-CSA").is_ok());

    // Total object count should be unchanged across roll
    assert_eq!(rolled.total_objects(), ctx.total_objects());
}

#[test]
fn market_context_bump_fx_spot_error_and_success_paths() {
    let date = sample_base_date();

    // Error when no FX matrix is present
    let ctx = MarketContext::new();
    assert!(ctx
        .bump_fx_spot(Currency::USD, Currency::EUR, 0.01, date)
        .is_err());

    // Success path with a static provider
    let ctx = MarketContext::new().insert_fx(sample_fx_matrix());
    let before = ctx
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            Currency::USD,
            Currency::EUR,
            date,
        ))
        .unwrap()
        .rate;

    let bumped = ctx
        .bump_fx_spot(Currency::USD, Currency::EUR, 0.10, date)
        .unwrap();
    let after = bumped
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            Currency::USD,
            Currency::EUR,
            date,
        ))
        .unwrap()
        .rate;

    assert!((after - before * 1.10).abs() < 1e-12);
}

#[test]
fn market_context_bumps_inflation_triangular_key_rate_branch() {
    let ctx = MarketContext::new().insert_inflation(sample_inflation_curve("USD-CPI"));

    let orig = ctx.get_inflation_ref("USD-CPI").unwrap();
    let orig_levels = orig.cpi_levels().to_vec();

    let mut bumps = HashMap::new();
    bumps.insert(
        CurveId::from("USD-CPI"),
        BumpSpec::triangular_key_rate_bp(0.0, 1.0, 2.0, 1.0), // 1bp => +0.0001 fraction
    );

    let bumped = ctx.bump(bumps).unwrap();
    let bumped_levels = bumped.get_inflation_ref("USD-CPI").unwrap().cpi_levels();

    // Closest knot to 1.0y should be bumped multiplicatively
    assert!((bumped_levels[1] - orig_levels[1] * 1.0001).abs() < 1e-9);
    // Other points unchanged
    assert!((bumped_levels[0] - orig_levels[0]).abs() < 1e-12);
    assert!((bumped_levels[2] - orig_levels[2]).abs() < 1e-12);
}

#[test]
fn market_context_apply_bumps_exercises_all_variants() {
    use finstack_core::market_data::bumps::MarketBump;

    let date = sample_base_date();

    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_base_correlation(sample_base_correlation_curve("CDX-BC"))
        .insert_surface_arc(sample_vol_surface())
        .insert_fx(sample_fx_matrix());

    let df_before = ctx.get_discount_ref("USD-OIS").unwrap().df(2.0);
    let fx_before = ctx
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            Currency::USD,
            Currency::EUR,
            date,
        ))
        .unwrap()
        .rate;

    let vol_before = ctx
        .surface_ref("EQ-VOL")
        .unwrap()
        .value_checked(0.5, 1.0)
        .unwrap();
    let bc_before = ctx
        .get_base_correlation_ref("CDX-BC")
        .unwrap()
        .correlations()[0];

    let bumps = vec![
        // Curve bump
        MarketBump::Curve {
            id: CurveId::from("USD-OIS"),
            spec: BumpSpec::parallel_bp(25.0),
        },
        // FX bump (pct)
        MarketBump::FxPct {
            base: Currency::USD,
            quote: Currency::EUR,
            pct: 10.0,
            as_of: date,
        },
        // Vol surface bucket bump: filter a single bucket so we hit the filtered path
        MarketBump::VolBucketPct {
            surface_id: CurveId::from("EQ-VOL"),
            expiries: Some(vec![0.5]),
            strikes: Some(vec![1.0]),
            pct: 10.0,
        },
        // Base correlation bucket bump
        MarketBump::BaseCorrBucketPts {
            surface_id: CurveId::from("CDX-BC"),
            detachments: Some(vec![3.0]),
            points: 0.02,
        },
    ];

    let bumped = ctx.apply_bumps(&bumps).unwrap();

    // Curve bumped
    let df_after = bumped.get_discount_ref("USD-OIS").unwrap().df(2.0);
    assert_ne!(df_after, df_before);

    // FX bumped
    let fx_after = bumped
        .fx
        .as_ref()
        .unwrap()
        .rate(finstack_core::money::fx::FxQuery::new(
            Currency::USD,
            Currency::EUR,
            date,
        ))
        .unwrap()
        .rate;
    assert!((fx_after - fx_before * 1.10).abs() < 1e-12);

    // Vol bucket bumped at (0.5, 1.0); non-bucket cells should remain unchanged.
    let vol_after = bumped
        .surface_ref("EQ-VOL")
        .unwrap()
        .value_checked(0.5, 1.0)
        .unwrap();
    assert!((vol_after - vol_before * 1.10).abs() < 1e-12);
    let vol_other_before = ctx
        .surface_ref("EQ-VOL")
        .unwrap()
        .value_checked(0.25, 0.9)
        .unwrap();
    let vol_other_after = bumped
        .surface_ref("EQ-VOL")
        .unwrap()
        .value_checked(0.25, 0.9)
        .unwrap();
    assert!((vol_other_after - vol_other_before).abs() < 1e-12);

    // Base correlation bumped (first point matches 3.0 detachment in test helper curve)
    let bc_after = bumped
        .get_base_correlation_ref("CDX-BC")
        .unwrap()
        .correlations()[0];
    assert!((bc_after - (bc_before + 0.02)).abs() < 1e-12);

    // Stats display covered
    let _ = format!("{}", bumped.stats());
}

#[test]
fn market_context_insert_and_stats_setters_cover_remaining_paths() {
    // Cover insert_surface(by value), insert_dividends(by value), insert_fx_arc/mut,
    // insert_credit_index_mut, insert_market_history(_mut), map_collateral_mut and
    // the mutable setters/iterators in stats.rs.
    let date = sample_base_date();

    let surface_by_value = finstack_core::market_data::surfaces::vol_surface::VolSurface::builder(
        "IR-VOL",
    )
    .expiries(&[1.0, 2.0])
    .strikes(&[90.0, 100.0])
    .row(&[0.2, 0.2])
    .row(&[0.2, 0.2])
    .build()
    .unwrap();

    let dividends_by_value =
        DividendSchedule::new("MSFT-DIVS").add_cash(date, Money::new(0.5, Currency::USD));

    let hazard = Arc::new(sample_hazard_curve("CDX"));
    let base_corr = Arc::new(sample_base_correlation_curve("CDX-BC"));
    let credit_index = CreditIndexData::builder()
        .num_constituents(2)
        .index_credit_curve(hazard)
        .base_correlation_curve(base_corr)
        .build()
        .unwrap();

    let fx_arc = Arc::new(sample_fx_matrix());

    let mut ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_surface(surface_by_value)
        .insert_dividends(dividends_by_value)
        .insert_fx_arc(fx_arc.clone());

    // insert_credit_index_mut + insert_fx_mut
    ctx.insert_credit_index_mut("CDX", credit_index)
        .insert_fx_mut(fx_arc.clone());

    // market history insertion
    let hist_1: Arc<dyn std::any::Any + Send + Sync> = Arc::new(123_i32);
    let hist_2: Arc<dyn std::any::Any + Send + Sync> = Arc::new("history");
    ctx = ctx.insert_market_history(hist_1);
    assert!(ctx.market_history.is_some());
    ctx.insert_market_history_mut(hist_2);
    assert!(ctx.market_history.is_some());

    // map_collateral_mut
    ctx.map_collateral_mut("USD-CSA", CurveId::from("USD-OIS"));
    assert!(ctx.collateral("USD-CSA").is_ok());

    // stats setters + iterators
    ctx.set_price_mut(CurveId::from("PX"), MarketScalar::Unitless(42.0));

    let series = ScalarTimeSeries::new("TS-2", vec![(date, 1.0), (date + time::Duration::days(1), 2.0)], None)
        .unwrap()
        .with_interpolation(SeriesInterpolation::Linear);
    ctx.set_series_mut(series);

    let idx = Arc::new(
        InflationIndex::new("US-CPI", vec![(date, 100.0)], Currency::USD)
            .unwrap()
            .with_interpolation(InflationInterpolation::Linear),
    );
    ctx.set_inflation_index_mut("US-CPI", idx);

    let divs = Arc::new(
        DividendSchedule::new("AAPL-DIVS")
            .add_cash(date, Money::new(1.0, Currency::USD)),
    );
    ctx.set_dividends_mut(divs);

    assert_eq!(ctx.prices_iter().count(), 1);
    assert_eq!(ctx.series_iter().count(), 1);
    assert_eq!(ctx.inflation_indices_iter().count(), 1);
    assert_eq!(ctx.dividends_iter().count(), 2); // MSFT-DIVS + AAPL-DIVS

    let stats = ctx.stats();
    assert!(stats.total_curves >= 1);
    assert!(stats.has_fx);
    assert!(stats.surface_count >= 1);
    assert_eq!(stats.collateral_mapping_count, 1);
}

#[test]
fn market_context_bump_more_curve_types_and_error_paths() {
    // Cover additional bump branches: forward multiplicative, hazard, inflation parallel, base correlation.
    let ctx = MarketContext::new()
        .insert_discount(sample_discount_curve("USD-OIS"))
        .insert_forward(sample_forward_curve("USD-LIBOR"))
        .insert_hazard(sample_hazard_curve("CDX"))
        .insert_inflation(sample_inflation_curve("USD-CPI"))
        .insert_base_correlation(sample_base_correlation_curve("CDX-BC"))
        .insert_surface_arc(sample_vol_surface());

    let df_before = ctx.get_discount_ref("USD-OIS").unwrap().df(2.0);
    let fwd_before = ctx.get_forward_ref("USD-LIBOR").unwrap().rate(2.0);
    let haz_before = ctx.get_hazard_ref("CDX").unwrap().hazard_rate(5.0);
    let cpi_before = ctx.get_inflation_ref("USD-CPI").unwrap().cpi_levels()[1];
    let bc_before = ctx
        .get_base_correlation_ref("CDX-BC")
        .unwrap()
        .correlations()[0];

    let mut bumps = HashMap::new();
    bumps.insert(CurveId::from("USD-OIS"), BumpSpec::parallel_bp(10.0));
    bumps.insert(CurveId::from("USD-LIBOR"), BumpSpec::multiplier(1.10));
    bumps.insert(CurveId::from("CDX"), BumpSpec::parallel_bp(5.0));
    bumps.insert(CurveId::from("USD-CPI"), BumpSpec::inflation_shift_pct(2.0));
    bumps.insert(CurveId::from("CDX-BC"), BumpSpec::correlation_shift_pct(10.0));

    let bumped = ctx.bump(bumps).unwrap();

    assert_ne!(bumped.get_discount_ref("USD-OIS").unwrap().df(2.0), df_before);
    assert_ne!(
        bumped.get_forward_ref("USD-LIBOR").unwrap().rate(2.0),
        fwd_before
    );
    assert_ne!(
        bumped.get_hazard_ref("CDX").unwrap().hazard_rate(5.0),
        haz_before
    );
    assert_ne!(
        bumped.get_inflation_ref("USD-CPI").unwrap().cpi_levels()[1],
        cpi_before
    );
    assert_ne!(
        bumped
            .get_base_correlation_ref("CDX-BC")
            .unwrap()
            .correlations()[0],
        bc_before
    );

    // Error path: attempting to apply a key-rate bump to a VolSurface via ctx.bump should fail
    let mut bad = HashMap::new();
    bad.insert(
        CurveId::from("EQ-VOL"),
        BumpSpec::triangular_key_rate_bp(0.25, 0.5, 1.0, 1.0),
    );
    assert!(ctx.bump(bad).is_err());
}

#[test]
fn curve_storage_from_impls_and_variant_accessors() {
    let disc = sample_discount_curve("USD-OIS");
    let fwd = sample_forward_curve("USD-LIBOR");
    let haz = sample_hazard_curve("CDX");
    let inf = sample_inflation_curve("USD-CPI");
    let bc = sample_base_correlation_curve("CDX-BC");

    let s_disc: CurveStorage = disc.into();
    assert_eq!(s_disc.id().as_str(), "USD-OIS");
    assert!(s_disc.discount().is_some());
    assert!(s_disc.forward().is_none());
    assert!(s_disc.hazard().is_none());
    assert!(s_disc.inflation().is_none());
    assert!(s_disc.base_correlation().is_none());

    let s_fwd: CurveStorage = Arc::new(fwd).into();
    assert_eq!(s_fwd.id().as_str(), "USD-LIBOR");
    assert!(s_fwd.discount().is_none());
    assert!(s_fwd.forward().is_some());

    let s_haz: CurveStorage = haz.into();
    assert_eq!(s_haz.id().as_str(), "CDX");
    assert!(s_haz.hazard().is_some());

    let s_inf: CurveStorage = Arc::new(inf).into();
    assert_eq!(s_inf.id().as_str(), "USD-CPI");
    assert!(s_inf.inflation().is_some());

    let s_bc: CurveStorage = bc.into();
    assert_eq!(s_bc.id().as_str(), "CDX-BC");
    assert!(s_bc.base_correlation().is_some());
}

#[test]
fn curve_storage_arc_from_impls_and_curve_type_coverage() {
    let disc = Arc::new(sample_discount_curve("USD-OIS"));
    let fwd = Arc::new(sample_forward_curve("USD-LIBOR"));
    let haz = Arc::new(sample_hazard_curve("CDX"));
    let inf = Arc::new(sample_inflation_curve("USD-CPI"));
    let bc = Arc::new(sample_base_correlation_curve("CDX-BC"));

    let s_disc: CurveStorage = disc.clone().into();
    assert!(Arc::ptr_eq(s_disc.discount().unwrap(), &disc));
    assert_eq!(s_disc.curve_type(), "Discount");

    let s_fwd: CurveStorage = fwd.clone().into();
    assert!(Arc::ptr_eq(s_fwd.forward().unwrap(), &fwd));
    assert_eq!(s_fwd.curve_type(), "Forward");

    let s_haz: CurveStorage = haz.clone().into();
    assert!(Arc::ptr_eq(s_haz.hazard().unwrap(), &haz));
    assert_eq!(s_haz.curve_type(), "Hazard");

    let s_inf: CurveStorage = inf.clone().into();
    assert!(Arc::ptr_eq(s_inf.inflation().unwrap(), &inf));
    assert_eq!(s_inf.curve_type(), "Inflation");

    let s_bc: CurveStorage = bc.clone().into();
    assert!(Arc::ptr_eq(s_bc.base_correlation().unwrap(), &bc));
    assert_eq!(s_bc.curve_type(), "BaseCorrelation");
}

#[test]
fn market_context_apply_bumps_additional_branches_and_errors() {
    use finstack_core::market_data::bumps::MarketBump;

    let date = sample_base_date();

    // FxPct error branch (missing FX)
    let ctx = MarketContext::new().insert_discount(sample_discount_curve("USD-OIS"));
    assert!(ctx
        .apply_bumps(&[MarketBump::FxPct {
            base: Currency::USD,
            quote: Currency::EUR,
            pct: 1.0,
            as_of: date,
        }])
        .is_err());

    // VolBucketPct parallel fallback (no filters) path
    let surface = sample_vol_surface();
    let base_vol = surface.value_checked(0.5, 1.0).unwrap();
    let ctx = MarketContext::new().insert_surface_arc(surface);
    let bumped = ctx
        .apply_bumps(&[MarketBump::VolBucketPct {
            surface_id: CurveId::from("EQ-VOL"),
            expiries: None,
            strikes: None,
            pct: 10.0,
        }])
        .unwrap();
    let bumped_vol = bumped
        .surface_ref("EQ-VOL")
        .unwrap()
        .value_checked(0.5, 1.0)
        .unwrap();
    // Parallel vol bump via BumpSpec(Additive/Percent) adds +0.10
    assert!((bumped_vol - (base_vol + 0.10)).abs() < 1e-12);

    // VolBucketPct missing surface error branch
    let ctx = MarketContext::new();
    assert!(ctx
        .apply_bumps(&[MarketBump::VolBucketPct {
            surface_id: CurveId::from("MISSING-VOL"),
            expiries: Some(vec![0.5]),
            strikes: Some(vec![1.0]),
            pct: 1.0,
        }])
        .is_err());

    // BaseCorrBucketPts "all buckets" (detachments None) path
    let ctx = MarketContext::new().insert_base_correlation(sample_base_correlation_curve("CDX-BC"));
    let before = ctx
        .get_base_correlation_ref("CDX-BC")
        .unwrap()
        .correlations()[0];
    let bumped = ctx
        .apply_bumps(&[MarketBump::BaseCorrBucketPts {
            surface_id: CurveId::from("CDX-BC"),
            detachments: None,
            points: 0.01,
        }])
        .unwrap();
    let after = bumped
        .get_base_correlation_ref("CDX-BC")
        .unwrap()
        .correlations()[0];
    assert!((after - (before + 0.01)).abs() < 1e-12);

    // BaseCorrBucketPts missing curve error branch
    let ctx = MarketContext::new();
    assert!(ctx
        .apply_bumps(&[MarketBump::BaseCorrBucketPts {
            surface_id: CurveId::from("MISSING-BC"),
            detachments: None,
            points: 0.01,
        }])
        .is_err());
}

#[test]
fn market_context_instrument_registry_works() {
    let ctx = MarketContext::new().insert_instrument("X", Arc::new(123_i32));
    let any = ctx.get_instrument("X").expect("instrument should exist");
    assert_eq!(*any.downcast_ref::<i32>().unwrap(), 123);

    // Alias method
    let any2 = ctx.instrument("X").expect("instrument should exist");
    assert!(any2.downcast_ref::<i32>().is_some());
}
