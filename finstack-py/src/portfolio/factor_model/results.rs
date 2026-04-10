use crate::portfolio::types::PyPosition;
use finstack_core::factor_model::{FactorId, RiskMeasure};
use finstack_portfolio::factor_model::{
    FactorAssignmentReport, FactorConstraint, FactorContribution, FactorContributionDelta,
    FactorOptimizationResult, PositionAssignment, PositionChange, PositionFactorContribution,
    RiskDecomposition, StressResult, UnmatchedEntry, WhatIfResult,
};
use finstack_valuations::factor_model::sensitivity::SensitivityMatrix;
use pyo3::prelude::*;

use super::market::PyMarketDependency;

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionAssignment",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPositionAssignment {
    pub(crate) inner: PositionAssignment,
}

impl PyPositionAssignment {
    pub(super) fn from_inner(inner: PositionAssignment) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionAssignment {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn mappings(&self) -> Vec<(PyMarketDependency, String)> {
        self.inner
            .mappings
            .iter()
            .cloned()
            .map(|(dependency, factor_id)| {
                (
                    PyMarketDependency::from_inner(dependency),
                    factor_id.as_str().to_string(),
                )
            })
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "UnmatchedEntry",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyUnmatchedEntry {
    pub(crate) inner: UnmatchedEntry,
}

impl PyUnmatchedEntry {
    pub(super) fn from_inner(inner: UnmatchedEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnmatchedEntry {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn dependency(&self) -> PyMarketDependency {
        PyMarketDependency::from_inner(self.inner.dependency.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorAssignmentReport",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorAssignmentReport {
    pub(crate) inner: FactorAssignmentReport,
}

impl PyFactorAssignmentReport {
    pub(super) fn from_inner(inner: FactorAssignmentReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorAssignmentReport {
    #[getter]
    fn assignments(&self) -> Vec<PyPositionAssignment> {
        self.inner
            .assignments
            .iter()
            .cloned()
            .map(PyPositionAssignment::from_inner)
            .collect()
    }

    #[getter]
    fn unmatched(&self) -> Vec<PyUnmatchedEntry> {
        self.inner
            .unmatched
            .iter()
            .cloned()
            .map(PyUnmatchedEntry::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "SensitivityMatrix",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySensitivityMatrix {
    pub(crate) inner: SensitivityMatrix,
}

impl PySensitivityMatrix {
    pub(super) fn from_inner(inner: SensitivityMatrix) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySensitivityMatrix {
    fn n_positions(&self) -> usize {
        self.inner.n_positions()
    }

    fn n_factors(&self) -> usize {
        self.inner.n_factors()
    }

    fn position_ids(&self) -> Vec<String> {
        self.inner.position_ids().to_vec()
    }

    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .factor_ids()
            .iter()
            .map(|factor_id| factor_id.as_str().to_string())
            .collect()
    }

    fn delta(&self, position_idx: usize, factor_idx: usize) -> f64 {
        self.inner.delta(position_idx, factor_idx)
    }

    fn position_deltas(&self, position_idx: usize) -> Vec<f64> {
        self.inner.position_deltas(position_idx).to_vec()
    }

    fn factor_deltas(&self, factor_idx: usize) -> Vec<f64> {
        self.inner.factor_deltas(factor_idx)
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorContribution",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorContribution {
    pub(crate) inner: FactorContribution,
}

impl PyFactorContribution {
    pub(super) fn from_inner(inner: FactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContribution {
    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn absolute_risk(&self) -> f64 {
        self.inner.absolute_risk
    }

    #[getter]
    fn relative_risk(&self) -> f64 {
        self.inner.relative_risk
    }

    #[getter]
    fn marginal_risk(&self) -> f64 {
        self.inner.marginal_risk
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionFactorContribution",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPositionFactorContribution {
    pub(crate) inner: PositionFactorContribution,
}

impl PyPositionFactorContribution {
    pub(super) fn from_inner(inner: PositionFactorContribution) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionFactorContribution {
    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_string()
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn risk_contribution(&self) -> f64 {
        self.inner.risk_contribution
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "RiskDecomposition",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyRiskDecomposition {
    pub(crate) inner: RiskDecomposition,
}

impl PyRiskDecomposition {
    pub(super) fn from_inner(inner: RiskDecomposition) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRiskDecomposition {
    #[getter]
    fn total_risk(&self) -> f64 {
        self.inner.total_risk
    }

    #[getter]
    fn measure(&self) -> String {
        match self.inner.measure {
            RiskMeasure::Variance => "Variance".to_string(),
            RiskMeasure::Volatility => "Volatility".to_string(),
            RiskMeasure::VaR { .. } => "VaR".to_string(),
            RiskMeasure::ExpectedShortfall { .. } => "ExpectedShortfall".to_string(),
        }
    }

    #[getter]
    fn factor_contributions(&self) -> Vec<PyFactorContribution> {
        self.inner
            .factor_contributions
            .iter()
            .cloned()
            .map(PyFactorContribution::from_inner)
            .collect()
    }

    #[getter]
    fn residual_risk(&self) -> f64 {
        self.inner.residual_risk
    }

    #[getter]
    fn position_factor_contributions(&self) -> Vec<PyPositionFactorContribution> {
        self.inner
            .position_factor_contributions
            .iter()
            .cloned()
            .map(PyPositionFactorContribution::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorContributionDelta",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorContributionDelta {
    pub(crate) inner: FactorContributionDelta,
}

impl PyFactorContributionDelta {
    pub(super) fn from_inner(inner: FactorContributionDelta) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorContributionDelta {
    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }

    #[getter]
    fn absolute_change(&self) -> f64 {
        self.inner.absolute_change
    }

    #[getter]
    fn relative_change(&self) -> f64 {
        self.inner.relative_change
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "WhatIfResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyWhatIfResult {
    pub(crate) inner: WhatIfResult,
}

impl PyWhatIfResult {
    pub(super) fn from_inner(inner: WhatIfResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyWhatIfResult {
    #[getter]
    fn before(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.before.clone())
    }

    #[getter]
    fn after(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.after.clone())
    }

    #[getter]
    fn delta(&self) -> Vec<PyFactorContributionDelta> {
        self.inner
            .delta
            .iter()
            .cloned()
            .map(PyFactorContributionDelta::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "StressResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyStressResult {
    pub(crate) inner: StressResult,
}

impl PyStressResult {
    pub(super) fn from_inner(inner: StressResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyStressResult {
    #[getter]
    fn total_pnl(&self) -> f64 {
        self.inner.total_pnl
    }

    #[getter]
    fn position_pnl(&self) -> Vec<(String, f64)> {
        self.inner
            .position_pnl
            .iter()
            .map(|(position_id, pnl)| (position_id.as_str().to_string(), *pnl))
            .collect()
    }

    #[getter]
    fn stressed_decomposition(&self) -> PyRiskDecomposition {
        PyRiskDecomposition::from_inner(self.inner.stressed_decomposition.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "PositionChange",
    from_py_object
)]
#[derive(Clone)]
pub struct PyPositionChange {
    pub(crate) inner: PositionChange,
}

impl PyPositionChange {
    pub(super) fn from_inner(inner: PositionChange) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionChange {
    #[staticmethod]
    fn add(position: PyRef<'_, PyPosition>) -> Self {
        Self::from_inner(PositionChange::Add {
            position: Box::new(position.inner.clone()),
        })
    }

    #[staticmethod]
    fn remove(position_id: String) -> Self {
        Self::from_inner(PositionChange::Remove {
            position_id: finstack_portfolio::PositionId::new(position_id),
        })
    }

    #[staticmethod]
    fn resize(position_id: String, new_quantity: f64) -> Self {
        Self::from_inner(PositionChange::Resize {
            position_id: finstack_portfolio::PositionId::new(position_id),
            new_quantity,
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorConstraint",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFactorConstraint {
    pub(crate) inner: FactorConstraint,
}

impl PyFactorConstraint {
    pub(super) fn from_inner(inner: FactorConstraint) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorConstraint {
    #[staticmethod]
    fn max_factor_risk(factor_id: String, max_risk: f64) -> Self {
        Self::from_inner(FactorConstraint::MaxFactorRisk {
            factor_id: FactorId::new(factor_id),
            max_risk,
        })
    }

    #[staticmethod]
    fn max_factor_concentration(factor_id: String, max_fraction: f64) -> Self {
        Self::from_inner(FactorConstraint::MaxFactorConcentration {
            factor_id: FactorId::new(factor_id),
            max_fraction,
        })
    }

    #[staticmethod]
    fn factor_neutral(factor_id: String) -> Self {
        Self::from_inner(FactorConstraint::FactorNeutral {
            factor_id: FactorId::new(factor_id),
        })
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorOptimizationResult",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorOptimizationResult {
    pub(crate) inner: FactorOptimizationResult,
}

impl PyFactorOptimizationResult {
    pub(super) fn from_inner(inner: FactorOptimizationResult) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorOptimizationResult {
    #[getter]
    fn optimized_quantities(&self) -> Vec<(String, f64)> {
        self.inner
            .optimized_quantities
            .iter()
            .map(|(position_id, quantity)| (position_id.as_str().to_string(), *quantity))
            .collect()
    }
}
