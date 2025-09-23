//! CDS Index pricing facade and engine re-export.
//!
//! Exposes the pricing entrypoints for `CDSIndex`. Core pricing logic
//! lives in `engine`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.

pub mod engine;
pub mod pricer;

pub use engine::CDSIndexPricer;
