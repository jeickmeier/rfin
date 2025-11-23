use super::config::{PyCalibrationConfig, PyMultiCurveConfig};
use super::quote::{PyCreditQuote, PyInflationQuote, PyRatesQuote, PyVolQuote};
use super::report::PyCalibrationReport;
use crate::core::common::args::{CurrencyArg, DayCountArg, InterpStyleArg};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyBaseCorrelationCurve, PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use crate::errors::core_to_py;
use finstack_core::market_data::context::MarketContext as CoreMarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationInterpolation, InflationLag};
use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_core::types::CurveId;
use finstack_valuations::calibration::methods::{
    BaseCorrelationCalibrator, DiscountCurveCalibrator, ForwardCurveCalibrator,
    HazardCurveCalibrator, InflationCurveCalibrator, SurfaceInterp, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::{
    Calibrator, CreditQuote, InflationQuote, RatesQuote, VolQuote,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

fn parse_seniority(label: &str) -> PyResult<Seniority> {
    label
        .parse()
        .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e))
}

fn parse_inflation_interp(name: Option<&str>) -> PyResult<InflationInterpolation> {
    match name {
        None => Ok(InflationInterpolation::Linear),
        Some(s) => s
            .parse()
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e)),
    }
}

fn parse_surface_interp(name: Option<&str>) -> PyResult<SurfaceInterp> {
    match name {
        None => Ok(SurfaceInterp::Bilinear),
        Some(s) => s
            .parse()
            .map_err(|e: String| pyo3::exceptions::PyValueError::new_err(e)),
    }
}

fn collect_rates_quotes(quotes: Vec<Py<PyRatesQuote>>) -> PyResult<Vec<RatesQuote>> {
    Python::with_gil(|py| {
        let mut result = Vec::with_capacity(quotes.len());
        for quote in quotes {
            let bound = quote.bind(py);
            result.push(bound.borrow().inner.clone());
        }
        Ok(result)
    })
}

fn collect_credit_quotes(quotes: Vec<Py<PyCreditQuote>>) -> PyResult<Vec<CreditQuote>> {
    Python::with_gil(|py| {
        let mut result = Vec::with_capacity(quotes.len());
        for quote in quotes {
            let bound = quote.bind(py);
            result.push(bound.borrow().inner.clone());
        }
        Ok(result)
    })
}

fn collect_inflation_quotes(quotes: Vec<Py<PyInflationQuote>>) -> PyResult<Vec<InflationQuote>> {
    Python::with_gil(|py| {
        let mut result = Vec::with_capacity(quotes.len());
        for quote in quotes {
            let bound = quote.bind(py);
            result.push(bound.borrow().inner.clone());
        }
        Ok(result)
    })
}

fn collect_vol_quotes(quotes: Vec<Py<PyVolQuote>>) -> PyResult<Vec<VolQuote>> {
    Python::with_gil(|py| {
        let mut result = Vec::with_capacity(quotes.len());
        for quote in quotes {
            let bound = quote.bind(py);
            result.push(bound.borrow().inner.clone());
        }
        Ok(result)
    })
}

fn base_context_from_option(market: Option<PyRef<PyMarketContext>>) -> CoreMarketContext {
    market
        .map(|ctx| ctx.inner.clone())
        .unwrap_or_else(CoreMarketContext::new)
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "DiscountCurveCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyDiscountCurveCalibrator {
    inner: DiscountCurveCalibrator,
}

impl PyDiscountCurveCalibrator {
    pub(crate) fn new(inner: DiscountCurveCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDiscountCurveCalibrator {
    #[new]
    #[pyo3(text_signature = "(curve_id, base_date, currency)")]
    fn ctor(
        curve_id: &str,
        base_date: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(DiscountCurveCalibrator::new(curve_id, date, ccy)))
    }

    #[pyo3(text_signature = "(self, config)")]
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        let inner = self.inner.clone().with_config(config.inner.clone());
        Self::new(inner)
    }

    #[pyo3(text_signature = "(self, multi_curve)")]
    fn with_multi_curve_config(&self, multi_curve: PyRef<PyMultiCurveConfig>) -> Self {
        let mut inner = self.inner.clone();
        inner = inner.with_multi_curve_config(multi_curve.inner.clone());
        Self::new(inner)
    }

    #[pyo3(text_signature = "(self, interp)")]
    fn with_solve_interp(&self, interp: Bound<'_, PyAny>) -> PyResult<Self> {
        let InterpStyleArg(style) = InterpStyleArg::extract_bound(&interp)?;
        let inner = self.inner.clone().with_solve_interp(style);
        Ok(Self::new(inner))
    }

    #[pyo3(signature = (quotes, market=None))]
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyRatesQuote>>,
        market: Option<PyRef<PyMarketContext>>,
    ) -> PyResult<(PyDiscountCurve, PyCalibrationReport)> {
        let rust_quotes = collect_rates_quotes(quotes)?;
        let base_context = base_context_from_option(market);

        // Release GIL for compute-heavy calibration work
        let (curve, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &base_context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyDiscountCurve::new_arc(Arc::new(curve)),
            PyCalibrationReport::new(report),
        ))
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "ForwardCurveCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyForwardCurveCalibrator {
    inner: ForwardCurveCalibrator,
}

impl PyForwardCurveCalibrator {
    pub(crate) fn new(inner: ForwardCurveCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyForwardCurveCalibrator {
    #[new]
    #[pyo3(text_signature = "(curve_id, tenor_years, base_date, currency, discount_curve_id)")]
    fn ctor(
        curve_id: &str,
        tenor_years: f64,
        base_date: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(ForwardCurveCalibrator::new(
            curve_id,
            tenor_years,
            date,
            ccy,
            discount_curve_id,
        )))
    }

    #[pyo3(text_signature = "(self, config)")]
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        Self::new(self.inner.clone().with_config(config.inner.clone()))
    }

    #[pyo3(text_signature = "(self, interp)")]
    fn with_solve_interp(&self, interp: Bound<'_, PyAny>) -> PyResult<Self> {
        let InterpStyleArg(style) = InterpStyleArg::extract_bound(&interp)?;
        Ok(Self::new(self.inner.clone().with_solve_interp(style)))
    }

    #[pyo3(text_signature = "(self, day_count)")]
    fn with_time_dc(&self, day_count: Bound<'_, PyAny>) -> PyResult<Self> {
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(self.inner.clone().with_time_dc(dc)))
    }

    #[pyo3(signature = (quotes, market))]
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyRatesQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyForwardCurve, PyCalibrationReport)> {
        let rust_quotes = collect_rates_quotes(quotes)?;
        let base_context = market.inner.clone();

        // Release GIL for compute-heavy calibration work
        let (curve, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &base_context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyForwardCurve::new_arc(Arc::new(curve)),
            PyCalibrationReport::new(report),
        ))
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "HazardCurveCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyHazardCurveCalibrator {
    inner: HazardCurveCalibrator,
}

impl PyHazardCurveCalibrator {
    pub(crate) fn new(inner: HazardCurveCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHazardCurveCalibrator {
    #[new]
    #[pyo3(signature = (entity, seniority, recovery_rate, base_date, currency, discount_curve=None))]
    fn ctor(
        entity: &str,
        seniority: &str,
        recovery_rate: f64,
        base_date: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        discount_curve: Option<&str>,
    ) -> PyResult<Self> {
        let seniority_enum = parse_seniority(seniority)?;
        let date = py_to_date(&base_date)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        let inner = if let Some(curve_id) = discount_curve {
            HazardCurveCalibrator::new(entity, seniority_enum, recovery_rate, date, ccy, curve_id)
        } else {
            HazardCurveCalibrator::new_with_default_discount(
                entity,
                seniority_enum,
                recovery_rate,
                date,
                ccy,
            )
        };
        Ok(Self::new(inner))
    }

    #[pyo3(text_signature = "(self, config)")]
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        Self::new(self.inner.clone().with_config(config.inner.clone()))
    }

    #[pyo3(text_signature = "(self, interpolation)")]
    fn with_par_interp(&self, interpolation: &str) -> PyResult<Self> {
        let interp = match interpolation.to_ascii_lowercase().as_str() {
            "linear" => {
                finstack_core::market_data::term_structures::hazard_curve::ParInterp::Linear
            }
            "log_linear" | "loglinear" => {
                finstack_core::market_data::term_structures::hazard_curve::ParInterp::LogLinear
            }
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown par interpolation: {other}"
                )))
            }
        };
        Ok(Self::new(self.inner.clone().with_par_interp(interp)))
    }

    #[pyo3(signature = (quotes, market))]
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyCreditQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyHazardCurve, PyCalibrationReport)> {
        let rust_quotes = collect_credit_quotes(quotes)?;
        let context = market.inner.clone();

        // Release GIL for compute-heavy calibration work
        let (curve, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyHazardCurve::new_arc(Arc::new(curve)),
            PyCalibrationReport::new(report),
        ))
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "InflationCurveCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyInflationCurveCalibrator {
    inner: InflationCurveCalibrator,
}

impl PyInflationCurveCalibrator {
    pub(crate) fn new(inner: InflationCurveCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationCurveCalibrator {
    #[new]
    #[pyo3(text_signature = "(curve_id, base_date, currency, base_cpi, discount_curve_id)")]
    fn ctor(
        curve_id: &str,
        base_date: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        base_cpi: f64,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(InflationCurveCalibrator::new(
            curve_id,
            date,
            ccy,
            base_cpi,
            discount_curve_id,
        )))
    }

    #[pyo3(text_signature = "(self, config)")]
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        Self::new(self.inner.clone().with_config(config.inner.clone()))
    }

    #[pyo3(text_signature = "(self, interp)")]
    fn with_solve_interp(&self, interp: Bound<'_, PyAny>) -> PyResult<Self> {
        let InterpStyleArg(style) = InterpStyleArg::extract_bound(&interp)?;
        Ok(Self::new(self.inner.clone().with_solve_interp(style)))
    }

    #[pyo3(text_signature = "(self, day_count)")]
    fn with_time_dc(&self, day_count: Bound<'_, PyAny>) -> PyResult<Self> {
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(self.inner.clone().with_time_dc(dc)))
    }

    #[pyo3(text_signature = "(self, day_count)")]
    fn with_accrual_dc(&self, day_count: Bound<'_, PyAny>) -> PyResult<Self> {
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(self.inner.clone().with_accrual_dc(dc)))
    }

    #[pyo3(text_signature = "(self, months)")]
    fn with_inflation_lag_months(&self, months: u8) -> Self {
        Self::new(
            self.inner
                .clone()
                .with_inflation_lag(InflationLag::Months(months)),
        )
    }

    #[pyo3(text_signature = "(self, days)")]
    fn with_inflation_lag_days(&self, days: u16) -> Self {
        Self::new(
            self.inner
                .clone()
                .with_inflation_lag(InflationLag::Days(days)),
        )
    }

    #[pyo3(text_signature = "(self)")]
    fn with_no_inflation_lag(&self) -> Self {
        Self::new(self.inner.clone().with_inflation_lag(InflationLag::None))
    }

    #[pyo3(text_signature = "(self, adjustments)")]
    fn with_seasonality_adjustments(&self, adjustments: Vec<f64>) -> PyResult<Self> {
        if adjustments.len() != 12 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "seasonality adjustments must contain 12 monthly factors",
            ));
        }
        let mut factors = [0.0_f64; 12];
        factors.copy_from_slice(&adjustments);
        Ok(Self::new(
            self.inner.clone().with_seasonality_adjustments(factors),
        ))
    }

    #[pyo3(text_signature = "(self, interpolation)")]
    fn with_inflation_interpolation(&self, interpolation: Option<&str>) -> PyResult<Self> {
        let interp = parse_inflation_interp(interpolation)?;
        Ok(Self::new(
            self.inner.clone().with_inflation_interpolation(interp),
        ))
    }

    #[pyo3(signature = (quotes, market=None))]
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyInflationQuote>>,
        market: Option<PyRef<PyMarketContext>>,
    ) -> PyResult<(PyInflationCurve, PyCalibrationReport)> {
        let rust_quotes = collect_inflation_quotes(quotes)?;
        let context = base_context_from_option(market);

        // Release GIL for compute-heavy calibration work
        let (curve, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyInflationCurve::new_arc(Arc::new(curve)),
            PyCalibrationReport::new(report),
        ))
    }
}

#[pyclass(
    module = "finstack.valuations.calibration",
    name = "VolSurfaceCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyVolSurfaceCalibrator {
    inner: VolSurfaceCalibrator,
}

impl PyVolSurfaceCalibrator {
    pub(crate) fn new(inner: VolSurfaceCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyVolSurfaceCalibrator {
    #[new]
    #[pyo3(text_signature = "(surface_id, beta, target_expiries, target_strikes)")]
    fn ctor(
        surface_id: &str,
        beta: f64,
        target_expiries: Vec<f64>,
        target_strikes: Vec<f64>,
    ) -> PyResult<Self> {
        if target_expiries.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "target_expiries must not be empty",
            ));
        }
        if target_strikes.len() < 3 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "target_strikes must contain at least three points",
            ));
        }
        Ok(Self::new(VolSurfaceCalibrator::new(
            surface_id,
            beta,
            target_expiries,
            target_strikes,
        )))
    }

    #[pyo3(text_signature = "(self, base_date)")]
    fn with_base_date(&self, base_date: Bound<'_, PyAny>) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        Ok(Self::new(self.inner.clone().with_base_date(date)))
    }

    #[pyo3(text_signature = "(self, config)")]
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        Self::new(self.inner.clone().with_config(config.inner.clone()))
    }

    #[pyo3(text_signature = "(self, currency)")]
    fn with_base_currency(&self, currency: Bound<'_, PyAny>) -> PyResult<Self> {
        let CurrencyArg(ccy) = CurrencyArg::extract_bound(&currency)?;
        Ok(Self::new(self.inner.clone().with_base_currency(ccy)))
    }

    #[pyo3(text_signature = "(self, day_count)")]
    fn with_time_dc(&self, day_count: Bound<'_, PyAny>) -> PyResult<Self> {
        let DayCountArg(dc) = DayCountArg::extract_bound(&day_count)?;
        Ok(Self::new(self.inner.clone().with_time_dc(dc)))
    }

    #[pyo3(text_signature = "(self, interpolation)")]
    fn with_surface_interp(&self, interpolation: Option<&str>) -> PyResult<Self> {
        let interp = parse_surface_interp(interpolation)?;
        Ok(Self::new(self.inner.clone().with_surface_interp(interp)))
    }

    #[pyo3(text_signature = "(self, discount_curve_id)")]
    fn with_discount_id(&self, discount_curve_id: &str) -> Self {
        Self::new(self.inner.clone().with_discount_id(discount_curve_id))
    }

    #[pyo3(signature = (quotes, market))]
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyVolQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyVolSurface, PyCalibrationReport)> {
        let rust_quotes = collect_vol_quotes(quotes)?;
        let context = market.inner.clone();

        // Release GIL for compute-heavy calibration work
        let (surface, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyVolSurface::new_arc(Arc::new(surface)),
            PyCalibrationReport::new(report),
        ))
    }
}

/// Base correlation curve calibrator for CDS tranches.
///
/// Calibrates base correlation curves from CDS tranche quotes using the
/// market-standard bootstrapping methodology with one-factor Gaussian Copula.
///
/// The bootstrapping works by:
/// 1. Sorting tranches by detachment point (equity to senior)
/// 2. For each tranche [A, D], solving for ρ(D) such that:
///    Price([A, D]) = Price([0, D], ρ(D)) - Price([0, A], ρ(A))
/// 3. Using previously solved correlations for [0, A] pricing
///
/// Examples:
///     >>> from finstack.valuations.calibration import (
///     ...     BaseCorrelationCalibrator, CalibrationConfig, CreditQuote
///     ... )
///     >>> import datetime
///     ///
///     >>> calibrator = BaseCorrelationCalibrator(
///     ...     index_id="CDX.NA.IG.42",
///     ...     series=42,
///     ...     maturity_years=5.0,
///     ...     base_date=datetime.date(2025, 1, 1)
///     ... )
///     >>> calibrator = calibrator.with_discount_curve_id("USD-OIS")
///     >>> calibrator = calibrator.with_detachment_points([3.0, 7.0, 10.0, 15.0, 30.0])
///     ///
///     >>> # Prepare tranche quotes
///     >>> quotes = [
///     ...     CreditQuote.cds_tranche(
///     ...         index="CDX.NA.IG.42",
///     ...         attachment=0.0,
///     ...         detachment=3.0,
///     ...         maturity=datetime.date(2030, 1, 1),
///     ...         upfront_pct=15.0,
///     ...         running_spread_bp=500.0
///     ...     ),
///     ...     # ... more tranches
///     ... ]
///     ///
///     >>> # Calibrate
///     >>> curve, report = calibrator.calibrate(quotes, market)
///     >>> print(f"Success: {report.success}, Iterations: {report.iterations}")
#[pyclass(
    module = "finstack.valuations.calibration",
    name = "BaseCorrelationCalibrator",
    frozen
)]
#[derive(Clone)]
pub struct PyBaseCorrelationCalibrator {
    inner: BaseCorrelationCalibrator,
}

impl PyBaseCorrelationCalibrator {
    pub(crate) fn new(inner: BaseCorrelationCalibrator) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBaseCorrelationCalibrator {
    #[new]
    #[pyo3(text_signature = "(index_id, series, maturity_years, base_date)")]
    /// Create a base correlation calibrator for a single maturity.
    ///
    /// Args:
    ///     index_id: Index identifier (e.g., "CDX.NA.IG.42", "iTraxx.Europe.40")
    ///     series: Index series number
    ///     maturity_years: Maturity for correlation curve in years (e.g., 5.0)
    ///     base_date: Base date for calibration
    ///
    /// Returns:
    ///     BaseCorrelationCalibrator: Configured calibrator with default settings
    ///
    /// Notes:
    ///     - Default discount curve: "USD-OIS"
    ///     - Default detachment points: [3.0, 7.0, 10.0, 15.0, 30.0] percent
    ///     - Default interpolation: Linear
    fn ctor(
        index_id: &str,
        series: u16,
        maturity_years: f64,
        base_date: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let date = py_to_date(&base_date)?;
        Ok(Self::new(BaseCorrelationCalibrator::new(
            index_id,
            series,
            maturity_years,
            date,
        )))
    }

    #[pyo3(text_signature = "(self, config)")]
    /// Set calibration configuration.
    ///
    /// Args:
    ///     config: Calibration configuration for solver tolerances and settings
    ///
    /// Returns:
    ///     BaseCorrelationCalibrator: Updated calibrator
    fn with_config(&self, config: PyRef<PyCalibrationConfig>) -> Self {
        Self::new(self.inner.clone().with_config(config.inner.clone()))
    }

    #[pyo3(text_signature = "(self, points)")]
    /// Set custom detachment points for calibration.
    ///
    /// Args:
    ///     points: List of detachment points in percent (e.g., [3.0, 7.0, 10.0, 15.0, 30.0])
    ///
    /// Returns:
    ///     BaseCorrelationCalibrator: Updated calibrator
    ///
    /// Raises:
    ///     ValueError: If points list is empty or contains invalid values
    fn with_detachment_points(&self, points: Vec<f64>) -> PyResult<Self> {
        if points.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "detachment_points cannot be empty",
            ));
        }
        for &p in &points {
            if p <= 0.0 || p > 100.0 {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "detachment point {} must be in (0, 100]",
                    p
                )));
            }
        }
        Ok(Self::new(self.inner.clone().with_detachment_points(points)))
    }

    #[pyo3(text_signature = "(self, discount_curve_id)")]
    /// Set the discount curve identifier for tranche pricing.
    ///
    /// Args:
    ///     discount_curve_id: Discount curve identifier (e.g., "USD-OIS", "EUR-ESTR")
    ///
    /// Returns:
    ///     BaseCorrelationCalibrator: Updated calibrator
    fn with_discount_curve_id(&self, discount_curve_id: Bound<'_, PyAny>) -> PyResult<Self> {
        let curve_id = CurveId::new(discount_curve_id.extract::<&str>()?);
        Ok(Self::new(
            self.inner.clone().with_discount_curve_id(curve_id),
        ))
    }

    #[pyo3(signature = (quotes, market))]
    /// Calibrate base correlation curve from CDS tranche quotes.
    ///
    /// Args:
    ///     quotes: List of CreditQuote objects (must contain CDSTranche quotes)
    ///     market: Market context with discount and credit curves
    ///
    /// Returns:
    ///     tuple[BaseCorrelationCurve, CalibrationReport]: Calibrated curve and report
    ///
    /// Raises:
    ///     RuntimeError: If calibration fails or insufficient quotes provided
    ///     ValueError: If quotes are invalid
    ///
    /// Notes:
    ///     Only CDSTranche quotes matching the calibrator's index_id will be used.
    ///     Quotes are automatically sorted by detachment point for bootstrapping.
    fn calibrate(
        &self,
        py: Python<'_>,
        quotes: Vec<Py<PyCreditQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyBaseCorrelationCurve, PyCalibrationReport)> {
        let rust_quotes = collect_credit_quotes(quotes)?;
        let context = market.inner.clone();

        // Release GIL for compute-heavy calibration work
        let (curve, report) = py.allow_threads(|| {
            self.inner
                .calibrate(&rust_quotes, &context)
                .map_err(core_to_py)
        })?;

        Ok((
            PyBaseCorrelationCurve::new_arc(Arc::new(curve)),
            PyCalibrationReport::new(report),
        ))
    }

    fn __repr__(&self) -> String {
        format!(
            "BaseCorrelationCalibrator(index='{}', series={}, maturity={}y)",
            self.inner.index_id, self.inner.series, self.inner.maturity_years
        )
    }
}

pub(crate) fn register<'py>(
    py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyDiscountCurveCalibrator>()?;
    module.add_class::<PyForwardCurveCalibrator>()?;
    module.add_class::<PyHazardCurveCalibrator>()?;
    module.add_class::<PyInflationCurveCalibrator>()?;
    module.add_class::<PyVolSurfaceCalibrator>()?;
    module.add_class::<PyBaseCorrelationCalibrator>()?;

    let exports = [
        "DiscountCurveCalibrator",
        "ForwardCurveCalibrator",
        "HazardCurveCalibrator",
        "InflationCurveCalibrator",
        "VolSurfaceCalibrator",
        "BaseCorrelationCalibrator",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
