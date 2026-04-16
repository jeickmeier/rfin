//! Credit risk modeling primitives.
//!
//! - [`migration`]: Credit migration modeling (JLT / CreditMetrics-style).
//! - [`lgd`]: Loss Given Default models (seniority, workout, downturn, EAD).
//! - [`scoring`]: academic credit scoring models (Altman, Ohlson, Zmijewski).
//! - [`pd`]: PD calibration, term structures, and master scale mapping.

/// Loss Given Default: seniority recovery distributions, workout LGD,
/// downturn adjustments, and EAD computation.
pub mod lgd;

pub mod migration;
pub mod pd;
pub mod scoring;
