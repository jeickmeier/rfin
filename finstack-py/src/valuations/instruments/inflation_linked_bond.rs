use crate::core::common::args::DayCountArg;
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
use finstack_valuations::instruments::inflation_linked_bond::parameters::InflationLinkedBondParams;
use finstack_valuations::instruments::inflation_linked_bond::{
    DeflationProtection, IndexationMethod, InflationLinkedBond,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_indexation_method(label: Option<&str>) -> PyResult<IndexationMethod> {
    match label
        .map(crate::core::common::labels::normalize_label)
        .as_deref()
    {
        None | Some("tips") | Some("us") => Ok(IndexationMethod::TIPS),
        Some("canadian") => Ok(IndexationMethod::Canadian),
        Some("uk") => Ok(IndexationMethod::UK),
        Some("french") => Ok(IndexationMethod::French),
        Some("japanese") | Some("jgb") => Ok(IndexationMethod::Japanese),
        Some(other) => Err(PyValueError::new_err(format!(
            "Unknown indexation method: {other}",
        ))),
    }
}

fn parse_deflation_protection(label: Option<&str>) -> PyResult<DeflationProtection> {
    match label
        .map(crate::core::common::labels::normalize_label)
        .as_deref()
    {
        None | Some("maturity_only") | Some("maturity") => Ok(DeflationProtection::MaturityOnly),
        Some("none") => Ok(DeflationProtection::None),
        Some("all_payments") | Some("all") => Ok(DeflationProtection::AllPayments),
        Some(other) => Err(PyValueError::new_err(format!(
            "Unknown deflation protection: {other}",
        ))),
    }
}

fn leak_optional_str(value: Option<&str>) -> Option<&'static str> {
    value.map(|s| {
        let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
        leaked
    })
}

/// Inflation-linked bond binding with a convenience constructor.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "InflationLinkedBond",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyInflationLinkedBond {
    pub(crate) inner: InflationLinkedBond,
}

impl PyInflationLinkedBond {
    pub(crate) fn new(inner: InflationLinkedBond) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyInflationLinkedBond {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, real_coupon, issue, maturity, base_index, discount_curve, inflation_curve, /, *, indexation='tips', frequency='semi_annual', day_count='act_act', deflation_protection='maturity_only', calendar=None)",
        signature = (
            instrument_id,
            notional,
            real_coupon,
            issue,
            maturity,
            base_index,
            discount_curve,
            inflation_curve,
            *,
            indexation = None,
            frequency = None,
            day_count = None,
            deflation_protection = None,
            calendar = None,
        )
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        real_coupon: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        base_index: f64,
        discount_curve: Bound<'_, PyAny>,
        inflation_curve: Bound<'_, PyAny>,
        indexation: Option<&str>,
        frequency: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        deflation_protection: Option<&str>,
        calendar: Option<&str>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let inflation_id = extract_curve_id(&inflation_curve)?;
        let indexation_method = parse_indexation_method(indexation)?;
        let freq = match frequency
            .map(crate::core::common::labels::normalize_label)
            .as_deref()
        {
            None | Some("semi_annual") | Some("semiannual") => Frequency::semi_annual(),
            Some("annual") => Frequency::annual(),
            Some("quarterly") => Frequency::quarterly(),
            Some("monthly") => Frequency::monthly(),
            Some(other) => {
                return Err(PyValueError::new_err(format!(
                    "Unsupported frequency: {other}",
                )))
            }
        };
        let dc = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::ActAct
        };
        let deflation = parse_deflation_protection(deflation_protection)?;

        let params = InflationLinkedBondParams::new(
            notional_money,
            real_coupon,
            issue_date,
            maturity_date,
            base_index,
            freq,
            dc,
        );

        let mut builder = InflationLinkedBond::builder();
        builder = builder.id(id);
        builder = builder.notional(params.notional);
        builder = builder.real_coupon(params.real_coupon);
        builder = builder.freq(params.frequency);
        builder = builder.dc(params.day_count);
        builder = builder.issue(params.issue);
        builder = builder.maturity(params.maturity);
        builder = builder.base_index(params.base_index);
        builder = builder.base_date(params.issue);
        builder = builder.indexation_method(indexation_method);
        builder = builder.lag(indexation_method.standard_lag());
        builder = builder.deflation_protection(deflation);
        builder = builder.bdc(BusinessDayConvention::Following);
        builder = builder.stub(StubKind::None);
        builder = builder.calendar_id_opt(leak_optional_str(calendar));
        builder = builder.disc_id(disc_id);
        builder = builder.inflation_id(inflation_id);
        builder = builder.attributes(Default::default());

        let bond = builder.build().map_err(core_to_py)?;
        Ok(Self::new(bond))
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    #[getter]
    fn real_coupon(&self) -> f64 {
        self.inner.real_coupon
    }

    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    #[getter]
    fn inflation_curve(&self) -> String {
        self.inner.inflation_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::InflationLinkedBond)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "InflationLinkedBond(id='{}', coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        ))
    }
}

impl fmt::Display for PyInflationLinkedBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "InflationLinkedBond({}, coupon={:.4})",
            self.inner.id, self.inner.real_coupon
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyInflationLinkedBond>()?;
    Ok(vec!["InflationLinkedBond"])
}
