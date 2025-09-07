//! Money primitives and FX conversion utilities.
//!
//! This module provides:
//! - [`Money`] – a currency‑tagged monetary amount with safe arithmetic
//! - [`fx`] – foreign‑exchange interfaces and helpers used by conversions
//!
//! Arithmetic refuses to mix currencies unless converted explicitly using an
//! [`fx::FxProvider`]. Rounding follows per‑currency scale with configurable
//! policies via `crate::config`.
/// Submodule for FX interfaces
pub mod fx;

mod rounding;
mod types;

pub use types::Money;
