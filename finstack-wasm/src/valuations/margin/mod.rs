//! Margin and collateral management WASM bindings.
//!
//! This module provides industry-standard margining and collateral management
//! following ISDA, BCBS-IOSCO, GMRA, and clearing house standards.

mod enums;
mod parameters;
mod csa;
mod calculator;

pub use enums::{JsImMethodology, JsMarginTenor, JsClearingStatus};
pub use parameters::{JsVmParameters, JsImParameters, JsMarginCallTiming};
pub use csa::JsCsaSpec;
pub use calculator::{JsVmCalculator, JsVmResult};
