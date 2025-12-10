#![cfg(feature = "ts_export")]

use finstack_valuations::calibration::{
    CalibrationConfig, CreditQuote, FutureSpecs, InflationQuote, MarketQuote, MultiCurveConfig,
    RateBounds, RatesQuote, SolverKind, ValidationMode, VolQuote,
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
    SolverKind::export().expect("export SolverKind");
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
