//! Margin and collateral management WASM bindings.
//!
//! This module provides industry-standard margining and collateral management
//! following ISDA, BCBS-IOSCO, GMRA, and clearing house standards.

mod calculator;
mod csa;
mod enums;
mod parameters;

pub use calculator::{JsVmCalculator, JsVmResult};
pub use csa::JsCsaSpec;
pub use enums::{JsClearingStatus, JsImMethodology, JsMarginTenor};
pub use parameters::{JsImParameters, JsMarginCallTiming, JsVmParameters};
