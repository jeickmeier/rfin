use crate::core::dates::utils::date_to_py;
use crate::core::money::PyMoney;
use finstack_margin::{
    ClearingStatus, CollateralAssetClass, MarginCall, MarginCallType, RepoMarginType, SimmRiskClass,
};
use pyo3::prelude::*;

/// Type of margin call (IM delivery, VM delivery, VM return, top-up, substitution).
#[pyclass(
    name = "MarginCallType",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyMarginCallType {
    pub(crate) inner: MarginCallType,
}

impl PyMarginCallType {
    pub(crate) const fn new(inner: MarginCallType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginCallType {
    #[classattr]
    const INITIAL_MARGIN: Self = Self::new(MarginCallType::InitialMargin);
    #[classattr]
    const VARIATION_MARGIN_DELIVERY: Self = Self::new(MarginCallType::VariationMarginDelivery);
    #[classattr]
    const VARIATION_MARGIN_RETURN: Self = Self::new(MarginCallType::VariationMarginReturn);
    #[classattr]
    const TOP_UP: Self = Self::new(MarginCallType::TopUp);
    #[classattr]
    const SUBSTITUTION: Self = Self::new(MarginCallType::Substitution);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("MarginCallType.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// BCBS-IOSCO collateral asset class.
#[pyclass(
    name = "CollateralAssetClass",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyCollateralAssetClass {
    pub(crate) inner: CollateralAssetClass,
}

impl PyCollateralAssetClass {
    pub(crate) fn new(inner: CollateralAssetClass) -> Self {
        Self { inner }
    }
}

#[pymethods]
#[allow(non_snake_case)]
impl PyCollateralAssetClass {
    #[classattr]
    fn CASH() -> Self {
        Self::new(CollateralAssetClass::Cash)
    }
    #[classattr]
    fn GOVERNMENT_BONDS() -> Self {
        Self::new(CollateralAssetClass::GovernmentBonds)
    }
    #[classattr]
    fn AGENCY_BONDS() -> Self {
        Self::new(CollateralAssetClass::AgencyBonds)
    }
    #[classattr]
    fn COVERED_BONDS() -> Self {
        Self::new(CollateralAssetClass::CoveredBonds)
    }
    #[classattr]
    fn CORPORATE_BONDS() -> Self {
        Self::new(CollateralAssetClass::CorporateBonds)
    }
    #[classattr]
    fn EQUITY() -> Self {
        Self::new(CollateralAssetClass::Equity)
    }
    #[classattr]
    fn GOLD() -> Self {
        Self::new(CollateralAssetClass::Gold)
    }
    #[classattr]
    fn MUTUAL_FUNDS() -> Self {
        Self::new(CollateralAssetClass::MutualFunds)
    }

    #[staticmethod]
    fn custom(name: String) -> Self {
        Self::new(CollateralAssetClass::Custom(name))
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.as_str().to_string()
    }

    fn standard_haircut(&self) -> PyResult<f64> {
        self.inner
            .standard_haircut()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!("CollateralAssetClass({})", self.inner.as_str())
    }

    fn __str__(&self) -> String {
        self.inner.as_str().to_string()
    }
}

/// Clearing status: bilateral or cleared through a CCP.
#[pyclass(
    name = "ClearingStatus",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PyClearingStatus {
    pub(crate) inner: ClearingStatus,
}

impl PyClearingStatus {
    pub(crate) fn new(inner: ClearingStatus) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyClearingStatus {
    #[staticmethod]
    fn bilateral() -> Self {
        Self::new(ClearingStatus::Bilateral)
    }

    #[staticmethod]
    fn cleared(ccp: String) -> Self {
        Self::new(ClearingStatus::Cleared { ccp })
    }

    #[getter]
    fn is_bilateral(&self) -> bool {
        matches!(self.inner, ClearingStatus::Bilateral)
    }

    #[getter]
    fn is_cleared(&self) -> bool {
        matches!(self.inner, ClearingStatus::Cleared { .. })
    }

    #[getter]
    fn ccp(&self) -> Option<String> {
        match &self.inner {
            ClearingStatus::Cleared { ccp } => Some(ccp.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        format!("ClearingStatus({})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// ISDA SIMM risk class.
#[pyclass(
    name = "SimmRiskClass",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PySimmRiskClass {
    pub(crate) inner: SimmRiskClass,
}

impl PySimmRiskClass {
    pub(crate) const fn new(inner: SimmRiskClass) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PySimmRiskClass {
    #[classattr]
    const INTEREST_RATE: Self = Self::new(SimmRiskClass::InterestRate);
    #[classattr]
    const CREDIT_QUALIFYING: Self = Self::new(SimmRiskClass::CreditQualifying);
    #[classattr]
    const CREDIT_NON_QUALIFYING: Self = Self::new(SimmRiskClass::CreditNonQualifying);
    #[classattr]
    const EQUITY: Self = Self::new(SimmRiskClass::Equity);
    #[classattr]
    const COMMODITY: Self = Self::new(SimmRiskClass::Commodity);
    #[classattr]
    const FX: Self = Self::new(SimmRiskClass::Fx);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("SimmRiskClass.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// Repo margin mechanism type.
#[pyclass(
    name = "RepoMarginType",
    module = "finstack.valuations.margin",
    frozen,
    from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PyRepoMarginType {
    pub(crate) inner: RepoMarginType,
}

impl PyRepoMarginType {
    pub(crate) const fn new(inner: RepoMarginType) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyRepoMarginType {
    #[classattr]
    const NONE: Self = Self::new(RepoMarginType::None);
    #[classattr]
    const MARK_TO_MARKET: Self = Self::new(RepoMarginType::MarkToMarket);
    #[classattr]
    const NET_EXPOSURE: Self = Self::new(RepoMarginType::NetExposure);
    #[classattr]
    const TRIPARTY: Self = Self::new(RepoMarginType::Triparty);

    #[getter]
    fn name(&self) -> String {
        self.inner.to_string()
    }

    fn __repr__(&self) -> String {
        format!("RepoMarginType.{}", self.name().to_ascii_uppercase())
    }

    fn __str__(&self) -> String {
        self.name()
    }
}

/// A margin call event with all relevant details.
#[pyclass(
    name = "MarginCall",
    module = "finstack.valuations.margin",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyMarginCall {
    pub(crate) inner: MarginCall,
}

impl PyMarginCall {
    pub(crate) fn new(inner: MarginCall) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyMarginCall {
    #[getter]
    fn call_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.call_date)
    }

    #[getter]
    fn settlement_date(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        date_to_py(py, self.inner.settlement_date)
    }

    #[getter]
    fn call_type(&self) -> PyMarginCallType {
        PyMarginCallType::new(self.inner.call_type)
    }

    #[getter]
    fn amount(&self) -> PyMoney {
        PyMoney::new(self.inner.amount)
    }

    #[getter]
    fn collateral_type(&self) -> Option<PyCollateralAssetClass> {
        self.inner
            .collateral_type
            .clone()
            .map(PyCollateralAssetClass::new)
    }

    #[getter]
    fn mtm_trigger(&self) -> PyMoney {
        PyMoney::new(self.inner.mtm_trigger)
    }

    #[getter]
    fn threshold(&self) -> PyMoney {
        PyMoney::new(self.inner.threshold)
    }

    #[getter]
    fn mta_applied(&self) -> PyMoney {
        PyMoney::new(self.inner.mta_applied)
    }

    fn is_delivery(&self) -> bool {
        self.inner.is_delivery()
    }

    fn is_return(&self) -> bool {
        self.inner.is_return()
    }

    fn days_to_settle(&self) -> i64 {
        self.inner.days_to_settle()
    }

    fn __repr__(&self) -> String {
        format!(
            "MarginCall(type={}, amount={}, call_date={})",
            self.inner.call_type, self.inner.amount, self.inner.call_date
        )
    }
}
