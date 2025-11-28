//! Serialization tests for finstack_core.
//!
//! This module contains:
//! - [`golden`]: Wire format stability tests (serde golden tests)
//! - [`roundtrip`]: Roundtrip serialization tests for various types

#[cfg(feature = "serde")]
mod golden;

#[cfg(feature = "serde")]
mod roundtrip;
