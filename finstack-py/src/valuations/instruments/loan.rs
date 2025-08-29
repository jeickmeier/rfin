//! Python bindings for loan instruments - simplified implementation.

use pyo3::prelude::*;
use finstack_core::F;
use finstack_valuations::instruments::fixed_income::loan::term_loan::{Loan, InterestSpec};

use crate::core::dates::PyDate;
use crate::core::money::PyMoney;
use std::sync::Arc;

/// Private credit loan instrument.
///
/// A loan represents a private credit instrument with customizable interest,
/// amortization, fees, and covenants. It supports various interest structures
/// including fixed, floating, PIK (Payment-In-Kind), and toggle structures.
///
/// Examples:
///     >>> from finstack import Date, Money, Currency
///     >>> from finstack.instruments import Loan
///     
///     # Create a simple fixed-rate term loan
///     >>> loan = Loan(
///     ...     id="LOAN-001",
///     ...     amount=Money(10_000_000, Currency.usd()),
///     ...     issue_date=Date(2025, 1, 1),
///     ...     maturity_date=Date(2030, 1, 1),
///     ...     rate=0.065  # 6.5% fixed rate
///     ... )
///     
///     # Loan with customizations
///     >>> loan = (Loan("LOAN-002", Money(5_000_000, Currency.eur()),
///     ...         Date(2025, 1, 1), Date(2028, 1, 1), 0.05)
///     ...     .with_borrower("ACME Corp")
///     ...     .with_discount_curve("EUR-OIS"))
#[pyclass(name = "Loan", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyLoan {
    inner: Arc<Loan>,
}

#[pymethods]
impl PyLoan {
    /// Create a new fixed-rate loan.
    ///
    /// Args:
    ///     id (str): Unique identifier for the loan
    ///     amount (Money): Principal loan amount
    ///     issue_date (Date): Loan origination/issue date
    ///     maturity_date (Date): Final maturity date
    ///     rate (float): Fixed interest rate (e.g., 0.065 for 6.5%)
    ///
    /// Returns:
    ///     Loan: A new loan instrument
    ///
    /// Raises:
    ///     ValueError: If maturity is before issue date
    ///
    /// Examples:
    ///     >>> loan = Loan(
    ///     ...     "LOAN-001",
    ///     ...     Money(10_000_000, Currency.usd()),
    ///     ...     Date(2025, 1, 1),
    ///     ...     Date(2030, 1, 1),
    ///     ...     0.065
    ///     ... )
    #[new]
    #[pyo3(signature = (id, amount, issue_date, maturity_date, rate))]
    fn new(
        id: String,
        amount: &PyMoney,
        issue_date: &PyDate,
        maturity_date: &PyDate,
        rate: F,
    ) -> PyResult<Self> {
        if maturity_date.inner() <= issue_date.inner() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Maturity date must be after issue date"
            ));
        }
        
        let interest = InterestSpec::Fixed { 
            rate, 
            step_ups: None 
        };
        
        let loan = Loan::new(
            id.as_str(),
            amount.inner(),
            issue_date.inner(),
            maturity_date.inner(),
            interest,
        );
        
        Ok(Self { inner: Arc::new(loan) })
    }
    
    /// The unique identifier of the loan.
    ///
    /// Returns:
    ///     str: The loan's identifier
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }
    
    /// The borrower entity ID.
    ///
    /// Returns:
    ///     str: The borrower's identifier
    #[getter]
    fn borrower(&self) -> String {
        self.inner.borrower.clone()
    }
    
    /// The original loan amount.
    ///
    /// Returns:
    ///     Money: The original principal amount
    #[getter]
    fn original_amount(&self, py: Python) -> PyResult<PyObject> {
        PyMoney::from_inner(self.inner.original_amount).into_py_any(py)
    }
    
    /// The current outstanding amount.
    ///
    /// Returns:
    ///     Money: The current outstanding principal
    #[getter]
    fn outstanding(&self, py: Python) -> PyResult<PyObject> {
        PyMoney::from_inner(self.inner.outstanding).into_py_any(py)
    }
    
    /// The loan issue/origination date.
    ///
    /// Returns:
    ///     Date: The issue date
    #[getter]
    fn issue_date(&self, py: Python) -> PyResult<PyObject> {
        PyDate::from_core(self.inner.issue_date).into_py_any(py)
    }
    
    /// The loan maturity date.
    ///
    /// Returns:
    ///     Date: The maturity date
    #[getter]
    fn maturity_date(&self, py: Python) -> PyResult<PyObject> {
        PyDate::from_core(self.inner.maturity_date).into_py_any(py)
    }
    
    /// Set the borrower entity ID.
    ///
    /// Args:
    ///     borrower (str): Borrower identifier
    ///
    /// Returns:
    ///     Loan: Self for method chaining
    ///
    /// Examples:
    ///     >>> loan.with_borrower("ACME Corp")
    fn with_borrower(&mut self, borrower: String) -> PyResult<Self> {
        let mut loan = (*self.inner).clone();
        loan.borrower = borrower;
        self.inner = Arc::new(loan);
        Ok(self.clone())
    }
    
    /// Set the discount curve ID for valuation.
    ///
    /// Args:
    ///     disc_id (str): Discount curve identifier
    ///
    /// Returns:
    ///     Loan: Self for method chaining
    ///
    /// Examples:
    ///     >>> loan.with_discount_curve("USD-OIS")
    fn with_discount_curve(&mut self, disc_id: String) -> PyResult<Self> {
        let mut loan = (*self.inner).clone();
        loan.disc_id = Box::leak(disc_id.into_boxed_str());
        self.inner = Arc::new(loan);
        Ok(self.clone())
    }
    
    /// Get string representation.
    fn __str__(&self) -> String {
        format!(
            "Loan(id='{}', amount={}, maturity={})", 
            self.inner.id,
            self.inner.original_amount.amount(),
            self.inner.maturity_date
        )
    }
    
    /// Get detailed representation.
    fn __repr__(&self) -> String {
        format!(
            "Loan(id='{}', borrower='{}', amount={}, outstanding={}, issue={}, maturity={})",
            self.inner.id,
            self.inner.borrower,
            self.inner.original_amount.amount(),
            self.inner.outstanding.amount(),
            self.inner.issue_date,
            self.inner.maturity_date
        )
    }
}

/// Draw event for DDTL/RCF.
///
/// Represents a draw or repayment event with date, amount, and optional metadata.
///
/// Examples:
///     >>> from finstack import Date, Money, Currency
///     >>> from finstack.instruments import DrawEvent
///     
///     >>> draw = DrawEvent(
///     ...     date=Date(2025, 6, 1),
///     ...     amount=Money(5_000_000, Currency.usd()),
///     ...     purpose="Working capital"
///     ... )
#[pyclass(name = "DrawEvent", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyDrawEvent {
    #[pyo3(get, set)]
    pub date: PyDate,
    #[pyo3(get, set)]
    pub amount: PyMoney,
    #[pyo3(get, set)]
    pub purpose: Option<String>,
    #[pyo3(get, set)]
    pub conditional: bool,
}

#[pymethods]
impl PyDrawEvent {
    #[new]
    #[pyo3(signature = (date, amount, purpose=None, conditional=false))]
    fn new(
        date: PyDate,
        amount: PyMoney,
        purpose: Option<String>,
        conditional: bool,
    ) -> Self {
        Self { date, amount, purpose, conditional }
    }
    
    fn __str__(&self) -> String {
        format!("DrawEvent(date={}, amount={}, conditional={})", 
            self.date.inner(), 
            self.amount.inner().amount(),
            self.conditional
        )
    }
}

/// Expected funding curve for DDTL pricing.
///
/// Represents expected future draws with optional probabilities for valuation.
///
/// Examples:
///     >>> from finstack.instruments import ExpectedFundingCurve, DrawEvent
///     
///     >>> curve = ExpectedFundingCurve([
///     ...     DrawEvent(Date(2025, 6, 1), Money(5_000_000, Currency.usd())),
///     ...     DrawEvent(Date(2025, 12, 1), Money(3_000_000, Currency.usd()))
///     ... ])
///     
///     # With probabilities
///     >>> curve = ExpectedFundingCurve(
///     ...     expected_draws=[draw1, draw2],
///     ...     draw_probabilities=[0.9, 0.7]  # 90% and 70% probability
///     ... )
#[pyclass(name = "ExpectedFundingCurve", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyExpectedFundingCurve {
    #[pyo3(get, set)]
    pub expected_draws: Vec<PyDrawEvent>,
    #[pyo3(get, set)]
    pub draw_probabilities: Option<Vec<f64>>,
}

#[pymethods]
impl PyExpectedFundingCurve {
    #[new]
    #[pyo3(signature = (expected_draws, draw_probabilities=None))]
    fn new(
        expected_draws: Vec<PyDrawEvent>,
        draw_probabilities: Option<Vec<f64>>,
    ) -> PyResult<Self> {
        if let Some(ref probs) = draw_probabilities {
            if probs.len() != expected_draws.len() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Number of probabilities must match number of draws"
                ));
            }
        }
        Ok(Self { expected_draws, draw_probabilities })
    }
    
    fn __str__(&self) -> String {
        format!("ExpectedFundingCurve({} draws)", self.expected_draws.len())
    }
}

/// Delayed-draw term loan (DDTL).
///
/// A delayed-draw term loan allows the borrower to draw funds over time
/// up to a commitment amount, with fees on both drawn and undrawn amounts.
///
/// Examples:
///     >>> from finstack.instruments import DelayedDrawTermLoan
///     >>> ddtl = DelayedDrawTermLoan(
///     ...     "DDTL-001",
///     ...     Money(20_000_000, Currency.usd()),
///     ...     Date(2025, 12, 31),  # commitment expiry
///     ...     Date(2030, 12, 31)   # maturity
///     ... )
///     
///     # Add expected funding curve for pricing
///     >>> ddtl.with_expected_draws([
///     ...     DrawEvent(Date(2025, 6, 1), Money(5_000_000, Currency.usd())),
///     ...     DrawEvent(Date(2025, 12, 1), Money(5_000_000, Currency.usd()))
///     ... ])
#[pyclass(name = "DelayedDrawTermLoan", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyDelayedDrawTermLoan {
    #[pyo3(get)]
    pub id: String,
    pub commitment: PyMoney,
    pub drawn: PyMoney,
    #[pyo3(get)]
    pub commitment_expiry: PyDate,
    #[pyo3(get)]
    pub maturity: PyDate,
    #[pyo3(get, set)]
    pub expected_funding_curve: Option<PyExpectedFundingCurve>,
}

#[pymethods]
impl PyDelayedDrawTermLoan {
    /// Create a new delayed-draw term loan.
    ///
    /// Args:
    ///     id (str): Unique identifier
    ///     commitment (Money): Total commitment amount
    ///     commitment_expiry (Date): Date when draw rights expire
    ///     maturity (Date): Final maturity date
    ///
    /// Returns:
    ///     DelayedDrawTermLoan: A new DDTL instrument
    #[new]
    fn new(
        id: String,
        commitment: PyMoney,
        commitment_expiry: PyDate,
        maturity: PyDate,
    ) -> Self {
        let drawn = PyMoney::from_inner(
            finstack_core::money::Money::new(0.0, commitment.inner().currency())
        );
        Self {
            id,
            commitment,
            drawn,
            commitment_expiry,
            maturity,
            expected_funding_curve: None,
        }
    }
    
    /// The total commitment amount.
    #[getter]
    fn commitment(&self) -> PyMoney {
        self.commitment.clone()
    }
    
    /// The currently drawn amount.
    #[getter]
    fn drawn(&self) -> PyMoney {
        self.drawn.clone()
    }
    
    /// Draw funds from the commitment.
    ///
    /// Args:
    ///     amount (float): Amount to draw
    ///
    /// Raises:
    ///     ValueError: If draw exceeds available commitment
    fn draw(&mut self, amount: f64) -> PyResult<()> {
        let current_drawn = self.drawn.inner().amount();
        let commitment_amount = self.commitment.inner().amount();
        
        if current_drawn + amount > commitment_amount {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Draw amount {} exceeds available commitment {}", 
                    amount, commitment_amount - current_drawn)
            ));
        }
        
        self.drawn = PyMoney::from_inner(
            finstack_core::money::Money::new(
                current_drawn + amount,
                self.drawn.inner().currency()
            )
        );
        Ok(())
    }
    
    /// Get undrawn amount available.
    fn undrawn(&self) -> PyMoney {
        PyMoney::from_inner(
            finstack_core::money::Money::new(
                self.commitment.inner().amount() - self.drawn.inner().amount(),
                self.commitment.inner().currency()
            )
        )
    }
    
    /// Set expected funding curve for pricing.
    ///
    /// Args:
    ///     curve (ExpectedFundingCurve): Expected future draws for pricing
    ///
    /// Returns:
    ///     DelayedDrawTermLoan: Self for method chaining
    fn with_expected_funding_curve(&mut self, curve: PyExpectedFundingCurve) -> PyResult<Self> {
        self.expected_funding_curve = Some(curve);
        Ok(self.clone())
    }
    
    /// Add expected draws for pricing.
    ///
    /// Args:
    ///     draws (List[DrawEvent]): Expected future draws
    ///
    /// Returns:
    ///     DelayedDrawTermLoan: Self for method chaining  
    fn with_expected_draws(&mut self, draws: Vec<PyDrawEvent>) -> PyResult<Self> {
        self.expected_funding_curve = Some(PyExpectedFundingCurve {
            expected_draws: draws,
            draw_probabilities: None,
        });
        Ok(self.clone())
    }
    
    /// Get string representation.
    fn __str__(&self) -> String {
        format!(
            "DelayedDrawTermLoan(id='{}', commitment={}, drawn={}, undrawn={})",
            self.id,
            self.commitment.inner().amount(),
            self.drawn.inner().amount(),
            self.commitment.inner().amount() - self.drawn.inner().amount()
        )
    }
}

/// Revolving credit facility (RCF).
///
/// A revolving credit facility allows drawing and repaying funds multiple times
/// within the commitment period, with fees based on utilization levels.
///
/// Examples:
///     >>> from finstack.instruments import RevolvingCreditFacility
///     >>> rcf = RevolvingCreditFacility(
///     ...     "RCF-001",
///     ...     Money(50_000_000, Currency.usd()),
///     ...     Date(2025, 1, 1),   # availability start
///     ...     Date(2027, 12, 31)  # maturity
///     ... )
///     
///     # Add expected funding curve
///     >>> rcf.with_expected_events([
///     ...     DrawEvent(Date(2025, 3, 1), Money(10_000_000, Currency.usd())),   # draw
///     ...     DrawEvent(Date(2025, 9, 1), Money(-5_000_000, Currency.usd())),  # repay
///     ...     DrawEvent(Date(2026, 3, 1), Money(15_000_000, Currency.usd()))   # draw
///     ... ])
#[pyclass(name = "RevolvingCreditFacility", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyRevolvingCreditFacility {
    #[pyo3(get)]
    pub id: String,
    pub commitment: PyMoney,
    pub drawn: PyMoney,
    #[pyo3(get)]
    pub availability_start: PyDate,
    #[pyo3(get)]
    pub maturity: PyDate,
    #[pyo3(get, set)]
    pub expected_funding_curve: Option<PyExpectedFundingCurve>,
}

#[pymethods]
impl PyRevolvingCreditFacility {
    /// Create a new revolving credit facility.
    ///
    /// Args:
    ///     id (str): Unique identifier
    ///     commitment (Money): Total commitment amount
    ///     availability_start (Date): Start of availability period
    ///     maturity (Date): Final maturity date
    ///
    /// Returns:
    ///     RevolvingCreditFacility: A new RCF instrument
    #[new]
    fn new(
        id: String,
        commitment: PyMoney,
        availability_start: PyDate,
        maturity: PyDate,
    ) -> Self {
        let drawn = PyMoney::from_inner(
            finstack_core::money::Money::new(0.0, commitment.inner().currency())
        );
        Self {
            id,
            commitment,
            drawn,
            availability_start,
            maturity,
            expected_funding_curve: None,
        }
    }
    
    /// The total commitment amount.
    #[getter]
    fn commitment(&self) -> PyMoney {
        self.commitment.clone()
    }
    
    /// The currently drawn amount.
    #[getter]
    fn drawn(&self) -> PyMoney {
        self.drawn.clone()
    }
    
    /// Draw funds from the facility.
    ///
    /// Args:
    ///     amount (float): Amount to draw
    ///
    /// Raises:
    ///     ValueError: If draw exceeds available commitment
    fn draw(&mut self, amount: f64) -> PyResult<()> {
        let current_drawn = self.drawn.inner().amount();
        let commitment_amount = self.commitment.inner().amount();
        
        let new_drawn = current_drawn + amount;
        if new_drawn > commitment_amount {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                format!("Draw would exceed commitment: {} > {}", 
                    new_drawn, commitment_amount)
            ));
        }
        
        if new_drawn < 0.0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Cannot have negative drawn amount"
            ));
        }
        
        self.drawn = PyMoney::from_inner(
            finstack_core::money::Money::new(new_drawn, self.drawn.inner().currency())
        );
        Ok(())
    }
    
    /// Repay funds to the facility.
    ///
    /// Args:
    ///     amount (float): Amount to repay
    ///
    /// Raises:
    ///     ValueError: If repayment exceeds drawn amount
    fn repay(&mut self, amount: f64) -> PyResult<()> {
        self.draw(-amount)
    }
    
    /// Get undrawn amount available.
    fn undrawn(&self) -> PyMoney {
        PyMoney::from_inner(
            finstack_core::money::Money::new(
                self.commitment.inner().amount() - self.drawn.inner().amount(),
                self.commitment.inner().currency()
            )
        )
    }
    
    /// Get utilization percentage (0.0 to 1.0).
    fn utilization(&self) -> f64 {
        let commitment_amount = self.commitment.inner().amount();
        if commitment_amount > 0.0 {
            self.drawn.inner().amount() / commitment_amount
        } else {
            0.0
        }
    }
    
    /// Set expected funding curve for pricing.
    ///
    /// Args:
    ///     curve (ExpectedFundingCurve): Expected future draws/repayments for pricing
    ///
    /// Returns:
    ///     RevolvingCreditFacility: Self for method chaining
    fn with_expected_funding_curve(&mut self, curve: PyExpectedFundingCurve) -> PyResult<Self> {
        self.expected_funding_curve = Some(curve);
        Ok(self.clone())
    }
    
    /// Add expected events for pricing.
    ///
    /// Args:
    ///     events (List[DrawEvent]): Expected future draws/repayments
    ///         Positive amounts are draws, negative are repayments
    ///
    /// Returns:
    ///     RevolvingCreditFacility: Self for method chaining  
    fn with_expected_events(&mut self, events: Vec<PyDrawEvent>) -> PyResult<Self> {
        self.expected_funding_curve = Some(PyExpectedFundingCurve {
            expected_draws: events,
            draw_probabilities: None,
        });
        Ok(self.clone())
    }
    
    /// Get string representation.
    fn __str__(&self) -> String {
        format!(
            "RevolvingCreditFacility(id='{}', commitment={}, drawn={}, utilization={:.1}%)",
            self.id,
            self.commitment.inner().amount(),
            self.drawn.inner().amount(),
            self.utilization() * 100.0
        )
    }
}

// Helper trait to convert Python objects
trait IntoPyAny {
    fn into_py_any(self, py: Python) -> PyResult<PyObject>;
}

impl IntoPyAny for PyMoney {
    fn into_py_any(self, py: Python) -> PyResult<PyObject> {
        let money_class = py.import("finstack")?.getattr("Money")?;
        money_class.call1((self.inner().amount(), format!("{}", self.inner().currency())))
            .map(|obj| obj.into())
    }
}

impl IntoPyAny for PyDate {
    fn into_py_any(self, py: Python) -> PyResult<PyObject> {
        let date_class = py.import("finstack")?.getattr("Date")?;
        let d = self.inner();
        date_class.call1((d.year(), d.month() as u8, d.day()))
            .map(|obj| obj.into())
    }
}