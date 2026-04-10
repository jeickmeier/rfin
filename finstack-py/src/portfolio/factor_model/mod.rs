//! Python bindings for portfolio factor-model configuration and analysis.

mod engine;
mod helpers;
mod market;
mod matching;
mod results;

pub(crate) use engine::{PyFactorModel, PyFactorModelBuilder, PyWhatIfEngine};
pub(crate) use market::{
    PyBumpSizeConfig, PyFactorCovarianceMatrix, PyFactorDefinition, PyMarketDependency,
    PyMarketMapping,
};
pub(crate) use matching::{
    PyAttributeFilter, PyDependencyFilter, PyFactorModelConfig, PyFactorNode, PyHierarchicalConfig,
    PyMappingRule, PyMatchingConfig,
};
pub(crate) use results::{
    PyFactorAssignmentReport, PyFactorConstraint, PyFactorContribution, PyFactorContributionDelta,
    PyFactorOptimizationResult, PyPositionAssignment, PyPositionChange,
    PyPositionFactorContribution, PyRiskDecomposition, PySensitivityMatrix, PyStressResult,
    PyUnmatchedEntry, PyWhatIfResult,
};

use pyo3::prelude::*;
use pyo3::types::PyModule;

pub(crate) fn register<'py>(
    _py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<String>> {
    parent.add_class::<PyMarketDependency>()?;
    parent.add_class::<PyBumpSizeConfig>()?;
    parent.add_class::<PyMarketMapping>()?;
    parent.add_class::<PyFactorDefinition>()?;
    parent.add_class::<PyFactorCovarianceMatrix>()?;
    parent.add_class::<PyAttributeFilter>()?;
    parent.add_class::<PyDependencyFilter>()?;
    parent.add_class::<PyMappingRule>()?;
    parent.add_class::<PyFactorNode>()?;
    parent.add_class::<PyHierarchicalConfig>()?;
    parent.add_class::<PyMatchingConfig>()?;
    parent.add_class::<PyFactorModelConfig>()?;
    parent.add_class::<PyPositionAssignment>()?;
    parent.add_class::<PyUnmatchedEntry>()?;
    parent.add_class::<PyFactorAssignmentReport>()?;
    parent.add_class::<PySensitivityMatrix>()?;
    parent.add_class::<PyFactorContribution>()?;
    parent.add_class::<PyPositionFactorContribution>()?;
    parent.add_class::<PyRiskDecomposition>()?;
    parent.add_class::<PyFactorContributionDelta>()?;
    parent.add_class::<PyWhatIfResult>()?;
    parent.add_class::<PyStressResult>()?;
    parent.add_class::<PyPositionChange>()?;
    parent.add_class::<PyFactorConstraint>()?;
    parent.add_class::<PyFactorOptimizationResult>()?;
    parent.add_class::<PyFactorModelBuilder>()?;
    parent.add_class::<PyFactorModel>()?;
    parent.add_class::<PyWhatIfEngine>()?;

    Ok(vec![
        "MarketDependency".to_string(),
        "BumpSizeConfig".to_string(),
        "MarketMapping".to_string(),
        "FactorDefinition".to_string(),
        "FactorCovarianceMatrix".to_string(),
        "AttributeFilter".to_string(),
        "DependencyFilter".to_string(),
        "MappingRule".to_string(),
        "FactorNode".to_string(),
        "HierarchicalConfig".to_string(),
        "MatchingConfig".to_string(),
        "FactorModelConfig".to_string(),
        "PositionAssignment".to_string(),
        "UnmatchedEntry".to_string(),
        "FactorAssignmentReport".to_string(),
        "SensitivityMatrix".to_string(),
        "FactorContribution".to_string(),
        "PositionFactorContribution".to_string(),
        "RiskDecomposition".to_string(),
        "FactorContributionDelta".to_string(),
        "WhatIfResult".to_string(),
        "StressResult".to_string(),
        "PositionChange".to_string(),
        "FactorConstraint".to_string(),
        "FactorOptimizationResult".to_string(),
        "FactorModelBuilder".to_string(),
        "FactorModel".to_string(),
        "WhatIfEngine".to_string(),
    ])
}
