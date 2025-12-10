#[cfg(feature = "ts_export")]
use finstack_wasm::{
    BondSpec, CurvePointWire, DiscountCurveWire, MarketContextWire, ValuationResultWire,
};
#[cfg(feature = "ts_export")]
use std::error::Error;
#[cfg(feature = "ts_export")]
use ts_rs::TS;

#[cfg(feature = "ts_export")]
fn main() -> Result<(), Box<dyn Error>> {
    CurvePointWire::export()?;
    DiscountCurveWire::export()?;
    MarketContextWire::export()?;
    BondSpec::export()?;
    ValuationResultWire::export()?;
    Ok(())
}

#[cfg(not(feature = "ts_export"))]
fn main() {
    eprintln!("Enable ts_export feature to emit TypeScript bindings.");
}
