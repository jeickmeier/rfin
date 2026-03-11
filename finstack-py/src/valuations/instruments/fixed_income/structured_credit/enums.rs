//! Python bindings for structured credit enum types.
//!
//! Provides PyO3 wrappers for:
//! - AssetType, PaymentMode, TriggerConsequence
//! - TrancheBehaviorType, TrancheCoupon, RecipientType
//! - ManagementFeeType, CoverageTestType, RoundingConvention
//! - DiversionCondition, ValidationError, PaymentCalculation

use finstack_core::currency::Currency;
use finstack_core::money::Money;
use finstack_valuations::instruments::fixed_income::structured_credit::{
    AssetType, CoverageTestType, DiversionCondition, ManagementFeeType, PaymentCalculation,
    PaymentMode, RecipientType, RoundingConvention, TrancheBehaviorType, TrancheCoupon,
    TriggerConsequence, ValidationError,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyType;

use super::utils::{from_dict_json, to_dict_via_serde, value_to_json};
use crate::core::dates::utils::py_to_date;

// ============================================================================
// ASSET TYPE
// ============================================================================

/// Collateral asset type classification for pool composition.
///
/// A complex enum with ~30 variants covering loans, bonds, mortgages,
/// auto loans, credit cards, student loans, and generic asset types.
///
/// Use the category-specific classmethods and classattrs for construction.
///
/// Examples:
///     >>> asset = PoolAssetType.first_lien_loan(industry="Technology")
///     >>> asset.is_amortizing()
///     False
///     >>> mortgage = PoolAssetType.single_family_mortgage(ltv=0.80)
///     >>> mortgage.is_amortizing()
///     True
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PoolAssetType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyAssetType {
    pub(crate) inner: AssetType,
}

impl From<AssetType> for PyAssetType {
    fn from(inner: AssetType) -> Self {
        Self { inner }
    }
}

impl From<PyAssetType> for AssetType {
    fn from(py: PyAssetType) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyAssetType {
    // ========== LOAN TYPES ==========

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn first_lien_loan(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::FirstLienLoan { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn second_lien_loan(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::SecondLienLoan { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn revolver_loan(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::RevolverLoan { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn bridge_loan(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::BridgeLoan { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn mezzanine_loan(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::MezzanineLoan { industry },
        }
    }

    // ========== BOND TYPES ==========

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn high_yield_bond(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::HighYieldBond { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn investment_grade_bond(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::InvestmentGradeBond { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn distressed_bond(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::DistressedBond { industry },
        }
    }

    #[classmethod]
    #[pyo3(signature = (industry=None))]
    fn emerging_markets_bond(_cls: &Bound<'_, PyType>, industry: Option<String>) -> Self {
        Self {
            inner: AssetType::EmergingMarketsBond { industry },
        }
    }

    // ========== MORTGAGE TYPES ==========

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn single_family_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::SingleFamilyMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn multifamily_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::MultifamilyMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn commercial_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::CommercialMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn industrial_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::IndustrialMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn retail_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::RetailMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn office_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::OfficeMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn hotel_mortgage(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::HotelMortgage { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (property_type, ltv=None))]
    fn other_mortgage(_cls: &Bound<'_, PyType>, property_type: String, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::OtherMortgage { property_type, ltv },
        }
    }

    // ========== AUTO TYPES ==========

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn new_auto_loan(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::NewAutoLoan { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn used_auto_loan(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::UsedAutoLoan { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn lease_auto_loan(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::LeaseAutoLoan { ltv },
        }
    }

    #[classmethod]
    #[pyo3(signature = (ltv=None))]
    fn fleet_auto_loan(_cls: &Bound<'_, PyType>, ltv: Option<f64>) -> Self {
        Self {
            inner: AssetType::FleetAutoLoan { ltv },
        }
    }

    // ========== CREDIT CARD TYPES ==========

    #[classattr]
    #[pyo3(name = "PRIME_CREDIT_CARD")]
    fn prime_credit_card() -> Self {
        Self {
            inner: AssetType::PrimeCreditCard,
        }
    }

    #[classattr]
    #[pyo3(name = "SUBPRIME_CREDIT_CARD")]
    fn subprime_credit_card() -> Self {
        Self {
            inner: AssetType::SubPrimeCreditCard,
        }
    }

    #[classattr]
    #[pyo3(name = "SUPER_PRIME_CREDIT_CARD")]
    fn super_prime_credit_card() -> Self {
        Self {
            inner: AssetType::SuperPrimeCreditCard,
        }
    }

    #[classattr]
    #[pyo3(name = "COMMERCIAL_CREDIT_CARD")]
    fn commercial_credit_card() -> Self {
        Self {
            inner: AssetType::CommercialCreditCard,
        }
    }

    // ========== STUDENT LOAN TYPES ==========

    #[classattr]
    #[pyo3(name = "FEDERAL_STUDENT_LOAN")]
    fn federal_student_loan() -> Self {
        Self {
            inner: AssetType::FederalStudentLoan,
        }
    }

    #[classattr]
    #[pyo3(name = "PRIVATE_STUDENT_LOAN")]
    fn private_student_loan() -> Self {
        Self {
            inner: AssetType::PrivateStudentLoan,
        }
    }

    #[classattr]
    #[pyo3(name = "FFELP_STUDENT_LOAN")]
    fn ffelp_student_loan() -> Self {
        Self {
            inner: AssetType::FFELPStudentLoan,
        }
    }

    #[classattr]
    #[pyo3(name = "CONSOLIDATION_STUDENT_LOAN")]
    fn consolidation_student_loan() -> Self {
        Self {
            inner: AssetType::ConsolidationStudentLoan,
        }
    }

    // ========== OTHER TYPES ==========

    #[classmethod]
    #[pyo3(text_signature = "(cls, equipment_type)")]
    fn equipment(_cls: &Bound<'_, PyType>, equipment_type: String) -> Self {
        Self {
            inner: AssetType::Equipment { equipment_type },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, description, asset_class)")]
    fn generic(_cls: &Bound<'_, PyType>, description: String, asset_class: String) -> Self {
        Self {
            inner: AssetType::Generic {
                description,
                asset_class,
            },
        }
    }

    // ========== METHODS ==========

    /// Returns ``True`` for asset types that amortize through level payments.
    fn is_amortizing(&self) -> bool {
        self.inner.is_amortizing()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("AssetType({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            AssetType::FirstLienLoan { .. } => 0,
            AssetType::SecondLienLoan { .. } => 1,
            AssetType::RevolverLoan { .. } => 2,
            AssetType::BridgeLoan { .. } => 3,
            AssetType::MezzanineLoan { .. } => 4,
            AssetType::HighYieldBond { .. } => 5,
            AssetType::InvestmentGradeBond { .. } => 6,
            AssetType::DistressedBond { .. } => 7,
            AssetType::EmergingMarketsBond { .. } => 8,
            AssetType::SingleFamilyMortgage { .. } => 9,
            AssetType::MultifamilyMortgage { .. } => 10,
            AssetType::CommercialMortgage { .. } => 11,
            AssetType::IndustrialMortgage { .. } => 12,
            AssetType::RetailMortgage { .. } => 13,
            AssetType::OfficeMortgage { .. } => 14,
            AssetType::HotelMortgage { .. } => 15,
            AssetType::OtherMortgage { .. } => 16,
            AssetType::NewAutoLoan { .. } => 17,
            AssetType::UsedAutoLoan { .. } => 18,
            AssetType::LeaseAutoLoan { .. } => 19,
            AssetType::FleetAutoLoan { .. } => 20,
            AssetType::PrimeCreditCard => 21,
            AssetType::SubPrimeCreditCard => 22,
            AssetType::SuperPrimeCreditCard => 23,
            AssetType::CommercialCreditCard => 24,
            AssetType::FederalStudentLoan => 25,
            AssetType::PrivateStudentLoan => 26,
            AssetType::FFELPStudentLoan => 27,
            AssetType::ConsolidationStudentLoan => 28,
            AssetType::Equipment { .. } => 29,
            AssetType::Generic { .. } => 30,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// PAYMENT MODE
// ============================================================================

/// Payment distribution mode (pro-rata, sequential, or hybrid).
///
/// Examples:
///     >>> mode = PaymentMode.pro_rata()
///     >>> mode = PaymentMode.sequential("oc_breach", datetime.date(2024, 6, 15))
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PaymentMode",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPaymentMode {
    pub(crate) inner: PaymentMode,
}

impl From<PaymentMode> for PyPaymentMode {
    fn from(inner: PaymentMode) -> Self {
        Self { inner }
    }
}

impl From<PyPaymentMode> for PaymentMode {
    fn from(py: PyPaymentMode) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyPaymentMode {
    #[classmethod]
    fn pro_rata(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            inner: PaymentMode::ProRata,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, triggered_by, trigger_date)")]
    fn sequential(
        _cls: &Bound<'_, PyType>,
        triggered_by: String,
        trigger_date: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let date = py_to_date(&trigger_date)?;
        Ok(Self {
            inner: PaymentMode::Sequential {
                triggered_by,
                trigger_date: date,
            },
        })
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, description)")]
    fn hybrid(_cls: &Bound<'_, PyType>, description: String) -> Self {
        Self {
            inner: PaymentMode::Hybrid { description },
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("PaymentMode({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            PaymentMode::ProRata => 0,
            PaymentMode::Sequential { .. } => 1,
            PaymentMode::Hybrid { .. } => 2,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// TRIGGER CONSEQUENCE
// ============================================================================

/// Consequence when a coverage trigger is breached.
///
/// Examples:
///     >>> consequence = TriggerConsequence.DIVERT_CASH_FLOW
///     >>> consequence = TriggerConsequence.custom("Lock-out interest")
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TriggerConsequence",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTriggerConsequence {
    pub(crate) inner: TriggerConsequence,
}

impl From<TriggerConsequence> for PyTriggerConsequence {
    fn from(inner: TriggerConsequence) -> Self {
        Self { inner }
    }
}

impl From<PyTriggerConsequence> for TriggerConsequence {
    fn from(py: PyTriggerConsequence) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyTriggerConsequence {
    #[classattr]
    #[pyo3(name = "DIVERT_CASH_FLOW")]
    fn divert_cash_flow() -> Self {
        Self {
            inner: TriggerConsequence::DivertCashFlow,
        }
    }

    #[classattr]
    #[pyo3(name = "TRAP_EXCESS_SPREAD")]
    fn trap_excess_spread() -> Self {
        Self {
            inner: TriggerConsequence::TrapExcessSpread,
        }
    }

    #[classattr]
    #[pyo3(name = "ACCELERATE_AMORTIZATION")]
    fn accelerate_amortization() -> Self {
        Self {
            inner: TriggerConsequence::AccelerateAmortization,
        }
    }

    #[classattr]
    #[pyo3(name = "STOP_REINVESTMENT")]
    fn stop_reinvestment() -> Self {
        Self {
            inner: TriggerConsequence::StopReinvestment,
        }
    }

    #[classattr]
    #[pyo3(name = "REDUCE_MANAGER_FEE")]
    fn reduce_manager_fee() -> Self {
        Self {
            inner: TriggerConsequence::ReduceManagerFee,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, description)")]
    fn custom(_cls: &Bound<'_, PyType>, description: String) -> Self {
        Self {
            inner: TriggerConsequence::Custom(description),
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("TriggerConsequence({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            TriggerConsequence::DivertCashFlow => 0,
            TriggerConsequence::TrapExcessSpread => 1,
            TriggerConsequence::AccelerateAmortization => 2,
            TriggerConsequence::StopReinvestment => 3,
            TriggerConsequence::ReduceManagerFee => 4,
            TriggerConsequence::Custom(_) => 5,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// TRANCHE BEHAVIOR TYPE
// ============================================================================

/// Tranche behavioral classification.
///
/// Examples:
///     >>> TrancheBehaviorType.Standard
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheBehaviorType",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyTrancheBehaviorType {
    Standard = 0,
}

impl From<PyTrancheBehaviorType> for TrancheBehaviorType {
    fn from(value: PyTrancheBehaviorType) -> Self {
        match value {
            PyTrancheBehaviorType::Standard => TrancheBehaviorType::Standard,
        }
    }
}

impl From<TrancheBehaviorType> for PyTrancheBehaviorType {
    fn from(value: TrancheBehaviorType) -> Self {
        match value {
            TrancheBehaviorType::Standard => PyTrancheBehaviorType::Standard,
            _ => PyTrancheBehaviorType::Standard,
        }
    }
}

#[pymethods]
impl PyTrancheBehaviorType {
    fn __repr__(&self) -> String {
        match self {
            PyTrancheBehaviorType::Standard => "TrancheBehaviorType.Standard".to_string(),
        }
    }
}

// ============================================================================
// TRANCHE COUPON
// ============================================================================

/// Tranche coupon specification (fixed or floating).
///
/// Examples:
///     >>> coupon = TrancheCoupon.fixed(0.05)
///     >>> coupon.current_rate(datetime.date(2024, 6, 15))
///     0.05
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "TrancheCoupon",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyTrancheCoupon {
    pub(crate) inner: TrancheCoupon,
}

impl From<TrancheCoupon> for PyTrancheCoupon {
    fn from(inner: TrancheCoupon) -> Self {
        Self { inner }
    }
}

impl From<PyTrancheCoupon> for TrancheCoupon {
    fn from(py: PyTrancheCoupon) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyTrancheCoupon {
    #[classmethod]
    #[pyo3(text_signature = "(cls, rate)")]
    fn fixed(_cls: &Bound<'_, PyType>, rate: f64) -> Self {
        Self {
            inner: TrancheCoupon::Fixed { rate },
        }
    }

    /// Create a floating-rate coupon from a FloatingRateSpec dict/JSON.
    #[classmethod]
    #[pyo3(text_signature = "(cls, spec)")]
    fn floating(_cls: &Bound<'_, PyType>, spec: Bound<'_, PyAny>) -> PyResult<Self> {
        let json_str = value_to_json(&spec)?;
        let floating_spec: finstack_valuations::cashflow::builder::FloatingRateSpec =
            serde_json::from_str(&json_str)
                .map_err(|e| PyValueError::new_err(format!("Invalid FloatingRateSpec: {e}")))?;
        Ok(Self {
            inner: TrancheCoupon::Floating(floating_spec),
        })
    }

    /// Get current rate for a given date (without index lookup).
    #[pyo3(text_signature = "($self, date)")]
    fn current_rate(&self, date: Bound<'_, PyAny>) -> PyResult<f64> {
        let d = py_to_date(&date)?;
        Ok(self.inner.current_rate(d))
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("TrancheCoupon({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (TrancheCoupon::Fixed { rate: a }, TrancheCoupon::Fixed { rate: b }) => {
                (a - b).abs() < f64::EPSILON
            }
            (TrancheCoupon::Floating(a), TrancheCoupon::Floating(b)) => {
                format!("{a:?}") == format!("{b:?}")
            }
            _ => false,
        }
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            TrancheCoupon::Fixed { .. } => 0,
            TrancheCoupon::Floating(_) => 1,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// RECIPIENT TYPE
// ============================================================================

/// Waterfall payment recipient type.
///
/// Examples:
///     >>> recipient = RecipientType.service_provider("Trustee")
///     >>> recipient = RecipientType.tranche("CLASS_A")
///     >>> recipient = RecipientType.EQUITY
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RecipientType",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecipientType {
    pub(crate) inner: RecipientType,
}

impl From<RecipientType> for PyRecipientType {
    fn from(inner: RecipientType) -> Self {
        Self { inner }
    }
}

impl From<PyRecipientType> for RecipientType {
    fn from(py: PyRecipientType) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyRecipientType {
    #[classmethod]
    #[pyo3(text_signature = "(cls, name)")]
    fn service_provider(_cls: &Bound<'_, PyType>, name: String) -> Self {
        Self {
            inner: RecipientType::ServiceProvider(name),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, fee_type)")]
    fn manager_fee(_cls: &Bound<'_, PyType>, fee_type: PyManagementFeeType) -> Self {
        Self {
            inner: RecipientType::ManagerFee(fee_type.into()),
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, tranche_id)")]
    fn tranche(_cls: &Bound<'_, PyType>, tranche_id: String) -> Self {
        Self {
            inner: RecipientType::Tranche(tranche_id),
        }
    }

    #[classattr]
    #[pyo3(name = "EQUITY")]
    fn equity() -> Self {
        Self {
            inner: RecipientType::Equity,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, account_id)")]
    fn reserve_account(_cls: &Bound<'_, PyType>, account_id: String) -> Self {
        Self {
            inner: RecipientType::ReserveAccount(account_id),
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("RecipientType({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn __hash__(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.inner.hash(&mut h);
        h.finish()
    }
}

// ============================================================================
// MANAGEMENT FEE TYPE
// ============================================================================

/// Type of management fee (senior, subordinated, or incentive).
///
/// Examples:
///     >>> ManagementFeeType.Senior
///     >>> ManagementFeeType.Incentive
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ManagementFeeType",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyManagementFeeType {
    Senior = 0,
    Subordinated = 1,
    Incentive = 2,
}

impl From<PyManagementFeeType> for ManagementFeeType {
    fn from(value: PyManagementFeeType) -> Self {
        match value {
            PyManagementFeeType::Senior => ManagementFeeType::Senior,
            PyManagementFeeType::Subordinated => ManagementFeeType::Subordinated,
            PyManagementFeeType::Incentive => ManagementFeeType::Incentive,
        }
    }
}

impl From<ManagementFeeType> for PyManagementFeeType {
    fn from(value: ManagementFeeType) -> Self {
        match value {
            ManagementFeeType::Senior => PyManagementFeeType::Senior,
            ManagementFeeType::Subordinated => PyManagementFeeType::Subordinated,
            ManagementFeeType::Incentive => PyManagementFeeType::Incentive,
            _ => PyManagementFeeType::Senior,
        }
    }
}

#[pymethods]
impl PyManagementFeeType {
    fn __repr__(&self) -> String {
        match self {
            PyManagementFeeType::Senior => "ManagementFeeType.Senior".to_string(),
            PyManagementFeeType::Subordinated => "ManagementFeeType.Subordinated".to_string(),
            PyManagementFeeType::Incentive => "ManagementFeeType.Incentive".to_string(),
        }
    }
}

// ============================================================================
// COVERAGE TEST TYPE
// ============================================================================

/// Coverage test type (OC or IC).
///
/// Examples:
///     >>> CoverageTestType.OC
///     >>> CoverageTestType.IC
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "CoverageTestType",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyCoverageTestType {
    OC = 0,
    IC = 1,
}

impl From<PyCoverageTestType> for CoverageTestType {
    fn from(value: PyCoverageTestType) -> Self {
        match value {
            PyCoverageTestType::OC => CoverageTestType::OC,
            PyCoverageTestType::IC => CoverageTestType::IC,
        }
    }
}

impl From<CoverageTestType> for PyCoverageTestType {
    fn from(value: CoverageTestType) -> Self {
        match value {
            CoverageTestType::OC => PyCoverageTestType::OC,
            CoverageTestType::IC => PyCoverageTestType::IC,
            _ => PyCoverageTestType::OC,
        }
    }
}

#[pymethods]
impl PyCoverageTestType {
    fn __repr__(&self) -> String {
        match self {
            PyCoverageTestType::OC => "CoverageTestType.OC".to_string(),
            PyCoverageTestType::IC => "CoverageTestType.IC".to_string(),
        }
    }
}

// ============================================================================
// ROUNDING CONVENTION
// ============================================================================

/// Rounding convention for payment calculations.
///
/// Examples:
///     >>> RoundingConvention.Nearest
///     >>> RoundingConvention.Floor
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "RoundingConvention",
    eq,
    eq_int,
    from_py_object
)]
#[derive(Clone, Debug, PartialEq)]
pub enum PyRoundingConvention {
    Nearest = 0,
    Floor = 1,
    Ceiling = 2,
}

impl From<PyRoundingConvention> for RoundingConvention {
    fn from(value: PyRoundingConvention) -> Self {
        match value {
            PyRoundingConvention::Nearest => RoundingConvention::Nearest,
            PyRoundingConvention::Floor => RoundingConvention::Floor,
            PyRoundingConvention::Ceiling => RoundingConvention::Ceiling,
        }
    }
}

impl From<RoundingConvention> for PyRoundingConvention {
    fn from(value: RoundingConvention) -> Self {
        match value {
            RoundingConvention::Nearest => PyRoundingConvention::Nearest,
            RoundingConvention::Floor => PyRoundingConvention::Floor,
            RoundingConvention::Ceiling => PyRoundingConvention::Ceiling,
            _ => PyRoundingConvention::Nearest,
        }
    }
}

#[pymethods]
impl PyRoundingConvention {
    fn __repr__(&self) -> String {
        match self {
            PyRoundingConvention::Nearest => "RoundingConvention.Nearest".to_string(),
            PyRoundingConvention::Floor => "RoundingConvention.Floor".to_string(),
            PyRoundingConvention::Ceiling => "RoundingConvention.Ceiling".to_string(),
        }
    }
}

// ============================================================================
// DIVERSION CONDITION
// ============================================================================

/// Condition that triggers a cash flow diversion in the waterfall.
///
/// Examples:
///     >>> condition = DiversionCondition.coverage_test_failed("oc_test_a")
///     >>> condition = DiversionCondition.ALWAYS
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "DiversionCondition",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDiversionCondition {
    pub(crate) inner: DiversionCondition,
}

impl From<DiversionCondition> for PyDiversionCondition {
    fn from(inner: DiversionCondition) -> Self {
        Self { inner }
    }
}

impl From<PyDiversionCondition> for DiversionCondition {
    fn from(py: PyDiversionCondition) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyDiversionCondition {
    #[classmethod]
    #[pyo3(text_signature = "(cls, test_id)")]
    fn coverage_test_failed(_cls: &Bound<'_, PyType>, test_id: String) -> Self {
        Self {
            inner: DiversionCondition::CoverageTestFailed { test_id },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, expr)")]
    fn custom_expression(_cls: &Bound<'_, PyType>, expr: String) -> Self {
        Self {
            inner: DiversionCondition::CustomExpression { expr },
        }
    }

    #[classattr]
    #[pyo3(name = "ALWAYS")]
    fn always() -> Self {
        Self {
            inner: DiversionCondition::Always,
        }
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("DiversionCondition({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            DiversionCondition::CoverageTestFailed { .. } => 0,
            DiversionCondition::CustomExpression { .. } => 1,
            DiversionCondition::Always => 2,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// VALIDATION ERROR
// ============================================================================

/// Waterfall validation error (read-only, no constructors).
///
/// Returned by waterfall validation routines.
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "ValidationError",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyValidationError {
    pub(crate) inner: ValidationError,
}

impl From<ValidationError> for PyValidationError {
    fn from(inner: ValidationError) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyValidationError {
    fn __repr__(&self) -> String {
        format!("ValidationError({:?})", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

// ============================================================================
// PAYMENT CALCULATION
// ============================================================================

/// How to calculate a waterfall payment amount.
///
/// Examples:
///     >>> calc = PaymentCalculation.fixed_amount(50000.0, "USD")
///     >>> calc = PaymentCalculation.tranche_interest("CLASS_A")
///     >>> calc = PaymentCalculation.RESIDUAL_CASH
#[pyclass(
    module = "finstack.valuations.instruments",
    name = "PaymentCalculation",
    frozen,
    from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPaymentCalculation {
    pub(crate) inner: PaymentCalculation,
}

impl From<PaymentCalculation> for PyPaymentCalculation {
    fn from(inner: PaymentCalculation) -> Self {
        Self { inner }
    }
}

impl From<PyPaymentCalculation> for PaymentCalculation {
    fn from(py: PyPaymentCalculation) -> Self {
        py.inner
    }
}

#[pymethods]
impl PyPaymentCalculation {
    #[classmethod]
    #[pyo3(text_signature = "(cls, amount, currency)")]
    fn fixed_amount(_cls: &Bound<'_, PyType>, amount: f64, currency: &str) -> PyResult<Self> {
        let curr: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        Ok(Self {
            inner: PaymentCalculation::FixedAmount {
                amount: Money::new(amount, curr),
                rounding: None,
            },
        })
    }

    #[classmethod]
    #[pyo3(signature = (rate, annualized=true))]
    fn percentage_of_collateral(_cls: &Bound<'_, PyType>, rate: f64, annualized: bool) -> Self {
        Self {
            inner: PaymentCalculation::PercentageOfCollateral {
                rate,
                annualized,
                day_count: None,
                rounding: None,
            },
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, tranche_id)")]
    fn tranche_interest(_cls: &Bound<'_, PyType>, tranche_id: String) -> Self {
        Self {
            inner: PaymentCalculation::TrancheInterest {
                tranche_id,
                rounding: None,
            },
        }
    }

    #[classmethod]
    #[pyo3(signature = (tranche_id, target_balance=None, currency="USD"))]
    fn tranche_principal(
        _cls: &Bound<'_, PyType>,
        tranche_id: String,
        target_balance: Option<f64>,
        currency: &str,
    ) -> PyResult<Self> {
        let target = target_balance.map(|amt| {
            let ccy: Currency = currency.parse().unwrap_or(Currency::USD);
            Money::new(amt, ccy)
        });
        Ok(Self {
            inner: PaymentCalculation::TranchePrincipal {
                tranche_id,
                target_balance: target,
                rounding: None,
            },
        })
    }

    #[classattr]
    #[pyo3(name = "RESIDUAL_CASH")]
    fn residual_cash() -> Self {
        Self {
            inner: PaymentCalculation::ResidualCash,
        }
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, target_balance_amount, currency)")]
    fn reserve_replenishment(
        _cls: &Bound<'_, PyType>,
        target_balance_amount: f64,
        currency: &str,
    ) -> PyResult<Self> {
        let curr: Currency = currency
            .parse()
            .map_err(|e| PyValueError::new_err(format!("Invalid currency: {e:?}")))?;
        Ok(Self {
            inner: PaymentCalculation::ReserveReplenishment {
                target_balance: Money::new(target_balance_amount, curr),
            },
        })
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        to_dict_via_serde(py, &self.inner)
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, data)")]
    fn from_dict(_cls: &Bound<'_, PyType>, data: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: from_dict_json(&data)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("PaymentCalculation({:?})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (
                PaymentCalculation::FixedAmount {
                    amount: a,
                    rounding: ra,
                },
                PaymentCalculation::FixedAmount {
                    amount: b,
                    rounding: rb,
                },
            ) => a == b && ra == rb,
            (
                PaymentCalculation::PercentageOfCollateral {
                    rate: r1,
                    annualized: a1,
                    day_count: dc1,
                    rounding: rd1,
                },
                PaymentCalculation::PercentageOfCollateral {
                    rate: r2,
                    annualized: a2,
                    day_count: dc2,
                    rounding: rd2,
                },
            ) => (r1 - r2).abs() < f64::EPSILON && a1 == a2 && dc1 == dc2 && rd1 == rd2,
            (
                PaymentCalculation::TrancheInterest {
                    tranche_id: t1,
                    rounding: r1,
                },
                PaymentCalculation::TrancheInterest {
                    tranche_id: t2,
                    rounding: r2,
                },
            ) => t1 == t2 && r1 == r2,
            (
                PaymentCalculation::TranchePrincipal {
                    tranche_id: t1,
                    target_balance: tb1,
                    rounding: r1,
                },
                PaymentCalculation::TranchePrincipal {
                    tranche_id: t2,
                    target_balance: tb2,
                    rounding: r2,
                },
            ) => t1 == t2 && tb1 == tb2 && r1 == r2,
            (PaymentCalculation::ResidualCash, PaymentCalculation::ResidualCash) => true,
            (
                PaymentCalculation::ReserveReplenishment {
                    target_balance: tb1,
                },
                PaymentCalculation::ReserveReplenishment {
                    target_balance: tb2,
                },
            ) => tb1 == tb2,
            _ => false,
        }
    }

    fn discriminant(&self) -> isize {
        match &self.inner {
            PaymentCalculation::FixedAmount { .. } => 0,
            PaymentCalculation::PercentageOfCollateral { .. } => 1,
            PaymentCalculation::TrancheInterest { .. } => 2,
            PaymentCalculation::TranchePrincipal { .. } => 3,
            PaymentCalculation::ResidualCash => 4,
            PaymentCalculation::ReserveReplenishment { .. } => 5,
            _ => -1,
        }
    }

    fn __hash__(&self) -> isize {
        self.discriminant()
    }
}

// ============================================================================
// REGISTRATION
// ============================================================================

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyAssetType>()?;
    module.add_class::<PyPaymentMode>()?;
    module.add_class::<PyTriggerConsequence>()?;
    module.add_class::<PyTrancheBehaviorType>()?;
    module.add_class::<PyTrancheCoupon>()?;
    module.add_class::<PyRecipientType>()?;
    module.add_class::<PyManagementFeeType>()?;
    module.add_class::<PyCoverageTestType>()?;
    module.add_class::<PyRoundingConvention>()?;
    module.add_class::<PyDiversionCondition>()?;
    module.add_class::<PyValidationError>()?;
    module.add_class::<PyPaymentCalculation>()?;

    Ok(vec![
        "PoolAssetType",
        "PaymentMode",
        "TriggerConsequence",
        "TrancheBehaviorType",
        "TrancheCoupon",
        "RecipientType",
        "ManagementFeeType",
        "CoverageTestType",
        "RoundingConvention",
        "DiversionCondition",
        "ValidationError",
        "PaymentCalculation",
    ])
}
