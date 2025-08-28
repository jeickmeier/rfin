//! Python bindings for bond instruments.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::str::FromStr;
use finstack_valuations::instruments::bond::Bond;
use crate::core::{
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
    
    /// Calculate years to maturity from a given date.
    ///
    /// Args:
    ///     as_of (Date): The valuation date
    ///
    /// Returns:
    ///     float: Years remaining to maturity
    ///
    /// Examples:
    ///     >>> bond.years_to_maturity(Date(2024, 1, 1))
    ///     4.0  # For a bond maturing in 2028
    fn years_to_maturity(&self, as_of: &PyDate) -> PyResult<f64> {
        let as_of_inner = as_of.inner();
        
        if as_of_inner >= self.inner.maturity {
            return Ok(0.0);
        }
        
        let years = self.inner.dc.year_fraction(as_of_inner, self.inner.maturity)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate years to maturity: {:?}", e)
            ))?;
            
        Ok(years)
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
    
    /// Price the bond using market data.
    ///
    /// Calculates the present value and risk metrics for the bond using
    /// the provided market context and valuation date.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///     metrics: Optional list of metric names to compute
    ///
    /// Returns:
    ///     ValuationResult: Pricing result with PV and metrics
    ///
    /// Examples:
    ///     >>> from finstack.market_data import MarketContext
    ///     >>> from finstack import Date
    ///     >>> 
    ///     >>> context = MarketContext()
    ///     >>> # ... add curves to context ...
    ///     >>> 
    ///     >>> result = bond.price(context, Date(2024, 1, 1))
    ///     >>> print(f"PV: ${result.value.amount:,.2f}")
    ///     >>> print(f"YTM: {result.get_metric('Ytm', 0):.2%}")
    fn price(
        &self, 
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<crate::valuations::results::PyValuationResult> {
        use finstack_valuations::traits::Priceable;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        // Call the Rust pricing implementation
        let result = self.inner.price(&curves, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to price bond: {:?}", e)
            ))?;
        
        Ok(crate::valuations::results::PyValuationResult::from_inner(result))
    }
    
    /// Calculate the present value only (no metrics).
    ///
    /// This is faster than `price()` when you only need the PV.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     Money: Present value of the bond
    ///
    /// Examples:
    ///     >>> pv = bond.value(context, Date(2024, 1, 1))
    ///     >>> print(f"PV: ${pv.amount:,.2f}")
    fn value(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<PyMoney> {
        use finstack_valuations::traits::Priceable;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        // Call the Rust value implementation
        let value = self.inner.value(&curves, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate bond value: {:?}", e)
            ))?;
        
        Ok(PyMoney::from_inner(value))
    }
    
    /// Calculate yield to maturity (YTM).
    ///
    /// Requires a quoted clean price to be set on the bond. Returns the yield
    /// that equates the bond's market price to its present value.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Yield to maturity as a decimal (e.g., 0.045 for 4.5%)
    ///
    /// Examples:
    ///     >>> # Bond with quoted price for YTM calculation
    ///     >>> bond_with_price = Bond(..., quoted_clean_price=98.5)
    ///     >>> ytm = bond_with_price.yield_to_maturity(context, Date(2024, 1, 1))
    ///     >>> print(f"YTM: {ytm:.2%}")
    fn yield_to_maturity(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        if self.inner.quoted_clean.is_none() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Bond must have a quoted clean price to calculate YTM"
            ));
        }
        
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["ytm".to_string()]
        )?;
        
        Ok(result.get_metric("ytm", None))
    }
    
    /// Calculate modified duration.
    ///
    /// Modified duration measures price sensitivity to yield changes.
    /// It represents the approximate percentage change in bond price
    /// for a 1% change in yield.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Modified duration in years
    ///
    /// Examples:
    ///     >>> duration = bond.modified_duration(context, Date(2024, 1, 1))
    ///     >>> print(f"Modified duration: {duration:.2f} years")
    fn modified_duration(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["duration_mod".to_string()]
        )?;
        
        Ok(result.get_metric("duration_mod", None))
    }
    
    /// Calculate Macaulay duration.
    ///
    /// Macaulay duration is the weighted average time to receive the bond's
    /// cash flows, where weights are the present values of each cash flow.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Macaulay duration in years
    ///
    /// Examples:
    ///     >>> mac_dur = bond.macaulay_duration(context, Date(2024, 1, 1))
    ///     >>> print(f"Macaulay duration: {mac_dur:.2f} years")
    fn macaulay_duration(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["duration_mac".to_string()]
        )?;
        
        Ok(result.get_metric("duration_mac", None))
    }
    
    /// Calculate bond convexity.
    ///
    /// Convexity measures the curvature of the price-yield relationship.
    /// It provides a better approximation of price changes for large yield moves
    /// than duration alone.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Bond convexity
    ///
    /// Examples:
    ///     >>> convexity = bond.convexity(context, Date(2024, 1, 1))
    ///     >>> print(f"Convexity: {convexity:.2f}")
    fn convexity(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["convexity".to_string()]
        )?;
        
        Ok(result.get_metric("convexity", None))
    }
    
    /// Calculate accrued interest.
    ///
    /// Computes the interest that has accrued since the last coupon payment
    /// up to the valuation date.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Accrued interest amount in the bond's currency
    ///
    /// Examples:
    ///     >>> accrued = bond.accrued_interest(context, Date(2024, 3, 15))
    ///     >>> print(f"Accrued interest: ${accrued:,.2f}")
    fn accrued_interest(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["accrued".to_string()]
        )?;
        
        Ok(result.get_metric("accrued", None))
    }
    
    /// Calculate clean price.
    ///
    /// For bonds with quoted clean prices, returns the quoted price.
    /// Otherwise, calculates clean price as dirty price minus accrued interest.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Clean price as percentage of par (e.g., 98.5 for 98.5%)
    ///
    /// Examples:
    ///     >>> clean = bond.clean_price(context, Date(2024, 1, 1))
    ///     >>> print(f"Clean price: {clean:.2f}%")
    fn clean_price(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["clean_price".to_string()]
        )?;
        
        Ok(result.get_metric("clean_price", None))
    }
    
    /// Calculate dirty price.
    ///
    /// Dirty price includes accrued interest and represents the actual
    /// amount paid for the bond in a transaction.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Dirty price as percentage of par (e.g., 101.2 for 101.2%)
    ///
    /// Examples:
    ///     >>> dirty = bond.dirty_price(context, Date(2024, 1, 1))
    ///     >>> print(f"Dirty price: {dirty:.2f}%")
    fn dirty_price(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["dirty_price".to_string()]
        )?;
        
        Ok(result.get_metric("dirty_price", None))
    }
    
    /// Calculate credit spread sensitivity (CS01).
    ///
    /// CS01 measures the change in bond value for a 1 basis point
    /// change in the credit spread. This is important for credit
    /// risk management and portfolio hedging.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: CS01 value (change in bond value for 1bp spread change)
    ///
    /// Examples:
    ///     >>> cs01 = bond.cs01(context, Date(2024, 1, 1))
    ///     >>> print(f"CS01: ${cs01:.2f}")
    fn cs01(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["cs01".to_string()]
        )?;
        
        Ok(result.get_metric("cs01", None))
    }
    
    /// Calculate yield to worst (YTW).
    ///
    /// For callable or puttable bonds, calculates the yield assuming
    /// the worst-case scenario for the bondholder (lowest yield).
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: Yield to worst as a decimal
    ///
    /// Examples:
    ///     >>> # For a callable bond
    ///     >>> ytw = callable_bond.yield_to_worst(context, Date(2024, 1, 1))
    ///     >>> print(f"YTW: {ytw:.2%}")
    fn yield_to_worst(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate
    ) -> PyResult<f64> {
        let result = self.price_with_metrics(
            market_context, 
            as_of, 
            vec!["ytw".to_string()]
        )?;
        
        Ok(result.get_metric("ytw", None))
    }
    
    /// Calculate comprehensive bond metrics in one call.
    ///
    /// Efficiently computes multiple bond metrics in a single operation
    /// by leveraging the metrics framework's dependency management.
    ///
    /// Args:
    ///     market_context: Market data including discount curves
    ///     as_of: Valuation date
    ///     metrics: Optional list of metric names to compute. If None, 
    ///             computes standard bond metrics: ytm, duration_mod, convexity, accrued
    ///
    /// Returns:
    ///     dict: Dictionary mapping metric names to their computed values
    ///
    /// Examples:
    ///     >>> # Calculate standard metrics
    ///     >>> metrics = bond.calculate_metrics(context, Date(2024, 1, 1))
    ///     >>> print(f"YTM: {metrics['ytm']:.2%}")
    ///     >>> print(f"Modified Duration: {metrics['duration_mod']:.2f}")
    ///     >>> print(f"Convexity: {metrics['convexity']:.2f}")
    ///     >>> 
    ///     >>> # Calculate specific metrics
    ///     >>> custom_metrics = bond.calculate_metrics(
    ///     ...     context, Date(2024, 1, 1), 
    ///     ...     metrics=["ytm", "clean_price", "cs01"]
    ///     ... )
    fn calculate_metrics(
        &self,
        py: Python,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
        metrics: Option<Vec<String>>
    ) -> PyResult<Py<PyDict>> {
        // Use standard bond metrics if none specified
        let metric_names = metrics.unwrap_or_else(|| {
            vec![
                "ytm".to_string(),
                "duration_mod".to_string(), 
                "convexity".to_string(),
                "accrued".to_string()
            ]
        });
        
        let result = self.price_with_metrics(market_context, as_of, metric_names)?;
        result.measures_dict(py)
    }
    
    /// Helper method to call price_with_metrics.
    fn price_with_metrics(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
        metrics: Vec<String>
    ) -> PyResult<crate::valuations::results::PyValuationResult> {
        use finstack_valuations::traits::Priceable;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        // Convert metric names to MetricId
        let metric_ids: Vec<finstack_valuations::metrics::MetricId> = metrics.iter()
            .map(|name| finstack_valuations::metrics::MetricId::from_str(name)
                .unwrap_or_else(|_| finstack_valuations::metrics::MetricId::custom(name)))
            .collect();
        
        // Call the Rust price_with_metrics implementation
        let result = self.inner.price_with_metrics(&curves, as_of_date, &metric_ids)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate bond metrics: {:?}", e)
            ))?;
        
        Ok(crate::valuations::results::PyValuationResult::from_inner(result))
    }
}
