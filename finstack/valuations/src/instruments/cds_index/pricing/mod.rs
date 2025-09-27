//! CDS Index pricing facade and pricer re-export.
//!
//! Exposes the pricing entrypoints for `CDSIndex`. Core pricing logic
//! lives in `pricer`. Instruments and metrics should depend on this
//! module rather than private files to keep the public API stable.

pub mod pricer;

pub use pricer::CDSIndexPricer;
