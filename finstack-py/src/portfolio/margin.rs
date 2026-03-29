//! Python bindings for portfolio margin aggregation.

use crate::core::currency::{extract_currency, PyCurrency};
use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::core::money::PyMoney;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::extract_portfolio;
use crate::portfolio::types::extract_position;
use finstack_margin::{ImMethodology, NettingSetId, SimmSensitivities};
use finstack_portfolio::margin::{
    NettingSet, NettingSetManager, NettingSetMargin, PortfolioMarginAggregator,
    PortfolioMarginResult,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyModule};
use pyo3::Bound;
use pythonize::depythonize;

/// Identifier for a margin netting set.
#[pyclass(module = "finstack.portfolio", name = "NettingSetId", from_py_object)]
#[derive(Clone)]
pub struct PyNettingSetId {
    pub(crate) inner: NettingSetId,
}

impl PyNettingSetId {
    pub(crate) fn new(inner: NettingSetId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNettingSetId {
    #[staticmethod]
    fn bilateral(counterparty_id: String, csa_id: String) -> Self {
        Self::new(NettingSetId::bilateral(counterparty_id, csa_id))
    }

    #[staticmethod]
    fn cleared(ccp_id: String) -> Self {
        Self::new(NettingSetId::cleared(ccp_id))
    }

    #[getter]
    fn counterparty_id(&self) -> String {
        self.inner.counterparty_id.clone()
    }

    #[getter]
    fn csa_id(&self) -> Option<String> {
        self.inner.csa_id.clone()
    }

    #[getter]
    fn ccp_id(&self) -> Option<String> {
        self.inner.ccp_id.clone()
    }

    fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    fn __repr__(&self) -> String {
        format!("NettingSetId({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// Margin results for a single netting set.
#[pyclass(
    module = "finstack.portfolio",
    name = "NettingSetMargin",
    from_py_object
)]
#[derive(Clone)]
pub struct PyNettingSetMargin {
    pub(crate) inner: NettingSetMargin,
}

impl PyNettingSetMargin {
    pub(crate) fn new(inner: NettingSetMargin) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNettingSetMargin {
    #[new]
    #[pyo3(signature = (netting_set_id, as_of, initial_margin, variation_margin, position_count, im_methodology))]
    fn new_py(
        netting_set_id: &PyNettingSetId,
        as_of: &Bound<'_, PyAny>,
        initial_margin: &Bound<'_, PyAny>,
        variation_margin: &Bound<'_, PyAny>,
        position_count: usize,
        im_methodology: &str,
    ) -> PyResult<Self> {
        let as_of_date = crate::core::dates::utils::py_to_date(as_of)?;
        let im = initial_margin.extract::<PyRef<PyMoney>>()?.inner;
        let vm = variation_margin.extract::<PyRef<PyMoney>>()?.inner;
        let methodology: ImMethodology = im_methodology.parse().map_err(|e: String| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid IM methodology: {}", e))
        })?;
        Ok(Self::new(NettingSetMargin::new(
            netting_set_id.inner.clone(),
            as_of_date,
            im,
            vm,
            position_count,
            methodology,
        )))
    }

    #[getter]
    fn netting_set_id(&self) -> String {
        self.inner.netting_set_id.to_string()
    }

    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::core::dates::utils::date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn initial_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.initial_margin)
    }

    #[getter]
    fn variation_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.variation_margin)
    }

    #[getter]
    fn total_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_margin)
    }

    #[getter]
    fn position_count(&self) -> usize {
        self.inner.position_count
    }

    #[getter]
    fn im_methodology(&self) -> String {
        self.inner.im_methodology.to_string()
    }

    #[getter]
    fn sensitivities(&self, py: Python<'_>) -> PyResult<Option<Py<PyAny>>> {
        if let Some(sens) = &self.inner.sensitivities {
            let obj = pythonize::pythonize(py, sens)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
            Ok(Some(obj.unbind()))
        } else {
            Ok(None)
        }
    }

    #[getter]
    fn im_breakdown(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (bucket, money) in &self.inner.im_breakdown {
            dict.set_item(bucket, PyMoney::new(*money))?;
        }
        Ok(dict.into())
    }

    fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    fn __repr__(&self) -> String {
        format!(
            "NettingSetMargin(id={}, im={}, vm={}, positions={})",
            self.inner.netting_set_id,
            self.inner.initial_margin,
            self.inner.variation_margin,
            self.inner.position_count
        )
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("JSON serialization failed: {}", e)))
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: NettingSetMargin = serde_json::from_str(json_str)
            .map_err(|e| PyValueError::new_err(format!("JSON deserialization failed: {}", e)))?;
        Ok(Self::new(inner))
    }
}

/// Portfolio-wide margin calculation results.
#[pyclass(
    module = "finstack.portfolio",
    name = "PortfolioMarginResult",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPortfolioMarginResult {
    pub(crate) inner: PortfolioMarginResult,
}

impl PyPortfolioMarginResult {
    pub(crate) fn new(inner: PortfolioMarginResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPortfolioMarginResult {
    #[getter]
    fn as_of(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        crate::core::dates::utils::date_to_py(py, self.inner.as_of)
    }

    #[getter]
    fn base_currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.base_currency)
    }

    #[getter]
    fn total_initial_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_initial_margin)
    }

    #[getter]
    fn total_variation_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_variation_margin)
    }

    #[getter]
    fn total_margin(&self) -> PyMoney {
        PyMoney::new(self.inner.total_margin)
    }

    #[getter]
    fn by_netting_set(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let dict = PyDict::new(py);
        for (id, result) in &self.inner.by_netting_set {
            dict.set_item(id.to_string(), PyNettingSetMargin::new(result.clone()))?;
        }
        Ok(dict.into())
    }

    #[getter]
    fn total_positions(&self) -> usize {
        self.inner.total_positions
    }

    #[getter]
    fn positions_without_margin(&self) -> usize {
        self.inner.positions_without_margin
    }

    fn add_netting_set(&mut self, result: &PyNettingSetMargin) -> PyResult<()> {
        self.inner
            .add_netting_set(result.inner.clone())
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
    }

    fn add_netting_set_with_fx(&mut self, result: &PyNettingSetMargin, fx_rate: f64) {
        self.inner
            .add_netting_set_with_fx(result.inner.clone(), fx_rate);
    }

    fn cleared_bilateral_split(&self) -> (PyMoney, PyMoney) {
        let (cleared, bilateral) = self.inner.cleared_bilateral_split();
        (PyMoney::new(cleared), PyMoney::new(bilateral))
    }

    fn netting_set_count(&self) -> usize {
        self.inner.netting_set_count()
    }

    fn __len__(&self) -> usize {
        self.inner.netting_set_count()
    }

    fn __iter__(&self) -> PyPortfolioMarginResultIterator {
        let items: Vec<(NettingSetId, NettingSetMargin)> = self
            .inner
            .iter()
            .map(|(id, m)| (id.clone(), m.clone()))
            .collect();
        PyPortfolioMarginResultIterator { items, index: 0 }
    }

    fn __repr__(&self) -> String {
        format!(
            "PortfolioMarginResult(im={}, vm={}, netting_sets={})",
            self.inner.total_initial_margin,
            self.inner.total_variation_margin,
            self.inner.by_netting_set.len()
        )
    }

    /// Serialize to JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("JSON serialization failed: {}", e)))
    }

    /// Deserialize from JSON string.
    #[staticmethod]
    fn from_json(json_str: &str) -> PyResult<Self> {
        let inner: PortfolioMarginResult = serde_json::from_str(json_str)
            .map_err(|e| PyValueError::new_err(format!("JSON deserialization failed: {}", e)))?;
        Ok(Self::new(inner))
    }
}

/// Manager for organizing positions into netting sets.
#[pyclass(module = "finstack.portfolio", name = "NettingSetManager")]
pub struct PyNettingSetManager {
    pub(crate) inner: NettingSetManager,
}

impl PyNettingSetManager {
    pub(crate) fn new(inner: NettingSetManager) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNettingSetManager {
    #[new]
    fn new_py() -> Self {
        Self::new(NettingSetManager::new())
    }

    #[pyo3(text_signature = "($self, id)")]
    fn with_default_set<'py>(
        mut slf: PyRefMut<'py, Self>,
        id: &PyNettingSetId,
    ) -> PyRefMut<'py, Self> {
        let tmp = std::mem::replace(&mut slf.inner, NettingSetManager::new());
        slf.inner = tmp.with_default_set(id.inner.clone());
        slf
    }

    fn add_position(
        &mut self,
        position: &Bound<'_, PyAny>,
        netting_set_id: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        let pos = extract_position(position)?;
        let ns_id = if let Some(ns) = netting_set_id {
            Some(ns.extract::<PyRef<PyNettingSetId>>()?.inner.clone())
        } else {
            None
        };
        self.inner.add_position(&pos, ns_id);
        Ok(())
    }

    fn count(&self) -> usize {
        self.inner.count()
    }

    fn ids(&self) -> Vec<PyNettingSetId> {
        self.inner.ids().cloned().map(PyNettingSetId::new).collect()
    }

    fn get(&self, id: &PyNettingSetId) -> Option<PyNettingSet> {
        self.inner.get(&id.inner).cloned().map(PyNettingSet::new)
    }

    fn __len__(&self) -> usize {
        self.inner.count()
    }

    fn __iter__(&self) -> PyNettingSetManagerIterator {
        let items: Vec<(NettingSetId, NettingSet)> = self
            .inner
            .iter()
            .map(|(id, ns)| (id.clone(), ns.clone()))
            .collect();
        PyNettingSetManagerIterator { items, index: 0 }
    }

    fn get_or_create(&mut self, id: &PyNettingSetId) -> PyNettingSet {
        let ns = self.inner.get_or_create(id.inner.clone());
        PyNettingSet::new(ns.clone())
    }

    fn merge_sensitivities(
        &mut self,
        netting_set_id: &PyNettingSetId,
        sensitivities: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let sens: SimmSensitivities = depythonize(sensitivities).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid sensitivities: {}", e))
        })?;
        self.inner.merge_sensitivities(&netting_set_id.inner, &sens);
        Ok(())
    }
}

/// A netting set containing positions for margin aggregation.
#[pyclass(module = "finstack.portfolio", name = "NettingSet", from_py_object)]
#[derive(Clone)]
pub struct PyNettingSet {
    pub(crate) inner: NettingSet,
}

impl PyNettingSet {
    pub(crate) fn new(inner: NettingSet) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNettingSet {
    #[new]
    fn new_py(id: &PyNettingSetId) -> Self {
        Self::new(NettingSet::new(id.inner.clone()))
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    fn add_position(&mut self, position_id: String) {
        self.inner.add_position(position_id.into());
    }

    fn position_count(&self) -> usize {
        self.inner.position_count()
    }

    fn is_cleared(&self) -> bool {
        self.inner.is_cleared()
    }

    fn merge_sensitivities(&mut self, sensitivities: &Bound<'_, PyAny>) -> PyResult<()> {
        let sens: SimmSensitivities = depythonize(sensitivities).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("Invalid sensitivities: {}", e))
        })?;
        self.inner.merge_sensitivities(&sens);
        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "NettingSet(id={}, positions={})",
            self.inner.id,
            self.inner.positions.len()
        )
    }
}

/// Iterator over (NettingSetId, NettingSet) pairs from NettingSetManager.
#[pyclass(module = "finstack.portfolio", name = "NettingSetManagerIterator")]
pub struct PyNettingSetManagerIterator {
    items: Vec<(NettingSetId, NettingSet)>,
    index: usize,
}

#[pymethods]
impl PyNettingSetManagerIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(PyNettingSetId, PyNettingSet)> {
        if slf.index < slf.items.len() {
            let (id, ns) = slf.items[slf.index].clone();
            slf.index += 1;
            Some((PyNettingSetId::new(id), PyNettingSet::new(ns)))
        } else {
            None
        }
    }
}

/// Iterator over (netting_set_id_str, NettingSetMargin) pairs from PortfolioMarginResult.
#[pyclass(module = "finstack.portfolio", name = "PortfolioMarginResultIterator")]
pub struct PyPortfolioMarginResultIterator {
    items: Vec<(NettingSetId, NettingSetMargin)>,
    index: usize,
}

#[pymethods]
impl PyPortfolioMarginResultIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<(String, PyNettingSetMargin)> {
        if slf.index < slf.items.len() {
            let (id, margin) = slf.items[slf.index].clone();
            slf.index += 1;
            Some((id.to_string(), PyNettingSetMargin::new(margin)))
        } else {
            None
        }
    }
}

/// Aggregates margin requirements across a portfolio.
#[pyclass(module = "finstack.portfolio", name = "PortfolioMarginAggregator")]
pub struct PyPortfolioMarginAggregator {
    inner: PortfolioMarginAggregator,
}

#[pymethods]
impl PyPortfolioMarginAggregator {
    #[new]
    fn new_py(base_ccy: &Bound<'_, PyAny>) -> PyResult<Self> {
        let currency = extract_currency(base_ccy)?;
        Ok(Self {
            inner: PortfolioMarginAggregator::new(currency),
        })
    }

    #[staticmethod]
    fn from_portfolio(portfolio: &Bound<'_, PyAny>) -> PyResult<Self> {
        let portfolio_inner = extract_portfolio(portfolio)?;
        Ok(Self {
            inner: PortfolioMarginAggregator::from_portfolio(&portfolio_inner),
        })
    }

    fn add_position(&mut self, position: &Bound<'_, PyAny>) -> PyResult<()> {
        let pos = extract_position(position)?;
        self.inner.add_position(&pos);
        Ok(())
    }

    fn netting_set_count(&self) -> usize {
        self.inner.netting_set_count()
    }

    fn calculate(
        &mut self,
        portfolio: &Bound<'_, PyAny>,
        market_context: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyPortfolioMarginResult> {
        let portfolio_inner = extract_portfolio(portfolio)?;
        let market = market_context.extract::<PyRef<PyMarketContext>>()?;
        let as_of_date = py_to_date(as_of)?;
        let result = self
            .inner
            .calculate(&portfolio_inner, &market.inner, as_of_date)
            .map_err(portfolio_to_py)?;
        Ok(PyPortfolioMarginResult::new(result))
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyNettingSetId>()?;
    parent.add_class::<PyNettingSet>()?;
    parent.add_class::<PyNettingSetManager>()?;
    parent.add_class::<PyNettingSetMargin>()?;
    parent.add_class::<PyPortfolioMarginResult>()?;
    parent.add_class::<PyPortfolioMarginAggregator>()?;
    // Iterators are registered for runtime use but excluded from __all__
    parent.add_class::<PyNettingSetManagerIterator>()?;
    parent.add_class::<PyPortfolioMarginResultIterator>()?;

    Ok(vec![
        "NettingSetId".to_string(),
        "NettingSet".to_string(),
        "NettingSetManager".to_string(),
        "NettingSetMargin".to_string(),
        "PortfolioMarginResult".to_string(),
        "PortfolioMarginAggregator".to_string(),
    ])
}
