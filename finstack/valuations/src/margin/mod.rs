//! Marginable trait implementations for valuations instruments.
//!
//! All margin public types live in the standalone `finstack-margin` crate.
//! Import them from there directly (e.g. `use finstack_margin::OtcMarginSpec;`).
//! This module exists only to host the `Marginable` implementations for
//! valuations' concrete instrument types.

mod impls;
