use crate::core::common::args::DayCountArg;
use crate::core::common::labels::normalize_label;
use crate::core::dates::schedule::PyFrequency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::{DayCount, Tenor};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::rates::cms_option::CmsOption;
use finstack_valuations::instruments::OptionType;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};
use std::sync::Arc;

/// CMS option instrument.
#[pyclass(module = "finstack.valuations.instruments", name = "CmsOption", frozen)]
#[derive(Clone, Debug)]
pub struct PyCmsOption {
    pub(crate) inner: Arc<CmsOption>,
}

impl PyCmsOption {
    pub(crate) fn new(inner: CmsOption) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyCmsOption {
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, strike, cms_tenor, fixing_dates, accrual_fractions, option_type, notional, discount_curve, forward_curve, vol_surface, *, payment_dates=None, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None),
        text_signature = "(cls, instrument_id, strike, cms_tenor, fixing_dates, accrual_fractions, option_type, notional, discount_curve, forward_curve, vol_surface, *, payment_dates=None, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CMS option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     strike: Strike rate in decimal form.
    ///     cms_tenor: Tenor of the CMS swap (e.g., 10.0 for 10Y).
    ///     fixing_dates: List of fixing dates.
    ///     accrual_fractions: List of accrual fractions for each period.
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     forward_curve: Forward/projection curve identifier for CMS rate.
    ///     vol_surface: Volatility surface identifier.
    ///     payment_dates: Optional list of payment dates (defaults to fixing dates).
    ///     swap_fixed_freq: Optional fixed leg frequency (default: Semiannual).
    ///     swap_float_freq: Optional floating leg frequency (default: Quarterly).
    ///     swap_day_count: Optional swap fixed leg day count (default: 30/360).
    ///
    /// Returns:
    ///     CmsOption: Configured CMS option instrument.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        strike: f64,
        cms_tenor: f64,
        fixing_dates: Bound<'_, PyList>,
        accrual_fractions: Bound<'_, PyList>,
        option_type: &str,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        payment_dates: Option<Bound<'_, PyList>>,
        swap_fixed_freq: Option<Bound<'_, PyAny>>,
        swap_float_freq: Option<Bound<'_, PyAny>>,
        swap_day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;
        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let forward_curve_id =
            CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        // Parse fixing dates
        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            fixing_dates_vec.push(py_to_date(&item).context("fixing_dates")?);
        }

        // Parse payment dates
        let mut payment_dates_vec = Vec::new();
        if let Some(dates) = payment_dates {
            for item in dates.iter() {
                payment_dates_vec.push(py_to_date(&item).context("payment_dates")?);
            }
        } else {
            payment_dates_vec = fixing_dates_vec.clone();
        }

        // Parse accrual fractions
        let mut accrual_fractions_vec = Vec::new();
        for item in accrual_fractions.iter() {
            accrual_fractions_vec.push(item.extract::<f64>().context("accrual_fractions")?);
        }

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        // Parse swap conventions
        let fixed_freq = if let Some(f) = swap_fixed_freq {
            f.extract::<PyRef<PyFrequency>>()?.inner
        } else {
            Tenor::semi_annual()
        };

        let float_freq = if let Some(f) = swap_float_freq {
            f.extract::<PyRef<PyFrequency>>()?.inner
        } else {
            Tenor::quarterly()
        };

        let swap_dc = if let Some(dc) = swap_day_count {
            let DayCountArg(d) = dc.extract()?;
            d
        } else {
            DayCount::Thirty360
        };

        let mut builder = CmsOption::builder();
        builder = builder.id(id);
        builder = builder.strike(rust_decimal::Decimal::try_from(strike).unwrap_or_default());
        builder = builder.cms_tenor(cms_tenor);
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.payment_dates(payment_dates_vec);
        builder = builder.accrual_fractions(accrual_fractions_vec);
        builder = builder.option_type(opt_type);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder.swap_fixed_freq(fixed_freq);
        builder = builder.swap_float_freq(float_freq);
        builder = builder.swap_day_count(swap_dc);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.discount_curve_id(discount_curve_id);
        builder = builder.forward_curve_id(forward_curve_id);
        builder = builder.vol_surface_id(vol_surface_id);
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build CmsOption: {e}"))
        })?;
        Ok(Self::new(option))
    }

    #[classmethod]
    #[pyo3(
        signature = (instrument_id, start_date, maturity, frequency, cms_tenor, strike, option_type, notional, discount_curve, forward_curve, vol_surface, *, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None, day_count=None),
        text_signature = "(cls, instrument_id, start_date, maturity, frequency, cms_tenor, strike, option_type, notional, discount_curve, forward_curve, vol_surface, *, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None, day_count=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CMS option from a schedule specification.
    ///
    /// Generates fixing and payment dates from start_date, maturity, and frequency
    /// using standard market conventions (Modified Following BDC, weekends-only calendar).
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     start_date: Start of the first accrual period.
    ///     maturity: End of the last accrual period.
    ///     frequency: Coupon/observation frequency.
    ///     cms_tenor: Tenor of the CMS swap in years (e.g., 10.0 for 10Y).
    ///     strike: Strike rate in decimal form (e.g., 0.035 for 3.5%).
    ///     option_type: Option type ("call" for cap, "put" for floor).
    ///     notional: Contract notional as Money.
    ///     discount_curve: Discount curve identifier.
    ///     forward_curve: Forward/projection curve identifier for CMS rate.
    ///     vol_surface: Volatility surface identifier.
    ///     swap_fixed_freq: Fixed leg frequency of underlying swap (default: semi-annual).
    ///     swap_float_freq: Floating leg frequency of underlying swap (default: quarterly).
    ///     swap_day_count: Day count for the underlying swap fixed leg (default: 30/360).
    ///     day_count: Day count for accrual fractions (default: Act/365F).
    ///
    /// Returns:
    ///     CmsOption: Configured CMS option instrument.
    fn from_schedule(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        frequency: Bound<'_, PyAny>,
        cms_tenor: f64,
        strike: f64,
        option_type: &str,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        vol_surface: Bound<'_, PyAny>,
        swap_fixed_freq: Option<Bound<'_, PyAny>>,
        swap_float_freq: Option<Bound<'_, PyAny>>,
        swap_day_count: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let start = py_to_date(&start_date).context("start_date")?;
        let end = py_to_date(&maturity).context("maturity")?;
        let freq: Tenor = frequency.extract::<PyRef<PyFrequency>>()?.inner;
        let notional_money = extract_money(&notional).context("notional")?;
        let discount_curve_id =
            CurveId::new(discount_curve.extract::<&str>().context("discount_curve")?);
        let forward_curve_id =
            CurveId::new(forward_curve.extract::<&str>().context("forward_curve")?);
        let vol_surface_id = CurveId::new(vol_surface.extract::<&str>().context("vol_surface")?);

        let opt_type = match normalize_label(option_type).as_str() {
            "call" => OptionType::Call,
            "put" => OptionType::Put,
            other => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Unknown option type: {other}"
                )))
            }
        };

        let fixed_freq = if let Some(f) = swap_fixed_freq {
            f.extract::<PyRef<PyFrequency>>()?.inner
        } else {
            Tenor::semi_annual()
        };

        let float_freq = if let Some(f) = swap_float_freq {
            f.extract::<PyRef<PyFrequency>>()?.inner
        } else {
            Tenor::quarterly()
        };

        let swap_dc = if let Some(dc) = swap_day_count {
            let DayCountArg(d) = dc.extract()?;
            d
        } else {
            DayCount::Thirty360
        };

        let option_dc = if let Some(dc) = day_count {
            let DayCountArg(d) = dc.extract()?;
            d
        } else {
            DayCount::Act365F
        };

        let option = CmsOption::from_schedule(
            id,
            start,
            end,
            freq,
            cms_tenor,
            rust_decimal::Decimal::try_from(strike).unwrap_or_default(),
            opt_type,
            notional_money,
            option_dc,
            fixed_freq,
            float_freq,
            swap_dc,
            discount_curve_id,
            forward_curve_id,
            vol_surface_id,
        )
        .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Instrument type classification.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::CmsOption)
    }

    /// Strike rate.
    #[getter]
    fn strike(&self) -> f64 {
        rust_decimal::prelude::ToPrimitive::to_f64(&self.inner.strike).unwrap_or_default()
    }

    /// CMS tenor.
    #[getter]
    fn cms_tenor(&self) -> f64 {
        self.inner.cms_tenor
    }

    /// Option type label.
    #[getter]
    fn option_type(&self) -> &'static str {
        match self.inner.option_type {
            OptionType::Call => "call",
            OptionType::Put => "put",
        }
    }

    /// Notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Fixing dates.
    #[getter]
    fn fixing_dates(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dates = PyList::empty(py);
        for d in &self.inner.fixing_dates {
            dates.append(date_to_py(py, *d)?)?;
        }
        Ok(dates.into())
    }

    fn __repr__(&self) -> String {
        format!(
            "CmsOption(id='{}', strike={}, cms_tenor={})",
            self.inner.id.as_str(),
            self.inner.strike,
            self.inner.cms_tenor
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    parent.add_class::<PyCmsOption>()?;
    Ok(vec!["CmsOption"])
}
