use finstack_valuations::calibration::{
    CalibrationConfig, RateBounds, SolverConfig, ValidationMode,
};
use finstack_valuations::market::quotes::bond::BondQuote;
use finstack_valuations::market::quotes::cds::CdsQuote;
use finstack_valuations::market::quotes::cds_tranche::CDSTrancheQuote;
use finstack_valuations::market::quotes::fx::FxQuote;
use finstack_valuations::market::quotes::inflation::InflationQuote;
use finstack_valuations::market::quotes::market_quote::MarketQuote;
use finstack_valuations::market::quotes::rates::RateQuote as RatesQuote;
use finstack_valuations::market::quotes::vol::VolQuote;
use finstack_valuations::market::quotes::xccy::XccyQuote;
use std::sync::OnceLock;
use ts_rs::TS;

const OUT_DIR: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../finstack-wasm/types/generated"
);

static CONFIG: OnceLock<ts_rs::Config> = OnceLock::new();

fn config() -> &'static ts_rs::Config {
    CONFIG.get_or_init(|| {
        std::fs::create_dir_all(OUT_DIR).expect("create generated types dir");
        ts_rs::Config::new().with_out_dir(OUT_DIR)
    })
}

#[test]
fn export_calibration_types() {
    let cfg = config();

    CalibrationConfig::export(cfg).expect("export CalibrationConfig");
    SolverConfig::export(cfg).expect("export SolverConfig");
    RateBounds::export(cfg).expect("export RateBounds");
    ValidationMode::export(cfg).expect("export ValidationMode");

    BondQuote::export(cfg).expect("export BondQuote");
    RatesQuote::export(cfg).expect("export RatesQuote");
    CdsQuote::export(cfg).expect("export CdsQuote");
    CDSTrancheQuote::export(cfg).expect("export CDSTrancheQuote");
    FxQuote::export(cfg).expect("export FxQuote");
    VolQuote::export(cfg).expect("export VolQuote");
    InflationQuote::export(cfg).expect("export InflationQuote");
    XccyQuote::export(cfg).expect("export XccyQuote");
    MarketQuote::export(cfg).expect("export MarketQuote");
}
