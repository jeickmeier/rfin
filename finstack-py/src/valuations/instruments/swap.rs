//! Python bindings for interest rate swap instruments.

use pyo3::prelude::*;
use finstack_valuations::instruments::irs::{InterestRateSwap, PayReceive, FixedLegSpec, FloatLegSpec};
use finstack_core::{
    dates::{BusinessDayConvention, StubKind},
};
use crate::core::{
    dates::{PyDate, PyDayCount, PyFrequency, PyBusDayConv, PyStubRule},
    money::PyMoney,
};
use std::sync::Arc;

/// Direction of an interest rate swap from the perspective of fixed rate.
///
/// Determines whether you pay or receive the fixed rate leg.
///
/// Examples:
///     >>> from finstack.instruments import PayReceive
///     >>> 
///     >>> # Pay fixed, receive floating (typical hedging position)
///     >>> direction = PayReceive.PayFixed
///     >>> 
///     >>> # Receive fixed, pay floating (typical investment position)
///     >>> direction = PayReceive.ReceiveFixed
#[pyclass(name = "PayReceive", module = "finstack.instruments")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PyPayReceive {
    /// Pay fixed rate, receive floating rate
    PayFixed,
    /// Receive fixed rate, pay floating rate
    ReceiveFixed,
}

impl From<PyPayReceive> for PayReceive {
    fn from(value: PyPayReceive) -> Self {
        match value {
            PyPayReceive::PayFixed => PayReceive::PayFixed,
            PyPayReceive::ReceiveFixed => PayReceive::ReceiveFixed,
        }
    }
}

/// Specification for a fixed rate leg of an interest rate swap.
///
/// Defines all parameters needed to generate the fixed leg cashflows.
///
/// Examples:
///     >>> from finstack.instruments import FixedLeg
///     >>> from finstack import Date, DayCount
///     >>> from finstack.dates import Frequency, BusDayConvention, StubRule
///     >>> 
///     >>> fixed_leg = FixedLeg(
///     ...     discount_curve="USD-OIS",
///     ...     rate=0.025,  # 2.5% fixed rate
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.thirty360(),
///     ...     business_day_conv=BusDayConvention.ModifiedFollowing,
///     ...     start_date=Date(2024, 1, 1),
///     ...     end_date=Date(2029, 1, 1)
///     ... )
#[pyclass(name = "FixedLeg", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyFixedLeg {
    inner: FixedLegSpec,
}

#[pymethods]
impl PyFixedLeg {
    #[new]
    #[pyo3(signature = (
        discount_curve,
        rate,
        frequency,
        day_count,
        start_date,
        end_date,
        business_day_conv = None,
        calendar_id = None,
        stub = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        discount_curve: &str,
        rate: f64,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
        start_date: &PyDate,
        end_date: &PyDate,
        business_day_conv: Option<&PyBusDayConv>,
        calendar_id: Option<&str>,
        stub: Option<&PyStubRule>,
    ) -> PyResult<Self> {
        let bdc = business_day_conv
            .map(|b| b.inner())
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
            
        let stub_kind = stub
            .map(|s| s.inner())
            .unwrap_or(StubKind::None);
        
        // Convert to static string for curve ID
        let disc_id: &'static str = Box::leak(discount_curve.to_string().into_boxed_str());
        let cal_id = calendar_id.map(|s| Box::leak(s.to_string().into_boxed_str()) as &'static str);
        
        Ok(Self {
            inner: FixedLegSpec {
                disc_id,
                rate,
                freq: frequency.inner(),
                dc: day_count.inner(),
                bdc,
                calendar_id: cal_id,
                stub: stub_kind,
                start: start_date.inner(),
                end: end_date.inner(),
            }
        })
    }
    
    #[getter]
    fn discount_curve(&self) -> &str {
        self.inner.disc_id
    }
    
    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }
    
    #[getter]
    fn frequency(&self) -> PyFrequency {
        PyFrequency::from_inner(self.inner.freq)
    }
    
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.dc)
    }
    
    #[getter]
    fn start_date(&self) -> PyDate {
        PyDate::from_core(self.inner.start)
    }
    
    #[getter]
    fn end_date(&self) -> PyDate {
        PyDate::from_core(self.inner.end)
    }
    
    fn __repr__(&self) -> String {
        format!(
            "FixedLeg(rate={:.2}%, freq={:?}, start={}, end={})",
            self.inner.rate * 100.0,
            self.inner.freq,
            self.inner.start,
            self.inner.end
        )
    }
}

/// Specification for a floating rate leg of an interest rate swap.
///
/// Defines all parameters needed to generate the floating leg cashflows.
///
/// Examples:
///     >>> from finstack.instruments import FloatLeg
///     >>> from finstack import Date, DayCount
///     >>> from finstack.dates import Frequency, BusDayConvention
///     >>> 
///     >>> float_leg = FloatLeg(
///     ...     discount_curve="USD-OIS",
///     ...     forward_curve="USD-SOFR-3M",
///     ...     spread_bp=10,  # 10 basis points spread
///     ...     frequency=Frequency.Quarterly,
///     ...     day_count=DayCount.act360(),
///     ...     start_date=Date(2024, 1, 1),
///     ...     end_date=Date(2029, 1, 1)
///     ... )
#[pyclass(name = "FloatLeg", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyFloatLeg {
    inner: FloatLegSpec,
}

#[pymethods]
impl PyFloatLeg {
    #[new]
    #[pyo3(signature = (
        discount_curve,
        forward_curve,
        spread_bp,
        frequency,
        day_count,
        start_date,
        end_date,
        business_day_conv = None,
        calendar_id = None,
        stub = None
    ))]
    #[allow(clippy::too_many_arguments)]
    fn new(
        discount_curve: &str,
        forward_curve: &str,
        spread_bp: f64,
        frequency: &PyFrequency,
        day_count: &PyDayCount,
        start_date: &PyDate,
        end_date: &PyDate,
        business_day_conv: Option<&PyBusDayConv>,
        calendar_id: Option<&str>,
        stub: Option<&PyStubRule>,
    ) -> PyResult<Self> {
        let bdc = business_day_conv
            .map(|b| b.inner())
            .unwrap_or(BusinessDayConvention::ModifiedFollowing);
            
        let stub_kind = stub
            .map(|s| s.inner())
            .unwrap_or(StubKind::None);
        
        // Convert to static strings for curve IDs
        let disc_id: &'static str = Box::leak(discount_curve.to_string().into_boxed_str());
        let fwd_id: &'static str = Box::leak(forward_curve.to_string().into_boxed_str());
        let cal_id = calendar_id.map(|s| Box::leak(s.to_string().into_boxed_str()) as &'static str);
        
        Ok(Self {
            inner: FloatLegSpec {
                disc_id,
                fwd_id,
                spread_bp,
                freq: frequency.inner(),
                dc: day_count.inner(),
                bdc,
                calendar_id: cal_id,
                stub: stub_kind,
                start: start_date.inner(),
                end: end_date.inner(),
            }
        })
    }
    
    #[getter]
    fn discount_curve(&self) -> &str {
        self.inner.disc_id
    }
    
    #[getter]
    fn forward_curve(&self) -> &str {
        self.inner.fwd_id
    }
    
    #[getter]
    fn spread_bp(&self) -> f64 {
        self.inner.spread_bp
    }
    
    #[getter]
    fn frequency(&self) -> PyFrequency {
        PyFrequency::from_inner(self.inner.freq)
    }
    
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.dc)
    }
    
    #[getter]
    fn start_date(&self) -> PyDate {
        PyDate::from_core(self.inner.start)
    }
    
    #[getter]
    fn end_date(&self) -> PyDate {
        PyDate::from_core(self.inner.end)
    }
    
    fn __repr__(&self) -> String {
        format!(
            "FloatLeg(index={}, spread={}bp, freq={:?}, start={}, end={})",
            self.inner.fwd_id,
            self.inner.spread_bp,
            self.inner.freq,
            self.inner.start,
            self.inner.end
        )
    }
}

/// Interest Rate Swap instrument.
///
/// An IRS is a derivative contract where two parties exchange interest payments
/// on a notional principal amount. One leg pays a fixed rate while the other
/// pays a floating rate based on a reference index (e.g., SOFR, EURIBOR).
///
/// The swap's value is the net present value of all future cashflows from both
/// legs, discounted using the appropriate curves.
///
/// Examples:
///     >>> from finstack.instruments import InterestRateSwap, PayReceive, FixedLeg, FloatLeg
///     >>> from finstack import Money, Currency, Date, DayCount
///     >>> from finstack.dates import Frequency
///     >>> 
///     >>> # Create swap legs
///     >>> fixed = FixedLeg(
///     ...     discount_curve="USD-OIS",
///     ...     rate=0.03,  # 3% fixed
///     ...     frequency=Frequency.SemiAnnual,
///     ...     day_count=DayCount.thirty360(),
///     ...     start_date=Date(2024, 1, 1),
///     ...     end_date=Date(2029, 1, 1)
///     ... )
///     >>> 
///     >>> floating = FloatLeg(
///     ...     discount_curve="USD-OIS", 
///     ...     forward_curve="USD-SOFR-3M",
///     ...     spread_bp=0,
///     ...     frequency=Frequency.Quarterly,
///     ...     day_count=DayCount.act360(),
///     ...     start_date=Date(2024, 1, 1),
///     ...     end_date=Date(2029, 1, 1)
///     ... )
///     >>> 
///     >>> # Create the swap
///     >>> swap = InterestRateSwap(
///     ...     id="USD-5Y-SOFR",
///     ...     notional=Money(10_000_000, Currency("USD")),
///     ...     side=PayReceive.PayFixed,  # Pay fixed, receive floating
///     ...     fixed_leg=fixed,
///     ...     float_leg=floating
///     ... )
///     >>> 
///     >>> # Price the swap
///     >>> result = swap.price(market_context, Date(2024, 1, 1))
///     >>> print(f"NPV: ${result.value.amount:,.2f}")
///     >>> print(f"Par Rate: {result.get_metric('ParRate', 0):.4%}")
///     >>> print(f"DV01: ${result.get_metric('Dv01', 0):,.2f}")
#[pyclass(name = "InterestRateSwap", module = "finstack.instruments")]
#[derive(Clone)]
pub struct PyInterestRateSwap {
    inner: Arc<InterestRateSwap>,
}

#[pymethods]
impl PyInterestRateSwap {
    #[new]
    fn new(
        id: String,
        notional: &PyMoney,
        side: PyPayReceive,
        fixed_leg: &PyFixedLeg,
        float_leg: &PyFloatLeg,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: Arc::new(InterestRateSwap {
                id,
                notional: notional.inner(),
                side: side.into(),
                fixed: fixed_leg.inner.clone(),
                float: float_leg.inner.clone(),
            })
        })
    }
    
    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }
    
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.notional)
    }
    
    #[getter]
    fn side(&self) -> PyPayReceive {
        match self.inner.side {
            PayReceive::PayFixed => PyPayReceive::PayFixed,
            PayReceive::ReceiveFixed => PyPayReceive::ReceiveFixed,
        }
    }
    
    #[getter]
    fn fixed_leg(&self) -> PyFixedLeg {
        PyFixedLeg {
            inner: self.inner.fixed.clone()
        }
    }
    
    #[getter]
    fn float_leg(&self) -> PyFloatLeg {
        PyFloatLeg {
            inner: self.inner.float.clone()
        }
    }
    
    /// Calculate the par swap rate.
    ///
    /// The par rate is the fixed rate that makes the swap have zero NPV
    /// at inception. This is useful for quoting swap rates in the market.
    ///
    /// Args:
    ///     market_context: Market data including discount and forward curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     float: The par swap rate
    ///
    /// Examples:
    ///     >>> par_rate = swap.par_rate(context, Date(2024, 1, 1))
    ///     >>> print(f"Par Rate: {par_rate:.4%}")
    fn par_rate(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
    ) -> PyResult<f64> {
        use finstack_valuations::metrics::{MetricId, standard_registry, MetricContext};
        use finstack_valuations::instruments::Instrument;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        // Create instrument wrapper
        let instrument = Instrument::IRS((*self.inner).clone());
        
        // Calculate base value first
        use finstack_valuations::traits::Priceable;
        let base_value = self.inner.value(&curves, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate swap value: {:?}", e)
            ))?;
        
        // Create metric context
        let mut context = MetricContext::new(
            Arc::new(instrument),
            curves.clone(),
            as_of_date,
            base_value,
        );
        
        // Get standard registry and compute par rate
        let registry = standard_registry();
        let metrics = vec![MetricId::ParRate];
        let results = registry.compute(&metrics, &mut context)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate par rate: {:?}", e)
            ))?;
        
        Ok(results.get(&MetricId::ParRate).copied().unwrap_or(0.0))
    }
    
    /// Price the swap using market data.
    ///
    /// Calculates the net present value and risk metrics for the swap.
    ///
    /// Args:
    ///     market_context: Market data including discount and forward curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     ValuationResult: Pricing result with NPV and metrics
    ///
    /// Examples:
    ///     >>> result = swap.price(context, Date(2024, 1, 1))
    ///     >>> print(f"NPV: ${result.value.amount:,.2f}")
    ///     >>> print(f"DV01: ${result.get_metric('Dv01', 0):,.2f}")
    fn price(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
    ) -> PyResult<crate::valuations::results::PyValuationResult> {
        use finstack_valuations::traits::Priceable;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        // Call the Rust pricing implementation
        let result = self.inner.price(&curves, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to price swap: {:?}", e)
            ))?;
        
        Ok(crate::valuations::results::PyValuationResult::from_inner(result))
    }
    
    /// Calculate the present value only (no metrics).
    ///
    /// Args:
    ///     market_context: Market data including discount and forward curves
    ///     as_of: Valuation date
    ///
    /// Returns:
    ///     Money: Net present value of the swap
    fn value(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
    ) -> PyResult<PyMoney> {
        use finstack_valuations::traits::Priceable;
        
        let curves = market_context.inner();
        let as_of_date = as_of.inner();
        
        let value = self.inner.value(&curves, as_of_date)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                format!("Failed to calculate swap value: {:?}", e)
            ))?;
        
        Ok(PyMoney::from_inner(value))
    }
    
    fn __repr__(&self) -> String {
        let side_str = match self.inner.side {
            PayReceive::PayFixed => "PayFixed",
            PayReceive::ReceiveFixed => "ReceiveFixed",
        };
        
        format!(
            "InterestRateSwap('{}', {}, notional={:.2} {})",
            self.inner.id,
            side_str,
            self.inner.notional.amount(),
            self.inner.notional.currency()
        )
    }
    
    fn __eq__(&self, other: &Self) -> bool {
        self.inner.id == other.inner.id
    }
}