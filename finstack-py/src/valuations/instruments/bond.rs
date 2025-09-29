use crate::core::money::{extract_money, PyMoney};
use crate::core::utils::{date_to_py, py_to_date};
use crate::valuations::cashflow::builder::PyCashFlowSchedule;
use crate::valuations::common::{extract_curve_id, extract_instrument_id, PyInstrumentType};
use finstack_valuations::instruments::bond::Bond;
use finstack_valuations::instruments::bond::BondFloatSpec;
use finstack_valuations::instruments::bond::{CallPut, CallPutSchedule};
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule, PyType};
use pyo3::Bound;
use std::fmt;

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
    fn fixed_semiannual(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = extract_curve_id(&discount_curve)?;
        Ok(Self::new(Bond::fixed_semiannual(
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
    fn treasury(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        coupon_rate: f64,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        Ok(Self::new(Bond::treasury(
            id,
            amt,
            coupon_rate,
            issue_date,
            maturity_date,
        )))
    }

    #[classmethod]
    #[pyo3(text_signature = "(cls, instrument_id, notional, issue, maturity, discount_curve)")]
    /// Create a zero-coupon bond discounted off ``discount_curve``.
    fn zero_coupon(
        _cls: &Bound<'_, PyType>,
        instrument_id: Bound<'_, PyAny>,
        notional: Bound<'_, PyAny>,
        issue: Bound<'_, PyAny>,
        maturity: Bound<'_, PyAny>,
        discount_curve: Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = extract_curve_id(&discount_curve)?;
        Ok(Self::new(Bond::zero_coupon(
            id,
            amt,
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
        amortization: Option<crate::core::cashflow::primitives::PyAmortizationSpec>,
        call_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        put_schedule: Option<Vec<(Bound<'_, PyAny>, f64)>>,
        quoted_clean_price: Option<f64>,
        forward_curve: Option<Bound<'_, PyAny>>,
        float_margin_bp: Option<f64>,
        float_gearing: Option<f64>,
        float_reset_lag_days: Option<i32>,
    ) -> PyResult<Self> {
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = extract_curve_id(&discount_curve)?;

        let mut builder = Bond::builder()
            .id(id)
            .notional(amt)
            .issue(issue_date)
            .maturity(maturity_date)
            .disc_id(disc);

        if let Some(px) = quoted_clean_price {
            builder = builder.pricing_overrides(
                finstack_valuations::instruments::PricingOverrides::default().with_clean_price(px),
            );
        }
        if let Some(am) = amortization.as_ref() {
            builder = builder.amortization(am.inner.clone());
        }
        if let Some(dc) = day_count {
            builder = builder.dc(dc.inner);
        }
        if let Some(f) = frequency {
            builder = builder.freq(f.inner);
        }
        if let Some(rule) = stub {
            builder = builder.stub(rule.inner);
        }
        if let Some(conv) = bdc {
            builder = builder.bdc(conv.inner);
        }
        if let Some(cal) = calendar_id {
            let leaked: &'static str = Box::leak(cal.to_string().into_boxed_str());
            builder = builder.calendar_id_opt(Some(leaked));
        }
        if let Some(c) = coupon_rate {
            builder = builder.coupon(c);
        }

        // Optional floating-rate configuration
        if let Some(fwd) = forward_curve {
            let fwd_id = extract_curve_id(&fwd)?;
            let spec = BondFloatSpec {
                fwd_id,
                margin_bp: float_margin_bp.unwrap_or(0.0),
                gearing: float_gearing.unwrap_or(1.0),
                reset_lag_days: float_reset_lag_days.unwrap_or(2),
            };
            builder = builder.float_opt(Some(spec));
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
            .map_err(crate::core::error::core_to_py)
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
        let id = extract_instrument_id(&instrument_id)?;
        let disc = extract_curve_id(&discount_curve)?;
        let mut bond = finstack_valuations::instruments::bond::Bond::from_cashflows(
            id,
            schedule.inner_clone(),
            disc,
            quoted_clean,
        )
        .map_err(crate::core::error::core_to_py)?;

        if let Some(fwd) = forward_curve {
            let fwd_id = extract_curve_id(&fwd)?;
            let spec = BondFloatSpec {
                fwd_id,
                margin_bp: float_margin_bp.unwrap_or(0.0),
                gearing: float_gearing.unwrap_or(1.0),
                reset_lag_days: float_reset_lag_days.unwrap_or(2),
            };
            bond.float = Some(spec);
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
        let id = extract_instrument_id(&instrument_id)?;
        let amt = extract_money(&notional)?;
        let issue_date = py_to_date(&issue)?;
        let maturity_date = py_to_date(&maturity)?;
        let disc = extract_curve_id(&discount_curve)?;
        let fwd = extract_curve_id(&forward_curve)?;
        Ok(Self::new(Bond::floating(
            id,
            amt,
            issue_date,
            maturity_date,
            disc,
            fwd,
            margin_bp,
        )))
    }

    /// Instrument identifier.
    #[getter]
    fn instrument_id(&self) -> &str {
        self.inner.id.as_str()
    }

    /// Notional principal amount.
    #[getter]
    fn notional(&self) -> PyMoney {
        PyMoney::new(self.inner.notional)
    }

    /// Annual coupon rate in decimal form.
    #[getter]
    fn coupon(&self) -> f64 {
        self.inner.coupon
    }

    /// Issue date.
    #[getter]
    fn issue(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.issue)
    }

    /// Maturity date.
    #[getter]
    fn maturity(&self, py: Python<'_>) -> PyResult<PyObject> {
        date_to_py(py, self.inner.maturity)
    }

    /// Discount curve identifier.
    #[getter]
    fn discount_curve(&self) -> String {
        self.inner.disc_id.as_str().to_string()
    }

    /// Optional hazard curve identifier enabling credit-sensitive pricing.
    #[getter]
    fn hazard_curve(&self) -> Option<String> {
        self.inner
            .hazard_id
            .as_ref()
            .map(|id| id.as_str().to_string())
    }

    /// Instrument type enum (``InstrumentType.BOND``).
    #[getter]
    fn instrument_type(&self) -> PyInstrumentType {
        PyInstrumentType::new(finstack_valuations::pricer::InstrumentType::Bond)
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!(
            "Bond(id='{}', coupon={:.4}, maturity='{}')",
            self.inner.id, self.inner.coupon, self.inner.maturity
        ))
    }
}

impl fmt::Display for PyBond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Bond({}, coupon={:.4})",
            self.inner.id, self.inner.coupon
        )
    }
}

pub(crate) fn register<'py>(
    _py: Python<'py>,
    module: &Bound<'py, PyModule>,
) -> PyResult<Vec<&'static str>> {
    module.add_class::<PyBond>()?;
    Ok(vec!["Bond"])
}
