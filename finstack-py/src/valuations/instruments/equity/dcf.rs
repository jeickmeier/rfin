//! Rust source: `finstack/valuations/src/instruments/equity/dcf_equity/`
//! Abbreviated to `dcf` for Python ergonomics.

use crate::core::common::args::CurrencyArg;
use crate::core::currency::PyCurrency;
use crate::core::dates::utils::{date_to_py, py_to_date};
use crate::core::market_data::PyMarketContext;
use crate::core::money::PyMoney;
use crate::errors::{core_to_py, PyContext};
use crate::valuations::common::PyInstrumentType;
use finstack_core::types::{CurveId, InstrumentId};
use finstack_valuations::instruments::equity::dcf_equity::{
    DilutionSecurity, DiscountedCashFlow, EquityBridge, TerminalValueSpec, ValuationDiscounts,
};
use finstack_valuations::instruments::{Attributes, PricingOverrides};
use finstack_valuations::prelude::Instrument;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyList, PyModule, PyType};
use pyo3::{Bound, Py, PyRef, PyRefMut};
use std::sync::Arc;

fn parse_flows(list: &Bound<'_, PyList>) -> PyResult<Vec<(time::Date, f64)>> {
    let mut flows = Vec::with_capacity(list.len());
    for (idx, item) in list.iter().enumerate() {
        let (date_obj, amount) = item
            .extract::<(Bound<'_, PyAny>, f64)>()
            .context(&format!("flows[{idx}]"))?;
        let date = py_to_date(&date_obj).context(&format!("flows[{idx}].date"))?;
        flows.push((date, amount));
    }
    Ok(flows)
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TerminalValueSpec",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTerminalValueSpec {
    pub(crate) inner: TerminalValueSpec,
}

impl PyTerminalValueSpec {
    pub(crate) fn new(inner: TerminalValueSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTerminalValueSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, growth_rate)")]
    fn gordon_growth(_cls: &Bound<'_, PyType>, growth_rate: f64) -> Self {
        Self::new(TerminalValueSpec::GordonGrowth { growth_rate })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, terminal_metric, multiple)")]
    fn exit_multiple(_cls: &Bound<'_, PyType>, terminal_metric: f64, multiple: f64) -> Self {
        Self::new(TerminalValueSpec::ExitMultiple {
            terminal_metric,
            multiple,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, high_growth_rate, stable_growth_rate, half_life_years)")]
    fn h_model(
        _cls: &Bound<'_, PyType>,
        high_growth_rate: f64,
        stable_growth_rate: f64,
        half_life_years: f64,
    ) -> Self {
        Self::new(TerminalValueSpec::HModel {
            high_growth_rate,
            stable_growth_rate,
            half_life_years,
        })
    }

    #[getter]
    fn growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { growth_rate } => Some(*growth_rate),
            _ => None,
        }
    }

    #[getter]
    fn terminal_metric(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::ExitMultiple {
                terminal_metric, ..
            } => Some(*terminal_metric),
            _ => None,
        }
    }

    #[getter]
    fn multiple(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::ExitMultiple { multiple, .. } => Some(*multiple),
            _ => None,
        }
    }

    #[getter]
    fn high_growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                high_growth_rate, ..
            } => Some(*high_growth_rate),
            _ => None,
        }
    }

    #[getter]
    fn stable_growth_rate(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                stable_growth_rate, ..
            } => Some(*stable_growth_rate),
            _ => None,
        }
    }

    #[getter]
    fn half_life_years(&self) -> Option<f64> {
        match &self.inner {
            TerminalValueSpec::HModel {
                half_life_years, ..
            } => Some(*half_life_years),
            _ => None,
        }
    }

    #[getter]
    fn name(&self) -> &'static str {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { .. } => "gordon_growth",
            TerminalValueSpec::ExitMultiple { .. } => "exit_multiple",
            TerminalValueSpec::HModel { .. } => "h_model",
        }
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            TerminalValueSpec::GordonGrowth { growth_rate } => {
                format!("TerminalValueSpec.gordon_growth(growth_rate={growth_rate})")
            }
            TerminalValueSpec::ExitMultiple {
                terminal_metric,
                multiple,
            } => format!(
                "TerminalValueSpec.exit_multiple(terminal_metric={terminal_metric}, multiple={multiple})"
            ),
            TerminalValueSpec::HModel {
                high_growth_rate,
                stable_growth_rate,
                half_life_years,
            } => format!(
                "TerminalValueSpec.h_model(high_growth_rate={high_growth_rate}, stable_growth_rate={stable_growth_rate}, half_life_years={half_life_years})"
            ),
        }
    }

    fn __str__(&self) -> &'static str {
        self.name()
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "EquityBridge",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyEquityBridge {
    pub(crate) inner: EquityBridge,
}

impl PyEquityBridge {
    pub(crate) fn new(inner: EquityBridge) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyEquityBridge {
    #[new]
    #[pyo3(
        signature = (
            total_debt = 0.0,
            cash = 0.0,
            preferred_equity = 0.0,
            minority_interest = 0.0,
            non_operating_assets = 0.0,
            other_adjustments = None
        )
    )]
    fn py_new(
        total_debt: f64,
        cash: f64,
        preferred_equity: f64,
        minority_interest: f64,
        non_operating_assets: f64,
        other_adjustments: Option<Vec<(String, f64)>>,
    ) -> Self {
        Self::new(EquityBridge {
            total_debt,
            cash,
            preferred_equity,
            minority_interest,
            non_operating_assets,
            other_adjustments: other_adjustments.unwrap_or_default(),
        })
    }

    #[getter]
    fn total_debt(&self) -> f64 {
        self.inner.total_debt
    }

    #[getter]
    fn cash(&self) -> f64 {
        self.inner.cash
    }

    #[getter]
    fn preferred_equity(&self) -> f64 {
        self.inner.preferred_equity
    }

    #[getter]
    fn minority_interest(&self) -> f64 {
        self.inner.minority_interest
    }

    #[getter]
    fn non_operating_assets(&self) -> f64 {
        self.inner.non_operating_assets
    }

    #[getter]
    fn other_adjustments(&self) -> Vec<(String, f64)> {
        self.inner.other_adjustments.clone()
    }

    fn net_adjustment(&self) -> f64 {
        self.inner.net_adjustment()
    }

    fn __repr__(&self) -> String {
        format!(
            "EquityBridge(total_debt={}, cash={}, preferred_equity={}, minority_interest={}, non_operating_assets={})",
            self.inner.total_debt,
            self.inner.cash,
            self.inner.preferred_equity,
            self.inner.minority_interest,
            self.inner.non_operating_assets
        )
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ValuationDiscounts",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyValuationDiscounts {
    pub(crate) inner: ValuationDiscounts,
}

impl PyValuationDiscounts {
    pub(crate) fn new(inner: ValuationDiscounts) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValuationDiscounts {
    #[new]
    #[pyo3(signature = (dlom = None, dloc = None, other_discount = None))]
    fn py_new(dlom: Option<f64>, dloc: Option<f64>, other_discount: Option<f64>) -> Self {
        Self::new(ValuationDiscounts {
            dlom,
            dloc,
            other_discount,
        })
    }

    #[getter]
    fn dlom(&self) -> Option<f64> {
        self.inner.dlom
    }

    #[getter]
    fn dloc(&self) -> Option<f64> {
        self.inner.dloc
    }

    #[getter]
    fn other_discount(&self) -> Option<f64> {
        self.inner.other_discount
    }

    fn apply(&self, equity_value: f64) -> PyResult<f64> {
        self.inner.apply(equity_value).map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!(
            "ValuationDiscounts(dlom={:?}, dloc={:?}, other_discount={:?})",
            self.inner.dlom, self.inner.dloc, self.inner.other_discount
        )
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DilutionSecurity",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDilutionSecurity {
    pub(crate) inner: DilutionSecurity,
}

impl PyDilutionSecurity {
    pub(crate) fn new(inner: DilutionSecurity) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDilutionSecurity {
    #[new]
    fn py_new(name: String, quantity: f64, exercise_price: f64) -> Self {
        Self::new(DilutionSecurity {
            name,
            quantity,
            exercise_price,
        })
    }

    #[getter]
    fn name(&self) -> &str {
        &self.inner.name
    }

    #[getter]
    fn quantity(&self) -> f64 {
        self.inner.quantity
    }

    #[getter]
    fn exercise_price(&self) -> f64 {
        self.inner.exercise_price
    }

    fn __repr__(&self) -> String {
        format!(
            "DilutionSecurity(name='{}', quantity={}, exercise_price={})",
            self.inner.name, self.inner.quantity, self.inner.exercise_price
        )
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DiscountedCashFlow",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDiscountedCashFlow {
    pub(crate) inner: Arc<DiscountedCashFlow>,
}

impl PyDiscountedCashFlow {
    pub(crate) fn new(inner: DiscountedCashFlow) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DiscountedCashFlowBuilder",
    unsendable
)]
pub struct PyDiscountedCashFlowBuilder {
    instrument_id: InstrumentId,
    currency: Option<finstack_core::currency::Currency>,
    flows: Option<Vec<(time::Date, f64)>>,
    wacc: Option<f64>,
    terminal_value: Option<TerminalValueSpec>,
    net_debt: Option<f64>,
    valuation_date: Option<time::Date>,
    discount_curve_id: Option<CurveId>,
    mid_year_convention: bool,
    equity_bridge: Option<EquityBridge>,
    shares_outstanding: Option<f64>,
    dilution_securities: Vec<DilutionSecurity>,
    valuation_discounts: Option<ValuationDiscounts>,
}

impl PyDiscountedCashFlowBuilder {
    fn new_with_id(instrument_id: InstrumentId) -> Self {
        Self {
            instrument_id,
            currency: None,
            flows: None,
            wacc: None,
            terminal_value: None,
            net_debt: None,
            valuation_date: None,
            discount_curve_id: None,
            mid_year_convention: false,
            equity_bridge: None,
            shares_outstanding: None,
            dilution_securities: Vec::new(),
            valuation_discounts: None,
        }
    }
}

#[pymethods]
impl PyDiscountedCashFlowBuilder {
    #[new]
    #[pyo3(text_signature = "(instrument_id)")]
    fn py_new(instrument_id: &str) -> Self {
        Self::new_with_id(InstrumentId::new(instrument_id))
    }

    fn currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        currency: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let CurrencyArg(currency) = currency.extract().context("currency")?;
        slf.currency = Some(currency);
        Ok(slf)
    }

    fn flows<'py>(
        mut slf: PyRefMut<'py, Self>,
        flows: Bound<'py, PyList>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.flows = Some(parse_flows(&flows)?);
        Ok(slf)
    }

    fn wacc(mut slf: PyRefMut<'_, Self>, wacc: f64) -> PyRefMut<'_, Self> {
        slf.wacc = Some(wacc);
        slf
    }

    fn terminal_value<'py>(
        mut slf: PyRefMut<'py, Self>,
        terminal_value: PyRef<'py, PyTerminalValueSpec>,
    ) -> PyRefMut<'py, Self> {
        slf.terminal_value = Some(terminal_value.inner.clone());
        slf
    }

    fn net_debt(mut slf: PyRefMut<'_, Self>, net_debt: f64) -> PyRefMut<'_, Self> {
        slf.net_debt = Some(net_debt);
        slf
    }

    fn valuation_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        valuation_date: Bound<'py, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        slf.valuation_date = Some(py_to_date(&valuation_date).context("valuation_date")?);
        Ok(slf)
    }

    fn discount_curve(
        mut slf: PyRefMut<'_, Self>,
        discount_curve_id: String,
    ) -> PyRefMut<'_, Self> {
        slf.discount_curve_id = Some(CurveId::new(discount_curve_id.as_str()));
        slf
    }

    fn mid_year_convention(mut slf: PyRefMut<'_, Self>, enabled: bool) -> PyRefMut<'_, Self> {
        slf.mid_year_convention = enabled;
        slf
    }

    fn equity_bridge<'py>(
        mut slf: PyRefMut<'py, Self>,
        equity_bridge: PyRef<'py, PyEquityBridge>,
    ) -> PyRefMut<'py, Self> {
        slf.equity_bridge = Some(equity_bridge.inner.clone());
        slf
    }

    fn shares_outstanding(
        mut slf: PyRefMut<'_, Self>,
        shares_outstanding: f64,
    ) -> PyRefMut<'_, Self> {
        slf.shares_outstanding = Some(shares_outstanding);
        slf
    }

    fn dilution_securities<'py>(
        mut slf: PyRefMut<'py, Self>,
        dilution_securities: Vec<PyRef<'py, PyDilutionSecurity>>,
    ) -> PyRefMut<'py, Self> {
        slf.dilution_securities = dilution_securities
            .into_iter()
            .map(|security| security.inner.clone())
            .collect();
        slf
    }

    fn valuation_discounts<'py>(
        mut slf: PyRefMut<'py, Self>,
        valuation_discounts: PyRef<'py, PyValuationDiscounts>,
    ) -> PyRefMut<'py, Self> {
        slf.valuation_discounts = Some(valuation_discounts.inner.clone());
        slf
    }

    fn build(slf: PyRefMut<'_, Self>) -> PyResult<PyDiscountedCashFlow> {
        let mut builder = DiscountedCashFlow::builder()
            .id(slf.instrument_id.clone())
            .attributes(Attributes::default())
            .pricing_overrides(PricingOverrides::default());

        if let Some(currency) = slf.currency {
            builder = builder.currency(currency);
        }
        if let Some(flows) = slf.flows.clone() {
            builder = builder.flows(flows);
        }
        if let Some(wacc) = slf.wacc {
            builder = builder.wacc(wacc);
        }
        if let Some(terminal_value) = slf.terminal_value.clone() {
            builder = builder.terminal_value(terminal_value);
        }
        if let Some(net_debt) = slf.net_debt {
            builder = builder.net_debt(net_debt);
        }
        if let Some(valuation_date) = slf.valuation_date {
            builder = builder.valuation_date(valuation_date);
        }
        if let Some(discount_curve_id) = slf.discount_curve_id.clone() {
            builder = builder.discount_curve_id(discount_curve_id);
        }
        if slf.mid_year_convention {
            builder = builder.mid_year_convention(true);
        }
        if let Some(equity_bridge) = slf.equity_bridge.clone() {
            builder = builder.equity_bridge(equity_bridge);
        }
        if let Some(shares_outstanding) = slf.shares_outstanding {
            builder = builder.shares_outstanding(shares_outstanding);
        }
        if !slf.dilution_securities.is_empty() {
            builder = builder.dilution_securities(slf.dilution_securities.clone());
        }
        if let Some(valuation_discounts) = slf.valuation_discounts.clone() {
            builder = builder.valuation_discounts(valuation_discounts);
        }

        builder
            .build()
            .map(PyDiscountedCashFlow::new)
            .map_err(core_to_py)
    }

    fn __repr__(&self) -> String {
        format!("DiscountedCashFlowBuilder(id='{}')", self.instrument_id)
    }
}

#[pymethods]
impl PyDiscountedCashFlow {
    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id)")]
    fn builder(
        cls: &Bound<'_, PyType>,
        instrument_id: &str,
    ) -> PyResult<Py<PyDiscountedCashFlowBuilder>> {
        Py::new(
            cls.py(),
            PyDiscountedCashFlowBuilder::new_with_id(InstrumentId::new(instrument_id)),
        )
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn flows(&self, py: Python<'_>) -> PyResult<Vec<(Py<PyAny>, f64)>> {
        self.inner
            .flows
            .iter()
            .map(|(date, amount)| Ok((date_to_py(py, *date)?, *amount)))
            .collect()
    }

    #[getter]
    fn wacc(&self) -> f64 {
        self.inner.wacc
    }

    #[getter]
    fn terminal_value(&self) -> PyTerminalValueSpec {
        PyTerminalValueSpec::new(self.inner.terminal_value.clone())
    }

    #[getter]
    fn net_debt(&self) -> f64 {
        self.inner.net_debt
    }

    #[getter]
    fn valuation_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.valuation_date)
    }

    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    #[getter]
    fn mid_year_convention(&self) -> bool {
        self.inner.mid_year_convention
    }

    #[getter]
    fn equity_bridge(&self) -> Option<PyEquityBridge> {
        self.inner.equity_bridge.clone().map(PyEquityBridge::new)
    }

    #[getter]
    fn shares_outstanding(&self) -> Option<f64> {
        self.inner.shares_outstanding
    }

    #[getter]
    fn dilution_securities(&self) -> Vec<PyDilutionSecurity> {
        self.inner
            .dilution_securities
            .iter()
            .cloned()
            .map(PyDilutionSecurity::new)
            .collect()
    }

    #[getter]
    fn valuation_discounts(&self) -> Option<PyValuationDiscounts> {
        self.inner
            .valuation_discounts
            .clone()
            .map(PyValuationDiscounts::new)
    }

    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::DCF)
    }

    fn calculate_pv_explicit_flows(&self) -> f64 {
        self.inner.calculate_pv_explicit_flows()
    }

    fn calculate_terminal_value(&self) -> PyResult<f64> {
        self.inner.calculate_terminal_value().map_err(core_to_py)
    }

    fn discount_terminal_value(&self, terminal_value: f64) -> PyResult<f64> {
        self.inner
            .discount_terminal_value(terminal_value)
            .map_err(core_to_py)
    }

    fn effective_net_debt(&self) -> f64 {
        self.inner.effective_net_debt()
    }

    fn diluted_shares(&self, equity_value: f64) -> Option<f64> {
        self.inner.diluted_shares(equity_value)
    }

    fn equity_value_per_share(&self, equity_value: f64) -> Option<f64> {
        self.inner.equity_value_per_share(equity_value)
    }

    fn value(
        &self,
        py: Python<'_>,
        market: &PyMarketContext,
        as_of: Bound<'_, PyAny>,
    ) -> PyResult<PyMoney> {
        let as_of = py_to_date(&as_of)?;
        let value = py
            .detach(|| Instrument::value(self.inner.as_ref(), &market.inner, as_of))
            .map_err(core_to_py)?;
        Ok(PyMoney::new(value))
    }

    fn __repr__(&self) -> String {
        format!(
            "DiscountedCashFlow(id='{}', flows={}, wacc={}, terminal_value='{}')",
            self.inner.id,
            self.inner.flows.len(),
            self.inner.wacc,
            PyTerminalValueSpec::new(self.inner.terminal_value.clone()).name()
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyTerminalValueSpec>()?;
    module.add_class::<PyEquityBridge>()?;
    module.add_class::<PyValuationDiscounts>()?;
    module.add_class::<PyDilutionSecurity>()?;
    module.add_class::<PyDiscountedCashFlow>()?;
    module.add_class::<PyDiscountedCashFlowBuilder>()?;
    module.setattr(
        "__doc__",
        "Corporate DCF instruments mirroring finstack-valuations Rust types.",
    )?;
    Ok(vec![
        "TerminalValueSpec",
        "EquityBridge",
        "ValuationDiscounts",
        "DilutionSecurity",
        "DiscountedCashFlow",
        "DiscountedCashFlowBuilder",
    ])
}
