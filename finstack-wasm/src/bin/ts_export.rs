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
use ts_rs::TS;

#[cfg(feature = "ts_export")]
fn main() -> Result<(), Box<dyn Error>> {
    // Market data types
    CurvePointWire::export()?;
    DiscountCurveWire::export()?;
    MarketContextWire::export()?;
    BondSpec::export()?;
    ValuationResultWire::export()?;

    // Statement types
    NodeSpecWire::export()?;
    FinancialModelWire::export()?;
    StatementResultsWire::export()?;
    StatementResultsMetaWire::export()?;

    // Scenario types
    ScenarioSpecWire::export()?;
    ScenarioOperationWire::export()?;

    // Error type
    ErrorWire::export()?;

    println!("TypeScript types exported successfully!");
    Ok(())
}

#[cfg(not(feature = "ts_export"))]
fn main() {
    eprintln!("Enable ts_export feature to emit TypeScript bindings.");
}
