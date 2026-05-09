//! Reference envelope integration tests.
//!
//! Each test loads one of the JSON examples under
//! `finstack/valuations/examples/market_bootstrap/`, runs it through the
//! calibration engine, and asserts that the resulting `MarketContext` answers
//! a typical analyst-style accessor query. The reference envelopes are the
//! canonical user-facing examples for the "build a MarketContext from quotes"
//! workflow.

use finstack_core::market_data::context::MarketContext;
use finstack_valuations::calibration::api::engine;
use finstack_valuations::calibration::api::schema::CalibrationEnvelope;
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/market_bootstrap")
}

pub(crate) fn load_envelope(file_name: &str) -> CalibrationEnvelope {
    let path = examples_dir().join(file_name);
    let json = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&json).unwrap_or_else(|err| {
        panic!(
            "deserialize {} as CalibrationEnvelope: {err}",
            path.display()
        )
    })
}

pub(crate) fn execute(envelope: &CalibrationEnvelope) -> MarketContext {
    let result = engine::execute(envelope).expect("calibration engine succeeded");
    MarketContext::try_from(result.result.final_market)
        .expect("rehydrate MarketContext from final_market state")
}

#[test]
fn example_01_usd_discount_builds_queryable_curve() {
    let envelope = load_envelope("01_usd_discount.json");
    let market = execute(&envelope);

    let curve = market
        .get_discount("USD-OIS")
        .expect("USD-OIS discount curve present in calibrated market");

    // Discount factor at the curve base date is 1.0 by construction; a
    // forward date should produce a strictly positive DF less than 1 as a
    // sanity check that the bootstrap actually populated knot points.
    let df_today = curve.df(0.0);
    assert!(
        (df_today - 1.0).abs() < 1e-9,
        "df at t=0 should be 1.0, got {df_today}"
    );

    let df_one_year = curve.df(1.0);
    assert!(
        df_one_year > 0.0 && df_one_year < 1.0,
        "df at t=1y should be in (0, 1), got {df_one_year}"
    );
}
