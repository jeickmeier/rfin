use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id};
use finstack_valuations::instruments::cms_option::CmsOption;
use finstack_valuations::instruments::OptionType;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::Bound;

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
        text_signature = "(cls, instrument_id, strike_rate, cms_tenor, fixing_dates, accrual_fractions, option_type, notional, discount_curve, *, vol_surface=None)"
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
    ) -> PyResult<Self> {
        use crate::core::common::labels::normalize_label;
        use finstack_core::dates::DayCount;

        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let disc_id = extract_curve_id(&discount_curve)?;

        // Parse fixing dates
        let mut fixing_dates_vec = Vec::new();
        for item in fixing_dates.iter() {
            fixing_dates_vec.push(py_to_date(&item)?);
        }

        // Parse accrual fractions
        let mut accrual_fractions_vec = Vec::new();
        for item in accrual_fractions.iter() {
            accrual_fractions_vec.push(item.extract::<f64>()?);
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

        let vol_id = vol_surface.map(|v| extract_curve_id(&v).ok()).flatten();

        let mut builder = CmsOption::builder();
        builder = builder.id(id);
        builder = builder.strike_rate(strike_rate);
        builder = builder.cms_tenor(cms_tenor);
        builder = builder.fixing_dates(fixing_dates_vec);
        builder = builder.accrual_fractions(accrual_fractions_vec);
        builder = builder.option_type(opt_type);
        builder = builder.notional(notional_money);
        builder = builder.day_count(DayCount::Act365F);
        builder = builder
            .pricing_overrides(finstack_valuations::instruments::PricingOverrides::default());
        builder = builder.disc_id(disc_id);
        if let Some(vol) = vol_id {
            builder = builder.vol_id(vol);
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
    fn fixing_dates(&self, py: Python<'_>) -> PyResult<PyObject> {
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
