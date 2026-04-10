use crate::core::dates::utils::py_to_date;
use crate::core::market_data::context::PyMarketContext;
use crate::portfolio::error::portfolio_to_py;
use crate::portfolio::positions::PyPortfolio;
use finstack_core::factor_model::FactorId;
use finstack_portfolio::factor_model::{
    FactorConstraint, FactorModel, FactorModelBuilder, PositionChange, RiskDecomposition,
    WhatIfEngine,
};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use pyo3::prelude::*;
use pyo3::Bound;
use std::sync::Arc;

use super::market::PyFactorDefinition;
use super::matching::PyFactorModelConfig;
use super::results::{
    PyFactorAssignmentReport, PyFactorConstraint, PyFactorOptimizationResult, PyPositionChange,
    PyRiskDecomposition, PySensitivityMatrix, PyStressResult, PyWhatIfResult,
};

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModelBuilder",
    skip_from_py_object
)]
pub struct PyFactorModelBuilder {
    inner: FactorModelBuilder,
}

#[pymethods]
impl PyFactorModelBuilder {
    #[new]
    fn new() -> Self {
        Self {
            inner: FactorModelBuilder::new(),
        }
    }

    fn config<'py>(
        mut slf: PyRefMut<'py, Self>,
        config: PyRef<'py, PyFactorModelConfig>,
    ) -> PyRefMut<'py, Self> {
        let builder = std::mem::replace(&mut slf.inner, FactorModelBuilder::new());
        slf.inner = builder.config(config.inner.clone());
        slf
    }

    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyFactorModel> {
        let builder = std::mem::replace(&mut slf.inner, FactorModelBuilder::new());
        let model = builder.build().map_err(portfolio_to_py)?;
        Ok(PyFactorModel {
            inner: Arc::new(model),
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModel",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorModel {
    inner: Arc<FactorModel>,
}

#[pymethods]
impl PyFactorModel {
    fn factors(&self) -> Vec<PyFactorDefinition> {
        self.inner
            .factors()
            .iter()
            .cloned()
            .map(PyFactorDefinition::from_inner)
            .collect()
    }

    fn assign_factors(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
    ) -> PyResult<PyFactorAssignmentReport> {
        self.inner
            .assign_factors(&portfolio.inner)
            .map(PyFactorAssignmentReport::from_inner)
            .map_err(portfolio_to_py)
    }

    fn compute_sensitivities(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PySensitivityMatrix> {
        let as_of = py_to_date(as_of)?;
        self.inner
            .compute_sensitivities(&portfolio.inner, &market.inner, as_of)
            .map(PySensitivityMatrix::from_inner)
            .map_err(portfolio_to_py)
    }

    fn analyze(
        &self,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyRiskDecomposition> {
        let as_of = py_to_date(as_of)?;
        self.inner
            .analyze(&portfolio.inner, &market.inner, as_of)
            .map(PyRiskDecomposition::from_inner)
            .map_err(portfolio_to_py)
    }

    fn what_if(
        &self,
        base: PyRef<'_, PyRiskDecomposition>,
        sensitivities: PyRef<'_, PySensitivityMatrix>,
        portfolio: PyRef<'_, PyPortfolio>,
        market: PyRef<'_, PyMarketContext>,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyWhatIfEngine> {
        Ok(PyWhatIfEngine {
            model: self.inner.clone(),
            base: base.inner.clone(),
            sensitivities: sensitivities.inner.clone(),
            portfolio: portfolio.inner.clone(),
            market: market.inner.clone(),
            as_of: py_to_date(as_of)?,
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "WhatIfEngine",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWhatIfEngine {
    model: Arc<FactorModel>,
    base: RiskDecomposition,
    sensitivities: SensitivityMatrix,
    portfolio: finstack_portfolio::Portfolio,
    market: finstack_core::market_data::context::MarketContext,
    as_of: time::Date,
}

impl PyWhatIfEngine {
    fn with_inner<T>(
        &self,
        f: impl FnOnce(WhatIfEngine<'_>) -> finstack_portfolio::Result<T>,
    ) -> finstack_portfolio::Result<T> {
        let inner = WhatIfEngine::new(
            self.model.as_ref(),
            &self.base,
            &self.sensitivities,
            &self.portfolio,
            &self.market,
            self.as_of,
        );
        f(inner)
    }
}

#[pymethods]
impl PyWhatIfEngine {
    fn position_what_if(&self, changes: Vec<PyPositionChange>) -> PyResult<PyWhatIfResult> {
        let changes: Vec<PositionChange> = changes.into_iter().map(|change| change.inner).collect();
        self.with_inner(|inner| inner.position_what_if(&changes))
            .map(PyWhatIfResult::from_inner)
            .map_err(portfolio_to_py)
    }

    fn factor_stress(&self, stresses: Vec<(String, f64)>) -> PyResult<PyStressResult> {
        let stresses: Vec<(FactorId, f64)> = stresses
            .into_iter()
            .map(|(factor_id, shift)| (FactorId::new(factor_id), shift))
            .collect();
        self.with_inner(|inner| inner.factor_stress(&stresses))
            .map(PyStressResult::from_inner)
            .map_err(portfolio_to_py)
    }

    fn optimize(
        &self,
        constraints: Vec<PyFactorConstraint>,
    ) -> PyResult<PyFactorOptimizationResult> {
        let constraints: Vec<FactorConstraint> = constraints
            .into_iter()
            .map(|constraint| constraint.inner)
            .collect();
        self.with_inner(|inner| inner.optimize(&constraints))
            .map(PyFactorOptimizationResult::from_inner)
            .map_err(portfolio_to_py)
    }
}
