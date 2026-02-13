//! Python bindings for Market Conventions Registry.
//!
//! Provides access to market-standard conventions for rate indices, CDS, swaptions,
//! inflation swaps, options, and IR futures.

use crate::core::currency::PyCurrency;
use crate::core::dates::daycount::PyDayCount;
use crate::core::dates::tenor::PyTenor;
use crate::core::dates::PyBusinessDayConvention;
use crate::errors::core_to_py;
use finstack_core::types::IndexId;
use finstack_valuations::market::conventions::ids::{
    CdsConventionKey, CdsDocClause, InflationSwapConventionId, IrFutureContractId,
    OptionConventionId, SwaptionConventionId,
};
use finstack_valuations::market::conventions::{
    CdsConventions, ConventionRegistry, InflationSwapConventions, IrFutureConventions,
    OptionConventions, RateIndexConventions, RateIndexKind, SwaptionConventions,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule, PyType};
use pyo3::Bound;

// =============================================================================
// CDS Doc Clause Enum
// =============================================================================

/// CDS documentation clause (ISDA standard).
///
/// Represents the restructuring clause for CDS contracts.
///
/// Attributes:
///     CR14: Cum-Restructuring 2014
///     MR14: Modified-Restructuring 2014
///     MM14: Modified-Modified-Restructuring 2014
///     XR14: No-Restructuring 2014
///     ISDA_NA: ISDA North American Corporate
///     ISDA_EU: ISDA European Corporate
///     ISDA_AS: ISDA Asia Corporate
///     ISDA_AU: ISDA Australia Corporate
///     ISDA_NZ: ISDA New Zealand Corporate
///     CUSTOM: Custom / Other
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "CdsDocClause",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyCdsDocClause {
    pub(crate) inner: CdsDocClause,
}

impl PyCdsDocClause {
    pub(crate) const fn new(inner: CdsDocClause) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            CdsDocClause::Cr14 => "CR14",
            CdsDocClause::Mr14 => "MR14",
            CdsDocClause::Mm14 => "MM14",
            CdsDocClause::Xr14 => "XR14",
            CdsDocClause::IsdaNa => "ISDA_NA",
            CdsDocClause::IsdaEu => "ISDA_EU",
            CdsDocClause::IsdaAs => "ISDA_AS",
            CdsDocClause::IsdaAu => "ISDA_AU",
            CdsDocClause::IsdaNz => "ISDA_NZ",
            CdsDocClause::Custom => "CUSTOM",
        }
    }
}

#[pymethods]
impl PyCdsDocClause {
    #[classattr]
    const CR14: Self = Self::new(CdsDocClause::Cr14);
    #[classattr]
    const MR14: Self = Self::new(CdsDocClause::Mr14);
    #[classattr]
    const MM14: Self = Self::new(CdsDocClause::Mm14);
    #[classattr]
    const XR14: Self = Self::new(CdsDocClause::Xr14);
    #[classattr]
    const ISDA_NA: Self = Self::new(CdsDocClause::IsdaNa);
    #[classattr]
    const ISDA_EU: Self = Self::new(CdsDocClause::IsdaEu);
    #[classattr]
    const ISDA_AS: Self = Self::new(CdsDocClause::IsdaAs);
    #[classattr]
    const ISDA_AU: Self = Self::new(CdsDocClause::IsdaAu);
    #[classattr]
    const ISDA_NZ: Self = Self::new(CdsDocClause::IsdaNz);
    #[classattr]
    const CUSTOM: Self = Self::new(CdsDocClause::Custom);

    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    /// Parse a CDS doc clause from string.
    fn from_name(_cls: &Bound<'_, PyType>, name: &str) -> PyResult<Self> {
        name.parse::<CdsDocClause>()
            .map(Self::new)
            .map_err(PyValueError::new_err)
    }

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("CdsDocClause.{}", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

// =============================================================================
// CDS Convention Key
// =============================================================================

/// Key for looking up CDS conventions (currency + doc clause).
///
/// Args:
///     currency: Currency code (e.g., "USD").
///     doc_clause: CDS documentation clause.
///
/// Examples:
///     >>> key = CdsConventionKey(currency="USD", doc_clause=CdsDocClause.ISDA_NA)
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "CdsConventionKey",
    frozen
)]
#[derive(Clone, Debug)]
pub struct PyCdsConventionKey {
    pub(crate) inner: CdsConventionKey,
}

#[pymethods]
impl PyCdsConventionKey {
    #[new]
    fn new(currency: &str, doc_clause: &PyCdsDocClause) -> PyResult<Self> {
        let currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("invalid currency: {}", e)))?;
        Ok(Self {
            inner: CdsConventionKey {
                currency,
                doc_clause: doc_clause.inner,
            },
        })
    }

    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn doc_clause(&self) -> PyCdsDocClause {
        PyCdsDocClause::new(self.inner.doc_clause)
    }

    fn __repr__(&self) -> String {
        format!(
            "CdsConventionKey(currency='{}', doc_clause={:?})",
            self.inner.currency, self.inner.doc_clause
        )
    }
}

// =============================================================================
// Rate Index Kind Enum
// =============================================================================

/// Type of rate index.
///
/// Attributes:
///     OVERNIGHT_RFR: Overnight Risk-Free Rate (e.g., SOFR, SONIA)
///     TERM: Term index (e.g., 3M LIBOR, 6M EURIBOR)
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "RateIndexKind",
    frozen
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRateIndexKind {
    pub(crate) inner: RateIndexKind,
}

impl PyRateIndexKind {
    pub(crate) const fn new(inner: RateIndexKind) -> Self {
        Self { inner }
    }

    fn label(&self) -> &'static str {
        match self.inner {
            RateIndexKind::OvernightRfr => "OVERNIGHT_RFR",
            RateIndexKind::Term => "TERM",
        }
    }
}

#[pymethods]
impl PyRateIndexKind {
    #[classattr]
    const OVERNIGHT_RFR: Self = Self::new(RateIndexKind::OvernightRfr);
    #[classattr]
    const TERM: Self = Self::new(RateIndexKind::Term);

    #[getter]
    fn name(&self) -> &'static str {
        self.label()
    }

    fn __repr__(&self) -> String {
        format!("RateIndexKind.{}", self.label())
    }

    fn __str__(&self) -> &'static str {
        self.label()
    }
}

// =============================================================================
// Convention Types (Read-only wrappers)
// =============================================================================

/// Conventions for rate index instruments.
///
/// Contains day count, payment frequency, reset lag, and other market-standard
/// parameters for instruments referencing a rate index.
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "RateIndexConventions",
    frozen
)]
#[derive(Clone)]
pub struct PyRateIndexConventions {
    inner: RateIndexConventions,
}

#[pymethods]
impl PyRateIndexConventions {
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::new(self.inner.currency)
    }

    #[getter]
    fn kind(&self) -> PyRateIndexKind {
        PyRateIndexKind::new(self.inner.kind)
    }

    #[getter]
    fn tenor(&self) -> Option<PyTenor> {
        self.inner.tenor.map(PyTenor::new)
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn default_payment_frequency(&self) -> PyTenor {
        PyTenor::new(self.inner.default_payment_frequency)
    }

    #[getter]
    fn default_payment_delay_days(&self) -> i32 {
        self.inner.default_payment_delay_days
    }

    #[getter]
    fn default_reset_lag_days(&self) -> i32 {
        self.inner.default_reset_lag_days
    }

    #[getter]
    fn market_calendar_id(&self) -> &str {
        &self.inner.market_calendar_id
    }

    #[getter]
    fn market_settlement_days(&self) -> i32 {
        self.inner.market_settlement_days
    }

    #[getter]
    fn market_business_day_convention(&self) -> PyBusinessDayConvention {
        PyBusinessDayConvention::new(self.inner.market_business_day_convention)
    }

    #[getter]
    fn default_fixed_leg_day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.default_fixed_leg_day_count)
    }

    #[getter]
    fn default_fixed_leg_frequency(&self) -> PyTenor {
        PyTenor::new(self.inner.default_fixed_leg_frequency)
    }

    fn __repr__(&self) -> String {
        format!(
            "RateIndexConventions(currency={}, kind={:?}, day_count={:?})",
            self.inner.currency, self.inner.kind, self.inner.day_count
        )
    }
}

/// Conventions for Credit Default Swaps.
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "CdsConventions",
    frozen
)]
#[derive(Clone)]
pub struct PyCdsConventions {
    inner: CdsConventions,
}

#[pymethods]
impl PyCdsConventions {
    #[getter]
    fn calendar_id(&self) -> &str {
        &self.inner.calendar_id
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn business_day_convention(&self) -> PyBusinessDayConvention {
        PyBusinessDayConvention::new(self.inner.bdc)
    }

    #[getter]
    fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    #[getter]
    fn payment_frequency(&self) -> PyTenor {
        PyTenor::new(self.inner.frequency)
    }

    fn __repr__(&self) -> String {
        format!(
            "CdsConventions(calendar='{}', payment_frequency={})",
            self.inner.calendar_id, self.inner.frequency
        )
    }
}

/// Conventions for Swaptions.
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "SwaptionConventions",
    frozen
)]
#[derive(Clone)]
pub struct PySwaptionConventions {
    inner: SwaptionConventions,
}

#[pymethods]
impl PySwaptionConventions {
    #[getter]
    fn calendar_id(&self) -> &str {
        &self.inner.calendar_id
    }

    #[getter]
    fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    #[getter]
    fn business_day_convention(&self) -> PyBusinessDayConvention {
        PyBusinessDayConvention::new(self.inner.business_day_convention)
    }

    #[getter]
    fn fixed_leg_frequency(&self) -> PyTenor {
        PyTenor::new(self.inner.fixed_leg_frequency)
    }

    #[getter]
    fn fixed_leg_day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.fixed_leg_day_count)
    }

    #[getter]
    fn float_leg_index(&self) -> &str {
        &self.inner.float_leg_index
    }

    fn __repr__(&self) -> String {
        format!(
            "SwaptionConventions(calendar='{}', fixed_leg_frequency={})",
            self.inner.calendar_id, self.inner.fixed_leg_frequency
        )
    }
}

/// Conventions for Inflation Swaps.
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "InflationSwapConventions",
    frozen
)]
#[derive(Clone)]
pub struct PyInflationSwapConventions {
    inner: InflationSwapConventions,
}

#[pymethods]
impl PyInflationSwapConventions {
    #[getter]
    fn calendar_id(&self) -> &str {
        &self.inner.calendar_id
    }

    #[getter]
    fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    #[getter]
    fn business_day_convention(&self) -> PyBusinessDayConvention {
        PyBusinessDayConvention::new(self.inner.business_day_convention)
    }

    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::new(self.inner.day_count)
    }

    #[getter]
    fn inflation_lag(&self) -> PyTenor {
        PyTenor::new(self.inner.inflation_lag)
    }

    fn __repr__(&self) -> String {
        format!(
            "InflationSwapConventions(calendar='{}', inflation_lag={})",
            self.inner.calendar_id, self.inner.inflation_lag
        )
    }
}

/// Conventions for Options (Equity/FX/Commodity).
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "OptionConventions",
    frozen
)]
#[derive(Clone)]
pub struct PyOptionConventions {
    inner: OptionConventions,
}

#[pymethods]
impl PyOptionConventions {
    #[getter]
    fn calendar_id(&self) -> &str {
        &self.inner.calendar_id
    }

    #[getter]
    fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    #[getter]
    fn business_day_convention(&self) -> PyBusinessDayConvention {
        PyBusinessDayConvention::new(self.inner.business_day_convention)
    }

    fn __repr__(&self) -> String {
        format!(
            "OptionConventions(calendar='{}', settlement_days={})",
            self.inner.calendar_id, self.inner.settlement_days
        )
    }
}

/// Conventions for Interest Rate Futures.
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "IrFutureConventions",
    frozen
)]
#[derive(Clone)]
pub struct PyIrFutureConventions {
    inner: IrFutureConventions,
}

#[pymethods]
impl PyIrFutureConventions {
    #[getter]
    fn index_id(&self) -> String {
        self.inner.index_id.to_string()
    }

    #[getter]
    fn calendar_id(&self) -> &str {
        &self.inner.calendar_id
    }

    #[getter]
    fn settlement_days(&self) -> i32 {
        self.inner.settlement_days
    }

    #[getter]
    fn delivery_months(&self) -> u8 {
        self.inner.delivery_months
    }

    #[getter]
    fn face_value(&self) -> f64 {
        self.inner.face_value
    }

    #[getter]
    fn tick_size(&self) -> f64 {
        self.inner.tick_size
    }

    #[getter]
    fn tick_value(&self) -> f64 {
        self.inner.tick_value
    }

    #[getter]
    fn convexity_adjustment(&self) -> Option<f64> {
        self.inner.convexity_adjustment
    }

    fn __repr__(&self) -> String {
        format!(
            "IrFutureConventions(index='{}', face_value={})",
            self.inner.index_id, self.inner.face_value
        )
    }
}

// =============================================================================
// Convention Registry
// =============================================================================

/// Global registry of market conventions.
///
/// Provides lookup methods for rate index, CDS, swaption, inflation swap,
/// option, and IR future conventions.
///
/// Examples:
///     >>> registry = ConventionRegistry.global_instance()
///     >>> sofr = registry.require_rate_index("USD-SOFR-OIS")
///     >>> print(sofr.day_count)
#[pyclass(
    module = "finstack.valuations.conventions",
    name = "ConventionRegistry",
    frozen
)]
pub struct PyConventionRegistry {
    // We don't need to store anything - we use the global singleton
}

#[pymethods]
impl PyConventionRegistry {
    #[staticmethod]
    /// Get the global convention registry instance.
    ///
    /// Returns:
    ///     ConventionRegistry: The global singleton registry.
    fn global_instance() -> PyResult<Self> {
        // Ensure the registry is initialized
        ConventionRegistry::try_global().map_err(core_to_py)?;
        Ok(Self {})
    }

    /// Look up conventions for a rate index.
    ///
    /// Args:
    ///     index_id: Rate index identifier (e.g., "USD-SOFR-OIS").
    ///
    /// Returns:
    ///     RateIndexConventions: Conventions for the rate index.
    ///
    /// Raises:
    ///     ValueError: If the index is not found.
    fn require_rate_index(&self, index_id: &str) -> PyResult<PyRateIndexConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let id = IndexId::new(index_id);
        let conv = registry.require_rate_index(&id).map_err(core_to_py)?;
        Ok(PyRateIndexConventions {
            inner: conv.clone(),
        })
    }

    /// Look up conventions for a CDS.
    ///
    /// Args:
    ///     key: CDS convention key (currency + doc clause).
    ///
    /// Returns:
    ///     CdsConventions: Conventions for the CDS.
    ///
    /// Raises:
    ///     ValueError: If the conventions are not found.
    fn require_cds(&self, key: &PyCdsConventionKey) -> PyResult<PyCdsConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let conv = registry.require_cds(&key.inner).map_err(core_to_py)?;
        Ok(PyCdsConventions {
            inner: conv.clone(),
        })
    }

    /// Look up conventions for a swaption.
    ///
    /// Args:
    ///     convention_id: Swaption convention identifier (e.g., "USD").
    ///
    /// Returns:
    ///     SwaptionConventions: Conventions for the swaption.
    ///
    /// Raises:
    ///     ValueError: If the conventions are not found.
    fn require_swaption(&self, convention_id: &str) -> PyResult<PySwaptionConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let id = SwaptionConventionId::new(convention_id);
        let conv = registry.require_swaption(&id).map_err(core_to_py)?;
        Ok(PySwaptionConventions {
            inner: conv.clone(),
        })
    }

    /// Look up conventions for an inflation swap.
    ///
    /// Args:
    ///     convention_id: Inflation swap convention identifier (e.g., "USD-CPI").
    ///
    /// Returns:
    ///     InflationSwapConventions: Conventions for the inflation swap.
    ///
    /// Raises:
    ///     ValueError: If the conventions are not found.
    fn require_inflation_swap(&self, convention_id: &str) -> PyResult<PyInflationSwapConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let id = InflationSwapConventionId::new(convention_id);
        let conv = registry.require_inflation_swap(&id).map_err(core_to_py)?;
        Ok(PyInflationSwapConventions {
            inner: conv.clone(),
        })
    }

    /// Look up conventions for an option.
    ///
    /// Args:
    ///     convention_id: Option convention identifier (e.g., "USD-EQUITY").
    ///
    /// Returns:
    ///     OptionConventions: Conventions for the option.
    ///
    /// Raises:
    ///     ValueError: If the conventions are not found.
    fn require_option(&self, convention_id: &str) -> PyResult<PyOptionConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let id = OptionConventionId::new(convention_id);
        let conv = registry.require_option(&id).map_err(core_to_py)?;
        Ok(PyOptionConventions {
            inner: conv.clone(),
        })
    }

    /// Look up conventions for an IR future contract.
    ///
    /// Args:
    ///     contract_id: IR future contract identifier (e.g., "CME:SR3").
    ///
    /// Returns:
    ///     IrFutureConventions: Conventions for the IR future.
    ///
    /// Raises:
    ///     ValueError: If the conventions are not found.
    fn require_ir_future(&self, contract_id: &str) -> PyResult<PyIrFutureConventions> {
        let registry = ConventionRegistry::try_global().map_err(core_to_py)?;
        let id = IrFutureContractId::new(contract_id);
        let conv = registry.require_ir_future(&id).map_err(core_to_py)?;
        Ok(PyIrFutureConventions {
            inner: conv.clone(),
        })
    }

    fn __repr__(&self) -> &'static str {
        "ConventionRegistry()"
    }
}

// =============================================================================
// Module Registration
// =============================================================================

pub(crate) fn register<'py>(
    py: Python<'py>,
    parent: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    let module = PyModule::new(py, "conventions")?;
    module.setattr(
        "__doc__",
        "Market conventions registry for rate indices, CDS, swaptions, and more.",
    )?;

    // Add enums
    module.add_class::<PyCdsDocClause>()?;
    module.add_class::<PyRateIndexKind>()?;

    // Add key types
    module.add_class::<PyCdsConventionKey>()?;

    // Add convention types
    module.add_class::<PyRateIndexConventions>()?;
    module.add_class::<PyCdsConventions>()?;
    module.add_class::<PySwaptionConventions>()?;
    module.add_class::<PyInflationSwapConventions>()?;
    module.add_class::<PyOptionConventions>()?;
    module.add_class::<PyIrFutureConventions>()?;

    // Add registry
    module.add_class::<PyConventionRegistry>()?;

    let exports = vec![
        "CdsDocClause",
        "RateIndexKind",
        "CdsConventionKey",
        "RateIndexConventions",
        "CdsConventions",
        "SwaptionConventions",
        "InflationSwapConventions",
        "OptionConventions",
        "IrFutureConventions",
        "ConventionRegistry",
    ];

    module.setattr("__all__", PyList::new(py, &exports)?)?;
    parent.add_submodule(&module)?;
    Ok(exports)
}
