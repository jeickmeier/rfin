//! Python wrapper for the type-state ModelBuilder.
//!
//! Since Python cannot model Rust type-state at the type level, we collapse
//! the two states into a single class and track readiness at runtime.

use super::capital_structure::PyWaterfallSpec;
use super::types::PyFinancialModelSpec;
use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, display_to_py};
use finstack_core::dates::PeriodId;
use finstack_core::money::fx::FxConversionPolicy;
use finstack_statements::builder::ModelBuilder;
use finstack_statements::types::AmountOrScalar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

/// Builder for financial models (type-state collapsed for Python).
///
/// Usage::
///
///     builder = ModelBuilder("Acme Corp")
///     builder.periods("2025Q1..Q4", "2025Q2")
///     builder.value("revenue", [("2025Q1", 10_000_000.0), ("2025Q2", 11_000_000.0)])
///     builder.compute("cogs", "revenue * 0.6")
///     model = builder.build()
#[pyclass(name = "ModelBuilder", module = "finstack.statements")]
pub struct PyModelBuilder {
    inner: Option<BuilderState>,
}

enum BuilderState {
    NeedPeriods(ModelBuilder<finstack_statements::builder::NeedPeriods>),
    Ready(ModelBuilder<finstack_statements::builder::Ready>),
}

#[pymethods]
impl PyModelBuilder {
    /// Create a new model builder.
    #[new]
    #[pyo3(text_signature = "(id)")]
    fn new(id: &str) -> Self {
        Self {
            inner: Some(BuilderState::NeedPeriods(ModelBuilder::new(id))),
        }
    }

    /// Define periods using a range expression (e.g. ``"2025Q1..Q4"``).
    ///
    /// Parameters
    /// ----------
    /// range : str
    ///     Period range expression.
    /// actuals_until : str | None
    ///     Optional cutoff for actual values.
    #[pyo3(signature = (range, actuals_until=None), text_signature = "($self, range, actuals_until=None)")]
    fn periods(&mut self, range: &str, actuals_until: Option<&str>) -> PyResult<()> {
        let state = self.take_any()?;
        match state {
            BuilderState::NeedPeriods(b) => {
                let ready = b.periods(range, actuals_until).map_err(display_to_py)?;
                self.inner = Some(BuilderState::Ready(ready));
                Ok(())
            }
            BuilderState::Ready(b) => {
                self.inner = Some(BuilderState::Ready(b));
                Err(PyValueError::new_err("Periods already set"))
            }
        }
    }

    /// Add a value node with explicit period values.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// values : list[tuple[str, float]]
    ///     List of (period_string, value) tuples.
    #[pyo3(text_signature = "($self, node_id, values)")]
    fn value(&mut self, node_id: &str, values: Vec<(String, f64)>) -> PyResult<()> {
        let state = self.take_ready()?;
        let parsed: Vec<(PeriodId, AmountOrScalar)> = values
            .into_iter()
            .map(|(p, v)| {
                let pid: PeriodId = p.parse().map_err(core_to_py)?;
                Ok((pid, AmountOrScalar::scalar(v)))
            })
            .collect::<PyResult<Vec<_>>>()?;

        let ready = state.value(node_id, &parsed);
        self.inner = Some(BuilderState::Ready(ready));
        Ok(())
    }

    /// Add a computed node with a formula.
    ///
    /// Parameters
    /// ----------
    /// node_id : str
    ///     Node identifier.
    /// formula : str
    ///     DSL formula expression (e.g. ``"revenue - cogs"``).
    #[pyo3(text_signature = "($self, node_id, formula)")]
    fn compute(&mut self, node_id: &str, formula: &str) -> PyResult<()> {
        let state = self.take_ready()?;
        let ready = state.compute(node_id, formula).map_err(display_to_py)?;
        self.inner = Some(BuilderState::Ready(ready));
        Ok(())
    }

    /// Add a fixed-rate bond to the capital structure (US conventions: 30/360, semi-annual).
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Principal amount (must be in a valid Currency).
    /// coupon_rate : float
    ///     Annual coupon rate (e.g. 0.05 for 5%).
    /// issue_date, maturity_date : datetime.date
    ///     Bond issue and maturity dates.
    /// discount_curve_id : str
    ///     Discount curve identifier used for pricing.
    #[pyo3(
        text_signature = "($self, id, notional, coupon_rate, issue_date, maturity_date, discount_curve_id)"
    )]
    fn add_bond(
        &mut self,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        coupon_rate: f64,
        issue_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<()> {
        let notional = notional.inner;
        let issue = py_to_date(issue_date)?;
        let maturity = py_to_date(maturity_date)?;
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_bond(
                    id,
                    notional,
                    coupon_rate,
                    issue,
                    maturity,
                    discount_curve_id,
                )
                .map_err(display_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_bond(
                    id,
                    notional,
                    coupon_rate,
                    issue,
                    maturity,
                    discount_curve_id,
                )
                .map_err(display_to_py)?,
            ),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Add an interest rate swap to the capital structure (US conventions).
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Swap notional.
    /// fixed_rate : float
    ///     Fixed leg rate (e.g. 0.04 for 4%).
    /// start_date, maturity_date : datetime.date
    /// discount_curve_id, forward_curve_id : str
    ///     Discount curve and floating-leg forward curve identifiers.
    #[pyo3(
        text_signature = "($self, id, notional, fixed_rate, start_date, maturity_date, discount_curve_id, forward_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn add_swap(
        &mut self,
        id: &str,
        notional: PyRef<'_, PyMoney>,
        fixed_rate: f64,
        start_date: &Bound<'_, PyAny>,
        maturity_date: &Bound<'_, PyAny>,
        discount_curve_id: &str,
        forward_curve_id: &str,
    ) -> PyResult<()> {
        let notional = notional.inner;
        let start = py_to_date(start_date)?;
        let maturity = py_to_date(maturity_date)?;
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(
                b.add_swap(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                )
                .map_err(display_to_py)?,
            ),
            BuilderState::Ready(b) => BuilderState::Ready(
                b.add_swap(
                    id,
                    notional,
                    fixed_rate,
                    start,
                    maturity,
                    discount_curve_id,
                    forward_curve_id,
                )
                .map_err(display_to_py)?,
            ),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Add a generic debt instrument via an opaque JSON specification.
    ///
    /// Use this for term loans, RCFs, or any instrument not covered by the
    /// convenience constructors. The ``spec`` is passed straight through to
    /// the capital-structure engine and must match the Rust deserialization
    /// contract for the intended instrument type.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// spec_json : str
    ///     JSON string matching the target instrument's serde shape.
    #[pyo3(text_signature = "($self, id, spec_json)")]
    fn add_custom_debt(&mut self, id: &str, spec_json: &str) -> PyResult<()> {
        let spec: serde_json::Value = serde_json::from_str(spec_json).map_err(display_to_py)?;
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.add_custom_debt(id, spec)),
            BuilderState::Ready(b) => BuilderState::Ready(b.add_custom_debt(id, spec)),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Set the reporting currency used for capital-structure totals.
    #[pyo3(text_signature = "($self, currency)")]
    fn reporting_currency(&mut self, currency: PyRef<'_, PyCurrency>) -> PyResult<()> {
        let ccy = currency.inner;
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.reporting_currency(ccy)),
            BuilderState::Ready(b) => BuilderState::Ready(b.reporting_currency(ccy)),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Set the FX conversion policy for capital-structure cashflows.
    ///
    /// Parameters
    /// ----------
    /// policy : str
    ///     One of ``"cashflow_date"``, ``"period_end"``, ``"period_average"``, ``"custom"``.
    #[pyo3(text_signature = "($self, policy)")]
    fn fx_policy(&mut self, policy: &str) -> PyResult<()> {
        let policy_value = serde_json::Value::String(policy.to_string());
        let parsed: FxConversionPolicy =
            serde_json::from_value(policy_value).map_err(|e| {
                PyValueError::new_err(format!(
                    "invalid fx_policy {policy:?}: {e}; expected one of cashflow_date, period_end, period_average, custom"
                ))
            })?;
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.fx_policy(parsed)),
            BuilderState::Ready(b) => BuilderState::Ready(b.fx_policy(parsed)),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Attach a waterfall specification (PIK toggle + ECF sweep + priority-of-payments).
    #[pyo3(text_signature = "($self, waterfall_spec)")]
    fn waterfall(&mut self, waterfall_spec: PyRef<'_, PyWaterfallSpec>) -> PyResult<()> {
        let spec = waterfall_spec.inner.clone();
        let state = self.take_any()?;
        let next = match state {
            BuilderState::NeedPeriods(b) => BuilderState::NeedPeriods(b.waterfall(spec)),
            BuilderState::Ready(b) => BuilderState::Ready(b.waterfall(spec)),
        };
        self.inner = Some(next);
        Ok(())
    }

    /// Build the model specification.
    ///
    /// Returns
    /// -------
    /// FinancialModelSpec
    ///     The completed model specification.
    #[pyo3(text_signature = "($self)")]
    fn build(&mut self) -> PyResult<PyFinancialModelSpec> {
        let state = self.take_ready()?;
        let spec = state.build().map_err(display_to_py)?;
        Ok(PyFinancialModelSpec { inner: spec })
    }
}

impl PyModelBuilder {
    fn take_any(&mut self) -> PyResult<BuilderState> {
        self.inner
            .take()
            .ok_or_else(|| PyValueError::new_err("Builder has already been consumed by build()"))
    }

    fn take_ready(&mut self) -> PyResult<ModelBuilder<finstack_statements::builder::Ready>> {
        let state = self.take_any()?;
        match state {
            BuilderState::Ready(b) => Ok(b),
            BuilderState::NeedPeriods(b) => {
                self.inner = Some(BuilderState::NeedPeriods(b));
                Err(PyValueError::new_err(
                    "Must call periods() before adding nodes",
                ))
            }
        }
    }
}

/// Register builder classes.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyModelBuilder>()?;
    Ok(())
}
