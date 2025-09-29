use super::config::{PyCalibrationConfig, PyMultiCurveConfig};
use super::quote::{PyCreditQuote, PyInflationQuote, PyRatesQuote, PyVolQuote};
use super::report::PyCalibrationReport;
use crate::core::common::args::{CurrencyArg, DayCountArg, InterpStyleArg};
use crate::core::error::core_to_py;
use crate::core::market_data::context::PyMarketContext;
use crate::core::market_data::surfaces::PyVolSurface;
use crate::core::market_data::term_structures::{
    PyDiscountCurve, PyForwardCurve, PyHazardCurve, PyInflationCurve,
};
use crate::core::utils::py_to_date;
use finstack_core::market_data::context::MarketContext as CoreMarketContext;
use finstack_core::market_data::scalars::inflation_index::{InflationInterpolation, InflationLag};
use finstack_core::market_data::term_structures::hazard_curve::Seniority;
use finstack_valuations::calibration::methods::{
    DiscountCurveCalibrator, ForwardCurveCalibrator, HazardCurveCalibrator,
    InflationCurveCalibrator, SurfaceInterp, VolSurfaceCalibrator,
};
use finstack_valuations::calibration::{
    Calibrator, CreditQuote, InflationQuote, RatesQuote, VolQuote,
};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::sync::Arc;

fn parse_seniority(label: &str) -> PyResult<Seniority> {
    match label.to_ascii_lowercase().as_str() {
        "senior_secured" | "senior-secured" => Ok(Seniority::SeniorSecured),
        "senior" => Ok(Seniority::Senior),
        "subordinated" => Ok(Seniority::Subordinated),
        "junior" => Ok(Seniority::Junior),
        other => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown seniority: {other}"
        ))),
    }
}

fn parse_inflation_interp(name: Option<&str>) -> PyResult<InflationInterpolation> {
    match name.map(|s| s.to_ascii_lowercase()) {
        None => Ok(InflationInterpolation::Linear),
        Some(ref s) if s == "step" => Ok(InflationInterpolation::Step),
        Some(ref s) if s == "linear" => Ok(InflationInterpolation::Linear),
        Some(other) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown inflation interpolation: {other}"
        ))),
    }
}

fn parse_surface_interp(name: Option<&str>) -> PyResult<SurfaceInterp> {
    match name.map(|s| s.to_ascii_lowercase()) {
        None => Ok(SurfaceInterp::Bilinear),
        Some(ref s) if s == "bilinear" => Ok(SurfaceInterp::Bilinear),
        Some(other) => Err(pyo3::exceptions::PyValueError::new_err(format!(
            "Unknown surface interpolation: {other}"
        ))),
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
        quotes: Vec<Py<PyRatesQuote>>,
        market: Option<PyRef<PyMarketContext>>,
    ) -> PyResult<(PyDiscountCurve, PyCalibrationReport)> {
        let rust_quotes = collect_rates_quotes(quotes)?;
        let base_context = base_context_from_option(market);
        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, &base_context)
            .map_err(core_to_py)?;
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
        quotes: Vec<Py<PyRatesQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyForwardCurve, PyCalibrationReport)> {
        let rust_quotes = collect_rates_quotes(quotes)?;
        let base_context = market.inner.clone();
        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, &base_context)
            .map_err(core_to_py)?;
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
        quotes: Vec<Py<PyCreditQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyHazardCurve, PyCalibrationReport)> {
        let rust_quotes = collect_credit_quotes(quotes)?;
        let context = market.inner.clone();
        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, &context)
            .map_err(core_to_py)?;
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
        quotes: Vec<Py<PyInflationQuote>>,
        market: Option<PyRef<PyMarketContext>>,
    ) -> PyResult<(PyInflationCurve, PyCalibrationReport)> {
        let rust_quotes = collect_inflation_quotes(quotes)?;
        let context = base_context_from_option(market);
        let (curve, report) = self
            .inner
            .calibrate(&rust_quotes, &context)
            .map_err(core_to_py)?;
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
        quotes: Vec<Py<PyVolQuote>>,
        market: PyRef<PyMarketContext>,
    ) -> PyResult<(PyVolSurface, PyCalibrationReport)> {
        let rust_quotes = collect_vol_quotes(quotes)?;
        let context = market.inner.clone();
        let (surface, report) = self
            .inner
            .calibrate(&rust_quotes, &context)
            .map_err(core_to_py)?;
        Ok((
            PyVolSurface::new_arc(Arc::new(surface)),
            PyCalibrationReport::new(report),
        ))
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

    let exports = [
        "DiscountCurveCalibrator",
        "ForwardCurveCalibrator",
        "HazardCurveCalibrator",
        "InflationCurveCalibrator",
        "VolSurfaceCalibrator",
    ];
    module.setattr("__all__", PyList::new(py, &exports)?)?;
    Ok(exports.to_vec())
}
