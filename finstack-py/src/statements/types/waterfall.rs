//! Waterfall type bindings.

use finstack_statements::capital_structure::{
    EcfSweepSpec, PaymentPriority, PikToggleSpec, WaterfallSpec,
};
use pyo3::prelude::*;
use pyo3::Bound;

/// Payment priority levels in the waterfall.
#[pyclass(
    module = "finstack.statements.types",
    name = "PaymentPriority",
    eq,
    eq_int
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PyPaymentPriority {
    Fees,
    Interest,
    Amortization,
    MandatoryPrepayment,
    VoluntaryPrepayment,
    Sweep,
    Equity,
}

impl From<PyPaymentPriority> for PaymentPriority {
    fn from(p: PyPaymentPriority) -> Self {
        match p {
            PyPaymentPriority::Fees => PaymentPriority::Fees,
            PyPaymentPriority::Interest => PaymentPriority::Interest,
            PyPaymentPriority::Amortization => PaymentPriority::Amortization,
            PyPaymentPriority::MandatoryPrepayment => PaymentPriority::MandatoryPrepayment,
            PyPaymentPriority::VoluntaryPrepayment => PaymentPriority::VoluntaryPrepayment,
            PyPaymentPriority::Sweep => PaymentPriority::Sweep,
            PyPaymentPriority::Equity => PaymentPriority::Equity,
        }
    }
}

impl From<PaymentPriority> for PyPaymentPriority {
    fn from(p: PaymentPriority) -> Self {
        match p {
            PaymentPriority::Fees => PyPaymentPriority::Fees,
            PaymentPriority::Interest => PyPaymentPriority::Interest,
            PaymentPriority::Amortization => PyPaymentPriority::Amortization,
            PaymentPriority::MandatoryPrepayment => PyPaymentPriority::MandatoryPrepayment,
            PaymentPriority::VoluntaryPrepayment => PyPaymentPriority::VoluntaryPrepayment,
            PaymentPriority::Sweep => PyPaymentPriority::Sweep,
            PaymentPriority::Equity => PyPaymentPriority::Equity,
        }
    }
}

/// Excess Cash Flow (ECF) sweep specification.
#[pyclass(module = "finstack.statements.types", name = "EcfSweepSpec")]
#[derive(Clone, Debug)]
pub struct PyEcfSweepSpec {
    pub(crate) inner: EcfSweepSpec,
}

#[pymethods]
impl PyEcfSweepSpec {
    #[new]
    #[pyo3(
        signature = (ebitda_node, sweep_percentage, *, taxes_node=None, capex_node=None, working_capital_node=None, cash_interest_node=None, target_instrument_id=None),
        text_signature = "(ebitda_node, sweep_percentage, *, taxes_node=None, capex_node=None, working_capital_node=None, cash_interest_node=None, target_instrument_id=None)"
    )]
    fn new(
        ebitda_node: String,
        sweep_percentage: f64,
        taxes_node: Option<String>,
        capex_node: Option<String>,
        working_capital_node: Option<String>,
        cash_interest_node: Option<String>,
        target_instrument_id: Option<String>,
    ) -> Self {
        Self {
            inner: EcfSweepSpec {
                ebitda_node,
                taxes_node,
                capex_node,
                working_capital_node,
                cash_interest_node,
                sweep_percentage,
                target_instrument_id,
            },
        }
    }

    #[getter]
    fn ebitda_node(&self) -> String {
        self.inner.ebitda_node.clone()
    }

    #[getter]
    fn sweep_percentage(&self) -> f64 {
        self.inner.sweep_percentage
    }

    #[getter]
    fn taxes_node(&self) -> Option<String> {
        self.inner.taxes_node.clone()
    }

    #[getter]
    fn capex_node(&self) -> Option<String> {
        self.inner.capex_node.clone()
    }

    #[getter]
    fn working_capital_node(&self) -> Option<String> {
        self.inner.working_capital_node.clone()
    }

    #[getter]
    fn cash_interest_node(&self) -> Option<String> {
        self.inner.cash_interest_node.clone()
    }

    #[getter]
    fn target_instrument_id(&self) -> Option<String> {
        self.inner.target_instrument_id.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "EcfSweepSpec(ebitda='{}', sweep={:.1}%)",
            self.inner.ebitda_node,
            self.inner.sweep_percentage * 100.0
        )
    }
}

/// PIK toggle specification.
#[pyclass(module = "finstack.statements.types", name = "PikToggleSpec")]
#[derive(Clone, Debug)]
pub struct PyPikToggleSpec {
    pub(crate) inner: PikToggleSpec,
}

#[pymethods]
impl PyPikToggleSpec {
    #[new]
    #[pyo3(
        signature = (liquidity_metric, threshold, *, target_instrument_ids=None, min_periods_in_pik=0),
        text_signature = "(liquidity_metric, threshold, *, target_instrument_ids=None, min_periods_in_pik=0)"
    )]
    fn new(
        liquidity_metric: String,
        threshold: f64,
        target_instrument_ids: Option<Vec<String>>,
        min_periods_in_pik: usize,
    ) -> Self {
        Self {
            inner: PikToggleSpec {
                liquidity_metric,
                threshold,
                target_instrument_ids,
                min_periods_in_pik,
            },
        }
    }

    #[getter]
    fn liquidity_metric(&self) -> String {
        self.inner.liquidity_metric.clone()
    }

    #[getter]
    fn threshold(&self) -> f64 {
        self.inner.threshold
    }

    #[getter]
    fn target_instrument_ids(&self) -> Option<Vec<String>> {
        self.inner.target_instrument_ids.clone()
    }

    #[getter]
    fn min_periods_in_pik(&self) -> usize {
        self.inner.min_periods_in_pik
    }

    fn __repr__(&self) -> String {
        format!(
            "PikToggleSpec(metric='{}', threshold={})",
            self.inner.liquidity_metric, self.inner.threshold
        )
    }
}

/// Waterfall specification.
#[pyclass(module = "finstack.statements.types", name = "WaterfallSpec")]
#[derive(Clone, Debug)]
pub struct PyWaterfallSpec {
    pub(crate) inner: WaterfallSpec,
}

#[pymethods]
impl PyWaterfallSpec {
    #[new]
    #[pyo3(
        signature = (*, priority_of_payments=None, ecf_sweep=None, pik_toggle=None),
        text_signature = "(*, priority_of_payments=None, ecf_sweep=None, pik_toggle=None)"
    )]
    fn new(
        priority_of_payments: Option<Vec<PyPaymentPriority>>,
        ecf_sweep: Option<PyEcfSweepSpec>,
        pik_toggle: Option<PyPikToggleSpec>,
    ) -> Self {
        let priority = priority_of_payments
            .map(|v| v.into_iter().map(PaymentPriority::from).collect())
            .unwrap_or_else(default_priority);

        Self {
            inner: WaterfallSpec {
                priority_of_payments: priority,
                ecf_sweep: ecf_sweep.map(|s| s.inner),
                pik_toggle: pik_toggle.map(|s| s.inner),
            },
        }
    }

    #[getter]
    fn priority_of_payments(&self) -> Vec<PyPaymentPriority> {
        self.inner
            .priority_of_payments
            .iter()
            .map(|&p| PyPaymentPriority::from(p))
            .collect()
    }

    #[getter]
    fn ecf_sweep(&self) -> Option<PyEcfSweepSpec> {
        self.inner
            .ecf_sweep
            .as_ref()
            .map(|s| PyEcfSweepSpec { inner: s.clone() })
    }

    #[getter]
    fn pik_toggle(&self) -> Option<PyPikToggleSpec> {
        self.inner
            .pik_toggle
            .as_ref()
            .map(|s| PyPikToggleSpec { inner: s.clone() })
    }

    fn __repr__(&self) -> String {
        format!(
            "WaterfallSpec(priority={}, ecf={}, pik={})",
            self.inner.priority_of_payments.len(),
            self.inner.ecf_sweep.is_some(),
            self.inner.pik_toggle.is_some()
        )
    }
}

fn default_priority() -> Vec<PaymentPriority> {
    vec![
        PaymentPriority::Fees,
        PaymentPriority::Interest,
        PaymentPriority::Amortization,
        PaymentPriority::Sweep,
        PaymentPriority::Equity,
    ]
}

pub(crate) fn register<'py>(_py: Python<'py>, module: &Bound<'py, PyModule>) -> PyResult<()> {
    module.add_class::<PyPaymentPriority>()?;
    module.add_class::<PyEcfSweepSpec>()?;
    module.add_class::<PyPikToggleSpec>()?;
    module.add_class::<PyWaterfallSpec>()?;
    Ok(())
}
