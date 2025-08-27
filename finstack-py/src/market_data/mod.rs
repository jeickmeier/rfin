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
pub mod fx;
pub mod inflation_index;
pub mod primitives;
pub mod interpolation;
pub mod context;
pub mod surfaces;

// Re-export commonly used types
pub use curve_id::PyCurveId;
pub use curve_set::PyCurveSet;
pub use curves::{PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve};
pub use fx::{PyFxConversionPolicy, PyFxMatrix, PySimpleFxProvider};
pub use inflation_index::{
    PyInflationIndex, PyInflationIndexBuilder, PyInflationInterpolation, PyInflationLag
};
pub use primitives::{PyMarketScalar, PyScalarTimeSeries, PySeriesInterpolation};
pub use interpolation::PyInterpStyle;
pub use surfaces::PyVolSurface;
pub use context::PyMarketContext;

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
    m.add_class::<PyMarketScalar>()?;
    m.add_class::<PyScalarTimeSeries>()?;
    m.add_class::<PySeriesInterpolation>()?;
    m.add_class::<PyMarketContext>()?;
    
    // Register FX classes
    m.add_class::<PyFxConversionPolicy>()?;
    m.add_class::<PySimpleFxProvider>()?;
    m.add_class::<PyFxMatrix>()?;
    
    // Register inflation index classes
    m.add_class::<PyInflationIndex>()?;
    m.add_class::<PyInflationIndexBuilder>()?;
    m.add_class::<PyInflationInterpolation>()?;
    m.add_class::<PyInflationLag>()?;

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.market_data", &m)?;

    Ok(())
}
