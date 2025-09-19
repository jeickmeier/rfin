//! Python bindings for interest rate swap instruments.

use crate::core::{
    dates::{PyBusDayConv, PyDate, PyDayCount, PyFrequency, PyStubRule},
    money::PyMoney,
};
use finstack_core::dates::{BusinessDayConvention, StubKind};
use finstack_valuations::instruments::irs::{
    FixedLegSpec, FloatLegSpec, InterestRateSwap, PayReceive,
};
use pyo3::prelude::*;
use std::str::FromStr;
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

        let stub_kind = stub.map(|s| s.inner()).unwrap_or(StubKind::None);

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
                par_method: None,
                compounding_simple: true,
            },
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

        let stub_kind = stub.map(|s| s.inner()).unwrap_or(StubKind::None);

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
                reset_lag_days: 2,
                start: start_date.inner(),
                end: end_date.inner(),
            },
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
///     >>> # Price the swap with metrics
///     >>> result = swap.price_with_metrics(market_context, Date(2024, 1, 1), ["par_rate"])
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
                id: id.into(),
                notional: notional.inner(),
                side: side.into(),
                fixed: fixed_leg.inner.clone(),
                float: float_leg.inner.clone(),
                attributes: finstack_valuations::instruments::traits::Attributes::new(),
            }),
        })
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
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
            inner: self.inner.fixed.clone(),
        }
    }

    #[getter]
    fn float_leg(&self) -> PyFloatLeg {
        PyFloatLeg {
            inner: self.inner.float.clone(),
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
        use finstack_valuations::instruments::traits::Priceable;
        use finstack_valuations::metrics::{standard_registry, MetricId};

        let curves = market_context.inner();
        let as_of_date = as_of.inner();

        // Calculate base value first
        // use already imported Priceable
        let base_value = self.inner.value(&curves, as_of_date).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to calculate swap value: {:?}",
                e
            ))
        })?;

        // Create metric context
        let mut context = finstack_valuations::metrics::MetricContext::new(
            Arc::new((*self.inner).clone()),
            curves.clone(),
            as_of_date,
            base_value,
        );

        // Get standard registry and compute par rate
        let registry = standard_registry();
        let metrics = vec![MetricId::ParRate];
        let results = registry.compute(&metrics, &mut context).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to calculate par rate: {:?}",
                e
            ))
        })?;

        Ok(results.get(&MetricId::ParRate).copied().unwrap_or(0.0))
    }

    /// Get the swap's attributes for tagging and metadata.
    ///
    /// Returns:
    ///     Attributes: The swap's attributes object
    #[getter]
    fn attributes(&self) -> crate::valuations::attributes::PyAttributes {
        use finstack_valuations::instruments::traits::Attributable;
        let attrs = self.inner.attributes().clone();
        crate::valuations::attributes::PyAttributes::from_inner(attrs)
    }

    /// Set the swap's attributes.
    ///
    /// Args:
    ///     attributes: New attributes to set
    #[setter]
    fn set_attributes(
        &mut self,
        attributes: &crate::valuations::attributes::PyAttributes,
    ) -> PyResult<()> {
        use finstack_valuations::instruments::traits::Attributable;
        use std::sync::Arc;

        let mut swap = (*self.inner).clone();
        *swap.attributes_mut() = attributes.inner.clone();
        self.inner = Arc::new(swap);
        Ok(())
    }

    /// Add a tag to the swap's attributes.
    ///
    /// Args:
    ///     tag: Tag to add
    fn add_tag(&mut self, tag: String) -> PyResult<()> {
        use finstack_valuations::instruments::traits::Attributable;
        use std::sync::Arc;

        let mut swap = (*self.inner).clone();
        swap.attributes_mut().tags.insert(tag);
        self.inner = Arc::new(swap);
        Ok(())
    }

    /// Check if the swap has a specific tag.
    ///
    /// Args:
    ///     tag: Tag to check
    ///
    /// Returns:
    ///     True if the tag exists
    fn has_tag(&self, tag: &str) -> bool {
        use finstack_valuations::instruments::traits::Attributable;
        self.inner.has_tag(tag)
    }

    /// Set a metadata value on the swap.
    ///
    /// Args:
    ///     key: Metadata key
    ///     value: Metadata value
    fn set_meta(&mut self, key: String, value: String) -> PyResult<()> {
        use finstack_valuations::instruments::traits::Attributable;
        use std::sync::Arc;

        let mut swap = (*self.inner).clone();
        swap.attributes_mut().meta.insert(key, value);
        self.inner = Arc::new(swap);
        Ok(())
    }

    /// Get a metadata value from the swap.
    ///
    /// Args:
    ///     key: Metadata key
    ///
    /// Returns:
    ///     The value if present
    fn get_meta(&self, key: &str) -> Option<String> {
        use finstack_valuations::instruments::traits::Attributable;
        self.inner.get_meta(key).map(|s| s.to_string())
    }

    /// Check if the swap matches a selector.
    ///
    /// Args:
    ///     selector: Selector string
    ///
    /// Returns:
    ///     True if the swap matches the selector
    fn matches_selector(&self, selector: &str) -> bool {
        use finstack_valuations::instruments::traits::Attributable;
        self.inner.matches_selector(selector)
    }

    /// Generate a comprehensive risk report for the swap.
    ///
    /// Calculates key risk metrics, bucketed sensitivities, and categorizes
    /// the swap into risk buckets based on its characteristics.
    ///
    /// Args:
    ///     market_context: Market data including curves
    ///     as_of: Valuation date
    ///     bucket_spec: Optional list of risk buckets for categorization
    ///
    /// Returns:
    ///     RiskReport: Comprehensive risk report
    ///
    /// Examples:
    ///     >>> report = swap.risk_report(context, Date(2024, 1, 1))
    ///     >>> print(f"DV01: {report.get_metric('Dv01', 0)}")
    ///     >>> print(f"Par Rate: {report.get_metric('ParRate', 0)}")
    fn risk_report(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
        bucket_spec: Option<Vec<crate::valuations::risk::PyRiskBucket>>,
    ) -> PyResult<crate::valuations::risk::PyRiskReport> {
        use finstack_valuations::metrics::RiskMeasurable;

        let curves = market_context.inner();
        let as_of_date = as_of.inner();

        // Convert Python bucket spec to Rust if provided
        let rust_buckets = bucket_spec.map(|buckets| {
            buckets
                .into_iter()
                .map(|b| finstack_valuations::metrics::RiskBucket {
                    id: b.inner.id,
                    tenor_years: b.inner.tenor_years,
                    classification: b.inner.classification,
                })
                .collect::<Vec<_>>()
        });

        let bucket_spec_ref = rust_buckets.as_deref();

        let report = self
            .inner
            .risk_report(&curves, as_of_date, bucket_spec_ref)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to generate risk report: {:?}",
                    e
                ))
            })?;

        Ok(crate::valuations::risk::PyRiskReport::from_inner(report))
    }

    /// Price the swap with selected metrics.
    ///
    /// Computes PV and provided metrics. For PV only, use `value()`.
    fn price_with_metrics(
        &self,
        market_context: &crate::core::market_data::context::PyMarketContext,
        as_of: &PyDate,
        metrics: Vec<String>,
    ) -> PyResult<crate::valuations::results::PyValuationResult> {
        use finstack_valuations::instruments::traits::Priceable;

        let curves = market_context.inner();
        let as_of_date = as_of.inner();

        let metric_ids: Vec<finstack_valuations::metrics::MetricId> = metrics
            .iter()
            .map(|name| finstack_valuations::metrics::MetricId::from_str(name))
            .collect::<Result<_, _>>()
            .unwrap_or_else(|_| {
                metrics
                    .iter()
                    .map(finstack_valuations::metrics::MetricId::custom)
                    .collect()
            });

        let result = self
            .inner
            .price_with_metrics(&curves, as_of_date, &metric_ids)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to calculate swap metrics: {:?}",
                    e
                ))
            })?;

        Ok(crate::valuations::results::PyValuationResult::from_inner(
            result,
        ))
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
        use finstack_valuations::instruments::traits::Priceable;

        let curves = market_context.inner();
        let as_of_date = as_of.inner();

        let value = self.inner.value(&curves, as_of_date).map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to calculate swap value: {:?}",
                e
            ))
        })?;

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
