//! Python bindings for market data functionality.
//!
//! This module provides Python-friendly wrappers around finstack-core's market data
//! structures including curves, surfaces, interpolation methods, and containers.
//!
//! The module is organized into:
//! - `interpolation`: Interpolation style enumeration
//! - `curves`: Discount, forward, hazard, and inflation curves
//! - `surfaces`: Volatility surfaces
//! - `curve_set`: Container for managing multiple curves
//! - `curve_id`: Curve identifier wrapper

use pyo3::prelude::*;

pub mod curve_id;
pub mod curve_set;
pub mod curves;
pub mod interpolation;
pub mod surfaces;

// Re-export commonly used types
pub use curve_id::PyCurveId;
pub use curve_set::PyCurveSet;
pub use curves::{PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve};
pub use interpolation::PyInterpStyle;
pub use surfaces::PyVolSurface;

/// Register the market_data submodule with Python
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "market_data")?;

    // Register enums and classes
    m.add_class::<PyInterpStyle>()?;
    m.add_class::<PyDiscountCurve>()?;
    m.add_class::<PyForwardCurve>()?;
    m.add_class::<PyHazardCurve>()?;
    m.add_class::<PyInflationCurve>()?;
    m.add_class::<PyVolSurface>()?;
    m.add_class::<PyCurveSet>()?;
    m.add_class::<PyCurveId>()?;

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.market_data", &m)?;

    Ok(())
}
