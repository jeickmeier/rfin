//! Python bindings for market data functionality.
//!
//! This module provides Python-friendly wrappers around finstack-core's market data
//! structures including curves, surfaces, interpolation methods, and containers.
//!
//! The module is organized into:
//! - `interpolation`: Interpolation style enumeration
//! - `curves`: Discount, forward, hazard, and inflation curves
//! - `surfaces`: Volatility surfaces
//! - `context`: MarketContext for managing multiple curves and market data
//! - `curve_id`: Curve identifier wrapper

use pyo3::prelude::*;

pub mod context;
pub mod curve_id;
pub mod curve_set;
pub mod curves;
pub mod fx;
pub mod inflation_index;
pub mod interpolation;
pub mod primitives;
pub mod surfaces;

// Re-export commonly used types
pub use context::PyMarketContext;
pub use curve_id::PyCurveId;

pub use curves::{PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve};
pub use fx::{PyFxConversionPolicy, PyFxMatrix, PySimpleFxProvider};
pub use inflation_index::{
    PyInflationIndex, PyInflationIndexBuilder, PyInflationInterpolation, PyInflationLag,
};
pub use interpolation::{PyExtrapolationPolicy, PyInterpStyle};
pub use primitives::{PyMarketScalar, PyScalarTimeSeries, PySeriesInterpolation};
pub use surfaces::PyVolSurface;

/// Register the market_data submodule with Python
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(parent.py(), "market_data")?;

    // Register enums and classes
    m.add_class::<PyInterpStyle>()?;
    m.add_class::<PyExtrapolationPolicy>()?;
    m.add_class::<PyDiscountCurve>()?;
    m.add_class::<PyForwardCurve>()?;
    m.add_class::<PyHazardCurve>()?;
    m.add_class::<PyInflationCurve>()?;
    m.add_class::<PyVolSurface>()?;

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

    // Register market context
    context::register_context(&m)?;

    // Add the submodule to parent
    parent.add_submodule(&m)?;
    parent
        .py()
        .import("sys")?
        .getattr("modules")?
        .set_item("finstack.market_data", &m)?;

    Ok(())
}
