# Horizon Total Return Analyzer

**Date:** 2026-04-12
**Module:** `finstack-scenarios`
**Status:** Design

## Problem

The codebase can compute carry, roll-down, and spread sensitivity independently. But there is no framework that answers: "If I hold this instrument for 3 months and spreads widen 25bp, what's my total return?" — composing carry + roll + spread change + default into a single horizon P&L under user assumptions. The scenario engine, carry decomposition, and CS01 have all the ingredients; the composition layer is missing.

## Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Instrument scope | Any instrument (trait-based) | Works through `Arc<dyn InstrumentExt>` (the internal instrument trait used by attribution), not bond-specific |
| User assumptions | Full `ScenarioSpec` composition | Leverages the entire scenario engine — curve shifts, vol changes, FX moves, time rolls. No restricted subset. |
| Attribution | Reuse existing P&L attribution | The horizon analyzer constructs the t1 market state; attribution decomposes the P&L. No new decomposition logic. |
| Time model | Everything in `ScenarioSpec` | No separate horizon parameter. Time-roll is a `TimeRollForward` operation in the spec like any other. |
| Module location | `finstack-scenarios` | The scenario module already owns market projection and depends on `finstack-valuations`. No new dependency edges. |
| API shape | `HorizonAnalysis` struct + `HorizonResult` | Struct holds config (attribution method, engine), result wraps `PnlAttribution` with convenience accessors. |

## Architecture

### Core Types

**File:** `finstack/scenarios/src/horizon.rs`

#### `HorizonAnalysis`

Stateless configuration struct that holds the attribution method and scenario engine:

```rust
pub struct HorizonAnalysis {
    /// Attribution methodology for decomposing the horizon P&L.
    pub attribution_method: AttributionMethod,
    /// Finstack configuration (rounding, tolerances).
    pub config: FinstackConfig,
    /// Scenario engine instance.
    pub engine: ScenarioEngine,
}
```

Constructors:
- `HorizonAnalysis::new(method: AttributionMethod, config: FinstackConfig)` — full control.
- `HorizonAnalysis::default()` — parallel attribution, default config, default engine.

#### `HorizonResult`

Wraps `PnlAttribution` with scenario context and convenience accessors:

```rust
pub struct HorizonResult {
    /// Full factor-decomposed P&L from the attribution framework.
    pub attribution: PnlAttribution,
    /// Initial instrument value at (market_t0, as_of_t0).
    pub initial_value: Money,
    /// Final instrument value at (market_t1, as_of_t1).
    pub terminal_value: Money,
    /// Number of calendar days in the horizon (None if no time-roll in spec).
    pub horizon_days: Option<i64>,
    /// Report from scenario engine application.
    pub scenario_report: ApplicationReport,
}
```

Convenience methods:
- `total_return_pct(&self) -> f64` — `total_pnl.amount() / initial_value.amount()`
- `annualized_return(&self) -> Option<f64>` — `(1 + total_return_pct)^(365/horizon_days) - 1`, or `None` if no time-roll.
- `factor_contribution(&self, factor: &AttributionFactor) -> f64` — factor P&L as fraction of initial value.

### Computation Flow

**Entry point:** `HorizonAnalysis::compute()`

```rust
pub fn compute(
    &self,
    instrument: &Arc<dyn InstrumentExt>,
    market_t0: &MarketContext,
    as_of_t0: Date,
    scenario: &ScenarioSpec,
) -> Result<HorizonResult>
```

**Internal steps:**

1. **Price at t0** — `instrument.value(market_t0, as_of_t0)` to get `initial_value`.
2. **Clone market** — `market_t0.clone()` into `market_t1`.
3. **Build ExecutionContext** — with `&mut market_t1`, `as_of: as_of_t0`, and a minimal empty `FinancialModelSpec` (the scenario engine tolerates this for instrument-only analysis).
4. **Apply scenario** — `self.engine.apply(scenario, &mut ctx)` mutates `market_t1` and advances `ctx.as_of` if a `TimeRollForward` is present. Captures the `ApplicationReport`.
5. **Extract as_of_t1** — from `ctx.as_of` after apply.
6. **Extract horizon_days** — `(as_of_t1 - as_of_t0).whole_days()`, stored as `None` if zero (no time-roll).
7. **Run attribution** — dispatch on `self.attribution_method`:
   - `Parallel` → `attribute_pnl_parallel(instrument, market_t0, &market_t1, as_of_t0, as_of_t1, &self.config, None)`
   - `Waterfall(order)` → `attribute_pnl_waterfall(...)` with the specified factor order
   - `MetricsBased` → requires calling `instrument.price_with_metrics()` at both t0 and t1 (with `default_attribution_metrics()`) before passing the two `ValuationResult`s to `attribute_pnl_metrics_based()`
   - `Taylor(config)` → `attribute_pnl_taylor(...)`
8. **Price at t1** — `instrument.value(&market_t1, as_of_t1)` to get `terminal_value`.
9. **Assemble HorizonResult** from the above components.

### Edge Cases and Semantics

**No time-roll in spec:** Valid — pure mark-to-scenario. Carry is zero, `horizon_days` is `None`, `annualized_return()` returns `None`. Result is the scenario-driven P&L decomposition only.

**Time-roll only, no shocks:** Valid — pure carry/roll-down analysis. All P&L lands in the carry factor. Answers "what do I earn holding this for N months assuming markets don't move?"

**Multiple time-rolls:** The scenario engine processes operations in order. The final `ctx.as_of` is the effective horizon date. `horizon_days` reflects the cumulative roll.

**Instrument expires within horizon:** Pricing infrastructure already handles this — cashflows after maturity are empty, value converges to zero/par. No special handling needed.

**Attribution residual:** Same behavior as existing attribution. `PnlAttribution` carries `residual` and `residual_pct` in metadata. The horizon analyzer introduces no new approximation.

**FinancialModelSpec:** `ExecutionContext` requires a `&mut FinancialModelSpec`. For instrument-only analysis, an empty model is constructed internally. If users later need statement-linked scenarios, the API can be extended with an optional model parameter.

## Public API Surface (Rust)

```rust
// In finstack/scenarios/src/lib.rs — new exports
pub mod horizon;
pub use horizon::{HorizonAnalysis, HorizonResult};

// Construction
HorizonAnalysis::new(method: AttributionMethod, config: FinstackConfig) -> Self
HorizonAnalysis::default() -> Self

// Computation (InstrumentExt is the internal instrument trait from finstack-valuations)
HorizonAnalysis::compute(
    &self,
    instrument: &Arc<dyn InstrumentExt>,
    market_t0: &MarketContext,
    as_of_t0: Date,
    scenario: &ScenarioSpec,
) -> Result<HorizonResult>

// Result access
HorizonResult::total_return_pct(&self) -> f64
HorizonResult::annualized_return(&self) -> Option<f64>
HorizonResult::factor_contribution(&self, factor: &AttributionFactor) -> f64
```

## Python Bindings

**File:** `finstack-py/src/bindings/scenarios/horizon.rs`

Follows the established binding pattern: instruments and scenario specs as JSON strings, market context as JSON or `PyMarketContext` (polymorphic extraction), results as `#[pyclass]` wrappers.

### Functions

```python
def compute_horizon_return(
    instrument_json: str,
    market: MarketContext | str,  # PyMarketContext or JSON
    as_of: str,                   # ISO 8601 date
    scenario_json: str,           # ScenarioSpec JSON
    method: str = "parallel",     # "parallel" | "waterfall" | "metrics_based" | "taylor"
    config: str | None = None,    # Optional FinstackConfig JSON
) -> HorizonResult: ...
```

### `HorizonResult` pyclass

Properties:
- `.attribution` — `PyPnlAttribution` (reuses existing wrapper)
- `.initial_value` — `f64`
- `.terminal_value` — `f64`
- `.horizon_days` — `int | None`
- `.total_return_pct` — `f64`
- `.annualized_return` — `float | None`
- `.scenario_report` — `dict` (serialized `ApplicationReport`)

Methods:
- `.factor_contribution(factor: str) -> float`
- `.to_json() -> str` — full result as JSON
- `.explain() -> str` — human-readable summary

### Module Registration

Added to `finstack-py/src/bindings/scenarios/mod.rs` via existing `register()` pattern.

## WASM Bindings

**File:** `finstack-wasm/src/api/scenarios/horizon.rs`

Follows the established WASM pattern: all inputs as JSON strings, results as JSON strings, camelCase function names.

### Functions

```typescript
// TypeScript signature (exposed via wasm-bindgen)
function computeHorizonReturn(
    instrumentJson: string,
    marketJson: string,
    asOf: string,
    scenarioJson: string,
    method?: string,
    configJson?: string,
): string;  // HorizonResult JSON
```

Implementation: deserialize inputs, construct `HorizonAnalysis`, call `.compute()`, serialize result to JSON.

## Testing Strategy

### Rust (`finstack/scenarios/src/horizon.rs` — `#[cfg(test)]` module)

1. **Carry-only** — time-roll with no shocks. Assert P&L lands in carry factor, other factors are zero (within tolerance).
2. **Shock-only** — spread or rate shock with no time-roll. Assert carry is zero, `horizon_days` is `None`, P&L lands in the appropriate factor.
3. **Combined** — time-roll + spread widening. Assert `total_pnl == sum(factors) + residual`.
4. **Return calculations** — `total_return_pct()` matches `total_pnl / initial_value`. `annualized_return()` is `None` without time-roll, computed with it.
5. **No-op scenario** — empty spec. Assert zero P&L, zero return.

### Python (`finstack-py/tests/`)

1. JSON round-trip — construct scenario spec as JSON, run `compute_horizon_return()`, verify result properties.
2. Consistency with Rust — same inputs produce matching `total_return_pct` values.
3. `PyPnlAttribution` reuse — `.attribution` property returns the same type as `attribute_pnl()`.

### WASM

Follow existing WASM test patterns. Verify JSON round-trip and that `computeHorizonReturn()` returns valid JSON matching the `HorizonResult` schema.

## Files Changed

| File | Change |
|------|--------|
| `finstack/scenarios/src/horizon.rs` | **New.** Core `HorizonAnalysis` and `HorizonResult` types + computation logic. |
| `finstack/scenarios/src/lib.rs` | Add `pub mod horizon` and re-exports. |
| `finstack-py/src/bindings/scenarios/horizon.rs` | **New.** Python binding for `compute_horizon_return()` and `HorizonResult` pyclass. |
| `finstack-py/src/bindings/scenarios/mod.rs` | Register horizon bindings. |
| `finstack-wasm/src/api/scenarios/horizon.rs` | **New.** WASM binding for `computeHorizonReturn()`. |
| `finstack-wasm/src/api/scenarios/mod.rs` | Register horizon WASM function. |
