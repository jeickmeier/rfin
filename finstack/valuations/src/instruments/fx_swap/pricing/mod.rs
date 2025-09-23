//! FX Swap pricing facade.
//!
//! Re-exports the core pricing engine and wires `Priceable` implementation
//! for `FxSwap` to delegate to the engine. This mirrors the layout used by
//! other instruments (e.g., CDS, IRS).

pub mod engine;
pub mod pricer;
