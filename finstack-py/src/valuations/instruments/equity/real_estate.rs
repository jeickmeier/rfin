//! Python bindings for RealEstateAsset instrument.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::money::{extract_money, PyMoney};
use crate::valuations::common::PyInstrumentType;
use finstack_core::dates::DayCount;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::real_estate::{
    RealEstateAsset, RealEstatePropertyType, RealEstateValuationMethod,
};
use finstack_valuations::instruments::Attributes;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyType};
use pyo3::{Bound, Py};
use std::fmt;
use std::sync::Arc;

/// Real estate asset valuation instrument.
///
/// Supports DCF (discounted cashflow with explicit NOI schedule) and
/// direct capitalization valuation methods.
///
/// Examples:
///     >>> # Direct cap valuation
///     >>> asset = RealEstateAsset.create_direct_cap(
///     ...     "OFFICE-NYC-123",
///     ...     currency="USD",
///     ...     valuation_date=Date(2024, 1, 1),
///     ...     stabilized_noi=5_000_000.0,
///     ...     cap_rate=0.06,
///     ...     discount_curve_id="USD-OIS"
///     ... )
///     >>>
///     >>> # DCF valuation
///     >>> noi_schedule = [
///     ...     (Date(2024, 12, 31), 4_500_000.0),
///     ...     (Date(2025, 12, 31), 4_800_000.0),
///     ...     (Date(2026, 12, 31), 5_000_000.0),
///     ... ]
///     >>> asset = RealEstateAsset.create_dcf(
///     ...     "OFFICE-NYC-123",
///     ...     currency="USD",
///     ...     valuation_date=Date(2024, 1, 1),
///     ...     noi_schedule=noi_schedule,
///     ...     discount_rate=0.08,
///     ...     terminal_cap_rate=0.065,
///     ...     discount_curve_id="USD-OIS"
///     ... )
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RealEstateAsset",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRealEstateAsset {
    pub(crate) inner: Arc<RealEstateAsset>,
}

impl PyRealEstateAsset {
    pub(crate) fn new(inner: RealEstateAsset) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    fn parse_schedule(list: &Bound<'_, PyList>, label: &str) -> PyResult<Vec<(time::Date, f64)>> {
        use crate::errors::PyContext;
        let mut schedule: Vec<(time::Date, f64)> = Vec::new();
        for item in list.iter() {
            let tuple = item
                .extract::<(Bound<'_, PyAny>, f64)>()
                .context(&format!("{label} item should be (date, amount) tuple"))?;
            let date = py_to_date(&tuple.0).context(&format!("{label} date"))?;
            schedule.push((date, tuple.1));
        }
        Ok(schedule)
    }

    fn parse_property_type(
        value: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Option<RealEstatePropertyType>> {
        if let Some(v) = value {
            if v.is_none() {
                return Ok(None);
            }
            let s = v.extract::<&str>()?.to_lowercase();
            let ty = match s.as_str() {
                "office" => RealEstatePropertyType::Office,
                "multifamily" | "multi_family" | "multi-family" | "residential" => {
                    RealEstatePropertyType::Multifamily
                }
                "retail" => RealEstatePropertyType::Retail,
                "industrial" => RealEstatePropertyType::Industrial,
                "hospitality" | "hotel" => RealEstatePropertyType::Hospitality,
                "mixeduse" | "mixed_use" | "mixed-use" => RealEstatePropertyType::MixedUse,
                "other" => RealEstatePropertyType::Other,
                other => {
                    return Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported property_type '{other}'"
                    )))
                }
            };
            Ok(Some(ty))
        } else {
            Ok(None)
        }
    }

    fn parse_money_list(
        list: Option<Bound<'_, PyList>>,
        label: &str,
    ) -> PyResult<Vec<finstack_core::money::Money>> {
        use crate::errors::PyContext;
        let Some(list) = list else {
            return Ok(Vec::new());
        };
        let mut out = Vec::with_capacity(list.len());
        for item in list.iter() {
            let m = extract_money(&item).context(label)?;
            out.push(m);
        }
        Ok(out)
    }
}

#[pymethods]
impl PyRealEstateAsset {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, currency, valuation_date, noi_schedule, discount_rate, discount_curve_id, terminal_cap_rate=None, terminal_growth_rate=None, capex_schedule=None, sale_date=None, sale_price=None, acquisition_cost=None, acquisition_costs=None, disposition_cost_pct=None, disposition_costs=None, purchase_price=None, property_type=None, day_count=None, appraisal_value=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            currency,
            valuation_date,
            noi_schedule,
            discount_rate,
            discount_curve_id,
            terminal_cap_rate = None,
            terminal_growth_rate = None,
            capex_schedule = None,
            sale_date = None,
            sale_price = None,
            acquisition_cost = None,
            acquisition_costs = None,
            disposition_cost_pct = None,
            disposition_costs = None,
            purchase_price = None,
            property_type = None,
            day_count = None,
            appraisal_value = None
        )
    )]
    /// Create a real estate asset with DCF valuation method.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     currency: Currency for valuation.
    ///     valuation_date: Base date for discounting.
    ///     noi_schedule: List of (date, noi_amount) tuples for cashflow schedule.
    ///     discount_rate: Discount rate for DCF (annualized).
    ///     discount_curve_id: Discount curve ID (for risk attribution).
    ///     terminal_cap_rate: Optional terminal cap rate (uses last NOI).
    ///     day_count: Day count convention (default Act365F).
    ///     appraisal_value: Optional appraisal override value (Money).
    ///
    /// Returns:
    ///     RealEstateAsset: Configured real estate asset with DCF valuation.
    #[allow(clippy::too_many_arguments)]
    fn create_dcf(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        valuation_date: Bound<'_, PyAny>,
        noi_schedule: Bound<'_, PyList>,
        discount_rate: f64,
        discount_curve_id: &str,
        terminal_cap_rate: Option<f64>,
        terminal_growth_rate: Option<f64>,
        capex_schedule: Option<Bound<'_, PyList>>,
        sale_date: Option<Bound<'_, PyAny>>,
        sale_price: Option<Bound<'_, PyAny>>,
        acquisition_cost: Option<f64>,
        acquisition_costs: Option<Bound<'_, PyList>>,
        disposition_cost_pct: Option<f64>,
        disposition_costs: Option<Bound<'_, PyList>>,
        purchase_price: Option<Bound<'_, PyAny>>,
        property_type: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
        appraisal_value: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let val_date = py_to_date(&valuation_date).context("valuation_date")?;

        // Parse NOI schedule
        let schedule = Self::parse_schedule(&noi_schedule, "noi_schedule")?;

        if schedule.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "noi_schedule must contain at least one entry",
            ));
        }

        // Parse optional CapEx schedule
        let capex = if let Some(list) = capex_schedule {
            let sched = Self::parse_schedule(&list, "capex_schedule")?;
            if sched.is_empty() {
                None
            } else {
                Some(sched)
            }
        } else {
            None
        };

        // Parse day count
        let dc = Self::parse_day_count(day_count)?;

        // Parse appraisal value
        let appraisal = if let Some(appraisal_arg) = appraisal_value {
            Some(extract_money(&appraisal_arg).context("appraisal_value")?)
        } else {
            None
        };

        // Parse optional purchase price
        let purchase_price = if let Some(px_arg) = purchase_price {
            Some(extract_money(&px_arg).context("purchase_price")?)
        } else {
            None
        };

        let property_type = Self::parse_property_type(property_type)?;

        let sale_date = match sale_date {
            Some(d) if !d.is_none() => Some(py_to_date(&d).context("sale_date")?),
            _ => None,
        };
        let sale_price = if let Some(px) = sale_price {
            if px.is_none() {
                None
            } else {
                Some(extract_money(&px).context("sale_price")?)
            }
        } else {
            None
        };
        let acquisition_costs = Self::parse_money_list(acquisition_costs, "acquisition_costs")?;
        let disposition_costs = Self::parse_money_list(disposition_costs, "disposition_costs")?;

        let mut builder = RealEstateAsset::builder()
            .id(id)
            .currency(ccy)
            .valuation_date(val_date)
            .valuation_method(RealEstateValuationMethod::Dcf)
            .noi_schedule(schedule)
            .discount_rate_opt(Some(discount_rate))
            .day_count(dc)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(term_cap) = terminal_cap_rate {
            builder = builder.terminal_cap_rate_opt(Some(term_cap));
        }
        if let Some(g) = terminal_growth_rate {
            builder = builder.terminal_growth_rate_opt(Some(g));
        }
        if let Some(capex) = capex {
            builder = builder.capex_schedule_opt(Some(capex));
        }
        if let Some(cost) = acquisition_cost {
            builder = builder.acquisition_cost_opt(Some(cost));
        }
        if let Some(d) = sale_date {
            builder = builder.sale_date_opt(Some(d));
        }
        if let Some(px) = sale_price {
            builder = builder.sale_price_opt(Some(px));
        }
        if !acquisition_costs.is_empty() {
            builder = builder.acquisition_costs(acquisition_costs);
        }
        if let Some(pct) = disposition_cost_pct {
            builder = builder.disposition_cost_pct_opt(Some(pct));
        }
        if !disposition_costs.is_empty() {
            builder = builder.disposition_costs(disposition_costs);
        }
        if let Some(px) = purchase_price {
            builder = builder.purchase_price_opt(Some(px));
        }
        if let Some(pt) = property_type {
            builder = builder.property_type_opt(Some(pt));
        }
        if let Some(appraisal) = appraisal {
            builder = builder.appraisal_value_opt(Some(appraisal));
        }

        let asset = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(asset))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, *, currency, valuation_date, stabilized_noi, cap_rate, discount_curve_id, noi_schedule=None, capex_schedule=None, acquisition_cost=None, disposition_cost_pct=None, purchase_price=None, property_type=None, day_count=None, appraisal_value=None)"
    )]
    #[pyo3(
        signature = (
            instrument_id,
            *,
            currency,
            valuation_date,
            stabilized_noi,
            cap_rate,
            discount_curve_id,
            noi_schedule = None,
            capex_schedule = None,
            acquisition_cost = None,
            disposition_cost_pct = None,
            purchase_price = None,
            property_type = None,
            day_count = None,
            appraisal_value = None
        )
    )]
    /// Create a real estate asset with direct capitalization valuation method.
    ///
    /// Args:
    ///     instrument_id: Unique identifier for this instrument.
    ///     currency: Currency for valuation.
    ///     valuation_date: Base date for valuation.
    ///     stabilized_noi: Stabilized NOI amount.
    ///     cap_rate: Capitalization rate (annualized).
    ///     discount_curve_id: Discount curve ID (for risk attribution).
    ///     noi_schedule: Optional NOI schedule (uses last value if not provided).
    ///     day_count: Day count convention (default Act365F).
    ///     appraisal_value: Optional appraisal override value (Money).
    ///
    /// Returns:
    ///     RealEstateAsset: Configured real estate asset with direct cap valuation.
    #[allow(clippy::too_many_arguments)]
    fn create_direct_cap(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        currency: Bound<'_, PyAny>,
        valuation_date: Bound<'_, PyAny>,
        stabilized_noi: f64,
        cap_rate: f64,
        discount_curve_id: &str,
        noi_schedule: Option<Bound<'_, PyList>>,
        capex_schedule: Option<Bound<'_, PyList>>,
        acquisition_cost: Option<f64>,
        disposition_cost_pct: Option<f64>,
        purchase_price: Option<Bound<'_, PyAny>>,
        property_type: Option<Bound<'_, PyAny>>,
        day_count: Option<Bound<'_, PyAny>>,
        appraisal_value: Option<Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        use crate::errors::PyContext;

        let id = InstrumentId::new(instrument_id.extract::<&str>().context("instrument_id")?);
        let CurrencyArg(ccy) = currency.extract().context("currency")?;
        let val_date = py_to_date(&valuation_date).context("valuation_date")?;

        // Parse optional NOI schedule
        let schedule = if let Some(noi_list) = noi_schedule {
            Self::parse_schedule(&noi_list, "noi_schedule")?
        } else {
            // Create a single-entry schedule with the stabilized NOI at valuation date
            vec![(val_date, stabilized_noi)]
        };

        // Parse optional CapEx schedule
        let capex = if let Some(list) = capex_schedule {
            let sched = Self::parse_schedule(&list, "capex_schedule")?;
            if sched.is_empty() {
                None
            } else {
                Some(sched)
            }
        } else {
            None
        };

        // Parse day count
        let dc = Self::parse_day_count(day_count)?;

        // Parse appraisal value
        let appraisal = if let Some(appraisal_arg) = appraisal_value {
            Some(extract_money(&appraisal_arg).context("appraisal_value")?)
        } else {
            None
        };

        // Parse optional purchase price
        let purchase_price = if let Some(px_arg) = purchase_price {
            Some(extract_money(&px_arg).context("purchase_price")?)
        } else {
            None
        };

        let property_type = Self::parse_property_type(property_type)?;

        let mut builder = RealEstateAsset::builder()
            .id(id)
            .currency(ccy)
            .valuation_date(val_date)
            .valuation_method(RealEstateValuationMethod::DirectCap)
            .noi_schedule(schedule)
            .cap_rate_opt(Some(cap_rate))
            .stabilized_noi_opt(Some(stabilized_noi))
            .day_count(dc)
            .discount_curve_id(CurveId::new(discount_curve_id))
            .attributes(Attributes::new());

        if let Some(capex) = capex {
            builder = builder.capex_schedule_opt(Some(capex));
        }
        if let Some(cost) = acquisition_cost {
            builder = builder.acquisition_cost_opt(Some(cost));
        }
        if let Some(pct) = disposition_cost_pct {
            builder = builder.disposition_cost_pct_opt(Some(pct));
        }
        if let Some(px) = purchase_price {
            builder = builder.purchase_price_opt(Some(px));
        }
        if let Some(pt) = property_type {
            builder = builder.property_type_opt(Some(pt));
        }
        if let Some(appraisal) = appraisal {
            builder = builder.appraisal_value_opt(Some(appraisal));
        }

        let asset = builder
            .build()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("{}", e)))?;

        Ok(Self::new(asset))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Currency.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    /// Valuation date.
    #[getter]
    fn valuation_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.valuation_date)
    }

    /// Valuation method (dcf or direct_cap).
    #[getter]
    fn valuation_method(&self) -> &str {
        match self.inner.valuation_method {
            RealEstateValuationMethod::Dcf => "dcf",
            RealEstateValuationMethod::DirectCap => "direct_cap",
        }
    }

    /// NOI schedule as list of (date, amount) tuples.
    #[getter]
    fn noi_schedule(&self, py: Python<'_>) -> PyResult<Py<PyList>> {
        let items: Vec<_> = self
            .inner
            .noi_schedule
            .iter()
            .map(|(date, amount)| {
                let py_date = date_to_py(py, *date)?;
                Ok((py_date, *amount))
            })
            .collect::<PyResult<Vec<_>>>()?;

        Ok(PyList::new(py, items)?.into())
    }

    /// Optional discount rate (for DCF).
    #[getter]
    fn discount_rate(&self) -> Option<f64> {
        self.inner.discount_rate
    }

    /// Optional capitalization rate (for direct cap).
    #[getter]
    fn cap_rate(&self) -> Option<f64> {
        self.inner.cap_rate
    }

    /// Optional stabilized NOI (for direct cap).
    #[getter]
    fn stabilized_noi(&self) -> Option<f64> {
        self.inner.stabilized_noi
    }

    /// Optional terminal capitalization rate (for DCF).
    #[getter]
    fn terminal_cap_rate(&self) -> Option<f64> {
        self.inner.terminal_cap_rate
    }

    /// Optional terminal growth rate (for DCF exit valuation).
    #[getter]
    fn terminal_growth_rate(&self) -> Option<f64> {
        self.inner.terminal_growth_rate
    }

    /// Optional acquisition cost (transaction cost).
    #[getter]
    fn acquisition_cost(&self) -> Option<f64> {
        self.inner.acquisition_cost
    }

    /// Optional disposition cost percentage.
    #[getter]
    fn disposition_cost_pct(&self) -> Option<f64> {
        self.inner.disposition_cost_pct
    }

    /// Optional purchase price.
    #[getter]
    fn purchase_price(&self) -> Option<PyMoney> {
        self.inner.purchase_price.map(PyMoney::new)
    }

    /// Optional property type classification.
    #[getter]
    fn property_type(&self) -> Option<&'static str> {
        self.inner.property_type.map(|pt| match pt {
            RealEstatePropertyType::Office => "office",
            RealEstatePropertyType::Multifamily => "multifamily",
            RealEstatePropertyType::Retail => "retail",
            RealEstatePropertyType::Industrial => "industrial",
            RealEstatePropertyType::Hospitality => "hospitality",
            RealEstatePropertyType::MixedUse => "mixed_use",
            RealEstatePropertyType::Other => "other",
        })
    }

    /// Optional appraisal value override.
    #[getter]
    fn appraisal_value(&self) -> Option<PyMoney> {
        self.inner.appraisal_value.map(PyMoney::new)
    }

    /// Day count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    /// Discount curve ID.
    #[getter]
    fn discount_curve_id(&self) -> &str {
        self.inner.discount_curve_id.as_str()
    }

    /// Instrument type key.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        use finstack_valuations::instruments::Instrument;
        PyInstrumentType::new(self.inner.key())
    }

    fn __repr__(&self) -> String {
        format!(
            "RealEstateAsset(id='{}', method='{}', currency='{}')",
            self.inner.id.as_str(),
            self.valuation_method(),
            self.inner.currency
        )
    }
}

impl PyRealEstateAsset {
    fn parse_day_count(day_count: Option<Bound<'_, PyAny>>) -> PyResult<DayCount> {
        if let Some(dc_arg) = day_count {
            if let Ok(py_dc) = dc_arg.extract::<pyo3::PyRef<PyDayCount>>() {
                Ok(py_dc.inner)
            } else if let Ok(name) = dc_arg.extract::<&str>() {
                match name.to_lowercase().as_str() {
                    "act_360" | "act/360" => Ok(DayCount::Act360),
                    "act_365f" | "act/365f" | "act365f" => Ok(DayCount::Act365F),
                    "act_act" | "act/act" | "actact" => Ok(DayCount::ActAct),
                    "thirty_360" | "30/360" | "30e/360" => Ok(DayCount::Thirty360),
                    other => Err(pyo3::exceptions::PyValueError::new_err(format!(
                        "Unsupported day count '{}'",
                        other
                    ))),
                }
            } else {
                Err(pyo3::exceptions::PyTypeError::new_err(
                    "day_count expects DayCount or str",
                ))
            }
        } else {
            Ok(DayCount::Act365F)
        }
    }
}

impl fmt::Display for PyRealEstateAsset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RealEstateAsset({}, {})",
            self.inner.id.as_str(),
            self.valuation_method()
        )
    }
}

/// Export module items for registration.
pub fn register_module(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_class::<PyRealEstateAsset>()?;
    Ok(())
}
