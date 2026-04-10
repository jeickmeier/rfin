use finstack_core::factor_model::{
    AttributeFilter, DependencyFilter, FactorId, FactorModelConfig, FactorNode, HierarchicalConfig,
    MappingRule, MatchingConfig,
};
use finstack_core::types::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::Bound;

use super::helpers::{
    build_validated_config, curve_type_to_string, dependency_type_to_string, mapping_to_json,
    parse_curve_type, parse_dependency_type, parse_pricing_mode, parse_risk_measure,
    parse_unmatched_policy, pricing_mode_to_string, risk_measure_to_py, unmatched_policy_to_string,
};
use super::market::{
    PyBumpSizeConfig, PyFactorCovarianceMatrix, PyFactorDefinition, PyMarketDependency,
};

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "AttributeFilter",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAttributeFilter {
    pub(crate) inner: AttributeFilter,
}

impl PyAttributeFilter {
    pub(super) fn from_inner(inner: AttributeFilter) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyAttributeFilter {
    #[new]
    #[pyo3(signature = (tags=None, meta=None))]
    fn new(tags: Option<Vec<String>>, meta: Option<Vec<(String, String)>>) -> Self {
        Self::from_inner(AttributeFilter {
            tags: tags.unwrap_or_default(),
            meta: meta.unwrap_or_default(),
        })
    }

    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.clone()
    }

    #[getter]
    fn meta(&self) -> Vec<(String, String)> {
        self.inner.meta.clone()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "DependencyFilter",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyDependencyFilter {
    pub(crate) inner: DependencyFilter,
}

impl PyDependencyFilter {
    pub(super) fn from_inner(inner: DependencyFilter) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyDependencyFilter {
    #[new]
    #[pyo3(signature = (dependency_type=None, curve_type=None, id=None))]
    fn new(
        dependency_type: Option<String>,
        curve_type: Option<String>,
        id: Option<String>,
    ) -> PyResult<Self> {
        Ok(Self::from_inner(DependencyFilter {
            dependency_type: dependency_type
                .as_deref()
                .map(parse_dependency_type)
                .transpose()?,
            curve_type: curve_type.as_deref().map(parse_curve_type).transpose()?,
            id,
        }))
    }

    #[getter]
    fn dependency_type(&self) -> Option<String> {
        self.inner.dependency_type.map(dependency_type_to_string)
    }

    #[getter]
    fn curve_type(&self) -> Option<String> {
        self.inner.curve_type.map(curve_type_to_string)
    }

    #[getter]
    fn id(&self) -> Option<String> {
        self.inner.id.clone()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MappingRule",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMappingRule {
    pub(crate) inner: MappingRule,
}

impl PyMappingRule {
    fn from_inner(inner: MappingRule) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMappingRule {
    #[new]
    fn new(
        dependency_filter: PyRef<'_, PyDependencyFilter>,
        attribute_filter: PyRef<'_, PyAttributeFilter>,
        factor_id: String,
    ) -> Self {
        Self::from_inner(MappingRule {
            dependency_filter: dependency_filter.inner.clone(),
            attribute_filter: attribute_filter.inner.clone(),
            factor_id: FactorId::new(factor_id),
        })
    }

    #[getter]
    fn dependency_filter(&self) -> PyDependencyFilter {
        PyDependencyFilter::from_inner(self.inner.dependency_filter.clone())
    }

    #[getter]
    fn attribute_filter(&self) -> PyAttributeFilter {
        PyAttributeFilter::from_inner(self.inner.attribute_filter.clone())
    }

    #[getter]
    fn factor_id(&self) -> String {
        self.inner.factor_id.as_str().to_string()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorNode",
    from_py_object
)]
#[derive(Clone)]
pub struct PyFactorNode {
    pub(crate) inner: FactorNode,
}

impl PyFactorNode {
    fn from_inner(inner: FactorNode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorNode {
    #[new]
    #[pyo3(signature = (factor_id=None, filter=None, children=None))]
    fn new(
        factor_id: Option<String>,
        filter: Option<PyRef<'_, PyAttributeFilter>>,
        children: Option<Vec<PyFactorNode>>,
    ) -> Self {
        Self::from_inner(FactorNode {
            factor_id: factor_id.map(FactorId::new),
            filter: filter
                .map(|filter| filter.inner.clone())
                .unwrap_or_default(),
            children: children
                .unwrap_or_default()
                .into_iter()
                .map(|node| node.inner)
                .collect(),
        })
    }

    #[getter]
    fn factor_id(&self) -> Option<String> {
        self.inner
            .factor_id
            .as_ref()
            .map(|factor_id| factor_id.as_str().to_string())
    }

    #[getter]
    fn filter(&self) -> PyAttributeFilter {
        PyAttributeFilter::from_inner(self.inner.filter.clone())
    }

    #[getter]
    fn children(&self) -> Vec<PyFactorNode> {
        self.inner
            .children
            .iter()
            .cloned()
            .map(PyFactorNode::from_inner)
            .collect()
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "HierarchicalConfig",
    from_py_object
)]
#[derive(Clone)]
pub struct PyHierarchicalConfig {
    pub(crate) inner: HierarchicalConfig,
}

impl PyHierarchicalConfig {
    fn from_inner(inner: HierarchicalConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyHierarchicalConfig {
    #[new]
    #[pyo3(signature = (root, dependency_filter=None))]
    fn new(root: PyFactorNode, dependency_filter: Option<PyRef<'_, PyDependencyFilter>>) -> Self {
        Self::from_inner(HierarchicalConfig {
            dependency_filter: dependency_filter
                .map(|filter| filter.inner.clone())
                .unwrap_or_default(),
            root: root.inner,
        })
    }

    #[getter]
    fn dependency_filter(&self) -> PyDependencyFilter {
        PyDependencyFilter::from_inner(self.inner.dependency_filter.clone())
    }

    #[getter]
    fn root(&self) -> PyFactorNode {
        PyFactorNode::from_inner(self.inner.root.clone())
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "MatchingConfig",
    from_py_object
)]
#[derive(Clone)]
pub struct PyMatchingConfig {
    pub(crate) inner: MatchingConfig,
}

impl PyMatchingConfig {
    fn from_inner(inner: MatchingConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMatchingConfig {
    #[staticmethod]
    fn mapping_table(rules: Vec<PyMappingRule>) -> Self {
        Self::from_inner(MatchingConfig::MappingTable(
            rules.into_iter().map(|rule| rule.inner).collect(),
        ))
    }

    #[staticmethod]
    fn cascade(configs: Vec<PyMatchingConfig>) -> Self {
        Self::from_inner(MatchingConfig::Cascade(
            configs.into_iter().map(|config| config.inner).collect(),
        ))
    }

    #[staticmethod]
    fn hierarchical(config: PyHierarchicalConfig) -> Self {
        Self::from_inner(MatchingConfig::Hierarchical(config.inner))
    }

    #[getter]
    fn kind(&self) -> String {
        match self.inner {
            MatchingConfig::MappingTable(_) => "MappingTable".to_string(),
            MatchingConfig::Cascade(_) => "Cascade".to_string(),
            MatchingConfig::Hierarchical(_) => "Hierarchical".to_string(),
        }
    }

    /// Build a matcher from this config and match a dependency/attributes pair.
    ///
    /// Parameters
    /// ----------
    /// dependency : MarketDependency
    ///     The market dependency to match.
    /// tags : list[str], optional
    ///     Tags for attribute matching.
    /// meta : list[tuple[str, str]], optional
    ///     Key-value metadata for attribute matching.
    ///
    /// Returns
    /// -------
    /// str or None
    ///     The matched factor ID string, or None if no match.
    #[pyo3(signature = (dependency, tags=None, meta=None))]
    fn match_factor(
        &self,
        dependency: PyRef<'_, PyMarketDependency>,
        tags: Option<Vec<String>>,
        meta: Option<Vec<(String, String)>>,
    ) -> Option<String> {
        let matcher = self.inner.build_matcher();
        let mut attrs = Attributes::new();
        if let Some(tag_list) = tags {
            attrs = attrs.with_tags(tag_list);
        }
        if let Some(meta_list) = meta {
            for (k, v) in meta_list {
                attrs = attrs.with_meta(k, v);
            }
        }
        matcher
            .match_factor(&dependency.inner, &attrs)
            .map(|fid| fid.as_str().to_string())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|err| PyValueError::new_err(err.to_string()))
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|err| PyValueError::new_err(err.to_string()))?;

        if let Ok(inner) = serde_json::from_value::<MatchingConfig>(value.clone()) {
            return Ok(Self::from_inner(inner));
        }

        let inner = if let Some(inner) = value.get("mapping_table") {
            MatchingConfig::MappingTable(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else if let Some(inner) = value.get("cascade") {
            MatchingConfig::Cascade(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else if let Some(inner) = value.get("hierarchical") {
            MatchingConfig::Hierarchical(
                serde_json::from_value(inner.clone())
                    .map_err(|err| PyValueError::new_err(err.to_string()))?,
            )
        } else {
            return Err(PyValueError::new_err("Unrecognized MatchingConfig format"));
        };
        Ok(Self::from_inner(inner))
    }
}

#[pyclass(
    module = "finstack.portfolio.factor_model",
    name = "FactorModelConfig",
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFactorModelConfig {
    pub(crate) inner: FactorModelConfig,
}

impl PyFactorModelConfig {
    fn from_inner(inner: FactorModelConfig) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorModelConfig {
    #[new]
    #[pyo3(signature = (factors, covariance, matching, pricing_mode, risk_measure=None, bump_size=None, unmatched_policy=None))]
    fn new(
        factors: Vec<PyFactorDefinition>,
        covariance: PyRef<'_, PyFactorCovarianceMatrix>,
        matching: PyRef<'_, PyMatchingConfig>,
        pricing_mode: String,
        risk_measure: Option<&Bound<'_, PyAny>>,
        bump_size: Option<PyRef<'_, PyBumpSizeConfig>>,
        unmatched_policy: Option<String>,
    ) -> PyResult<Self> {
        let config = FactorModelConfig {
            factors: factors.into_iter().map(|factor| factor.inner).collect(),
            covariance: covariance.inner.clone(),
            matching: matching.inner.clone(),
            pricing_mode: parse_pricing_mode(&pricing_mode)?,
            risk_measure: parse_risk_measure(risk_measure)?,
            bump_size: bump_size.map(|config| config.inner.clone()),
            unmatched_policy: unmatched_policy
                .as_deref()
                .map(parse_unmatched_policy)
                .transpose()?,
        };
        Ok(Self::from_inner(build_validated_config(config)?))
    }

    #[getter]
    fn factors(&self) -> Vec<PyFactorDefinition> {
        self.inner
            .factors
            .iter()
            .cloned()
            .map(PyFactorDefinition::from_inner)
            .collect()
    }

    #[getter]
    fn covariance(&self) -> PyFactorCovarianceMatrix {
        PyFactorCovarianceMatrix::from_inner(self.inner.covariance.clone())
    }

    #[getter]
    fn matching(&self) -> PyMatchingConfig {
        PyMatchingConfig::from_inner(self.inner.matching.clone())
    }

    #[getter]
    fn pricing_mode(&self) -> String {
        pricing_mode_to_string(self.inner.pricing_mode)
    }

    #[getter]
    fn risk_measure(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        risk_measure_to_py(py, &self.inner.risk_measure)
    }

    #[getter]
    fn bump_size(&self) -> Option<PyBumpSizeConfig> {
        self.inner
            .bump_size
            .clone()
            .map(PyBumpSizeConfig::from_inner)
    }

    #[getter]
    fn unmatched_policy(&self) -> Option<String> {
        self.inner.unmatched_policy.map(unmatched_policy_to_string)
    }

    fn to_json(&self) -> PyResult<String> {
        mapping_to_json(&self.inner)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let config: FactorModelConfig =
            serde_json::from_str(json).map_err(|err| PyValueError::new_err(err.to_string()))?;
        Ok(Self::from_inner(build_validated_config(config)?))
    }
}
