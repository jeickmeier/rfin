# Horizon Total Return Analyzer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a composition layer in `finstack-scenarios` that applies a `ScenarioSpec` to construct a future market state, then delegates to existing P&L attribution to produce a decomposed horizon total return.

**Architecture:** `HorizonAnalysis` struct holds config (attribution method, engine). Its `compute()` method clones the market, applies the scenario (including time-roll), runs attribution between t0 and t1, and wraps the result in a `HorizonResult` with convenience accessors. Python and WASM bindings follow established patterns.

**Tech Stack:** Rust (finstack-scenarios, finstack-valuations), PyO3 (finstack-py), wasm-bindgen (finstack-wasm)

**Spec:** `docs/superpowers/specs/2026-04-12-horizon-total-return-design.md`

---

## File Structure

| File | Responsibility |
|------|---------------|
| `finstack/scenarios/src/horizon.rs` | **New.** `HorizonAnalysis` struct, `HorizonResult` struct, `compute()` orchestration, convenience methods, unit tests. |
| `finstack/scenarios/src/lib.rs` | **Modify.** Add `pub mod horizon` and re-exports. |
| `finstack-py/src/bindings/scenarios/horizon.rs` | **New.** `compute_horizon_return()` pyfunction, `PyHorizonResult` pyclass. |
| `finstack-py/src/bindings/scenarios/mod.rs` | **Modify.** Add `mod horizon`, register functions. |
| `finstack-wasm/src/api/scenarios/mod.rs` | **Modify.** Add `computeHorizonReturn()` wasm-bindgen function. |

---

### Task 1: Core Types — `HorizonAnalysis` and `HorizonResult`

**Files:**
- Create: `finstack/scenarios/src/horizon.rs`
- Modify: `finstack/scenarios/src/lib.rs`

- [ ] **Step 1: Create `horizon.rs` with type definitions and imports**

```rust
// finstack/scenarios/src/horizon.rs

//! Horizon total return analysis.
//!
//! Composes [`ScenarioSpec`] application with P&L attribution to answer:
//! "If I hold this instrument under these market assumptions, what is my
//! decomposed total return?"
//!
//! The caller supplies a [`ScenarioSpec`] that may include a
//! [`OperationSpec::TimeRollForward`] (holding period) alongside any market
//! shocks (spread widening, rate shifts, vol changes, etc.).  The engine
//! applies the spec to construct a T₁ market state, then delegates to the
//! existing attribution framework to decompose the P&L.
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use finstack_scenarios::horizon::HorizonAnalysis;
//! use finstack_scenarios::{OperationSpec, ScenarioSpec, TimeRollMode};
//! use finstack_core::market_data::context::MarketContext;
//! use finstack_valuations::instruments::internal::InstrumentExt;
//! use std::sync::Arc;
//! use time::macros::date;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let instrument: Arc<dyn InstrumentExt> = todo!("your instrument");
//! let market = MarketContext::new();
//! let as_of = date!(2025-01-15);
//!
//! // Hold for 3 months, spreads widen 25bp
//! let scenario = ScenarioSpec {
//!     id: "hold_3m_spread_25".into(),
//!     name: None,
//!     description: None,
//!     operations: vec![
//!         OperationSpec::TimeRollForward {
//!             period: "3M".into(),
//!             apply_shocks: true,
//!             roll_mode: TimeRollMode::BusinessDays,
//!         },
//!         OperationSpec::CurveParallelBp {
//!             curve_kind: finstack_scenarios::CurveKind::Hazard,
//!             curve_id: "AAPL-CDS".into(),
//!             discount_curve_id: None,
//!             bp: 25.0,
//!         },
//!     ],
//!     priority: 0,
//!     resolution_mode: Default::default(),
//! };
//!
//! let analyzer = HorizonAnalysis::default();
//! let result = analyzer.compute(&instrument, &market, as_of, &scenario)?;
//!
//! println!("Total return: {:.2}%", result.total_return_pct() * 100.0);
//! println!("Carry: {}", result.attribution.carry);
//! println!("Credit P&L: {}", result.attribution.credit_curves_pnl);
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;

use finstack_core::config::FinstackConfig;
use finstack_core::dates::Date;
use finstack_core::market_data::context::MarketContext;
use finstack_core::money::Money;
use finstack_valuations::attribution::{
    attribute_pnl_metrics_based, attribute_pnl_parallel, attribute_pnl_taylor_standard,
    attribute_pnl_waterfall, default_attribution_metrics, AttributionFactor, AttributionMethod,
    PnlAttribution,
};
use finstack_valuations::instruments::internal::InstrumentExt as Instrument;
use finstack_valuations::instruments::PricingOptions;

use crate::engine::ApplicationReport;
use crate::{ExecutionContext, ScenarioEngine, ScenarioSpec};

/// Horizon total return analyzer.
///
/// Composes scenario application with P&L attribution.  Construct with an
/// [`AttributionMethod`] and [`FinstackConfig`], then call [`compute`] to
/// project an instrument forward under a [`ScenarioSpec`] and decompose the
/// resulting P&L.
///
/// [`compute`]: HorizonAnalysis::compute
#[derive(Debug, Clone)]
pub struct HorizonAnalysis {
    /// Attribution methodology for decomposing the horizon P&L.
    pub attribution_method: AttributionMethod,
    /// Finstack configuration (rounding, tolerances).
    pub config: FinstackConfig,
    /// Scenario engine instance.
    pub engine: ScenarioEngine,
}

impl Default for HorizonAnalysis {
    fn default() -> Self {
        Self {
            attribution_method: AttributionMethod::Parallel,
            config: FinstackConfig::default(),
            engine: ScenarioEngine::new(),
        }
    }
}

impl HorizonAnalysis {
    /// Create a new analyzer with the given attribution method and config.
    pub fn new(attribution_method: AttributionMethod, config: FinstackConfig) -> Self {
        Self {
            attribution_method,
            config,
            engine: ScenarioEngine::new(),
        }
    }
}

/// Result of a horizon total return computation.
///
/// Wraps a [`PnlAttribution`] with scenario context and convenience
/// accessors for total return percentage and annualized return.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HorizonResult {
    /// Full factor-decomposed P&L from the attribution framework.
    pub attribution: PnlAttribution,
    /// Initial instrument value at (market_t0, as_of_t0).
    pub initial_value: Money,
    /// Final instrument value at (market_t1, as_of_t1).
    pub terminal_value: Money,
    /// Number of calendar days in the horizon (`None` if no time-roll in spec).
    pub horizon_days: Option<i64>,
    /// Report from scenario engine application.
    pub scenario_report: ApplicationReport,
}

impl HorizonResult {
    /// Total return as a decimal fraction (e.g. 0.05 = 5%).
    ///
    /// Computed as `total_pnl / initial_value`.  Returns 0.0 if the initial
    /// value is zero to avoid division by zero.
    pub fn total_return_pct(&self) -> f64 {
        let iv = self.initial_value.amount();
        if iv == 0.0 {
            return 0.0;
        }
        self.attribution.total_pnl.amount() / iv
    }

    /// Annualized total return.
    ///
    /// Uses `(1 + total_return_pct)^(365 / horizon_days) - 1`.
    /// Returns `None` when there is no time-roll in the scenario (i.e.
    /// `horizon_days` is `None`).
    pub fn annualized_return(&self) -> Option<f64> {
        let days = self.horizon_days? as f64;
        if days <= 0.0 {
            return None;
        }
        let tr = self.total_return_pct();
        Some((1.0 + tr).powf(365.0 / days) - 1.0)
    }

    /// A single factor's P&L as a fraction of initial value.
    ///
    /// Returns 0.0 if initial value is zero.
    pub fn factor_contribution(&self, factor: &AttributionFactor) -> f64 {
        let iv = self.initial_value.amount();
        if iv == 0.0 {
            return 0.0;
        }
        let pnl = match factor {
            AttributionFactor::Carry => self.attribution.carry.amount(),
            AttributionFactor::RatesCurves => self.attribution.rates_curves_pnl.amount(),
            AttributionFactor::CreditCurves => self.attribution.credit_curves_pnl.amount(),
            AttributionFactor::InflationCurves => self.attribution.inflation_curves_pnl.amount(),
            AttributionFactor::Correlations => self.attribution.correlations_pnl.amount(),
            AttributionFactor::Fx => self.attribution.fx_pnl.amount(),
            AttributionFactor::Volatility => self.attribution.vol_pnl.amount(),
            AttributionFactor::ModelParameters => self.attribution.model_params_pnl.amount(),
            AttributionFactor::MarketScalars => self.attribution.market_scalars_pnl.amount(),
        };
        pnl / iv
    }
}
```

- [ ] **Step 2: Register the module in `lib.rs`**

Add to `finstack/scenarios/src/lib.rs` after the existing module declarations:

```rust
/// Horizon total return analysis.
pub mod horizon;
```

And add to the re-exports at the bottom of the file:

```rust
pub use horizon::{HorizonAnalysis, HorizonResult};
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p finstack-scenarios`
Expected: Compiles with no errors (types defined but `compute()` not yet implemented).

- [ ] **Step 4: Commit**

```bash
git add finstack/scenarios/src/horizon.rs finstack/scenarios/src/lib.rs
git commit -m "feat(horizon): add HorizonAnalysis and HorizonResult types"
```

---

### Task 2: Implement `HorizonAnalysis::compute()`

**Files:**
- Modify: `finstack/scenarios/src/horizon.rs`

- [ ] **Step 1: Add the `compute` method to `HorizonAnalysis`**

Add this `impl` block after the existing `HorizonAnalysis::new()` impl block in `horizon.rs`:

```rust
impl HorizonAnalysis {
    /// Compute horizon total return under a scenario.
    ///
    /// Applies the [`ScenarioSpec`] to the provided market context (cloned
    /// internally) and runs P&L attribution between the original and
    /// scenario-modified states.
    ///
    /// The spec may include a [`OperationSpec::TimeRollForward`] to define
    /// the holding period.  If no time-roll is present, the analysis is a
    /// pure mark-to-scenario (carry will be zero, `horizon_days` will be
    /// `None`).
    ///
    /// # Errors
    ///
    /// Returns an error if scenario application or attribution fails (e.g.
    /// missing market data for a curve referenced in the spec).
    pub fn compute(
        &self,
        instrument: &Arc<dyn Instrument>,
        market_t0: &MarketContext,
        as_of_t0: Date,
        scenario: &ScenarioSpec,
    ) -> crate::Result<HorizonResult> {
        // 1. Price at t0
        let initial_value = instrument
            .value(market_t0, as_of_t0)
            .map_err(|e| crate::Error::Internal(format!("t0 pricing failed: {e}")))?;

        // 2. Clone market and build execution context
        let mut market_t1 = market_t0.clone();
        let mut model = finstack_statements::FinancialModelSpec::new("__horizon_temp__", vec![]);
        let mut ctx = ExecutionContext {
            market: &mut market_t1,
            model: &mut model,
            instruments: None,
            rate_bindings: None,
            calendar: None,
            as_of: as_of_t0,
        };

        // 3. Apply scenario
        let scenario_report = self.engine.apply(scenario, &mut ctx)?;
        let as_of_t1 = ctx.as_of;

        // 4. Derive horizon
        let diff_days = (as_of_t1 - as_of_t0).whole_days();
        let horizon_days = if diff_days > 0 {
            Some(diff_days)
        } else {
            None
        };

        // 5. Run attribution
        let attribution = self.run_attribution(
            instrument,
            market_t0,
            &market_t1,
            as_of_t0,
            as_of_t1,
        )?;

        // 6. Price at t1
        let terminal_value = instrument
            .value(&market_t1, as_of_t1)
            .map_err(|e| crate::Error::Internal(format!("t1 pricing failed: {e}")))?;

        Ok(HorizonResult {
            attribution,
            initial_value,
            terminal_value,
            horizon_days,
            scenario_report,
        })
    }

    /// Dispatch to the appropriate attribution function based on `self.attribution_method`.
    fn run_attribution(
        &self,
        instrument: &Arc<dyn Instrument>,
        market_t0: &MarketContext,
        market_t1: &MarketContext,
        as_of_t0: Date,
        as_of_t1: Date,
    ) -> crate::Result<PnlAttribution> {
        let result = match &self.attribution_method {
            AttributionMethod::Parallel => attribute_pnl_parallel(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                &self.config,
                None,
            ),
            AttributionMethod::Waterfall(order) => attribute_pnl_waterfall(
                instrument,
                market_t0,
                market_t1,
                as_of_t0,
                as_of_t1,
                &self.config,
                order.clone(),
                false,
                None,
            ),
            AttributionMethod::MetricsBased => {
                let metrics = default_attribution_metrics();
                let val_t0 = instrument
                    .price_with_metrics(market_t0, as_of_t0, &metrics, PricingOptions::default())
                    .map_err(|e| {
                        crate::Error::Internal(format!("t0 metrics pricing failed: {e}"))
                    })?;
                let val_t1 = instrument
                    .price_with_metrics(market_t1, as_of_t1, &metrics, PricingOptions::default())
                    .map_err(|e| {
                        crate::Error::Internal(format!("t1 metrics pricing failed: {e}"))
                    })?;
                attribute_pnl_metrics_based(
                    instrument, market_t0, market_t1, &val_t0, &val_t1, as_of_t0, as_of_t1,
                )
            }
            AttributionMethod::Taylor(config) => attribute_pnl_taylor_standard(
                instrument, market_t0, market_t1, as_of_t0, as_of_t1, config,
            ),
        };
        result.map_err(|e| crate::Error::Internal(format!("attribution failed: {e}")))
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p finstack-scenarios`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add finstack/scenarios/src/horizon.rs
git commit -m "feat(horizon): implement HorizonAnalysis::compute()"
```

---

### Task 3: Rust Unit Tests

**Files:**
- Modify: `finstack/scenarios/src/horizon.rs` (add `#[cfg(test)]` module)

- [ ] **Step 1: Add test module with helper and first test**

Append to the bottom of `finstack/scenarios/src/horizon.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use finstack_core::currency::Currency;
    use finstack_core::dates::DayCount;
    use finstack_core::market_data::term_structures::DiscountCurve;
    use finstack_core::money::Money;
    use finstack_core::types::CurveId;
    use finstack_valuations::instruments::fixed_income::bond::{Bond, CashflowSpec};
    use finstack_valuations::instruments::pricing_overrides::PricingOverrides;
    use finstack_valuations::instruments::Attributes;
    use time::macros::date;

    /// Build a simple 2-year fixed-rate bond for testing.
    fn test_bond(base_date: Date) -> Arc<dyn Instrument> {
        Arc::new(
            Bond::builder()
                .id("TEST-BOND".into())
                .notional(Money::new(100.0, Currency::USD))
                .issue_date(base_date)
                .maturity(base_date + time::Duration::days(730))
                .cashflow_spec(CashflowSpec::fixed(
                    0.05,
                    finstack_core::dates::Tenor::annual(),
                    DayCount::Thirty360,
                ))
                .discount_curve_id(CurveId::new("USD-OIS"))
                .credit_curve_id_opt(None)
                .pricing_overrides(PricingOverrides::default())
                .attributes(Attributes::new())
                .build()
                .expect("bond builder should succeed"),
        )
    }

    /// Build a market with a flat discount curve.
    fn test_market(base_date: Date) -> MarketContext {
        let curve = DiscountCurve::builder("USD-OIS")
            .base_date(base_date)
            .knots(vec![(0.0, 1.0), (1.0, 0.98), (2.0, 0.95), (5.0, 0.90)])
            .build()
            .expect("curve builder should succeed");
        MarketContext::new().insert(curve)
    }

    #[test]
    fn no_op_scenario_returns_zero_pnl() {
        let as_of = date!(2025 - 01 - 15);
        let instrument = test_bond(as_of);
        let market = test_market(as_of);

        let scenario = ScenarioSpec {
            id: "no_op".into(),
            name: None,
            description: None,
            operations: vec![],
            priority: 0,
            resolution_mode: Default::default(),
        };

        let analyzer = HorizonAnalysis::default();
        let result = analyzer.compute(&instrument, &market, as_of, &scenario).unwrap();

        assert!(
            result.attribution.total_pnl.amount().abs() < 1e-10,
            "no-op scenario should produce zero P&L, got {}",
            result.attribution.total_pnl.amount()
        );
        assert!(result.horizon_days.is_none());
        assert!(result.annualized_return().is_none());
        assert!((result.total_return_pct()).abs() < 1e-10);
    }
}
```

- [ ] **Step 2: Run the first test**

Run: `cargo nextest run -p finstack-scenarios no_op_scenario`
Expected: PASS

- [ ] **Step 3: Add time-roll-only test (carry isolation)**

Add to the test module:

```rust
    #[test]
    fn time_roll_only_has_horizon_days_and_carry() {
        let as_of = date!(2025 - 01 - 15);
        let instrument = test_bond(as_of);
        let market = test_market(as_of);

        let scenario = ScenarioSpec {
            id: "roll_1m".into(),
            name: None,
            description: None,
            operations: vec![crate::OperationSpec::TimeRollForward {
                period: "1M".into(),
                apply_shocks: false,
                roll_mode: crate::TimeRollMode::BusinessDays,
            }],
            priority: 0,
            resolution_mode: Default::default(),
        };

        let analyzer = HorizonAnalysis::default();
        let result = analyzer.compute(&instrument, &market, as_of, &scenario).unwrap();

        // Should have a horizon
        assert!(result.horizon_days.is_some());
        assert!(result.horizon_days.unwrap() > 0);

        // Annualized return should be computable
        assert!(result.annualized_return().is_some());

        // Total P&L should equal carry (since no market shocks)
        // (within tolerance — parallel attribution may have small residual)
        let carry = result.attribution.carry.amount();
        let total = result.attribution.total_pnl.amount();
        assert!(
            (total - carry).abs() < 0.01,
            "time-roll only: total ({total}) should approximately equal carry ({carry})"
        );
    }
```

- [ ] **Step 4: Run the test**

Run: `cargo nextest run -p finstack-scenarios time_roll_only`
Expected: PASS

- [ ] **Step 5: Add shock-only test (no carry)**

Add to the test module:

```rust
    #[test]
    fn shock_only_has_no_horizon_and_zero_carry() {
        let as_of = date!(2025 - 01 - 15);
        let instrument = test_bond(as_of);
        let market = test_market(as_of);

        let scenario = ScenarioSpec {
            id: "rate_shock".into(),
            name: None,
            description: None,
            operations: vec![crate::OperationSpec::CurveParallelBp {
                curve_kind: crate::CurveKind::Discount,
                curve_id: "USD-OIS".into(),
                discount_curve_id: None,
                bp: 50.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
        };

        let analyzer = HorizonAnalysis::default();
        let result = analyzer.compute(&instrument, &market, as_of, &scenario).unwrap();

        // No time-roll → no horizon
        assert!(result.horizon_days.is_none());
        assert!(result.annualized_return().is_none());

        // Carry should be zero (no time passage)
        assert!(
            result.attribution.carry.amount().abs() < 1e-10,
            "shock-only: carry should be zero, got {}",
            result.attribution.carry.amount()
        );

        // Rates P&L should be non-zero (50bp parallel shock)
        assert!(
            result.attribution.rates_curves_pnl.amount().abs() > 1e-6,
            "shock-only: rates P&L should be non-zero"
        );
    }
```

- [ ] **Step 6: Run the test**

Run: `cargo nextest run -p finstack-scenarios shock_only`
Expected: PASS

- [ ] **Step 7: Add return calculation test**

Add to the test module:

```rust
    #[test]
    fn total_return_pct_matches_pnl_over_initial() {
        let as_of = date!(2025 - 01 - 15);
        let instrument = test_bond(as_of);
        let market = test_market(as_of);

        let scenario = ScenarioSpec {
            id: "rate_shock".into(),
            name: None,
            description: None,
            operations: vec![crate::OperationSpec::CurveParallelBp {
                curve_kind: crate::CurveKind::Discount,
                curve_id: "USD-OIS".into(),
                discount_curve_id: None,
                bp: 50.0,
            }],
            priority: 0,
            resolution_mode: Default::default(),
        };

        let analyzer = HorizonAnalysis::default();
        let result = analyzer.compute(&instrument, &market, as_of, &scenario).unwrap();

        let expected_pct = result.attribution.total_pnl.amount() / result.initial_value.amount();
        assert!(
            (result.total_return_pct() - expected_pct).abs() < 1e-12,
            "total_return_pct() should match manual calculation"
        );
    }
```

- [ ] **Step 8: Run all horizon tests**

Run: `cargo nextest run -p finstack-scenarios horizon`
Expected: All PASS

- [ ] **Step 9: Commit**

```bash
git add finstack/scenarios/src/horizon.rs
git commit -m "test(horizon): add unit tests for HorizonAnalysis"
```

---

### Task 4: Python Bindings

**Files:**
- Create: `finstack-py/src/bindings/scenarios/horizon.rs`
- Modify: `finstack-py/src/bindings/scenarios/mod.rs`

- [ ] **Step 1: Create the Python binding file**

```rust
// finstack-py/src/bindings/scenarios/horizon.rs

//! Python bindings for horizon total return analysis.

use crate::bindings::extract::extract_market;
use crate::bindings::valuations::attribution::PyPnlAttribution;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

fn horizon_to_py(e: impl std::fmt::Display) -> PyErr {
    PyValueError::new_err(e.to_string())
}

/// Compute horizon total return under a scenario.
///
/// Applies a scenario specification (which may include time-roll and market
/// shocks) to project an instrument forward, then decomposes the resulting
/// P&L using factor-based attribution.
///
/// Parameters
/// ----------
/// instrument_json : str
///     JSON-serialized instrument (tagged: ``{"type": "bond", "spec": {...}}``).
/// market : MarketContext | str
///     A ``MarketContext`` object or JSON string.
/// as_of : str
///     Valuation date in ISO 8601 format (e.g. ``"2025-01-15"``).
/// scenario_json : str
///     JSON-serialized ``ScenarioSpec``.
/// method : str, optional
///     Attribution method: ``"parallel"`` (default), ``"waterfall"``,
///     ``"metrics_based"``, or ``"taylor"``.
///
/// Returns
/// -------
/// HorizonResult
///     Decomposed total return with factor attribution.
#[pyfunction]
#[pyo3(signature = (instrument_json, market, as_of, scenario_json, method = "parallel"))]
pub(crate) fn compute_horizon_return<'py>(
    _py: Python<'py>,
    instrument_json: &str,
    market: &Bound<'py, PyAny>,
    as_of: &str,
    scenario_json: &str,
    method: &str,
) -> PyResult<PyHorizonResult> {
    use finstack_valuations::attribution::AttributionMethod;
    use finstack_valuations::instruments::InstrumentJson;
    use std::sync::Arc;

    // Parse instrument
    let inst: InstrumentJson =
        serde_json::from_str(instrument_json).map_err(horizon_to_py)?;
    let boxed = inst.into_boxed().map_err(horizon_to_py)?;
    let instrument: Arc<dyn finstack_valuations::instruments::internal::InstrumentExt> =
        Arc::from(boxed);

    // Parse market
    let market_ctx = extract_market(market)?;

    // Parse date
    let date = super::parse_date(as_of)?;

    // Parse scenario
    let scenario: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(horizon_to_py)?;

    // Parse method
    let attribution_method = match method {
        "parallel" => AttributionMethod::Parallel,
        "waterfall" => {
            AttributionMethod::Waterfall(
                finstack_valuations::attribution::default_waterfall_order(),
            )
        }
        "metrics_based" => AttributionMethod::MetricsBased,
        "taylor" => {
            AttributionMethod::Taylor(
                finstack_valuations::attribution::TaylorAttributionConfig::default(),
            )
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "Unknown attribution method '{other}'. Expected: parallel, waterfall, metrics_based, taylor"
            )));
        }
    };

    // Run analysis
    let analyzer = finstack_scenarios::horizon::HorizonAnalysis::new(
        attribution_method,
        finstack_core::config::FinstackConfig::default(),
    );
    let result = analyzer
        .compute(&instrument, &market_ctx, date, &scenario)
        .map_err(horizon_to_py)?;

    Ok(PyHorizonResult { inner: result })
}

/// Horizon total return result.
///
/// Wraps a full P&L attribution with scenario context and convenience
/// accessors for total return percentage, annualized return, and
/// per-factor contributions.
#[pyclass(
    name = "HorizonResult",
    module = "finstack.scenarios",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyHorizonResult {
    inner: finstack_scenarios::horizon::HorizonResult,
}

#[pymethods]
impl PyHorizonResult {
    /// Full P&L attribution breakdown.
    #[getter]
    fn attribution(&self) -> PyPnlAttribution {
        PyPnlAttribution {
            inner: self.inner.attribution.clone(),
        }
    }

    /// Initial instrument value.
    #[getter]
    fn initial_value(&self) -> f64 {
        self.inner.initial_value.amount()
    }

    /// Final instrument value after scenario.
    #[getter]
    fn terminal_value(&self) -> f64 {
        self.inner.terminal_value.amount()
    }

    /// Horizon in calendar days (``None`` if no time-roll).
    #[getter]
    fn horizon_days(&self) -> Option<i64> {
        self.inner.horizon_days
    }

    /// Total return as decimal fraction (0.05 = 5%).
    #[getter]
    fn total_return_pct(&self) -> f64 {
        self.inner.total_return_pct()
    }

    /// Annualized return (``None`` if no time-roll).
    #[getter]
    fn annualized_return(&self) -> Option<f64> {
        self.inner.annualized_return()
    }

    /// Number of scenario operations applied.
    #[getter]
    fn operations_applied(&self) -> usize {
        self.inner.scenario_report.operations_applied
    }

    /// Warnings from scenario application.
    #[getter]
    fn warnings(&self) -> Vec<String> {
        self.inner.scenario_report.warnings.clone()
    }

    /// Factor contribution as decimal fraction of initial value.
    fn factor_contribution(&self, factor: &str) -> PyResult<f64> {
        use finstack_valuations::attribution::AttributionFactor;
        let f = match factor {
            "carry" => AttributionFactor::Carry,
            "rates" | "rates_curves" => AttributionFactor::RatesCurves,
            "credit" | "credit_curves" => AttributionFactor::CreditCurves,
            "inflation" | "inflation_curves" => AttributionFactor::InflationCurves,
            "correlations" => AttributionFactor::Correlations,
            "fx" => AttributionFactor::Fx,
            "volatility" | "vol" => AttributionFactor::Volatility,
            "model_parameters" | "model_params" => AttributionFactor::ModelParameters,
            "market_scalars" | "scalars" => AttributionFactor::MarketScalars,
            other => {
                return Err(PyValueError::new_err(format!(
                    "Unknown factor '{other}'"
                )));
            }
        };
        Ok(self.inner.factor_contribution(&f))
    }

    /// Serialize to JSON.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(&self.inner).map_err(horizon_to_py)
    }

    /// Human-readable summary.
    fn explain(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "Horizon Total Return: {:.4}%\n",
            self.inner.total_return_pct() * 100.0
        ));
        if let Some(ann) = self.inner.annualized_return() {
            s.push_str(&format!("Annualized: {:.4}%\n", ann * 100.0));
        }
        if let Some(days) = self.inner.horizon_days {
            s.push_str(&format!("Horizon: {} days\n", days));
        }
        s.push_str(&format!(
            "Initial Value: {}\n",
            self.inner.initial_value
        ));
        s.push_str(&format!(
            "Terminal Value: {}\n",
            self.inner.terminal_value
        ));
        s.push_str(&format!(
            "Total P&L: {}\n",
            self.inner.attribution.total_pnl
        ));
        s.push_str(&format!(
            "  Carry: {}\n",
            self.inner.attribution.carry
        ));
        s.push_str(&format!(
            "  Rates: {}\n",
            self.inner.attribution.rates_curves_pnl
        ));
        s.push_str(&format!(
            "  Credit: {}\n",
            self.inner.attribution.credit_curves_pnl
        ));
        s.push_str(&format!(
            "  Residual: {}\n",
            self.inner.attribution.residual
        ));
        s
    }

    fn __repr__(&self) -> String {
        format!(
            "HorizonResult(total_return={:.4}%, horizon_days={:?})",
            self.inner.total_return_pct() * 100.0,
            self.inner.horizon_days,
        )
    }
}

/// Register horizon functions on the scenarios submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyHorizonResult>()?;
    m.add_function(pyo3::wrap_pyfunction!(compute_horizon_return, m)?)?;
    Ok(())
}
```

- [ ] **Step 2: Register in `mod.rs`**

In `finstack-py/src/bindings/scenarios/mod.rs`, add the module declaration after `mod engine;`:

```rust
mod horizon;
```

In the `register()` function, add after `engine::register(py, &m)?;`:

```rust
    horizon::register(py, &m)?;
```

In the `__all__` list, add the new exports:

```rust
    "compute_horizon_return",
    "HorizonResult",
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check -p finstack-py`
Expected: Compiles with no errors.

- [ ] **Step 4: Commit**

```bash
git add finstack-py/src/bindings/scenarios/horizon.rs finstack-py/src/bindings/scenarios/mod.rs
git commit -m "feat(python): add horizon total return bindings"
```

---

### Task 5: WASM Bindings

**Files:**
- Modify: `finstack-wasm/src/api/scenarios/mod.rs`

- [ ] **Step 1: Add `computeHorizonReturn` function**

Add to the bottom of `finstack-wasm/src/api/scenarios/mod.rs`, before the closing of the file:

```rust
/// Compute horizon total return under a scenario.
///
/// Applies a scenario specification to project an instrument forward, then
/// decomposes the resulting P&L using factor-based attribution.
///
/// # Arguments
///
/// * `instrument_json` - JSON-serialized instrument (tagged).
/// * `market_json` - JSON-serialized `MarketContext`.
/// * `as_of` - Valuation date (ISO 8601).
/// * `scenario_json` - JSON-serialized `ScenarioSpec`.
/// * `method` - Attribution method: "parallel", "waterfall", "metrics_based", "taylor".
///
/// # Returns
///
/// JSON-serialized `HorizonResult`.
#[wasm_bindgen(js_name = computeHorizonReturn)]
pub fn compute_horizon_return(
    instrument_json: &str,
    market_json: &str,
    as_of: &str,
    scenario_json: &str,
    method: Option<String>,
) -> Result<String, JsValue> {
    use finstack_valuations::attribution::AttributionMethod;
    use finstack_valuations::instruments::InstrumentJson;
    use std::sync::Arc;

    // Parse instrument
    let inst: InstrumentJson =
        serde_json::from_str(instrument_json).map_err(to_js_err)?;
    let boxed = inst.into_boxed().map_err(to_js_err)?;
    let instrument: Arc<dyn finstack_valuations::instruments::internal::InstrumentExt> =
        Arc::from(boxed);

    // Parse market
    let market: finstack_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;

    // Parse date
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;

    // Parse scenario
    let scenario: finstack_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;

    // Parse method
    let method_str = method.as_deref().unwrap_or("parallel");
    let attribution_method = match method_str {
        "parallel" => AttributionMethod::Parallel,
        "waterfall" => {
            AttributionMethod::Waterfall(
                finstack_valuations::attribution::default_waterfall_order(),
            )
        }
        "metrics_based" => AttributionMethod::MetricsBased,
        "taylor" => {
            AttributionMethod::Taylor(
                finstack_valuations::attribution::TaylorAttributionConfig::default(),
            )
        }
        other => return Err(to_js_err(format!(
            "Unknown attribution method '{other}'. Expected: parallel, waterfall, metrics_based, taylor"
        ))),
    };

    // Run analysis
    let analyzer = finstack_scenarios::horizon::HorizonAnalysis::new(
        attribution_method,
        finstack_core::config::FinstackConfig::default(),
    );
    let result = analyzer
        .compute(&instrument, &market, date, &scenario)
        .map_err(to_js_err)?;

    serde_json::to_string(&result).map_err(to_js_err)
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p finstack-wasm`
Expected: Compiles with no errors.

- [ ] **Step 3: Commit**

```bash
git add finstack-wasm/src/api/scenarios/mod.rs
git commit -m "feat(wasm): add computeHorizonReturn binding"
```

---

### Task 6: Full Test Suite and Clippy

**Files:**
- No new files. Runs existing tests + clippy.

- [ ] **Step 1: Run clippy on all changed crates**

Run: `cargo clippy -p finstack-scenarios -p finstack-py -p finstack-wasm -- -D warnings`
Expected: No warnings. If there are clippy issues, fix them before proceeding.

- [ ] **Step 2: Run all scenario tests**

Run: `cargo nextest run -p finstack-scenarios`
Expected: All tests pass (including new horizon tests).

- [ ] **Step 3: Run Python binding build check**

Run: `cargo check -p finstack-py`
Expected: Compiles cleanly.

- [ ] **Step 4: Run WASM binding build check**

Run: `cargo check -p finstack-wasm --target wasm32-unknown-unknown`
Expected: Compiles cleanly for WASM target.

- [ ] **Step 5: Fix any issues found, then commit**

```bash
git add -u
git commit -m "fix(horizon): address clippy and test issues"
```

(Skip this commit if no fixes were needed.)
