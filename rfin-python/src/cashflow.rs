use pyo3::prelude::*;

use rfin_core::cashflow::leg::CashFlowLeg;
use rfin_core::cashflow::notional::Notional;
use rfin_core::cashflow::npv::{DiscountCurve, Discountable};
use rfin_core::cashflow::primitives::{CFKind, CashFlow as CoreCashFlow};
use rfin_core::dates::ScheduleBuilder;
use std::sync::Arc;

use crate::currency::PyCurrency;
use crate::dates::PyDate;
use crate::daycount::PyDayCount;
use crate::schedule::PyFrequency;

/// Simple flat discount curve used for PV calculations.
struct FlatCurve {
    rate: f64,
}

impl DiscountCurve for FlatCurve {
    fn df(&self, _date: rfin_core::dates::Date) -> f64 {
        // Zero-rate flat curve: simply use exp(-r * t) with t = 0 (today) => 1.0 for now.
        // Future enhancement: compute based on year fraction to payment date.
        1.0 / (1.0 + self.rate)
    }
}

#[pyclass(name = "CashFlow", module = "rfin.cashflow")]
#[derive(Clone, Copy)]
pub struct PyCashFlow {
    inner: CoreCashFlow,
}

#[pymethods]
impl PyCashFlow {
    /// Payment date of the cash-flow.
    #[getter]
    fn date(&self) -> PyDate {
        PyDate::from_core(self.inner.date)
    }

    /// Amount of the cash-flow (signed).
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount.amount()
    }

    /// Currency of the cash-flow.
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.amount.currency())
    }

    /// Cash-flow kind ("Fixed", "Stub", …).
    #[getter]
    fn kind(&self) -> String {
        match self.inner.kind {
            CFKind::Fixed => "Fixed",
            CFKind::FloatReset => "FloatReset",
            CFKind::Notional => "Notional",
            CFKind::Fee => "Fee",
            CFKind::Stub => "Stub",
            _ => "Other",
        }
        .to_string()
    }

    /// Accrual factor used for the coupon.
    #[getter]
    fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!(
            "CashFlow(date={}, amount={:.4}, currency={}, kind={})",
            self.inner.date,
            self.inner.amount.amount(),
            self.inner.amount.currency(),
            self.kind()
        )
    }
}

#[pyclass(name = "FixedRateLeg", module = "rfin.cashflow")]
#[derive(Clone)]
pub struct PyFixedRateLeg {
    inner: Arc<CashFlowLeg>,
}

#[pymethods]
impl PyFixedRateLeg {
    #[new]
    #[pyo3(
        text_signature = "(notional_amount, currency, rate, start_date, end_date, frequency, day_count)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        notional_amount: f64,
        currency: &PyCurrency,
        rate: f64,
        start_date: &PyDate,
        end_date: &PyDate,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
    ) -> PyResult<Self> {
        let core_currency = currency.inner();

        let notional = Notional::par(notional_amount, core_currency);

        let freq = frequency.inner();
        let dc = day_count.inner();

        let sched = ScheduleBuilder::new(start_date.inner(), end_date.inner())
            .frequency(freq)
            .build_raw();

        let leg = CashFlowLeg::fixed_rate(notional, rate, sched, dc).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Failed to build leg: {:?}", e))
        })?;

        Ok(Self {
            inner: Arc::new(leg),
        })
    }

    /// Net present value using a flat discount factor of 1.0 (no discounting) for now.
    #[pyo3(text_signature = "(self)")]
    fn npv(&self) -> f64 {
        let curve = FlatCurve { rate: 0.0 };
        self.inner.npv(&curve).amount()
    }

    /// Accrued interest up to (but excluding) val_date.
    #[pyo3(text_signature = "(self, val_date)")]
    fn accrued(&self, val_date: &PyDate) -> f64 {
        self.inner.accrued(val_date.inner()).amount()
    }

    /// Number of cash-flows in the leg.
    #[getter]
    fn num_flows(&self) -> usize {
        self.inner.flows.len()
    }

    /// Return the cash-flows as a list of `CashFlow` objects.
    #[pyo3(text_signature = "(self)")]
    fn flows(&self) -> Vec<PyCashFlow> {
        self.inner
            .flows
            .iter()
            .copied()
            .map(|cf| PyCashFlow { inner: cf })
            .collect()
    }

    /// String representation
    fn __repr__(&self) -> String {
        format!("FixedRateLeg(n_flows={})", self.inner.flows.len())
    }
}
