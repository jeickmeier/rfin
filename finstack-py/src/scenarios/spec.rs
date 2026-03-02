//! Scenario specification bindings.

use crate::core::currency::PyCurrency;
use crate::scenarios::enums::{PyCurveKind, PyTenorMatchMode, PyVolSurfaceKind};
use crate::valuations::common::PyInstrumentType;
use finstack_core::HashMap;
use finstack_scenarios::{Compounding, OperationSpec, RateBindingSpec, ScenarioSpec, TimeRollMode};
use indexmap::IndexMap;
use pyo3::basic::CompareOp;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};

/// Compounding convention for rate conversions.
#[pyclass(
    module = "finstack.scenarios",
    name = "Compounding",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCompounding {
    pub(crate) inner: Compounding,
}

impl PyCompounding {
    pub(crate) fn new(inner: Compounding) -> Self {
        Self { inner }
    }
}

/// Roll interpretation mode for time roll-forward operations.
#[pyclass(
    module = "finstack.scenarios",
    name = "TimeRollMode",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyTimeRollMode {
    pub(crate) inner: TimeRollMode,
}

impl PyTimeRollMode {
    pub(crate) fn new(inner: TimeRollMode) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyTimeRollMode {
    #[classattr]
    #[allow(non_snake_case)]
    fn BusinessDays() -> Self {
        Self::new(TimeRollMode::BusinessDays)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn CalendarDays() -> Self {
        Self::new(TimeRollMode::CalendarDays)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Approximate() -> Self {
        Self::new(TimeRollMode::Approximate)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            TimeRollMode::BusinessDays => "TimeRollMode.BusinessDays".to_string(),
            TimeRollMode::CalendarDays => "TimeRollMode.CalendarDays".to_string(),
            TimeRollMode::Approximate => "TimeRollMode.Approximate".to_string(),
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            TimeRollMode::BusinessDays => "BusinessDays",
            TimeRollMode::CalendarDays => "CalendarDays",
            TimeRollMode::Approximate => "Approximate",
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner as u64
    }
}

#[pymethods]
impl PyCompounding {
    #[classattr]
    #[allow(non_snake_case)]
    fn Simple() -> Self {
        Self::new(Compounding::Simple)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Continuous() -> Self {
        Self::new(Compounding::Continuous)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Annual() -> Self {
        Self::new(Compounding::Annual)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn SemiAnnual() -> Self {
        Self::new(Compounding::SemiAnnual)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Quarterly() -> Self {
        Self::new(Compounding::Quarterly)
    }

    #[classattr]
    #[allow(non_snake_case)]
    fn Monthly() -> Self {
        Self::new(Compounding::Monthly)
    }

    fn __repr__(&self) -> String {
        match self.inner {
            Compounding::Simple => "Compounding.Simple".to_string(),
            Compounding::Continuous => "Compounding.Continuous".to_string(),
            Compounding::Annual => "Compounding.Annual".to_string(),
            Compounding::SemiAnnual => "Compounding.SemiAnnual".to_string(),
            Compounding::Quarterly => "Compounding.Quarterly".to_string(),
            Compounding::Monthly => "Compounding.Monthly".to_string(),
        }
    }

    fn __str__(&self) -> &'static str {
        match self.inner {
            Compounding::Simple => "Simple",
            Compounding::Continuous => "Continuous",
            Compounding::Annual => "Annual",
            Compounding::SemiAnnual => "SemiAnnual",
            Compounding::Quarterly => "Quarterly",
            Compounding::Monthly => "Monthly",
        }
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp) -> bool {
        match op {
            CompareOp::Eq => self.inner == other.inner,
            CompareOp::Ne => self.inner != other.inner,
            _ => false,
        }
    }

    fn __hash__(&self) -> u64 {
        self.inner as u64
    }
}

/// Configuration for rate binding between curves and statement nodes.
#[pyclass(
    module = "finstack.scenarios",
    name = "RateBindingSpec",
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRateBindingSpec {
    pub(crate) inner: RateBindingSpec,
}

impl PyRateBindingSpec {
    pub(crate) fn from_inner(inner: RateBindingSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRateBindingSpec {
    #[new]
    #[pyo3(signature = (node_id, curve_id, tenor, compounding=None, day_count=None))]
    /// Create a new rate binding specification.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Statement node ID to receive the rate.
    /// curve_id : str
    ///     Curve ID to extract rate from.
    /// tenor : str
    ///     Tenor for rate extraction (e.g., "3M", "1Y").
    /// compounding : Compounding, optional
    ///     Compounding convention (default: Continuous).
    /// day_count : str, optional
    ///     Optional day count override.
    fn new(
        node_id: String,
        curve_id: String,
        tenor: String,
        compounding: Option<&PyCompounding>,
        day_count: Option<String>,
    ) -> Self {
        Self::from_inner(RateBindingSpec {
            node_id,
            curve_id,
            tenor,
            compounding: compounding.map(|c| c.inner).unwrap_or_default(),
            day_count,
        })
    }

    #[getter]
    /// Statement node ID to receive the rate.
    fn node_id(&self) -> String {
        self.inner.node_id.clone()
    }

    #[getter]
    /// Curve ID to extract rate from.
    fn curve_id(&self) -> String {
        self.inner.curve_id.clone()
    }

    #[getter]
    /// Tenor string for rate extraction.
    fn tenor(&self) -> String {
        self.inner.tenor.clone()
    }

    #[getter]
    /// Compounding convention for rate conversion.
    fn compounding(&self) -> PyCompounding {
        PyCompounding::new(self.inner.compounding)
    }

    #[getter]
    /// Optional day count convention override.
    fn day_count(&self) -> Option<String> {
        self.inner.day_count.clone()
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to JSON-compatible dictionary.
    ///
    /// Returns
    /// -------
    /// dict
    ///     JSON-serializable dictionary.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Create from JSON-compatible dictionary.
    ///
    /// Parameters
    /// ----------
    /// data : dict
    ///     JSON-serializable dictionary.
    ///
    /// Returns
    /// -------
    /// RateBindingSpec
    ///     Rate binding specification.
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {}", e)))?;
        let inner: RateBindingSpec = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {}", e)))?;
        Ok(Self::from_inner(inner))
    }

    fn __repr__(&self) -> String {
        format!(
            "RateBindingSpec(node_id='{}', curve_id='{}', tenor='{}')",
            self.inner.node_id, self.inner.curve_id, self.inner.tenor
        )
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, node_id, curve_id)")]
    /// Build a binding from a legacy `(node_id, curve_id)` mapping.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Statement node ID to receive the rate.
    /// curve_id : str
    ///     Curve ID to extract rate from.
    ///
    /// Returns
    /// -------
    /// RateBindingSpec
    ///     Binding with 1Y tenor, continuous compounding, and no day-count override.
    fn from_legacy(_cls: &Bound<'_, PyType>, node_id: String, curve_id: String) -> Self {
        Self::from_inner(RateBindingSpec::from_legacy(node_id, curve_id))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, legacy)")]
    /// Convert a legacy ``dict[str, str]`` mapping to detailed binding specs.
    ///
    /// Each entry ``{node_id: curve_id}`` is converted to a
    /// :class:`RateBindingSpec` with 1Y tenor, continuous compounding,
    /// and no day-count override.
    ///
    /// Parameters
    /// ----------
    /// legacy : dict[str, str]
    ///     Mapping of node IDs to curve IDs.
    ///
    /// Returns
    /// -------
    /// dict[str, RateBindingSpec]
    ///     Detailed binding specs keyed by node ID.
    fn map_from_legacy(
        _cls: &Bound<'_, PyType>,
        legacy: std::collections::HashMap<String, String>,
    ) -> std::collections::HashMap<String, PyRateBindingSpec> {
        let index_map: IndexMap<String, String> = legacy.into_iter().collect();
        RateBindingSpec::map_from_legacy(index_map)
            .into_iter()
            .map(|(k, v)| (k, PyRateBindingSpec::from_inner(v)))
            .collect()
    }
}

/// Individual operation within a scenario.
///
/// Use class methods to construct specific operation types.
///
/// Examples
/// --------
/// >>> from finstack.scenarios import OperationSpec, CurveKind
/// >>> op = OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0)
#[pyclass(module = "finstack.scenarios", name = "OperationSpec", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyOperationSpec {
    pub(crate) inner: OperationSpec,
}

impl PyOperationSpec {
    pub fn new(inner: OperationSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyOperationSpec {
    #[classmethod]
    #[pyo3(text_signature = "(cls, base, quote, pct)")]
    /// FX rate percent shift.
    ///
    /// Parameters
    /// ----------
    /// base : Currency
    ///     Base currency.
    /// quote : Currency
    ///     Quote currency.
    /// pct : float
    ///     Percentage change (positive strengthens base).
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn market_fx_pct(
        _cls: &Bound<'_, PyType>,
        base: &PyCurrency,
        quote: &PyCurrency,
        pct: f64,
    ) -> Self {
        Self::new(OperationSpec::MarketFxPct {
            base: base.inner,
            quote: quote.inner,
            pct,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, ids, pct)")]
    /// Equity price percent shock.
    ///
    /// Parameters
    /// ----------
    /// ids : list[str]
    ///     List of equity identifiers.
    /// pct : float
    ///     Percentage change to apply.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn equity_price_pct(_cls: &Bound<'_, PyType>, ids: Vec<String>, pct: f64) -> Self {
        Self::new(OperationSpec::EquityPricePct { ids, pct })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, attrs, pct)")]
    /// Instrument price shock by exact attribute match.
    ///
    /// Parameters
    /// ----------
    /// attrs : dict[str, str]
    ///     Attribute filters (e.g., {"sector": "Energy", "rating": "BBB"}).
    /// pct : float
    ///     Percentage change to apply.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn instrument_price_pct_by_attr(
        _cls: &Bound<'_, PyType>,
        attrs: HashMap<String, String>,
        pct: f64,
    ) -> Self {
        let index_attrs: IndexMap<String, String> = attrs.into_iter().collect();
        Self::new(OperationSpec::InstrumentPricePctByAttr {
            attrs: index_attrs,
            pct,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, curve_kind, curve_id, bp)")]
    /// Parallel shift to a curve (additive in basis points).
    ///
    /// Parameters
    /// ----------
    /// curve_kind : CurveKind
    ///     Type of curve to shock.
    /// curve_id : str
    ///     Curve identifier.
    /// bp : float
    ///     Basis points to add.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn curve_parallel_bp(
        _cls: &Bound<'_, PyType>,
        curve_kind: &PyCurveKind,
        curve_id: String,
        bp: f64,
    ) -> Self {
        Self::new(OperationSpec::CurveParallelBp {
            curve_kind: curve_kind.inner,
            curve_id,
            bp,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, curve_kind, curve_id, nodes, match_mode=None)")]
    /// Node-specific basis point shifts for curve shaping.
    ///
    /// Parameters
    /// ----------
    /// curve_kind : CurveKind
    ///     Type of curve to shock.
    /// curve_id : str
    ///     Curve identifier.
    /// nodes : list[tuple[str, float]]
    ///     List of (tenor, bp) pairs (e.g., [("2Y", 25.0), ("10Y", -10.0)]).
    /// match_mode : TenorMatchMode, optional
    ///     Tenor matching strategy (default: Interpolate).
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn curve_node_bp(
        _cls: &Bound<'_, PyType>,
        curve_kind: &PyCurveKind,
        curve_id: String,
        nodes: Vec<(String, f64)>,
        match_mode: Option<&PyTenorMatchMode>,
    ) -> Self {
        Self::new(OperationSpec::CurveNodeBp {
            curve_kind: curve_kind.inner,
            curve_id,
            nodes,
            match_mode: match_mode
                .map(|m| m.inner)
                .unwrap_or(finstack_scenarios::TenorMatchMode::Interpolate),
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, surface_id, points)")]
    /// Parallel shift to base correlation surface (absolute points).
    ///
    /// Parameters
    /// ----------
    /// surface_id : str
    ///     Surface identifier.
    /// points : float
    ///     Correlation points to add (e.g., 0.05 for +5 percentage points).
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn basecorr_parallel_pts(_cls: &Bound<'_, PyType>, surface_id: String, points: f64) -> Self {
        Self::new(OperationSpec::BaseCorrParallelPts { surface_id, points })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, surface_id, points, detachment_bps=None, maturities=None)")]
    /// Bucket-specific base correlation shifts.
    ///
    /// Parameters
    /// ----------
    /// surface_id : str
    ///     Surface identifier.
    /// points : float
    ///     Correlation points to add.
    /// detachment_bps : list[int], optional
    ///     Detachment points in basis points (e.g., [300, 700] for 3% and 7%).
    /// maturities : list[str], optional
    ///     Maturity filters.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn basecorr_bucket_pts(
        _cls: &Bound<'_, PyType>,
        surface_id: String,
        points: f64,
        detachment_bps: Option<Vec<i32>>,
        maturities: Option<Vec<String>>,
    ) -> Self {
        Self::new(OperationSpec::BaseCorrBucketPts {
            surface_id,
            detachment_bps,
            maturities,
            points,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, surface_kind, surface_id, pct)")]
    /// Parallel percent shift to volatility surface.
    ///
    /// Parameters
    /// ----------
    /// surface_kind : VolSurfaceKind
    ///     Type of volatility surface.
    /// surface_id : str
    ///     Surface identifier.
    /// pct : float
    ///     Percentage change to apply.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn vol_surface_parallel_pct(
        _cls: &Bound<'_, PyType>,
        surface_kind: &PyVolSurfaceKind,
        surface_id: String,
        pct: f64,
    ) -> Self {
        Self::new(OperationSpec::VolSurfaceParallelPct {
            surface_kind: surface_kind.inner,
            surface_id,
            pct,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, surface_kind, surface_id, pct, tenors=None, strikes=None)")]
    /// Bucketed volatility surface shock.
    ///
    /// Parameters
    /// ----------
    /// surface_kind : VolSurfaceKind
    ///     Type of volatility surface.
    /// surface_id : str
    ///     Surface identifier.
    /// pct : float
    ///     Percentage change to apply.
    /// tenors : list[str], optional
    ///     Tenor filters (e.g., ["1M", "3M"]).
    /// strikes : list[float], optional
    ///     Strike filters.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn vol_surface_bucket_pct(
        _cls: &Bound<'_, PyType>,
        surface_kind: &PyVolSurfaceKind,
        surface_id: String,
        pct: f64,
        tenors: Option<Vec<String>>,
        strikes: Option<Vec<f64>>,
    ) -> Self {
        Self::new(OperationSpec::VolSurfaceBucketPct {
            surface_kind: surface_kind.inner,
            surface_id,
            tenors,
            strikes,
            pct,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, node_id, pct)")]
    /// Statement forecast percent change.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// pct : float
    ///     Percentage change to apply.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn stmt_forecast_percent(_cls: &Bound<'_, PyType>, node_id: String, pct: f64) -> Self {
        Self::new(OperationSpec::StmtForecastPercent { node_id, pct })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, node_id, value)")]
    /// Statement forecast value assignment.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// value : float
    ///     Value to assign.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn stmt_forecast_assign(_cls: &Bound<'_, PyType>, node_id: String, value: f64) -> Self {
        Self::new(OperationSpec::StmtForecastAssign { node_id, value })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, attrs, bp)")]
    /// Instrument spread shock by exact attribute match.
    ///
    /// Parameters
    /// ----------
    /// attrs : dict[str, str]
    ///     Attribute filters.
    /// bp : float
    ///     Basis points to add.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn instrument_spread_bp_by_attr(
        _cls: &Bound<'_, PyType>,
        attrs: HashMap<String, String>,
        bp: f64,
    ) -> Self {
        let index_attrs: IndexMap<String, String> = attrs.into_iter().collect();
        Self::new(OperationSpec::InstrumentSpreadBpByAttr {
            attrs: index_attrs,
            bp,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_types, pct)")]
    /// Instrument price shock by type.
    ///
    /// Parameters
    /// ----------
    /// instrument_types : list[InstrumentType]
    ///     List of instrument types to shock.
    /// pct : float
    ///     Percentage change to apply.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn instrument_price_pct_by_type(
        _cls: &Bound<'_, PyType>,
        instrument_types: Vec<PyInstrumentType>,
        pct: f64,
    ) -> Self {
        let types = instrument_types.iter().map(|t| t.inner).collect();
        Self::new(OperationSpec::InstrumentPricePctByType {
            instrument_types: types,
            pct,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_types, bp)")]
    /// Instrument spread shock by type.
    ///
    /// Parameters
    /// ----------
    /// instrument_types : list[InstrumentType]
    ///     List of instrument types to shock.
    /// bp : float
    ///     Basis points to add.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn instrument_spread_bp_by_type(
        _cls: &Bound<'_, PyType>,
        instrument_types: Vec<PyInstrumentType>,
        bp: f64,
    ) -> Self {
        let types = instrument_types.iter().map(|t| t.inner).collect();
        Self::new(OperationSpec::InstrumentSpreadBpByType {
            instrument_types: types,
            bp,
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, delta_pts)")]
    /// Shock asset correlation for structured credit instruments.
    ///
    /// Parameters
    /// ----------
    /// delta_pts : float
    ///     Additive shock in correlation points (e.g., 0.05 for +5%).
    fn asset_correlation_pts(_cls: &Bound<'_, PyType>, delta_pts: f64) -> Self {
        Self::new(OperationSpec::AssetCorrelationPts { delta_pts })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, delta_pts)")]
    /// Shock prepay-default correlation for structured credit instruments.
    ///
    /// Parameters
    /// ----------
    /// delta_pts : float
    ///     Additive shock in correlation points.
    fn prepay_default_correlation_pts(_cls: &Bound<'_, PyType>, delta_pts: f64) -> Self {
        Self::new(OperationSpec::PrepayDefaultCorrelationPts { delta_pts })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, delta_pts)")]
    /// Shock recovery-default correlation for structured credit instruments.
    ///
    /// Parameters
    /// ----------
    /// delta_pts : float
    ///     Additive shock in correlation points.
    fn recovery_correlation_pts(_cls: &Bound<'_, PyType>, delta_pts: f64) -> Self {
        Self::new(OperationSpec::RecoveryCorrelationPts { delta_pts })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, delta_pts)")]
    /// Shock prepayment factor loading (systematic factor sensitivity).
    ///
    /// Parameters
    /// ----------
    /// delta_pts : float
    ///     Additive shock to factor loading.
    fn prepay_factor_loading_pts(_cls: &Bound<'_, PyType>, delta_pts: f64) -> Self {
        Self::new(OperationSpec::PrepayFactorLoadingPts { delta_pts })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, period, apply_shocks=True, roll_mode=None)")]
    /// Roll forward horizon by a period with carry/theta.
    ///
    /// Parameters
    /// ----------
    /// period : str
    ///     Period to roll forward (e.g., "1D", "1W", "1M", "1Y").
    /// apply_shocks : bool, optional
    ///     Whether to apply market shocks after rolling (default: True).
    /// roll_mode : TimeRollMode, optional
    ///     Roll interpretation (defaults to business-day aware).
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn time_roll_forward(
        _cls: &Bound<'_, PyType>,
        period: String,
        apply_shocks: Option<bool>,
        roll_mode: Option<&PyTimeRollMode>,
    ) -> Self {
        Self::new(OperationSpec::TimeRollForward {
            period,
            apply_shocks: apply_shocks.unwrap_or(true),
            roll_mode: roll_mode.map_or_else(TimeRollMode::default, |m| m.inner),
        })
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to JSON-compatible dictionary.
    ///
    /// Returns
    /// -------
    /// dict
    ///     JSON-serializable dictionary.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Create from JSON-compatible dictionary.
    ///
    /// Parameters
    /// ----------
    /// data : dict
    ///     JSON-serializable dictionary.
    ///
    /// Returns
    /// -------
    /// OperationSpec
    ///     Operation specification.
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {}", e)))?;
        let inner: OperationSpec = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {}", e)))?;
        Ok(Self::new(inner))
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.inner)
    }

    #[pyo3(text_signature = "(self)")]
    /// Validate the operation specification.
    ///
    /// Checks for finite numeric values, non-empty identifiers, and
    /// logical consistency (e.g. base != quote for FX operations).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the operation is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(crate::scenarios::error::scenario_to_py)
    }
}

/// A complete scenario specification with metadata and ordered operations.
///
/// Parameters
/// ----------
/// id : str
///     Stable identifier used for persistence and reporting.
/// operations : list[OperationSpec]
///     Ordered list of operations to execute.
/// name : str, optional
///     Optional display name for UI or logs.
/// description : str, optional
///     Optional text describing the intent of the scenario.
/// priority : int, optional
///     Used by compose() to determine merge ordering (default: 0, lower = higher priority).
///
/// Examples
/// --------
/// >>> from finstack.scenarios import ScenarioSpec, OperationSpec, CurveKind
/// >>> ops = [OperationSpec.curve_parallel_bp(CurveKind.Discount, "USD_SOFR", 50.0)]
/// >>> scenario = ScenarioSpec("stress_test", ops, name="Q1 Stress Test")
#[pyclass(module = "finstack.scenarios", name = "ScenarioSpec", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyScenarioSpec {
    pub(crate) inner: ScenarioSpec,
}

impl PyScenarioSpec {
    pub fn from_inner(inner: ScenarioSpec) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyScenarioSpec {
    #[new]
    #[pyo3(signature = (id, operations, name=None, description=None, priority=0))]
    /// Create a new scenario specification.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Scenario identifier.
    /// operations : list[OperationSpec]
    ///     List of operations to apply.
    /// name : str, optional
    ///     Display name.
    /// description : str, optional
    ///     Description text.
    /// priority : int, optional
    ///     Priority for composition (default: 0).
    ///
    /// Returns
    /// -------
    /// ScenarioSpec
    ///     Scenario specification.
    fn new(
        id: String,
        operations: Vec<PyOperationSpec>,
        name: Option<String>,
        description: Option<String>,
        priority: Option<i32>,
    ) -> Self {
        let ops = operations.iter().map(|op| op.inner.clone()).collect();
        Self {
            inner: ScenarioSpec {
                id,
                name,
                description,
                operations: ops,
                priority: priority.unwrap_or(0),
            },
        }
    }

    #[getter]
    /// Scenario identifier.
    ///
    /// Returns
    /// -------
    /// str
    ///     Scenario ID.
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    /// Display name.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Name if set.
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    #[getter]
    /// Description text.
    ///
    /// Returns
    /// -------
    /// str | None
    ///     Description if set.
    fn description(&self) -> Option<String> {
        self.inner.description.clone()
    }

    #[getter]
    /// List of operations.
    ///
    /// Returns
    /// -------
    /// list[OperationSpec]
    ///     Operations to apply.
    fn operations(&self) -> Vec<PyOperationSpec> {
        self.inner
            .operations
            .iter()
            .map(|op| PyOperationSpec::new(op.clone()))
            .collect()
    }

    #[getter]
    /// Priority for composition.
    ///
    /// Returns
    /// -------
    /// int
    ///     Priority value (lower = higher priority).
    fn priority(&self) -> i32 {
        self.inner.priority
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to JSON-compatible dictionary.
    ///
    /// Returns
    /// -------
    /// dict
    ///     JSON-serializable dictionary.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json_str = serde_json::to_string(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))?;
        let json_value: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to parse JSON: {}", e)))?;
        pythonize::pythonize(py, &json_value)
            .map(|bound| bound.into())
            .map_err(|e| PyValueError::new_err(format!("Failed to convert to Python: {}", e)))
    }

    #[pyo3(text_signature = "(self)")]
    /// Convert to JSON string.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner)
            .map_err(|e| PyValueError::new_err(format!("Failed to serialize: {}", e)))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    /// Create from JSON-compatible dictionary.
    ///
    /// Parameters
    /// ----------
    /// data : dict
    ///     JSON-serializable dictionary.
    ///
    /// Returns
    /// -------
    /// ScenarioSpec
    ///     Scenario specification.
    fn from_dict(_cls: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let json_value: serde_json::Value = pythonize::depythonize(data)
            .map_err(|e| PyValueError::new_err(format!("Failed to convert from Python: {}", e)))?;
        let inner: ScenarioSpec = serde_json::from_value(json_value)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {}", e)))?;
        Ok(Self::from_inner(inner))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    /// Create from JSON string.
    ///
    /// Parameters
    /// ----------
    /// json_str : str
    ///     JSON string.
    ///
    /// Returns
    /// -------
    /// ScenarioSpec
    ///     Scenario specification.
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: ScenarioSpec = serde_json::from_str(json_str)
            .map_err(|e| PyValueError::new_err(format!("Failed to deserialize: {}", e)))?;
        Ok(Self::from_inner(inner))
    }

    fn __repr__(&self) -> String {
        format!(
            "ScenarioSpec(id='{}', operations={}, priority={})",
            self.inner.id,
            self.inner.operations.len(),
            self.inner.priority
        )
    }

    #[pyo3(text_signature = "(self)")]
    /// Validate the scenario specification for consistency.
    ///
    /// Checks for non-empty ID, valid operations, and at most one
    /// TimeRollForward operation.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the specification is invalid.
    fn validate(&self) -> PyResult<()> {
        self.inner
            .validate()
            .map_err(crate::scenarios::error::scenario_to_py)
    }
}

/// Register spec types with the scenarios module.
pub(crate) fn register(
    _py: Python<'_>,
    module: &Bound<'_, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyCompounding>()?;
    module.add_class::<PyTimeRollMode>()?;
    module.add_class::<PyRateBindingSpec>()?;
    module.add_class::<PyOperationSpec>()?;
    module.add_class::<PyScenarioSpec>()?;

    Ok(vec![
        "Compounding",
        "TimeRollMode",
        "RateBindingSpec",
        "OperationSpec",
        "ScenarioSpec",
    ])
}
