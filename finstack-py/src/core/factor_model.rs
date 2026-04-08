//! Python bindings for core factor-model vocabulary types.
//!
//! Exposes identifiers, enums, and config/data structs from
//! `finstack_core::factor_model` under `finstack.core.factor_model`.
//! Struct types already wrapped in [`crate::portfolio::factor_model`] are
//! re-registered here; enum and ID types get standalone frozen wrappers.

use crate::errors::core_to_py;
use crate::portfolio::factor_model::{
    PyAttributeFilter, PyBumpSizeConfig, PyDependencyFilter, PyFactorCovarianceMatrix,
    PyFactorDefinition, PyFactorModelConfig, PyFactorNode, PyHierarchicalConfig, PyMappingRule,
    PyMarketDependency, PyMarketMapping, PyMatchingConfig,
};
use finstack_core::factor_model::{
    CascadeMatcher, CurveType, DependencyType, FactorId, FactorMatcher, FactorType,
    HierarchicalMatcher, MappingTableMatcher, PricingMode, RiskMeasure, UnmatchedPolicy,
};
use finstack_core::types::Attributes;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};
use pyo3::Bound;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '_' | '-' | ' '))
        .flat_map(char::to_lowercase)
        .collect()
}

fn str_hash(value: &str) -> u64 {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}

// ===================================================================
// FactorId
// ===================================================================

/// Unique identifier for a risk factor.
#[pyclass(
    name = "FactorId",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFactorId {
    pub(crate) inner: FactorId,
}

impl PyFactorId {
    pub(crate) fn from_inner(inner: FactorId) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorId {
    #[new]
    fn new(id: &str) -> Self {
        Self::from_inner(FactorId::new(id))
    }

    /// Return the underlying string value.
    fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    fn __str__(&self) -> &str {
        self.inner.as_str()
    }

    fn __repr__(&self) -> String {
        format!("FactorId('{}')", self.inner.as_str())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(self.inner.as_str())
    }
}

// ===================================================================
// PricingMode
// ===================================================================

/// Strategy used when extracting factor sensitivities.
#[pyclass(
    name = "PricingMode",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPricingMode {
    pub(crate) inner: PricingMode,
}

impl PyPricingMode {
    pub(crate) fn from_inner(inner: PricingMode) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            PricingMode::DeltaBased => "DeltaBased",
            PricingMode::FullRepricing => "FullRepricing",
        }
    }
}

#[pymethods]
impl PyPricingMode {
    /// Parse from ``"DeltaBased"`` or ``"FullRepricing"``.
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        match normalized(value).as_str() {
            "deltabased" => Ok(Self::from_inner(PricingMode::DeltaBased)),
            "fullrepricing" => Ok(Self::from_inner(PricingMode::FullRepricing)),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported pricing mode '{value}'. Expected DeltaBased or FullRepricing"
            ))),
        }
    }

    /// Central finite differences for linear deltas.
    #[staticmethod]
    fn delta_based() -> Self {
        Self::from_inner(PricingMode::DeltaBased)
    }

    /// Reprice across a scenario grid.
    #[staticmethod]
    fn full_repricing() -> Self {
        Self::from_inner(PricingMode::FullRepricing)
    }

    /// Variant name as a string.
    #[getter]
    fn value(&self) -> &str {
        self.label()
    }

    fn __str__(&self) -> &str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("PricingMode('{}')", self.label())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(self.label())
    }
}

// ===================================================================
// FactorType
// ===================================================================

/// Broad classification of a risk factor.
#[pyclass(
    name = "FactorType",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFactorType {
    pub(crate) inner: FactorType,
}

impl PyFactorType {
    pub(crate) fn from_inner(inner: FactorType) -> Self {
        Self { inner }
    }

    fn label(&self) -> String {
        match &self.inner {
            FactorType::Rates => "Rates".to_string(),
            FactorType::Credit => "Credit".to_string(),
            FactorType::Equity => "Equity".to_string(),
            FactorType::FX => "FX".to_string(),
            FactorType::Volatility => "Volatility".to_string(),
            FactorType::Commodity => "Commodity".to_string(),
            FactorType::Inflation => "Inflation".to_string(),
            FactorType::Custom(name) => format!("Custom:{name}"),
        }
    }
}

#[pymethods]
impl PyFactorType {
    /// Parse from ``"Rates"``, ``"Credit"``, ``"Custom:Weather"``, etc.
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        let lower = normalized(value);
        match lower.as_str() {
            "rates" => Ok(Self::from_inner(FactorType::Rates)),
            "credit" => Ok(Self::from_inner(FactorType::Credit)),
            "equity" => Ok(Self::from_inner(FactorType::Equity)),
            "fx" => Ok(Self::from_inner(FactorType::FX)),
            "volatility" | "vol" => Ok(Self::from_inner(FactorType::Volatility)),
            "commodity" => Ok(Self::from_inner(FactorType::Commodity)),
            "inflation" => Ok(Self::from_inner(FactorType::Inflation)),
            _ if lower.starts_with("custom:") => {
                let tail = match value.split_once(':') {
                    Some((_, t)) => t.trim().to_string(),
                    None => String::new(),
                };
                Ok(Self::from_inner(FactorType::Custom(tail)))
            }
            _ => Err(PyValueError::new_err(format!(
                "Unsupported factor type '{value}'. Expected Rates, Credit, Equity, FX, \
                 Volatility, Commodity, Inflation, or Custom:<name>"
            ))),
        }
    }

    /// Interest-rate factor.
    #[staticmethod]
    fn rates() -> Self {
        Self::from_inner(FactorType::Rates)
    }

    /// Credit-spread or hazard factor.
    #[staticmethod]
    fn credit() -> Self {
        Self::from_inner(FactorType::Credit)
    }

    /// Equity price factor.
    #[staticmethod]
    fn equity() -> Self {
        Self::from_inner(FactorType::Equity)
    }

    /// Foreign-exchange factor.
    #[staticmethod]
    fn fx() -> Self {
        Self::from_inner(FactorType::FX)
    }

    /// Volatility factor.
    #[staticmethod]
    fn volatility() -> Self {
        Self::from_inner(FactorType::Volatility)
    }

    /// Commodity factor.
    #[staticmethod]
    fn commodity() -> Self {
        Self::from_inner(FactorType::Commodity)
    }

    /// Inflation factor.
    #[staticmethod]
    fn inflation() -> Self {
        Self::from_inner(FactorType::Inflation)
    }

    /// User-defined factor bucket.
    #[staticmethod]
    fn custom(name: &str) -> Self {
        Self::from_inner(FactorType::Custom(name.to_string()))
    }

    /// Variant name as a string.
    #[getter]
    fn value(&self) -> String {
        self.label()
    }

    fn __str__(&self) -> String {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("FactorType('{}')", self.label())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(&self.label())
    }
}

// ===================================================================
// DependencyType
// ===================================================================

/// Classification used by dependency filters and matching config.
#[pyclass(
    name = "DependencyType",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDependencyType {
    pub(crate) inner: DependencyType,
}

impl PyDependencyType {
    pub(crate) fn from_inner(inner: DependencyType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            DependencyType::Discount => "Discount",
            DependencyType::Forward => "Forward",
            DependencyType::Credit => "Credit",
            DependencyType::Spot => "Spot",
            DependencyType::Vol => "Vol",
            DependencyType::Fx => "Fx",
            DependencyType::Series => "Series",
        }
    }
}

#[pymethods]
impl PyDependencyType {
    /// Parse from ``"Discount"``, ``"Forward"``, ``"Credit"``, etc.
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        match normalized(value).as_str() {
            "discount" => Ok(Self::from_inner(DependencyType::Discount)),
            "forward" => Ok(Self::from_inner(DependencyType::Forward)),
            "credit" => Ok(Self::from_inner(DependencyType::Credit)),
            "spot" => Ok(Self::from_inner(DependencyType::Spot)),
            "vol" | "volsurface" | "volatility" => Ok(Self::from_inner(DependencyType::Vol)),
            "fx" => Ok(Self::from_inner(DependencyType::Fx)),
            "series" => Ok(Self::from_inner(DependencyType::Series)),
            "hazard" => Err(PyValueError::new_err(
                "Hazard is a CurveType, not a DependencyType. \
                 Use DependencyType('Credit') and CurveType('Hazard')",
            )),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported dependency type '{value}'"
            ))),
        }
    }

    /// Discounting curve dependency.
    #[staticmethod]
    fn discount() -> Self {
        Self::from_inner(DependencyType::Discount)
    }

    /// Forward projection curve dependency.
    #[staticmethod]
    fn forward() -> Self {
        Self::from_inner(DependencyType::Forward)
    }

    /// Credit or hazard curve dependency.
    #[staticmethod]
    fn credit() -> Self {
        Self::from_inner(DependencyType::Credit)
    }

    /// Equity or commodity spot dependency.
    #[staticmethod]
    fn spot() -> Self {
        Self::from_inner(DependencyType::Spot)
    }

    /// Volatility surface dependency.
    #[staticmethod]
    fn vol() -> Self {
        Self::from_inner(DependencyType::Vol)
    }

    /// FX pair dependency.
    #[staticmethod]
    fn fx() -> Self {
        Self::from_inner(DependencyType::Fx)
    }

    /// Time-series dependency.
    #[staticmethod]
    fn series() -> Self {
        Self::from_inner(DependencyType::Series)
    }

    /// Variant name as a string.
    #[getter]
    fn value(&self) -> &str {
        self.label()
    }

    fn __str__(&self) -> &str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("DependencyType('{}')", self.label())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(self.label())
    }
}

// ===================================================================
// CurveType
// ===================================================================

/// Classification of a curve dependency's role.
#[pyclass(
    name = "CurveType",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyCurveType {
    pub(crate) inner: CurveType,
}

impl PyCurveType {
    pub(crate) fn from_inner(inner: CurveType) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            CurveType::Discount => "Discount",
            CurveType::Forward => "Forward",
            CurveType::Hazard => "Hazard",
            CurveType::Inflation => "Inflation",
            CurveType::BaseCorrelation => "BaseCorrelation",
        }
    }
}

#[pymethods]
impl PyCurveType {
    /// Parse from ``"Discount"``, ``"Forward"``, ``"Hazard"``, etc.
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        match normalized(value).as_str() {
            "discount" => Ok(Self::from_inner(CurveType::Discount)),
            "forward" => Ok(Self::from_inner(CurveType::Forward)),
            "hazard" | "credit" => Ok(Self::from_inner(CurveType::Hazard)),
            "inflation" => Ok(Self::from_inner(CurveType::Inflation)),
            "basecorrelation" => Ok(Self::from_inner(CurveType::BaseCorrelation)),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported curve type '{value}'"
            ))),
        }
    }

    /// Discounting curve.
    #[staticmethod]
    fn discount() -> Self {
        Self::from_inner(CurveType::Discount)
    }

    /// Forward projection curve.
    #[staticmethod]
    fn forward() -> Self {
        Self::from_inner(CurveType::Forward)
    }

    /// Credit or hazard curve.
    #[staticmethod]
    fn hazard() -> Self {
        Self::from_inner(CurveType::Hazard)
    }

    /// Inflation curve.
    #[staticmethod]
    fn inflation() -> Self {
        Self::from_inner(CurveType::Inflation)
    }

    /// Base-correlation surface-backed curve.
    #[staticmethod]
    fn base_correlation() -> Self {
        Self::from_inner(CurveType::BaseCorrelation)
    }

    /// Variant name as a string.
    #[getter]
    fn value(&self) -> &str {
        self.label()
    }

    fn __str__(&self) -> &str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("CurveType('{}')", self.label())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(self.label())
    }
}

// ===================================================================
// UnmatchedPolicy
// ===================================================================

/// Policy for handling dependencies that do not match any factor.
#[pyclass(
    name = "UnmatchedPolicy",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyUnmatchedPolicy {
    pub(crate) inner: UnmatchedPolicy,
}

impl PyUnmatchedPolicy {
    pub(crate) fn from_inner(inner: UnmatchedPolicy) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            UnmatchedPolicy::Strict => "Strict",
            UnmatchedPolicy::Residual => "Residual",
            UnmatchedPolicy::Warn => "Warn",
        }
    }
}

#[pymethods]
impl PyUnmatchedPolicy {
    /// Parse from ``"Strict"``, ``"Residual"``, or ``"Warn"``.
    #[new]
    fn new(value: &str) -> PyResult<Self> {
        match normalized(value).as_str() {
            "strict" => Ok(Self::from_inner(UnmatchedPolicy::Strict)),
            "residual" => Ok(Self::from_inner(UnmatchedPolicy::Residual)),
            "warn" => Ok(Self::from_inner(UnmatchedPolicy::Warn)),
            _ => Err(PyValueError::new_err(format!(
                "Unsupported unmatched policy '{value}'. Expected Strict, Residual, or Warn"
            ))),
        }
    }

    /// Fail immediately when any dependency is unmatched.
    #[staticmethod]
    fn strict() -> Self {
        Self::from_inner(UnmatchedPolicy::Strict)
    }

    /// Roll unmatched risk into a residual bucket.
    #[staticmethod]
    fn residual() -> Self {
        Self::from_inner(UnmatchedPolicy::Residual)
    }

    /// Continue but surface a warning.
    #[staticmethod]
    fn warn() -> Self {
        Self::from_inner(UnmatchedPolicy::Warn)
    }

    /// Variant name as a string.
    #[getter]
    fn value(&self) -> &str {
        self.label()
    }

    fn __str__(&self) -> &str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("UnmatchedPolicy('{}')", self.label())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        str_hash(self.label())
    }
}

// ===================================================================
// RiskMeasure
// ===================================================================

/// Risk measure used when aggregating factor exposures.
#[pyclass(
    name = "RiskMeasure",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRiskMeasure {
    pub(crate) inner: RiskMeasure,
}

impl PyRiskMeasure {
    pub(crate) fn from_inner(inner: RiskMeasure) -> Self {
        Self { inner }
    }

    fn kind_label(&self) -> &'static str {
        match self.inner {
            RiskMeasure::Variance => "Variance",
            RiskMeasure::Volatility => "Volatility",
            RiskMeasure::VaR { .. } => "VaR",
            RiskMeasure::ExpectedShortfall { .. } => "ExpectedShortfall",
        }
    }
}

#[pymethods]
impl PyRiskMeasure {
    /// Parse from a variant name and optional confidence level.
    ///
    /// ``"Variance"`` and ``"Volatility"`` need no confidence;
    /// ``"VaR"`` and ``"ExpectedShortfall"`` require confidence in ``(0.5, 1)``.
    #[new]
    #[pyo3(signature = (value, confidence=None))]
    fn new(value: &str, confidence: Option<f64>) -> PyResult<Self> {
        match normalized(value).as_str() {
            "variance" => Ok(Self::from_inner(RiskMeasure::Variance)),
            "volatility" => Ok(Self::from_inner(RiskMeasure::Volatility)),
            "var" => {
                let c = confidence
                    .ok_or_else(|| PyValueError::new_err("VaR requires a confidence level"))?;
                let rm = RiskMeasure::VaR { confidence: c };
                rm.validate().map_err(core_to_py)?;
                Ok(Self::from_inner(rm))
            }
            "expectedshortfall" => {
                let c = confidence.ok_or_else(|| {
                    PyValueError::new_err("ExpectedShortfall requires a confidence level")
                })?;
                let rm = RiskMeasure::ExpectedShortfall { confidence: c };
                rm.validate().map_err(core_to_py)?;
                Ok(Self::from_inner(rm))
            }
            _ => Err(PyValueError::new_err(format!(
                "Unsupported risk measure '{value}'. Expected Variance, Volatility, \
                 VaR, or ExpectedShortfall"
            ))),
        }
    }

    /// Portfolio variance.
    #[staticmethod]
    fn variance() -> Self {
        Self::from_inner(RiskMeasure::Variance)
    }

    /// Portfolio volatility (standard deviation).
    #[staticmethod]
    fn volatility() -> Self {
        Self::from_inner(RiskMeasure::Volatility)
    }

    /// Value at Risk at a given confidence level in ``(0.5, 1)``.
    #[staticmethod]
    fn var(confidence: f64) -> PyResult<Self> {
        let rm = RiskMeasure::VaR { confidence };
        rm.validate().map_err(core_to_py)?;
        Ok(Self::from_inner(rm))
    }

    /// Expected Shortfall at a given confidence level in ``(0.5, 1)``.
    #[staticmethod]
    fn expected_shortfall(confidence: f64) -> PyResult<Self> {
        let rm = RiskMeasure::ExpectedShortfall { confidence };
        rm.validate().map_err(core_to_py)?;
        Ok(Self::from_inner(rm))
    }

    /// Kind of risk measure.
    #[getter]
    fn kind(&self) -> &str {
        self.kind_label()
    }

    /// Confidence level, or ``None`` for Variance / Volatility.
    #[getter]
    fn confidence(&self) -> Option<f64> {
        match self.inner {
            RiskMeasure::Variance | RiskMeasure::Volatility => None,
            RiskMeasure::VaR { confidence } | RiskMeasure::ExpectedShortfall { confidence } => {
                Some(confidence)
            }
        }
    }

    fn __str__(&self) -> String {
        match self.inner {
            RiskMeasure::Variance => "Variance".to_string(),
            RiskMeasure::Volatility => "Volatility".to_string(),
            RiskMeasure::VaR { confidence } => format!("VaR({confidence})"),
            RiskMeasure::ExpectedShortfall { confidence } => {
                format!("ExpectedShortfall({confidence})")
            }
        }
    }

    fn __repr__(&self) -> String {
        match self.inner {
            RiskMeasure::Variance => "RiskMeasure('Variance')".to_string(),
            RiskMeasure::Volatility => "RiskMeasure('Volatility')".to_string(),
            RiskMeasure::VaR { confidence } => {
                format!("RiskMeasure('VaR', confidence={confidence})")
            }
            RiskMeasure::ExpectedShortfall { confidence } => {
                format!("RiskMeasure('ExpectedShortfall', confidence={confidence})")
            }
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.kind_label().hash(&mut h);
        match self.inner {
            RiskMeasure::Variance | RiskMeasure::Volatility => {}
            RiskMeasure::VaR { confidence } | RiskMeasure::ExpectedShortfall { confidence } => {
                confidence.to_bits().hash(&mut h);
            }
        }
        h.finish()
    }
}

// ===================================================================
// Attributes
// ===================================================================

/// Instrument attributes used for factor matching.
///
/// A set of tags and key-value metadata used by matchers to classify
/// instruments into risk factors.
///
/// Parameters
/// ----------
/// tags : list[str], optional
///     Classification tags (e.g. ``["energy", "high_yield"]``).
/// meta : dict[str, str] or list[tuple[str, str]], optional
///     Key-value metadata (e.g. ``[("region", "NA"), ("rating", "CCC")]``).
#[pyclass(
    name = "Attributes",
    module = "finstack.core.factor_model",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAttributes {
    pub(crate) inner: Attributes,
}

#[pymethods]
impl PyAttributes {
    #[new]
    #[pyo3(signature = (tags=None, meta=None))]
    fn new(tags: Option<Vec<String>>, meta: Option<Vec<(String, String)>>) -> Self {
        let mut attrs = Attributes::new();
        if let Some(tag_list) = tags {
            attrs = attrs.with_tags(tag_list);
        }
        if let Some(meta_list) = meta {
            for (k, v) in meta_list {
                attrs = attrs.with_meta(k, v);
            }
        }
        Self { inner: attrs }
    }

    /// Tags present on this attribute set.
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.inner.tags.iter().cloned().collect()
    }

    /// Metadata key-value pairs.
    #[getter]
    fn meta(&self) -> Vec<(String, String)> {
        self.inner
            .meta
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check whether a tag is present.
    fn has_tag(&self, tag: &str) -> bool {
        self.inner.has_tag(tag)
    }

    /// Look up a metadata value by key.
    #[pyo3(name = "get_meta_value")]
    fn get_meta_value(&self, key: &str) -> Option<String> {
        self.inner.get_meta(key).map(|s| s.to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "Attributes(tags={:?}, meta={:?})",
            self.inner.tags.iter().collect::<Vec<_>>(),
            self.inner
                .meta
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>(),
        )
    }
}

// ===================================================================
// MappingTableMatcher
// ===================================================================

/// Flat lookup-table matcher where the first matching rule wins.
///
/// Parameters
/// ----------
/// rules : list[MappingRule]
///     Ordered matching rules. The first rule whose filters match is used.
#[pyclass(
    name = "MappingTableMatcher",
    module = "finstack.core.factor_model",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyMappingTableMatcher {
    inner: MappingTableMatcher,
}

#[pymethods]
impl PyMappingTableMatcher {
    #[new]
    fn new(rules: Vec<PyMappingRule>) -> Self {
        Self {
            inner: MappingTableMatcher::new(rules.into_iter().map(|r| r.inner.clone()).collect()),
        }
    }

    /// Match a dependency against this table's rules.
    ///
    /// Parameters
    /// ----------
    /// dependency : MarketDependency
    ///     The market dependency to match.
    /// attributes : Attributes
    ///     Instrument attributes for filtering.
    ///
    /// Returns
    /// -------
    /// str or None
    ///     The matched factor ID, or None if no rule matches.
    fn match_factor(
        &self,
        dependency: PyRef<'_, PyMarketDependency>,
        attributes: PyRef<'_, PyAttributes>,
    ) -> Option<String> {
        self.inner
            .match_factor(&dependency.inner, &attributes.inner)
            .map(|fid| fid.as_str().to_string())
    }
}

// ===================================================================
// HierarchicalMatcher
// ===================================================================

/// Tree-based matcher where the deepest matching factor assignment wins.
///
/// Parameters
/// ----------
/// root : FactorNode
///     Root of the classification tree.
/// dependency_filter : DependencyFilter, optional
///     Pre-filter on the dependency; if provided, dependencies that don't
///     pass this filter are immediately rejected.
#[pyclass(
    name = "HierarchicalMatcher",
    module = "finstack.core.factor_model",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub struct PyHierarchicalMatcher {
    inner: HierarchicalMatcher,
}

#[pymethods]
impl PyHierarchicalMatcher {
    #[new]
    #[pyo3(signature = (root, dependency_filter=None))]
    fn new(root: PyFactorNode, dependency_filter: Option<PyRef<'_, PyDependencyFilter>>) -> Self {
        let matcher = match dependency_filter {
            Some(df) => HierarchicalMatcher::new_scoped(df.inner.clone(), root.inner),
            None => HierarchicalMatcher::new(root.inner),
        };
        Self { inner: matcher }
    }

    /// Match a dependency against this tree.
    ///
    /// Parameters
    /// ----------
    /// dependency : MarketDependency
    ///     The market dependency to match.
    /// attributes : Attributes
    ///     Instrument attributes for tree traversal.
    ///
    /// Returns
    /// -------
    /// str or None
    ///     The deepest matching factor ID, or None.
    fn match_factor(
        &self,
        dependency: PyRef<'_, PyMarketDependency>,
        attributes: PyRef<'_, PyAttributes>,
    ) -> Option<String> {
        self.inner
            .match_factor(&dependency.inner, &attributes.inner)
            .map(|fid| fid.as_str().to_string())
    }
}

// ===================================================================
// CascadeMatcher
// ===================================================================

/// Priority-based fallback matcher that tries matchers in order.
///
/// Builds concrete matchers from :class:`MatchingConfig` items and
/// returns the first successful match.
///
/// Parameters
/// ----------
/// configs : list[MatchingConfig]
///     Ordered matcher configurations tried in priority order.
#[pyclass(name = "CascadeMatcher", module = "finstack.core.factor_model", frozen)]
pub struct PyCascadeMatcher {
    inner: CascadeMatcher,
}

#[pymethods]
impl PyCascadeMatcher {
    #[new]
    fn new(configs: Vec<PyMatchingConfig>) -> Self {
        let matchers: Vec<Box<dyn FactorMatcher>> = configs
            .iter()
            .map(|cfg| cfg.inner.build_matcher())
            .collect();
        Self {
            inner: CascadeMatcher::new(matchers),
        }
    }

    /// Match a dependency using the cascade chain.
    ///
    /// Parameters
    /// ----------
    /// dependency : MarketDependency
    ///     The market dependency to match.
    /// attributes : Attributes
    ///     Instrument attributes for filtering.
    ///
    /// Returns
    /// -------
    /// str or None
    ///     The first matched factor ID, or None if no matcher succeeds.
    fn match_factor(
        &self,
        dependency: PyRef<'_, PyMarketDependency>,
        attributes: PyRef<'_, PyAttributes>,
    ) -> Option<String> {
        self.inner
            .match_factor(&dependency.inner, &attributes.inner)
            .map(|fid| fid.as_str().to_string())
    }
}

// ===================================================================
// Module registration
// ===================================================================

/// Register the `finstack.core.factor_model` submodule.
pub(crate) fn register<'py>(py: Python<'py>, parent: &Bound<'py, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "factor_model")?;
    module.setattr(
        "__doc__",
        "Core factor-model vocabulary types: identifiers, enums, config, and data structs.",
    )?;

    // New standalone wrappers for enum / ID types
    module.add_class::<PyFactorId>()?;
    module.add_class::<PyPricingMode>()?;
    module.add_class::<PyFactorType>()?;
    module.add_class::<PyDependencyType>()?;
    module.add_class::<PyCurveType>()?;
    module.add_class::<PyUnmatchedPolicy>()?;
    module.add_class::<PyRiskMeasure>()?;

    // Re-register existing portfolio wrappers for core struct types
    module.add_class::<PyMarketDependency>()?;
    module.add_class::<PyBumpSizeConfig>()?;
    module.add_class::<PyMarketMapping>()?;
    module.add_class::<PyFactorDefinition>()?;
    module.add_class::<PyFactorCovarianceMatrix>()?;
    module.add_class::<PyFactorModelConfig>()?;
    module.add_class::<PyMatchingConfig>()?;
    module.add_class::<PyAttributeFilter>()?;
    module.add_class::<PyDependencyFilter>()?;
    module.add_class::<PyMappingRule>()?;
    module.add_class::<PyFactorNode>()?;
    module.add_class::<PyHierarchicalConfig>()?;

    // Attributes and concrete matchers
    module.add_class::<PyAttributes>()?;
    module.add_class::<PyMappingTableMatcher>()?;
    module.add_class::<PyHierarchicalMatcher>()?;
    module.add_class::<PyCascadeMatcher>()?;

    let exports = PyList::new(
        py,
        [
            "FactorId",
            "PricingMode",
            "FactorType",
            "DependencyType",
            "CurveType",
            "UnmatchedPolicy",
            "RiskMeasure",
            "MarketDependency",
            "BumpSizeConfig",
            "MarketMapping",
            "FactorDefinition",
            "FactorCovarianceMatrix",
            "FactorModelConfig",
            "MatchingConfig",
            "AttributeFilter",
            "DependencyFilter",
            "MappingRule",
            "FactorNode",
            "HierarchicalConfig",
            "Attributes",
            "MappingTableMatcher",
            "HierarchicalMatcher",
            "CascadeMatcher",
        ],
    )?;
    module.setattr("__all__", exports)?;
    parent.add_submodule(&module)?;
    Ok(())
}
