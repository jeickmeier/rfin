use crate::core::common::args::DayCountArg;
use crate::core::common::labels::normalize_label;
use crate::core::dates::schedule::PyFrequency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use finstack_core::dates::{DayCount, Frequency};
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::cms_option::CmsOption;
use finstack_valuations::instruments::OptionType;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, PyRef};

/// CMS option instrument.
#[pyclass(module = "finstack.valuations.instruments", name = "CmsOption", frozen)]
#[derive(Clone, Debug)]
pub struct PyCmsOption {
    pub(crate) inner: CmsOption,
}

impl PyCmsOption {
    pub(crate) fn new(inner: CmsOption) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyCmsOption {
    #[classmethod]
    #[pyo3(
        signature = (instrument_id, strike_rate, cms_tenor, fixing_dates, accrual_fractions, option_type, notional, discount_curve, *, vol_surface=None, payment_dates=None, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None),
        text_signature = "(cls, instrument_id, strike_rate, cms_tenor, fixing_dates, accrual_fractions, option_type, notional, discount_curve, *, vol_surface=None, payment_dates=None, swap_fixed_freq=None, swap_float_freq=None, swap_day_count=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a CMS option.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier.
    ///     strike_rate: Strike rate in decimal form.
    ///     cms_tenor: Tenor of the CMS swap (e.g., 10.0 for 10Y).
    ///     fixing_dates: List of fixing dates.
    ///     accrual_fractions: List of accrual fractions for each period.
    ///     option_type: Option type (``"call"`` or ``"put"``).
    ///     notional: Contract notional as :class:`finstack.core.money.Money`.
    ///     discount_curve: Discount curve identifier.
    ///     vol_surface: Optional volatility surface identifier.
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
        strike_rate: f64,
        cms_tenor: f64,
        fixing_dates: Bound<'_, PyList>,
        accrual_fractions: Bound<'_, PyList>,
        option_type: &str,
        notional: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        vol_surface: Option<Bound<'_, PyAny>>,
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
            // Default to fixing dates if not provided
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
            Frequency::semi_annual()
        };

        let float_freq = if let Some(f) = swap_float_freq {
            f.extract::<PyRef<PyFrequency>>()?.inner
        } else {
            Frequency::quarterly()
        };

        let swap_dc = if let Some(dc) = swap_day_count {
            let DayCountArg(d) = dc.extract()?;
            d
        } else {
            DayCount::Thirty360
        };

        let vol_surface_id =
            vol_surface.and_then(|v| v.extract::<&str>().ok().map(|s| CurveId::new(s)));

        let mut builder = CmsOption::builder();
        builder = builder.id(id);
        builder = builder.strike_rate(strike_rate);
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
        if let Some(vol) = vol_surface_id {
            builder = builder.vol_surface_id(vol);
        }
        let option = builder.build().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Failed to build CmsOption: {e}"))
        })?;
        Ok(Self::new(option))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Strike rate.
    #[getter]
    fn strike_rate(&self) -> f64 {
        self.inner.strike_rate
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
            "CmsOption(id='{}', strike_rate={}, cms_tenor={})",
            self.inner.id.as_str(),
            self.inner.strike_rate,
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
