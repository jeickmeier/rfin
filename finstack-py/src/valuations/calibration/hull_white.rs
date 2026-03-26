use crate::errors::core_to_py;
use crate::valuations::calibration::report::PyCalibrationReport;
use finstack_valuations::calibration::hull_white::{
    calibrate_hull_white_to_swaptions, HullWhiteParams, SwaptionQuote,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use pyo3::Bound;

/// Hull-White one-factor model parameters.
///
/// The Hull-White model specifies short rate dynamics:
///
///     dr(t) = [theta(t) - kappa * r(t)] dt + sigma * dW(t)
///
/// Parameters:
///     kappa: Mean reversion speed (must be positive).
///     sigma: Short rate volatility (must be positive).
///
/// Examples:
///     >>> params = HullWhiteParams(0.05, 0.01)
///     >>> params.kappa
///     0.05
///     >>> params.sigma
///     0.01
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "HullWhiteParams",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyHullWhiteParams {
    pub(crate) inner: HullWhiteParams,
}

impl PyHullWhiteParams {
    pub(crate) fn new(inner: HullWhiteParams) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHullWhiteParams {
    #[new]
    #[pyo3(text_signature = "(kappa, sigma)")]
    /// Create validated Hull-White parameters.
    ///
    /// Args:
    ///     kappa: Mean reversion speed (must be positive).
    ///     sigma: Short rate volatility (must be positive).
    ///
    /// Returns:
    ///     HullWhiteParams: Validated Hull-White parameters.
    ///
    /// Raises:
    ///     ValueError: If kappa <= 0 or sigma <= 0.
    fn ctor(kappa: f64, sigma: f64) -> PyResult<Self> {
        HullWhiteParams::new(kappa, sigma)
            .map(Self::new)
            .map_err(core_to_py)
    }

    /// Mean reversion speed (kappa).
    ///
    /// Returns:
    ///     float: Mean reversion speed parameter.
    #[getter]
    fn kappa(&self) -> f64 {
        self.inner.kappa
    }

    /// Short rate volatility (sigma).
    ///
    /// Returns:
    ///     float: Short rate volatility parameter.
    #[getter]
    fn sigma(&self) -> f64 {
        self.inner.sigma
    }

    #[pyo3(text_signature = "(self, t1, t2)")]
    /// B function: B(t1, t2) = (1 - exp(-kappa * (t2 - t1))) / kappa.
    ///
    /// For small kappa, uses Taylor expansion B ~ (t2 - t1) to avoid
    /// division by near-zero.
    ///
    /// Args:
    ///     t1: Start time.
    ///     t2: End time.
    ///
    /// Returns:
    ///     float: B function value.
    fn b_function(&self, t1: f64, t2: f64) -> f64 {
        self.inner.b_function(t1, t2)
    }

    #[pyo3(text_signature = "(self, t, big_t, s)")]
    /// Zero-coupon bond option volatility under HW1F.
    ///
    /// sigma_P(t, T, S) = B(T,S) * sigma * sqrt((1 - exp(-2*kappa*(T-t))) / (2*kappa))
    ///
    /// Args:
    ///     t: Current time.
    ///     big_t: Option expiry time (T).
    ///     s: Bond maturity time (S > T).
    ///
    /// Returns:
    ///     float: Bond option volatility.
    fn bond_option_vol(&self, t: f64, big_t: f64, s: f64) -> f64 {
        self.inner.bond_option_vol(t, big_t, s)
    }

    fn __repr__(&self) -> String {
        format!(
            "HullWhiteParams(kappa={:.6}, sigma={:.6})",
            self.inner.kappa, self.inner.sigma
        )
    }
}

/// Market quote for a European swaption used in HW1F calibration.
///
/// Represents an ATM (or off-ATM) European swaption with its market volatility.
///
/// Parameters:
///     expiry: Swaption expiry in years.
///     tenor: Underlying swap tenor in years.
///     volatility: Market-quoted volatility.
///     is_normal_vol: True for normal (Bachelier) vol, False for lognormal (Black-76) vol.
///
/// Examples:
///     >>> quote = SwaptionQuote(1.0, 5.0, 0.005, True)
///     >>> quote.expiry
///     1.0
///     >>> quote.is_normal_vol
///     True
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "SwaptionQuote",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PySwaptionQuote {
    pub(crate) inner: SwaptionQuote,
}

impl PySwaptionQuote {
    pub(crate) fn new(inner: SwaptionQuote) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySwaptionQuote {
    #[new]
    #[pyo3(text_signature = "(expiry, tenor, volatility, is_normal_vol)")]
    /// Create a swaption market quote.
    ///
    /// Args:
    ///     expiry: Swaption expiry in years (must be positive).
    ///     tenor: Underlying swap tenor in years (must be positive).
    ///     volatility: Market-quoted volatility (must be positive).
    ///     is_normal_vol: True for normal (Bachelier) vol, False for lognormal (Black-76) vol.
    ///
    /// Returns:
    ///     SwaptionQuote: Swaption market quote.
    ///
    /// Raises:
    ///     ValueError: If expiry, tenor, or volatility are not positive.
    fn ctor(expiry: f64, tenor: f64, volatility: f64, is_normal_vol: bool) -> PyResult<Self> {
        SwaptionQuote::try_new(expiry, tenor, volatility, is_normal_vol)
            .map(Self::new)
            .map_err(core_to_py)
    }

    /// Swaption expiry in years.
    ///
    /// Returns:
    ///     float: Expiry in years.
    #[getter]
    fn expiry(&self) -> f64 {
        self.inner.expiry
    }

    /// Underlying swap tenor in years.
    ///
    /// Returns:
    ///     float: Tenor in years.
    #[getter]
    fn tenor(&self) -> f64 {
        self.inner.tenor
    }

    /// Market-quoted volatility.
    ///
    /// Returns:
    ///     float: Volatility.
    #[getter]
    fn volatility(&self) -> f64 {
        self.inner.volatility
    }

    /// Whether the volatility is normal (Bachelier) or lognormal (Black-76).
    ///
    /// Returns:
    ///     bool: True for normal vol, False for lognormal vol.
    #[getter]
    fn is_normal_vol(&self) -> bool {
        self.inner.is_normal_vol
    }

    fn __repr__(&self) -> String {
        format!(
            "SwaptionQuote(expiry={}, tenor={}, volatility={:.6}, is_normal_vol={})",
            self.inner.expiry, self.inner.tenor, self.inner.volatility, self.inner.is_normal_vol
        )
    }
}

/// Calibrate Hull-White 1-factor parameters to European swaption market data.
///
/// Fits kappa (mean reversion) and sigma (short rate volatility) by minimising
/// squared differences between model and market swaption prices using the
/// Levenberg-Marquardt algorithm.
///
/// The discount factor function ``df`` must return ``P(0, t)`` for any time ``t >= 0``,
/// where ``P(0, 0) ~ 1``. Typically this is a lambda wrapping a flat rate or a
/// ``DiscountCurve.df`` method.
///
/// Args:
///     df: Discount factor function that maps time (float) to discount factor (float).
///         For example: ``lambda t: math.exp(-0.03 * t)`` for a flat 3% curve.
///     quotes: List of ``SwaptionQuote`` market data (at least 2 required).
///
/// Returns:
///     tuple[HullWhiteParams, CalibrationReport]: Calibrated parameters and
///         a report with residual diagnostics.
///
/// Raises:
///     ValueError: If fewer than 2 quotes are provided or inputs are invalid.
///     RuntimeError: If calibration fails to converge.
///
/// Examples:
///     >>> import math
///     >>> quotes = [
///     ...     SwaptionQuote(1.0, 5.0, 0.005, True),
///     ...     SwaptionQuote(5.0, 5.0, 0.006, True),
///     ...     SwaptionQuote(10.0, 5.0, 0.005, True),
///     ... ]
///     >>> params, report = calibrate_hull_white(lambda t: math.exp(-0.03 * t), quotes)
///     >>> report.success
///     True
#[pyfunction]
#[pyo3(name = "calibrate_hull_white", text_signature = "(df, quotes)")]
fn py_calibrate_hull_white(
    py: Python<'_>,
    df: Py<PyAny>,
    quotes: Vec<PySwaptionQuote>,
) -> PyResult<(PyHullWhiteParams, PyCalibrationReport)> {
    // Convert Python SwaptionQuote list to Rust SwaptionQuote vec
    let rust_quotes: Vec<SwaptionQuote> = quotes.iter().map(|q| q.inner).collect();

    // Wrap the Python callable into a Rust closure.
    // We hold the GIL throughout since the Levenberg-Marquardt solver
    // calls df_fn synchronously from within the calibration loop.
    let df_fn = |t: f64| -> f64 {
        df.call1(py, (t,))
            .and_then(|result| result.extract::<f64>(py))
            .unwrap_or(f64::NAN)
    };

    let result = calibrate_hull_white_to_swaptions(&df_fn, &rust_quotes).map_err(core_to_py)?;

    let (params, report) = result;
    Ok((
        PyHullWhiteParams::new(params),
        PyCalibrationReport::new(report),
    ))
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyHullWhiteParams>()?;
    module.add_class::<PySwaptionQuote>()?;
    module.add_function(wrap_pyfunction!(py_calibrate_hull_white, module)?)?;
    Ok(vec![
        "HullWhiteParams",
        "SwaptionQuote",
        "calibrate_hull_white",
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_python() {
        Python::initialize();
    }

    #[test]
    fn swaption_quote_invalid_inputs_map_to_validation_error() {
        init_python();
        let err =
            PySwaptionQuote::ctor(0.0, 5.0, 0.01, true).expect_err("non-positive expiry must fail");

        Python::attach(|py| {
            let type_name = err
                .get_type(py)
                .name()
                .and_then(|name| name.to_str().map(str::to_owned))
                .unwrap_or_else(|_| "<unknown>".to_string());
            assert_eq!(type_name, "ValidationError");
        });
    }
}
