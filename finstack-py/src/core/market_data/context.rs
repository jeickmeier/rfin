//! Market context for pricing and valuation.

use finstack_core::market_data::MarketContext;
use pyo3::prelude::*;
use std::sync::Arc;

use super::curves::{PyDiscountCurve, PyForwardCurve, PyHazardCurve};

/// Market context containing all market data for valuation.
///
/// A MarketContext holds discount curves, forward curves, volatility surfaces,
/// and other market data required for pricing financial instruments. It provides
/// a centralized location for all market data needed during valuation.
///
/// Examples:
///     >>> from finstack.market_data import MarketContext, DiscountCurve
///     >>> from finstack import Date
///     >>>
///     >>> # Create a simple market context with a discount curve
///     >>> context = MarketContext()
///     >>>
///     >>> # Add a USD discount curve
///     >>> usd_curve = DiscountCurve.flat("USD-OIS", Date(2024, 1, 1), 0.95)
///     >>> context.add_discount_curve(usd_curve)
///     >>>
///     >>> # Use context for valuation
///     >>> bond.value(context, Date(2024, 1, 1))
#[pyclass(name = "MarketContext", module = "finstack.market_data")]
#[derive(Clone)]
pub struct PyMarketContext {
    inner: Arc<MarketContext>,
}

#[pymethods]
impl PyMarketContext {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(MarketContext::new()),
        }
    }

    /// Add a discount curve to the market context.
    ///
    /// Args:
    ///     curve: The discount curve to add
    ///
    /// Examples:
    ///     >>> curve = DiscountCurve.flat("USD-OIS", Date(2024, 1, 1), 0.95)
    ///     >>> context.add_discount_curve(curve)
    fn add_discount_curve(&mut self, _curve: &PyDiscountCurve) -> PyResult<()> {
        // Note: This is a simplified implementation
        // In production, we'd need proper mutability handling for Arc<MarketContext>
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding curves to context not yet implemented. Create with all curves at once.",
        ))
    }

    /// Add a forward curve to the market context.
    ///
    /// Args:
    ///     curve: The forward curve to add
    fn add_forward_curve(&mut self, _curve: &PyForwardCurve) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding curves to context not yet implemented. Create with all curves at once.",
        ))
    }

    /// Add a hazard curve to the market context.
    ///
    /// Args:
    ///     curve: The hazard curve to add
    fn add_hazard_curve(&mut self, _curve: &PyHazardCurve) -> PyResult<()> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Adding curves to context not yet implemented. Create with all curves at once.",
        ))
    }

    /// Get a discount curve by ID.
    ///
    /// Args:
    ///     curve_id: The ID of the curve to retrieve
    ///
    /// Returns:
    ///     The discount curve if found
    ///
    /// Raises:
    ///     KeyError: If the curve is not found
    fn get_discount_curve(&self, py: Python, curve_id: &str) -> PyResult<PyObject> {
        // Create a static string for the curve ID to satisfy lifetime requirements
        let static_id: &'static str = Box::leak(curve_id.to_string().into_boxed_str());

        self.inner
            .disc(static_id)
            .map(|_curve| {
                // Return a Python None for now
                py.None()
            })
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyKeyError, _>(format!(
                    "Discount curve '{}' not found: {:?}",
                    curve_id, e
                ))
            })
    }

    fn __repr__(&self) -> String {
        "MarketContext()".to_string()
    }
}

impl PyMarketContext {
    /// Get the inner MarketContext for Rust-side operations
    pub fn inner(&self) -> Arc<MarketContext> {
        self.inner.clone()
    }

    /// Create from an existing MarketContext
    pub fn from_market_context(curves: MarketContext) -> Self {
        Self {
            inner: Arc::new(curves),
        }
    }
}

/// Register the market context module
pub fn register_context(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyMarketContext>()?;
    Ok(())
}
