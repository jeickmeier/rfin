#[cfg(feature = "ts_export")]
use finstack_wasm::{
    // Market data wire types
    BondSpec,
    CurvePointWire,
    DiscountCurveWire,
    // Statement wire types
    ErrorWire,
    FinancialModelWire,
    MarketContextWire,
    NodeSpecWire,
    ScenarioOperationWire,
    ScenarioSpecWire,
    StatementResultsMetaWire,
    StatementResultsWire,
    ValuationResultWire,
};
#[cfg(feature = "ts_export")]
use std::error::Error;
#[cfg(feature = "ts_export")]
use ts_rs::{Config, TS};

#[cfg(feature = "ts_export")]
fn main() -> Result<(), Box<dyn Error>> {
    let cfg = Config::default();

    // Market data types
    CurvePointWire::export(&cfg)?;
    DiscountCurveWire::export(&cfg)?;
    MarketContextWire::export(&cfg)?;
    BondSpec::export(&cfg)?;
    ValuationResultWire::export(&cfg)?;

    // Statement types
    NodeSpecWire::export(&cfg)?;
    FinancialModelWire::export(&cfg)?;
    StatementResultsWire::export(&cfg)?;
    StatementResultsMetaWire::export(&cfg)?;

    // Scenario types
    ScenarioSpecWire::export(&cfg)?;
    ScenarioOperationWire::export(&cfg)?;

    // Error type
    ErrorWire::export(&cfg)?;

    println!("TypeScript types exported successfully!");
    Ok(())
}

#[cfg(not(feature = "ts_export"))]
fn main() {
    eprintln!("Enable ts_export feature to emit TypeScript bindings.");
}
