use finstack_valuations::calibration::{
    CalibrationConfig, RateBounds, SolverConfig, ValidationMode,
};
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CdsTrancheQuote;
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote as RatesQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use std::sync::Once;
use ts_rs::TS;

const OUT_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../finstack-wasm/examples/src/types/generated"
);

/// Initialize the TS export directory exactly once across all test runs.
/// Using `Once` ensures thread-safety when tests run in parallel.
static INIT_EXPORT_DIR: Once = Once::new();

fn init_export_dir() {
    INIT_EXPORT_DIR.call_once(|| {
        std::fs::create_dir_all(OUT_DIR).expect("create generated types dir");
        // SAFETY: This is called exactly once via `Once::call_once`, so there's no
        // race condition with other threads reading/writing this env var.
        unsafe {
            std::env::set_var("TS_RS_EXPORT_DIR", OUT_DIR);
        }
    });
}

#[test]
fn export_calibration_types() {
    init_export_dir();

    CalibrationConfig::export().expect("export CalibrationConfig");
    SolverConfig::export().expect("export SolverConfig");
    RateBounds::export().expect("export RateBounds");
    ValidationMode::export().expect("export ValidationMode");

    RatesQuote::export().expect("export RatesQuote");
    CdsQuote::export().expect("export CdsQuote");
    CdsTrancheQuote::export().expect("export CdsTrancheQuote");
    VolQuote::export().expect("export VolQuote");
    InflationQuote::export().expect("export InflationQuote");
    MarketQuote::export().expect("export MarketQuote");
}
