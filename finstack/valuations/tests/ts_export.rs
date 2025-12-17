#![cfg(feature = "ts_export")]

use finstack_valuations::calibration::quotes::{
    CreditQuote, FutureSpecs, InflationQuote, MarketQuote, RatesQuote, VolQuote,
};
use finstack_valuations::calibration::{
    CalibrationConfig, MultiCurveConfig, RateBounds, SolverConfig, ValidationMode,
};
use ts_rs::TS;

const OUT_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../finstack-wasm/examples/src/types/generated"
);

#[test]
fn export_calibration_types() {
    std::fs::create_dir_all(OUT_DIR).expect("create generated types dir");
    std::env::set_var("TS_RS_EXPORT_DIR", OUT_DIR);

    CalibrationConfig::export().expect("export CalibrationConfig");
    SolverConfig::export().expect("export SolverConfig");
    MultiCurveConfig::export().expect("export MultiCurveConfig");
    RateBounds::export().expect("export RateBounds");
    ValidationMode::export().expect("export ValidationMode");

    RatesQuote::export().expect("export RatesQuote");
    CreditQuote::export().expect("export CreditQuote");
    VolQuote::export().expect("export VolQuote");
    InflationQuote::export().expect("export InflationQuote");
    MarketQuote::export().expect("export MarketQuote");
    FutureSpecs::export().expect("export FutureSpecs");
}
