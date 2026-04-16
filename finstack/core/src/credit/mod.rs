//! Credit risk modeling primitives.
//!
//! - [`migration`]: transition matrices, generator extraction, CTMC simulation.
//! - [`scoring`]: academic credit scoring models (Altman, Ohlson, Zmijewski).
//! - [`pd`]: PD calibration, term structures, and master scale mapping.

pub mod migration;
pub mod pd;
pub mod scoring;
