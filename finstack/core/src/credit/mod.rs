//! Credit risk modeling primitives.
//!
//! - [`migration`]: Credit migration modeling (JLT / CreditMetrics-style).
//! - [`lgd`]: Loss Given Default models (seniority, workout, downturn, EAD).

/// Loss Given Default: seniority recovery distributions, workout LGD,
/// downturn adjustments, and EAD computation.
pub mod lgd;

/// Credit migration: transition matrices, generator extraction, projection,
/// and CTMC path simulation.
pub mod migration;
