//! Repo pricing module.
//!
//! This module isolates pricing policy and calculations for `Repo` instruments
//! away from data shapes in `types`. Follow the project standard:
//! - `types` defines instrument data and trait impls
//! - `pricing` contains pricing engines/facades
//! - `metrics` contains metric calculators and the registry hook
//!
//! Public API:
//! - `RepoPricer` facade with methods for present value and key pricing helpers
//!
//! Heavy numerics and market interactions live here so we can unit test and
//! evolve pricing independently of the instrument type.

pub mod engine;

/// Back-compat re-export of the main pricing facade.
pub use engine::RepoPricer;
