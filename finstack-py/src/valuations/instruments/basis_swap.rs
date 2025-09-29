use crate::core::common::args::{BusinessDayConventionArg, DayCountArg};
use crate::core::error::core_to_py;
use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::py_to_date;
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency};
use finstack_valuations::instruments::basis_swap::{BasisSwap, BasisSwapLeg};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

fn parse_frequency(label: Option<&str>) -> PyResult<Frequency> {
    match label
        .map(crate::core::common::labels::normalize_label)
        .as_deref()
    {
        None | Some("quarterly") => Ok(Frequency::quarterly()),
        Some("monthly") => Ok(Frequency::monthly()),
        Some("semi_annual") | Some("semiannual") => Ok(Frequency::semi_annual()),
        Some("annual") => Ok(Frequency::annual()),
        Some("bimonthly") => Ok(Frequency::bimonthly()),
        Some(other) => Err(PyValueError::new_err(format!(
            "Unsupported frequency label: {other}",
        ))),
    }
}

fn parse_stub(label: Option<&str>) -> PyResult<finstack_core::dates::StubKind> {
    match label
        .map(crate::core::common::labels::normalize_label)
        .as_deref()
    {
        None | Some("none") => Ok(finstack_core::dates::StubKind::None),
        Some("short_front") => Ok(finstack_core::dates::StubKind::ShortFront),
        Some("short_back") => Ok(finstack_core::dates::StubKind::ShortBack),
        Some("long_front") => Ok(finstack_core::dates::StubKind::LongFront),
        Some("long_back") => Ok(finstack_core::dates::StubKind::LongBack),
        Some(other) => Err(PyValueError::new_err(format!(
            "Unsupported stub kind: {other}",
        ))),
    }
}

/// Basis swap leg helper mirroring `BasisSwapLeg`.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "BasisSwapLeg",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyBasisSwapLeg {
    pub(crate) inner: BasisSwapLeg,
}

#[pymethods]
impl PyBasisSwapLeg {
    #[new]
    #[pyo3(
        signature = (
            forward_curve,
            *,
            frequency=None,
            day_count=None,
            business_day_convention=None,
            spread=0.0
        ),
        text_signature = "(forward_curve, /, *, frequency='quarterly', day_count='act_360', business_day_convention='modified_following', spread=0.0)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        forward_curve: Bound<'_, PyAny>,
        frequency: Option<&str>,
        day_count: Option<Bound<'_, PyAny>>,
        business_day_convention: Option<Bound<'_, PyAny>>,
        spread: Option<f64>,
    ) -> PyResult<Self> {
        let forward_id = extract_curve_id(&forward_curve)?;
        let freq = parse_frequency(frequency)?;
        let dc = if let Some(obj) = day_count {
            let DayCountArg(value) = obj.extract()?;
            value
        } else {
            DayCount::Act360
        };
        let bdc = if let Some(obj) = business_day_convention {
            let BusinessDayConventionArg(value) = obj.extract()?;
            value
        } else {
            BusinessDayConvention::ModifiedFollowing
        };

        Ok(Self {
            inner: BasisSwapLeg {
                forward_curve_id: forward_id,
                frequency: freq,
                day_count: dc,
                bdc,
                spread: spread.unwrap_or(0.0),
            },
        })
    }

    #[getter]
    fn forward_curve(&self) -> String {
        self.inner.forward_curve_id.as_str().to_string()
    }

    #[getter]
    fn spread(&self) -> f64 {
        self.inner.spread
    }
}

/// Basis swap wrapper with convenience constructor.
#[pyclass(module = "finstack.valuations.instruments", name = "BasisSwap", frozen)]
#[derive(Clone, Debug)]
pub struct PyBasisSwap {
    pub(crate) inner: BasisSwap,
}

impl PyBasisSwap {
    pub(crate) fn new(inner: BasisSwap) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBasisSwap {
    #[classmethod]
    #[pyo3(
        signature = (
            instrument_id,
            notional,
            start_date,
            maturity,
            primary_leg,
            reference_leg,
            discount_curve,
            *,
            calendar=None,
            stub="none"
        ),
        text_signature = "(cls, instrument_id, notional, start_date, maturity, primary_leg, reference_leg, discount_curve, /, *, calendar=None, stub='none')"
    )]
    #[allow(clippy::too_many_arguments)]
    fn create(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        start_date: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        primary_leg: &PyBasisSwapLeg,
        reference_leg: &PyBasisSwapLeg,
        discount_curve: Bound<'_, PyAny>,
        calendar: Option<&str>,
        stub: Option<&str>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let notional_money = extract_money(&notional)?;
        let start = py_to_date(&start_date)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc_id = extract_curve_id(&discount_curve)?;
        let stub_kind = parse_stub(stub)?;

        let mut builder = BasisSwap::builder();
        builder = builder.id(id);
        builder = builder.notional(notional_money);
        builder = builder.start_date(start);
        builder = builder.maturity_date(maturity_date);
        builder = builder.primary_leg(primary_leg.inner.clone());
        builder = builder.reference_leg(reference_leg.inner.clone());
        builder = builder.discount_curve_id(disc_id);
        builder = builder.stub_kind(stub_kind);
        let cal_static: Option<&'static str> = calendar.map(|s| {
            let leaked: &'static mut str = Box::leak(s.to_string().into_boxed_str());
            &*leaked
        });
        builder = builder.calendar_id_opt(cal_static);
        builder = builder.attributes(Default::default());

        let swap = builder.build().map_err(core_to_py)?;
        Ok(Self::new(swap))
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
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::BasisSwap)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("BasisSwap(id='{}')", self.inner.id))
    }
}

impl fmt::Display for PyBasisSwap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BasisSwap({}, notional={})",
            self.inner.id, self.inner.notional
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBasisSwapLeg>()?;
    module.add_class::<PyBasisSwap>()?;
    Ok(vec!["BasisSwapLeg", "BasisSwap"])
}
