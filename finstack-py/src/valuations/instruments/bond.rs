use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::common::PyInstrumentType;
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::bond::{CallPut, CallPutSchedule, CashflowSpec};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;
use finstack_core::types::{CurveId, InstrumentId};

/// Fixed-income bond instrument with convenience constructors.
#[pyclass(module = "finstack.valuations.instruments", name = "Bond", frozen)]
#[derive(Clone, Debug)]
pub struct PyBond {
    pub(crate) inner: Bond,
}

impl PyBond {
    pub(crate) fn new(inner: Bond) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyBond {
    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, coupon_rate, issue, maturity, discount_curve)"
    )]
    /// Create a semi-annual fixed-rate bond with 30/360 day count and Following BDC.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     coupon_rate: Annual coupon in decimal form.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///     discount_curve: Discount curve identifier for valuation.
    ///
    /// Returns:
    ///     Bond: Configured fixed-rate bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///
    /// Examples:
    ///     >>> bond = Bond.fixed_semiannual(
    ///     ...     "corp_1",
    ///     ...     Money("USD", 1_000_000),
    ///     ...     0.045,
    ///     ...     date(2023, 1, 1),
    ///     ...     date(2028, 1, 1),
    ///     ...     "usd_discount"
    ///     ... )
    ///     >>> bond.coupon
    ///     0.045
    fn fixed_semiannual(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = CurveId::new(discount_curve.extract::<&str>()?);
        Ok(Self::new(Bond::fixed(
            id,
            amt,
            coupon_rate,
            issue_date,
            maturity_date,
            disc,
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, coupon_rate, issue, maturity)")]
    /// Create a U.S. Treasury-style bond with annual coupons and Act/Act ISMA day count.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional principal as :class:`finstack.core.money.Money`.
    ///     coupon_rate: Annual coupon in decimal form.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///
    /// Returns:
    ///     Bond: Configured Treasury-style bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///
    /// Examples:
    ///     >>> Bond.treasury("ust_5y", Money("USD", 1_000), 0.03, date(2024, 1, 1), date(2029, 1, 1))
    ///     Bond(id='ust_5y', coupon=0.03, maturity='2029-01-01')
    fn treasury(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        use finstack_valuations::instruments::common::parameters::BondConvention;
        Ok(Self::new(Bond::with_convention(
            id,
            amt,
            coupon_rate,
            issue_date,
            maturity_date,
            BondConvention::USTreasury,
            "USD-TREASURY",
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve)")]
    /// Create a zero-coupon bond discounted off ``discount_curve``.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Redemption amount as :class:`finstack.core.money.Money`.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///     discount_curve: Discount curve identifier for valuation.
    ///
    /// Returns:
    ///     Bond: Configured zero-coupon bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    fn zero_coupon(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = CurveId::new(discount_curve.extract::<&str>()?);
        Ok(Self::new(Bond::fixed(
            id,
            amt,
            0.0, // Zero coupon
            issue_date,
            maturity_date,
            disc,
        )))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve, *, coupon_rate=None, frequency=None, day_count=None, bdc=None, calendar_id=None, stub=None, amortization=None, call_schedule=None, put_schedule=None, quoted_clean_price=None, forward_curve=None, float_margin_bp=None, float_gearing=None, float_reset_lag_days=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    /// Create a bond via builder parameters. Supports amortization and call/put.
    ///
    /// Args:
    ///     instrument_id: Instrument identifier or string-like object.
    ///     notional: Notional amount as :class:`finstack.core.money.Money`.
    ///     issue: Issue date of the bond.
    ///     maturity: Maturity date of the bond.
    ///     discount_curve: Discount curve identifier for valuation.
    ///     coupon_rate: Optional fixed coupon rate in decimal form.
    ///     frequency: Optional payment frequency.
    ///     day_count: Optional day-count convention.
    ///     bdc: Optional business-day convention.
    ///     calendar_id: Optional calendar identifier for scheduling.
    ///     stub: Optional stub kind for schedule construction.
    ///     amortization: Optional amortization specification.
    ///     call_schedule: Optional list of (date, price %) call events.
    ///     put_schedule: Optional list of (date, price %) put events.
    ///     quoted_clean_price: Optional quoted clean price for overrides.
    ///     forward_curve: Optional forward curve identifier for float spec.
    ///     float_margin_bp: Optional floating margin in basis points.
    ///     float_gearing: Optional gearing multiplier for float leg.
    ///     float_reset_lag_days: Optional reset lag in days for float leg.
    ///
    /// Returns:
    ///     Bond: Fully specified bond instrument.
    ///
    /// Raises:
    ///     ValueError: If identifiers or dates cannot be parsed.
    ///     RuntimeError: When the underlying builder detects invalid input.
    fn builder(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        coupon_rate: Option<f64>,
        frequency: Option<crate::core::dates::schedule::PyFrequency>,
        day_count: Option<crate::core::dates::daycount::PyDayCount>,
        bdc: Option<crate::core::dates::calendar::PyBusinessDayConvention>,
        calendar_id: Option<&str>,
        stub: Option<crate::core::dates::schedule::PyStubKind>,
        amortization: Option<crate::valuations::cashflow::specs::PyAmortizationSpec>,
        call_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        put_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        quoted_clean_price: Option<f64>,
        forward_curve: Option<Bound<'_, PyAny>>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = CurveId::new(discount_curve.extract::<&str>()?);

        // Build the cashflow_spec from the provided parameters
        use finstack_core::dates::{BusinessDayConvention, DayCount, Frequency, StubKind};
        use finstack_valuations::cashflow::builder::specs::{
            CouponType, FixedCouponSpec, FloatingCouponSpec, FloatingRateSpec,
        };

        let freq = frequency
            .map(|f| f.inner)
            .unwrap_or(Frequency::semi_annual());
        let dc = day_count.map(|d| d.inner).unwrap_or(DayCount::Thirty360);
        let bdc_val = bdc
            .map(|b| b.inner)
            .unwrap_or(BusinessDayConvention::Following);
        let stub_val = stub.map(|s| s.inner).unwrap_or(StubKind::None);
        let calendar_str = calendar_id.map(|s| s.to_string());

        let cashflow_spec = if let Some(fwd) = forward_curve {
            // Floating-rate bond
            let forward_curve_id = CurveId::new(fwd.extract::<&str>()?);
            CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: forward_curve_id,
                    spread_bp: float_margin_bp.unwrap_or(0.0),
                    gearing: float_gearing.unwrap_or(1.0),
                    floor_bp: None,
                    cap_bp: None,
                    reset_freq: freq,
                    reset_lag_days: float_reset_lag_days.unwrap_or(2),
                    dc,
                    bdc: bdc_val,
                    calendar_id: calendar_str.clone(),
                },
                coupon_type: CouponType::Cash,
                freq,
                stub: stub_val,
            })
        } else {
            // Fixed-rate bond
            let rate = coupon_rate.unwrap_or(0.0);
            if let Some(am) = amortization.as_ref() {
                CashflowSpec::amortizing(
                    CashflowSpec::Fixed(FixedCouponSpec {
                        coupon_type: CouponType::Cash,
                        rate,
                        freq,
                        dc,
                        bdc: bdc_val,
                        calendar_id: calendar_str.clone(),
                        stub: stub_val,
                    }),
                    am.inner.clone(),
                )
            } else {
                CashflowSpec::Fixed(FixedCouponSpec {
                    coupon_type: CouponType::Cash,
                    rate,
                    freq,
                    dc,
                    bdc: bdc_val,
                    calendar_id: calendar_str.clone(),
                    stub: stub_val,
                })
            }
        };

        let mut builder = Bond::builder()
            .id(id)
            .notional(amt)
            .issue(issue_date)
            .maturity(maturity_date)
            .discount_curve_id(disc)
            .cashflow_spec(cashflow_spec);

        if let Some(px) = quoted_clean_price {
            builder = builder.pricing_overrides(
                finstack_valuations::instruments::PricingOverrides::default().with_clean_price(px),
            );
        }

        if call_schedule.is_some() || put_schedule.is_some() {
            let mut cps = CallPutSchedule::default();
            if let Some(calls) = call_schedule {
                for (d, pct) in calls {
                    cps.calls.push(CallPut {
                        date: py_to_date(&d)?,
                        price_pct_of_par: pct,
                    });
                }
            }
            if let Some(puts) = put_schedule {
                for (d, pct) in puts {
                    cps.puts.push(CallPut {
                        date: py_to_date(&d)?,
                        price_pct_of_par: pct,
                    });
                }
            }
            builder = builder.call_put_opt(Some(cps));
        }

        builder
            .build()
            .map(Self::new)
            .map_err(crate::errors::core_to_py)
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, schedule, discount_curve, quoted_clean=None, forward_curve=None, float_margin_bp=None, float_gearing=None, float_reset_lag_days=None)",
        signature = (
            instrument_id,
            schedule,
            discount_curve,
            quoted_clean=None,
            forward_curve=None,
            float_margin_bp=None,
            float_gearing=None,
            float_reset_lag_days=None,
        )
    )]
    /// Create a bond from a pre-built CashFlowSchedule (supports PIK, amort, custom coupons).
    /// Optionally attach a floating-rate spec (forward curve + margin) so ASW metrics
    /// can build a custom-swap on the same schedule.
    fn from_cashflows(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        schedule: PyRef<PyCashFlowSchedule>,
        discount_curve: Bound<'_, PyAny>,
        quoted_clean: Option<f64>,
        forward_curve: Option<Bound<'_, PyAny>>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let disc = CurveId::new(discount_curve.extract::<&str>()?);
        let mut bond = finstack_valuations::instruments::bond::Bond::from_cashflows(
            id,
            schedule.inner_clone(),
            disc,
            quoted_clean,
        )
        .map_err(crate::errors::core_to_py)?;

        if let Some(fwd) = forward_curve {
            // Update cashflow_spec to floating if forward curve is provided
            use finstack_valuations::cashflow::builder::specs::{
                CouponType, FloatingCouponSpec, FloatingRateSpec,
            };

            let forward_curve_id = CurveId::new(fwd.extract::<&str>()?);
            let freq = bond.cashflow_spec.frequency();
            let dc = bond.cashflow_spec.day_count();

            bond.cashflow_spec = CashflowSpec::Floating(FloatingCouponSpec {
                rate_spec: FloatingRateSpec {
                    index_id: forward_curve_id,
                    spread_bp: float_margin_bp.unwrap_or(0.0),
                    gearing: float_gearing.unwrap_or(1.0),
                    floor_bp: None,
                    cap_bp: None,
                    reset_freq: freq,
                    reset_lag_days: float_reset_lag_days.unwrap_or(2),
                    dc,
                    bdc: finstack_core::dates::BusinessDayConvention::Following,
                    calendar_id: None,
                },
                coupon_type: CouponType::Cash,
                freq,
                stub: finstack_core::dates::StubKind::None,
            });
        }

        Ok(Self::new(bond))
    }

    #[classmethod]
    #[pyo3(
        text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve, forward_curve, margin_bp)"
    )]
    /// Create a floating-rate note (FRN) using SOFR-like conventions.
    fn floating(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
        forward_curve: Bound<'_, PyAny>,
        margin_bp: f64,
    ) -> PyResult<Self> {
        let id = InstrumentId::new(instrument_id.extract::<&str>()?);
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = CurveId::new(discount_curve.extract::<&str>()?);
        let fwd = CurveId::new(forward_curve.extract::<&str>()?);

        use finstack_core::dates::{DayCount, Frequency};

        let bond = finstack_valuations::instruments::bond::Bond::floating(
            id,
            amt,
            fwd,
            margin_bp,
            issue_date,
            maturity_date,
            Frequency::quarterly(),
            DayCount::Act360,
            disc,
        );

        Ok(Self::new(bond))
    }

    /// Instrument identifier.
    ///
    /// Returns:
    ///     str: Unique identifier assigned to the instrument.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    ///
    /// Returns:
    ///     Money: Notional wrapped as :class:`finstack.core.money.Money`.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Annual coupon rate in decimal form (for fixed bonds only).
    ///
    /// Returns:
    ///     float: Annual coupon rate, or 0.0 for non-fixed bonds.
    #[getter]
    fn coupon(&self) -> f64 {
        match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate,
            CashflowSpec::Amortizing { base, .. } => match &**base {
                CashflowSpec::Fixed(spec) => spec.rate,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    /// Issue date for the bond.
    ///
    /// Returns:
    ///     datetime.date: Issue date converted to Python.
    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.issue)
    }

    /// Maturity date.
    ///
    /// Returns:
    ///     datetime.date: Maturity date converted to Python.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    ///
    /// Returns:
    ///     str: Identifier for the discount curve.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.discount_curve_id.as_str().to_string()
    }

    /// Optional hazard curve identifier enabling credit-sensitive pricing.
    ///
    /// Returns:
    ///     str | None: Hazard curve identifier when provided.
    #[getter]
    fn hazard_curve(&self) -> Option<String> {
        self.inner
            .credit_curve_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    /// Instrument type enum (``InstrumentType.BOND``).
    ///
    /// Returns:
    ///     InstrumentType: Enumeration value identifying the instrument family.
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Bond)
    }

    fn __repr__(&self) -> PyResult<String> {
        let coupon_rate = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate,
            _ => 0.0,
        };
        Ok(format!(
            "Bond(id='{}', coupon={:.4}, maturity='{}')",
            self.inner.id, coupon_rate, self.inner.maturity
        ))
    }
}

impl fmt::Display for PyBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let coupon_rate = match &self.inner.cashflow_spec {
            CashflowSpec::Fixed(spec) => spec.rate,
            _ => 0.0,
        };
        write!(f, "Bond({}, coupon={:.4})", self.inner.id, coupon_rate)
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBond>()?;
    Ok(vec!["Bond"])
}
