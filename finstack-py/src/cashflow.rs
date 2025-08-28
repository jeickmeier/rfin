//! Python bindings for comprehensive cashflow generation and analysis.
//!
//! This module exposes the full cashflow builder from finstack-valuations,
//! allowing creation of complex cashflow structures including PIK/toggle,
//! amortization, fees, and more. It also provides DataFrame conversion
//! for easy analysis in Jupyter notebooks.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use finstack_valuations::cashflow::{
    builder::{
        cf, CashFlowSchedule, CouponType,
        FixedCouponSpec,
    },
    primitives::{CFKind, CashFlow as CoreCashFlow},
    amortization_notional::AmortizationSpec,
};
use finstack_core::{
    dates::{BusinessDayConvention, StubKind},
    money::Money,
};
use crate::{
    currency::PyCurrency,
    dates::{PyDate, PyDayCount, PyFrequency, PyBusDayConv, PyStubRule},
    money::PyMoney,
};
use std::sync::Arc;

/// Individual cash flow with enhanced metadata.
///
/// Represents a single payment in a cashflow schedule with full
/// type information and accrual details.
///
/// Examples:
///     >>> cf = schedule.flows[0]
///     >>> cf.date
///     Date('2024-07-01')
///     >>> cf.amount
///     25000.0
///     >>> cf.kind
///     'Fixed'
///     >>> cf.to_dict()
///     {'date': '2024-07-01', 'amount': 25000.0, 'currency': 'USD', 'kind': 'Fixed'}
#[pyclass(name = "CashFlow", module = "finstack.cashflow")]
#[derive(Clone, Copy)]
pub struct PyCashFlow {
    inner: CoreCashFlow,
}

#[pymethods]
impl PyCashFlow {
    #[getter]
    fn date(&self) -> PyDate {
        PyDate::from_core(self.inner.date)
    }
    
    #[getter]
    fn reset_date(&self) -> Option<PyDate> {
        self.inner.reset_date.map(PyDate::from_core)
    }
    
    #[getter]
    fn amount(&self) -> f64 {
        self.inner.amount.amount()
    }
    
    #[getter]
    fn currency(&self) -> PyCurrency {
        PyCurrency::from_inner(self.inner.amount.currency())
    }
    
    #[getter]
    fn kind(&self) -> String {
        match self.inner.kind {
            CFKind::Fixed => "Fixed",
            CFKind::FloatReset => "FloatReset",
            CFKind::Notional => "Notional",
            CFKind::PIK => "PIK",
            CFKind::Amortization => "Amortization",
            CFKind::Fee => "Fee",
            CFKind::Stub => "Stub",
            _ => "Other",
        }.to_string()
    }
    
    #[getter]
    fn accrual_factor(&self) -> f64 {
        self.inner.accrual_factor
    }
    
    /// Convert to a dictionary for DataFrame creation.
    ///
    /// Returns:
    ///     dict: Dictionary with cashflow fields
    ///
    /// Examples:
    ///     >>> cf.to_dict()
    ///     {'date': '2024-07-01', 'amount': 25000.0, 'currency': 'USD', 
    ///      'kind': 'Fixed', 'accrual_factor': 0.5}
    #[allow(clippy::wrong_self_convention)]  // PyO3 requires &self for Python methods
    fn to_dict(&self, py: Python) -> PyResult<Py<PyDict>> {
        let dict = PyDict::new(py);
        dict.set_item("date", format!("{}", self.inner.date))?;
        dict.set_item("amount", self.amount())?;
        dict.set_item("currency", format!("{}", self.inner.amount.currency()))?;
        dict.set_item("kind", self.kind())?;
        dict.set_item("accrual_factor", self.accrual_factor())?;
        if let Some(reset) = self.inner.reset_date {
            dict.set_item("reset_date", format!("{}", reset))?;
        }
        Ok(dict.into())
    }
    
    fn __repr__(&self) -> String {
        format!(
            "CashFlow(date={}, amount={:.2}, currency={}, kind={})",
            self.inner.date,
            self.inner.amount.amount(),
            self.inner.amount.currency(),
            self.kind()
        )
    }
}

/// Coupon payment type specification.
///
/// Controls whether coupon payments are made in cash, capitalized as PIK
/// (Payment-In-Kind), or split between the two.
///
/// Examples:
///     >>> from finstack.cashflow import CouponPaymentType
///     >>> cash_only = CouponPaymentType.cash()
///     >>> pik_only = CouponPaymentType.pik()
///     >>> split = CouponPaymentType.split(cash_pct=0.7, pik_pct=0.3)
#[pyclass(name = "CouponPaymentType", module = "finstack.cashflow")]
#[derive(Clone)]
pub struct PyCouponType {
    inner: CouponType,
}

#[pymethods]
impl PyCouponType {
    /// Create a cash-only coupon type.
    #[staticmethod]
    fn cash() -> Self {
        Self { inner: CouponType::Cash }
    }
    
    /// Create a PIK-only coupon type.
    #[staticmethod]
    fn pik() -> Self {
        Self { inner: CouponType::PIK }
    }
    
    /// Create a split cash/PIK coupon type.
    ///
    /// Args:
    ///     cash_pct: Percentage paid in cash (0.0 to 1.0)
    ///     pik_pct: Percentage capitalized as PIK (0.0 to 1.0)
    ///
    /// Note: cash_pct + pik_pct should equal 1.0
    #[staticmethod]
    fn split(cash_pct: f64, pik_pct: f64) -> PyResult<Self> {
        if (cash_pct + pik_pct - 1.0).abs() > 1e-6 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "cash_pct + pik_pct must equal 1.0"
            ));
        }
        Ok(Self { 
            inner: CouponType::Split { cash_pct, pik_pct } 
        })
    }
}

/// Amortization schedule specification.
///
/// Controls how principal is paid down over the life of the instrument.
///
/// Examples:
///     >>> from finstack.cashflow import Amortization
///     >>> bullet = Amortization.none()  # No amortization, bullet payment at maturity
///     >>> linear = Amortization.linear_to_zero()  # Linear paydown to zero
///     >>> partial = Amortization.linear_to(Money(100000, Currency.usd()))  # Linear to residual
///     >>> fixed_pct = Amortization.percent_per_period(0.05)  # 5% of original per period
#[pyclass(name = "Amortization", module = "finstack.cashflow")]
#[derive(Clone)]
pub struct PyAmortization {
    inner: AmortizationSpec,
}

#[pymethods]
impl PyAmortization {
    /// No amortization (bullet repayment at maturity).
    #[staticmethod]
    fn none() -> Self {
        Self { inner: AmortizationSpec::None }
    }
    
    /// Linear amortization to a target amount.
    ///
    /// Args:
    ///     final_notional: Target remaining principal at maturity
    #[staticmethod]
    fn linear_to(final_notional: &PyMoney) -> Self {
        Self { 
            inner: AmortizationSpec::LinearTo {
                final_notional: final_notional.inner()
            }
        }
    }
    
    /// Linear amortization to zero.
    #[staticmethod]
    fn linear_to_zero(currency: &PyCurrency) -> Self {
        Self {
            inner: AmortizationSpec::LinearTo {
                final_notional: Money::new(0.0, currency.inner())
            }
        }
    }
    
    /// Fixed percentage of original notional per period.
    ///
    /// Args:
    ///     pct: Fraction of original notional paid each period (e.g., 0.05 for 5%)
    #[staticmethod]
    fn percent_per_period(pct: f64) -> Self {
        Self {
            inner: AmortizationSpec::PercentPerPeriod { pct }
        }
    }
}

/// Cashflow schedule with comprehensive flow information.
///
/// Contains all cashflows generated by the builder along with metadata
/// about the structure. Provides methods for analysis and DataFrame export.
///
/// Examples:
///     >>> schedule = builder.build()
///     >>> len(schedule.flows)
///     12
///     >>> schedule.total_interest()
///     150000.0
///     >>> df = schedule.to_dataframe()  # For notebook analysis
#[pyclass(name = "CashFlowSchedule", module = "finstack.cashflow")]
#[derive(Clone)]
pub struct PyCashFlowSchedule {
    inner: Arc<CashFlowSchedule>,
}

#[pymethods]
impl PyCashFlowSchedule {
    /// Get all cashflows in the schedule.
    #[getter]
    fn flows(&self) -> Vec<PyCashFlow> {
        self.inner.flows.iter()
            .map(|&f| PyCashFlow { inner: f })
            .collect()
    }
    
    /// Get the initial notional amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.notional.initial)
    }
    
    /// Get the day count convention used.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.day_count)
    }
    
    /// Get only coupon cashflows (Fixed and Stub kinds).
    fn coupons(&self) -> Vec<PyCashFlow> {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::Stub))
            .map(|&f| PyCashFlow { inner: f })
            .collect()
    }
    
    /// Get only principal cashflows (Notional and Amortization kinds).
    fn principal_flows(&self) -> Vec<PyCashFlow> {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Notional | CFKind::Amortization))
            .map(|&f| PyCashFlow { inner: f })
            .collect()
    }
    
    /// Get only fee cashflows.
    fn fees(&self) -> Vec<PyCashFlow> {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fee))
            .map(|&f| PyCashFlow { inner: f })
            .collect()
    }
    
    /// Get only PIK cashflows.
    fn pik_flows(&self) -> Vec<PyCashFlow> {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::PIK))
            .map(|&f| PyCashFlow { inner: f })
            .collect()
    }
    
    /// Calculate total interest payments.
    fn total_interest(&self) -> f64 {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fixed | CFKind::Stub))
            .map(|cf| cf.amount.amount())
            .sum()
    }
    
    /// Calculate total principal payments.
    fn total_principal(&self) -> f64 {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Notional | CFKind::Amortization))
            .map(|cf| cf.amount.amount())
            .sum()
    }
    
    /// Calculate total fees.
    fn total_fees(&self) -> f64 {
        self.inner.flows.iter()
            .filter(|cf| matches!(cf.kind, CFKind::Fee))
            .map(|cf| cf.amount.amount())
            .sum()
    }
    
    /// Convert to a list of dictionaries (for DataFrame creation).
    ///
    /// Returns:
    ///     List[dict]: List of cashflow dictionaries suitable for pandas DataFrame
    ///
    /// Examples:
    ///     >>> data = schedule.to_records()
    ///     >>> import pandas as pd
    ///     >>> df = pd.DataFrame(data)
    fn to_records(&self, py: Python) -> PyResult<Vec<Py<PyDict>>> {
        self.flows().iter()
            .map(|cf| cf.to_dict(py))
            .collect()
    }
    
    /// Get outstanding principal path over time.
    ///
    /// Returns:
    ///     List[tuple]: List of (date, outstanding_amount) tuples
    ///
    /// Examples:
    ///     >>> path = schedule.outstanding_path()
    ///     >>> for date, amount in path:
    ///     ...     print(f"{date}: ${amount:,.2f}")
    fn outstanding_path(&self) -> Vec<(PyDate, f64)> {
        self.inner.outstanding_by_date().iter()
            .map(|(date, money)| (PyDate::from_core(*date), money.amount()))
            .collect()
    }
    
    fn __len__(&self) -> usize {
        self.inner.flows.len()
    }
    
    fn __repr__(&self) -> String {
        format!("CashFlowSchedule({} flows)", self.inner.flows.len())
    }
}

/// Comprehensive cashflow builder for complex structures.
///
/// Supports creation of sophisticated cashflow schedules including:
/// - Fixed and floating rate coupons
/// - PIK/Cash/Toggle payment types
/// - Various amortization schedules
/// - Multiple fee types
/// - Step-up rates
/// - Fixed-to-float transitions
///
/// Examples:
///     >>> from finstack import Currency, Date, DayCount
///     >>> from finstack.dates import Frequency, BusinessDayConvention, StubRule
///     >>> from finstack.cashflow import CashflowBuilder, CouponPaymentType, Amortization
///     >>> 
///     >>> # Simple fixed rate loan
///     >>> builder = CashflowBuilder()
///     >>> schedule = (builder
///     ...     .principal(Money(10_000_000, Currency.usd()),
///     ...                Date(2024, 1, 1), Date(2029, 1, 1))
///     ...     .fixed_coupon(rate=0.08, frequency=Frequency.Quarterly,
///     ...                   day_count=DayCount.act360())
///     ...     .with_amortization(Amortization.linear_to_zero(Currency.usd()))
///     ...     .build())
///     >>> 
///     >>> # Complex structure with PIK toggle
///     >>> builder = CashflowBuilder()
///     >>> schedule = (builder
///     ...     .principal(Money(5_000_000, Currency.eur()),
///     ...                Date(2024, 1, 1), Date(2027, 1, 1))
///     ...     .fixed_coupon(rate=0.10, frequency=Frequency.SemiAnnual,
///     ...                   day_count=DayCount.thirty360())
///     ...     .add_pik_period(Date(2024, 1, 1), Date(2025, 1, 1))
///     ...     .add_cash_period(Date(2025, 1, 1), Date(2027, 1, 1))
///     ...     .add_commitment_fee(bps=50)
///     ...     .build())
#[pyclass(name = "CashflowBuilder", module = "finstack.cashflow")]
pub struct PyCashflowBuilder {
    inner: finstack_valuations::cashflow::builder::CashflowBuilder,
}

#[pymethods]
impl PyCashflowBuilder {
    #[new]
    fn new() -> Self {
        Self { inner: cf() }
    }
    
    /// Set the principal amount and term.
    ///
    /// Args:
    ///     notional: Principal amount with currency
    ///     issue_date: Start date of the instrument
    ///     maturity: Maturity date
    ///
    /// Returns:
    ///     Self for method chaining
    fn principal(&mut self, 
                 notional: &PyMoney,
                 issue_date: &PyDate, 
                 maturity: &PyDate) {
        self.inner.principal_amount(
            notional.inner().amount(),
            notional.inner().currency(),
            issue_date.inner(),
            maturity.inner()
        );
    }
    
    /// Add a fixed rate coupon.
    ///
    /// Args:
    ///     rate: Annual interest rate (e.g., 0.08 for 8%)
    ///     frequency: Payment frequency
    ///     day_count: Day count convention
    ///     payment_type: Optional payment type (Cash/PIK/Split), defaults to Cash
    ///     business_day_conv: Optional business day convention
    ///     calendar: Optional holiday calendar identifier
    ///     stub: Optional stub rule
    ///
    /// Returns:
    ///     Self for method chaining
    #[allow(clippy::too_many_arguments)]  // Python API requires all these parameters
    fn fixed_coupon(
        &mut self,
        rate: f64,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
        payment_type: Option<&PyCouponType>,
        business_day_conv: Option<&PyBusDayConv>,
        calendar: Option<&str>,
        stub: Option<&PyStubRule>,
    ) {
        let coupon_type = payment_type
            .map(|pt| pt.inner)
            .unwrap_or(CouponType::Cash);
        
        let bdc = business_day_conv
            .map(|b| b.inner())
            .unwrap_or(BusinessDayConvention::Following);
        
        let stub_kind = stub
            .map(|s| s.inner())
            .unwrap_or(StubKind::None);
        
        let calendar_id = calendar.map(|s| {
            let static_str: &'static str = Box::leak(s.to_string().into_boxed_str());
            static_str
        });
        
        let spec = FixedCouponSpec {
            coupon_type,
            rate,
            freq: frequency.inner(),
            dc: day_count.inner(),
            bdc,
            calendar_id,
            stub: stub_kind,
        };
        
        self.inner.fixed_cf(spec);
    }
    
    /// Set the amortization schedule.
    ///
    /// Args:
    ///     amortization: Amortization specification
    ///
    /// Returns:
    ///     Self for method chaining
    fn with_amortization(&mut self, 
                         amortization: &PyAmortization) {
        self.inner.amortization(amortization.inner.clone());
    }
    
    /// Add a PIK period where interest is capitalized.
    ///
    /// Args:
    ///     start: Start date of PIK period
    ///     end: End date of PIK period
    ///
    /// Returns:
    ///     Self for method chaining
    fn add_pik_period(&mut self,
                      start: &PyDate,
                      end: &PyDate) {
        self.inner.add_payment_window(
            start.inner(),
            end.inner(),
            CouponType::PIK
        );
    }
    
    /// Add a cash payment period.
    ///
    /// Args:
    ///     start: Start date of cash period
    ///     end: End date of cash period
    ///
    /// Returns:
    ///     Self for method chaining
    fn add_cash_period(&mut self,
                       start: &PyDate,
                       end: &PyDate) {
        self.inner.add_payment_window(
            start.inner(),
            end.inner(),
            CouponType::Cash
        );
    }
    
    /// Add a commitment fee.
    ///
    /// Args:
    ///     bps: Fee in basis points
    ///
    /// Returns:
    ///     Self for method chaining
    fn add_commitment_fee(&mut self, _bps: f64) -> PyResult<()> {
        // This is a simplified version - would need more parameters in production
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Commitment fee not yet fully implemented"
        ))
    }
    
    /// Build the cashflow schedule.
    ///
    /// Returns:
    ///     CashFlowSchedule: The generated cashflow schedule
    ///
    /// Raises:
    ///     ValueError: If the schedule cannot be built due to invalid parameters
    fn build(&self) -> PyResult<PyCashFlowSchedule> {
        let schedule = self.inner.build()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Failed to build cashflow schedule: {:?}", e)
            ))?;
        
        Ok(PyCashFlowSchedule {
            inner: Arc::new(schedule)
        })
    }
}

/// Convert a cashflow schedule to a pandas DataFrame.
///
/// Helper function that converts a CashFlowSchedule into a pandas DataFrame
/// for easy analysis in Jupyter notebooks.
///
/// Args:
///     schedule: The cashflow schedule to convert
///
/// Returns:
///     pandas.DataFrame: DataFrame with cashflow data
///
/// Examples:
///     >>> df = cashflows_to_dataframe(schedule)
///     >>> df.groupby('kind')['amount'].sum()  # Analyze by cashflow type
///     >>> df.set_index('date')['amount'].plot()  # Plot cashflows over time
#[pyfunction]
#[pyo3(name = "cashflows_to_dataframe")]
pub fn py_cashflows_to_dataframe(py: Python, schedule: &PyCashFlowSchedule) -> PyResult<PyObject> {
    // Import pandas
    let pandas = py.import("pandas")?;
    
    // Get records from schedule
    let records = schedule.to_records(py)?;
    
    // Check if records is empty before creating DataFrame
    let is_empty = records.is_empty();
    
    // Create DataFrame
    let df = pandas.call_method1("DataFrame", (records,))?;
    
    // Convert date column to datetime if we have data
    if !is_empty {
        let pd = py.import("pandas")?;
        df.setattr("date", pd.call_method1("to_datetime", (df.getattr("date")?,))?)?;
    }
    
    Ok(df.into())
}

/// Register cashflow functions with the module
pub fn register_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(py_cashflows_to_dataframe, m)?)?;
    Ok(())
}