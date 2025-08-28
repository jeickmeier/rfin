//! Python bindings for bond instruments.

use pyo3::prelude::*;
use finstack_valuations::instruments::bond::Bond;
use crate::{
    dates::{PyDate, PyDayCount, PyFrequency},
    money::PyMoney,
};
use std::sync::Arc;

/// Bond instrument for fixed-income valuation.
///
/// A Bond represents a fixed-income security that pays periodic coupons
/// and returns principal at maturity. It supports various day count conventions,
/// payment frequencies, and can include embedded options (calls/puts).
///
/// The bond can be priced using discount curves from a market context,
/// and various metrics can be calculated including yield to maturity (YTM),
/// duration, convexity, and spread measures.
///
/// Examples:
///     >>> from finstack import Currency, Date, DayCount
///     >>> from finstack.dates import Frequency
///     >>> from finstack.instruments import Bond
///     >>> from finstack.money import Money
///     
///     # Create a 5-year corporate bond
///     >>> bond = Bond(
///     ...     id="AAPL-5Y-2028",
///     ...     notional=Money(1000000, Currency.usd()),
///     ...     coupon=0.045,  # 4.5% annual coupon
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.thirty360(),
///     ...     issue_date=Date(2023, 1, 15),
///     ...     maturity=Date(2028, 1, 15),
///     ...     discount_curve="USD-OIS"
///     ... )
///     
///     # Bond with quoted price for yield calculation
///     >>> bond_with_price = Bond(
///     ...     id="GOVT-10Y",
///     ...     notional=Money(1000000, Currency.usd()),
///     ...     coupon=0.03,
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.act_act(),
///     ...     issue_date=Date(2020, 1, 1),
///     ...     maturity=Date(2030, 1, 1),
///     ...     discount_curve="USD-TREASURY",
///     ...     quoted_clean_price=98.5  # Trading at 98.5% of par
///     ... )
#[pyclass(name = "Bond", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyBond {
    inner: Arc<Bond>,
}

#[pymethods]
impl PyBond {
    /// Create a new bond instrument.
    ///
    /// Args:
    ///     id (str): Unique identifier for the bond
    ///     notional (Money): Principal amount of the bond
    ///     coupon (float): Annual coupon rate (e.g., 0.05 for 5%)
    ///     frequency (Frequency): Coupon payment frequency
    ///     day_count (DayCount): Day count convention for accrual
    ///     issue_date (Date): Issue date of the bond
    ///     maturity (Date): Maturity date of the bond
    ///     discount_curve (str): Identifier of the discount curve to use for pricing
    ///     quoted_clean_price (float, optional): Quoted clean price (% of par) for YTM calculation
    ///
    /// Returns:
    ///     Bond: A new bond instrument
    ///
    /// Raises:
    ///     ValueError: If the bond parameters are invalid (e.g., maturity before issue)
    ///
    /// Examples:
    ///     >>> bond = Bond(
    ///     ...     id="CORP-5Y",
    ///     ...     notional=Money(1000000, Currency.usd()),
    ///     ...     coupon=0.05,
    ///     ...     frequency=Frequency.SemiAnnual,
    ///     ...     day_count=DayCount.thirty360(),
    ///     ...     issue_date=Date(2024, 1, 1),
    ///     ...     maturity=Date(2029, 1, 1),
    ///     ...     discount_curve="USD-OIS"
    ///     ... )
    #[new]
    #[pyo3(signature = (id, notional, coupon, frequency, day_count, issue_date, maturity, discount_curve, quoted_clean_price=None))]
    #[allow(clippy::too_many_arguments)]  // Python API requires all these parameters
    fn new(
        id: String,
        notional: &PyMoney,
        coupon: f64,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
        issue_date: &PyDate,
        maturity: &PyDate,
        discount_curve: &str,
        quoted_clean_price: Option<f64>,
    ) -> PyResult<Self> {
        // Validate dates
        if maturity.inner() <= issue_date.inner() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Maturity date must be after issue date"
            ));
        }
        
        // Create static str for discount curve id
        let disc_id: &'static str = Box::leak(discount_curve.to_string().into_boxed_str());
        
        let bond = Bond {
            id: id.clone(),
            notional: notional.inner(),
            coupon,
            freq: frequency.inner(),
            dc: day_count.inner(),
            issue: issue_date.inner(),
            maturity: maturity.inner(),
            disc_id,
            quoted_clean: quoted_clean_price,
            call_put: None,
            amortization: None,
        };
        
        Ok(Self {
            inner: Arc::new(bond),
        })
    }
    
    /// The unique identifier of the bond.
    ///
    /// Returns:
    ///     str: The bond's identifier
    ///
    /// Examples:
    ///     >>> bond.id
    ///     'CORP-5Y-2029'
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }
    
    /// The principal amount of the bond.
    ///
    /// Returns:
    ///     Money: The notional/principal amount
    ///
    /// Examples:
    ///     >>> bond.notional
    ///     Money(1000000.00, Currency('USD'))
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.notional)
    }
    
    /// The annual coupon rate.
    ///
    /// Returns:
    ///     float: The coupon rate (e.g., 0.05 for 5%)
    ///
    /// Examples:
    ///     >>> bond.coupon
    ///     0.05
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }
    
    /// The coupon payment frequency.
    ///
    /// Returns:
    ///     Frequency: The payment frequency
    ///
    /// Examples:
    ///     >>> bond.frequency
    ///     Frequency.SemiAnnual
    #[getter]
    fn frequency(&self) -> PyFrequency {
        PyFrequency::from_inner(self.inner.freq)
    }
    
    /// The day count convention.
    ///
    /// Returns:
    ///     DayCount: The day count convention used for accrual
    ///
    /// Examples:
    ///     >>> bond.day_count
    ///     DayCount.Thirty360
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.dc)
    }
    
    /// The issue date of the bond.
    ///
    /// Returns:
    ///     Date: The date when the bond was issued
    ///
    /// Examples:
    ///     >>> bond.issue_date
    ///     Date('2024-01-01')
    #[getter]
    fn issue_date(&self) -> PyDate {
        PyDate::from_core(self.inner.issue)
    }
    
    /// The maturity date of the bond.
    ///
    /// Returns:
    ///     Date: The date when the bond matures
    ///
    /// Examples:
    ///     >>> bond.maturity
    ///     Date('2029-01-01')
    #[getter]
    fn maturity(&self) -> PyDate {
        PyDate::from_core(self.inner.maturity)
    }
    
    /// The discount curve identifier used for pricing.
    ///
    /// Returns:
    ///     str: The identifier of the discount curve
    ///
    /// Examples:
    ///     >>> bond.discount_curve
    ///     'USD-OIS'
    #[getter]
    fn discount_curve(&self) -> &str {
        self.inner.disc_id
    }
    
    /// The quoted clean price (if provided).
    ///
    /// Returns:
    ///     float or None: The quoted clean price as percentage of par, or None if not provided
    ///
    /// Examples:
    ///     >>> bond.quoted_clean_price
    ///     98.5
    #[getter]
    fn quoted_clean_price(&self) -> Option<f64> {
        self.inner.quoted_clean
    }
    
    /// Calculate the number of coupon payments remaining.
    ///
    /// Args:
    ///     as_of (Date): The valuation date
    ///
    /// Returns:
    ///     int: The number of remaining coupon payments
    ///
    /// Examples:
    ///     >>> bond.num_coupons_remaining(Date(2024, 7, 1))
    ///     8  # For a semi-annual bond with 4 years remaining
    fn num_coupons_remaining(&self, as_of: &PyDate) -> PyResult<usize> {
        let as_of_inner = as_of.inner();
        
        if as_of_inner >= self.inner.maturity {
            return Ok(0);
        }
        
        if as_of_inner <= self.inner.issue {
            // Full term remaining
            let years = self.inner.dc.year_fraction(self.inner.issue, self.inner.maturity)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to calculate year fraction: {:?}", e)
                ))?;
            
            let payments_per_year = match self.inner.freq.months() {
                Some(12) => 1.0,
                Some(6) => 2.0,
                Some(3) => 4.0,
                Some(1) => 12.0,
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Unsupported frequency for coupon count"
                )),
            };
            
            Ok((years * payments_per_year).ceil() as usize)
        } else {
            // Calculate based on remaining time
            let years_remaining = self.inner.dc.year_fraction(as_of_inner, self.inner.maturity)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Failed to calculate year fraction: {:?}", e)
                ))?;
            
            let payments_per_year = match self.inner.freq.months() {
                Some(12) => 1.0,
                Some(6) => 2.0,
                Some(3) => 4.0,
                Some(1) => 12.0,
                _ => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Unsupported frequency for coupon count"
                )),
            };
            
            Ok((years_remaining * payments_per_year).ceil() as usize)
        }
    }
    
    /// String representation of the bond.
    ///
    /// Returns:
    ///     str: A formatted string describing the bond
    ///
    /// Examples:
    ///     >>> str(bond)
    ///     "Bond('CORP-5Y', 5.00% Semi-Annual, Matures 2029-01-01)"
    fn __repr__(&self) -> String {
        let freq_str = match self.inner.freq.months() {
            Some(12) => "Annual",
            Some(6) => "Semi-Annual",
            Some(3) => "Quarterly",
            Some(1) => "Monthly",
            _ => "Custom",
        };
        
        format!(
            "Bond('{}', {:.2}% {}, Matures {})",
            self.inner.id,
            self.inner.coupon * 100.0,
            freq_str,
            self.inner.maturity
        )
    }
    
    /// Compare bonds by ID for equality.
    fn __eq__(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}
