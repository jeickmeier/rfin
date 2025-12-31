# Market Standards Review: `finstack/scenarios`

## 1. Executive Summary

The `finstack/scenarios` crate provides a lightweight, deterministic engine for applying market data shocks and financial statement adjustments. It is generally well-structured, safe, and adheres to many market conventions.

**Biggest Wins:**

* **Determinism:** Strong focus on deterministic execution ordering (priority-based) and reproducible results (no `HashMap`, only `IndexMap`).
* **Safety:** `unwrap` is properly denied; error propagation is robust.
* **Arbitrage Checks:** Volatility surface adapter includes basic calendar spread and positivity checks.
* **Architecture:** Separation of `spec` (data) from `engine` (logic) allows for easy serialization and distributed execution.

**Biggest Risks:**

* **Math Concessions:** The Hazard curve (CDS) bumping logic uses a first-order approximation (`lambda ~ spread / (1-R)`) which breaks down for distressed credits or high recovery rates.
* **Inflation Conventions:** Inflation curve shocks are applied as simple multiplicative factors on CPI levels without explicit handling of seasoning or index lag conventions.

**Top 5 Action Items:**

1. [Major] **Fix Hazard Curve Bumping:** Replace the approximate lambda bump with an exact bootstrap-based bump or a robust root-solver approach to handle high spreads/recoveries correctly.
2. [Minor] **Enhance Inflation Shocks:** Add support for additive inflation swaps or real-rate bumps, rather than just multiplicative CPI level bumps.
3. [Minor] **Standardize Tenor Parsing:** Ensure `parse_tenor_to_years_with_context` is used by default in all adapters where a context is available, deprecating the simple approximation.
4. [Info] **Performance bench:** Add micro-benchmarks for the `apply` loop to ensure curve rebuilding overhead is acceptable for large scenarios.
5. [Info] **Documentation:** Explicitly document the "linear on rates" interpolation assumption for curve nodes.

## 2. System Map

* **Entry Point**: `lib.rs` -> exports `ScenarioEngine`, `ScenarioSpec`, `ExecutionContext`.
* **Specification** (`spec.rs`):
  * `ScenarioSpec`: Container for ID, metadata, priority, and operations.
  * `OperationSpec`: Enum of all possible shocks (Curves, Vol, FX, Equity, Statements).
* **Engine** (`engine.rs`):
  * `ScenarioEngine`: Stateless orchestrator.
    * `compose()`: Merges scenarios by priority.
    * `apply()`: Executes operations in phases (Roll -> Market -> Bindings -> Statements -> Re-eval).
* **Adapters** (`src/adapters/`):
  * `traits.rs`: `ScenarioAdapter` trait definition.
  * `curves.rs`: Discount, Forward, Hazard, Inflation curve logic.
  * `vol.rs`: Volatility surface logic + Arbitrage checks.
  * `time_roll.rs`: Time advancement + Carry/Theta calculation.
  * `instruments.rs`: Instrument price/spread overrides.
  * `basecorr.rs`, `equity.rs`, `fx.rs`, `statements.rs`, `asset_corr.rs`: Domain specifics.
* **Utilities** (`utils.rs`):
  * Tenor parsing (Simple vs Context-aware).
  * Interpolation weights.

## 3. Findings by Component

### A. Math / Algorithms: Hazard Curve Bumping

* **Severity:** 🟠 Major
* **Area:** Math
* **Location:** `src/adapters/curves.rs` • `CurveAdapter::try_generate_effects` • `L202`
* **Problem:** The code uses an approximation `lambda_bump = (*bp / 10_000.0) / div` where `div = 1 - recovery`. This assumes `Spread ≈ Lambda * (1 - R)`, which is only true for small spreads and constant hazard rates. It introduces significant error for distressed credits (high spread) or when `R` is close to 1.
* **Why it matters:** In stress testing, credit spreads often blow out. An inaccurate bump magnitude means the stress test doesn't reflect the intended economic shock.
* **Recommendation:** Use the existing hazard curve pricing logic (or `finstack_math` root finder) to solve for the new hazard rate that exactly matches the bumped par spread.
* **Test/Benchmark:** Compare `approx_bump` vs `exact_bump` for Spread=500bp, 1000bp, 2000bp.
* **Acceptance:** Error < 1bp in resulting spread.

### B. Conventions: Inflation Shocks

* **Severity:** 🔵 Minor
* **Area:** Conventions
* **Location:** `src/adapters/curves.rs` • `CurveAdapter::try_generate_effects` • `L275`
* **Problem:** Inflation shocks simply multiply CPI levels by `1 + bp/10000`. This treats the shock as an instantaneous price level jump rather than a change in inflation expectations (forward CPI slope).
* **Why it matters:** Traders typically think in terms of "Inflation Breakevens" (additive bumps to the inflation rate), not multiplicative jumps in the index.
* **Recommendation:** Implement `CurveParallelBp` for Inflation as a bump to the *forward inflation rate*, then reintegrate to get CPI levels.
* **Test/Benchmark:** Apply +50bp shock. Verify 1Y inflation rate increases by ~0.50%.
* **Acceptance:** Accurate repricing of inflation swaps.

### C. Numerical Stability: ParCDS Division Protection

* **Severity:** 🟢 Info (Positive)
* **Area:** Numerical Stability
* **Location:** `src/adapters/curves.rs` • `L189`
* **Problem:** (Observation) The code correctly guards against division by zero: `if (1.0 - recovery).abs() < 1e-4`.
* **Why it matters:** Prevents panics or Infs when Recovery=100% (guaranteed).
* **Recommendation:** Keep as is. Adding a warning log when clamped would be nice.

### D. Safety: Unwrap Usage

* **Severity:** 🟢 Info (Positive)
* **Area:** Safety
* **Location:** `src/lib.rs` • `L3`
* **Problem:** (Observation) `#![deny(clippy::unwrap_used)]` is enabled at crate level.
* **Why it matters:** Ensures library doesn't crash the host application on unexpected data.
* **Recommendation:** Maintain this discipline.

### E. Conventions: Tenor Parsing

* **Severity:** 🔵 Minor
* **Area:** Conventions
* **Location:** `src/utils.rs` • `parse_tenor_to_years` • `L24`
* **Problem:** Simple approximation treats 1Y as 1.0, 1M as 1/12.
* **Why it matters:** Inaccurate time grids can lead to interpolation errors or mismatch with market conventions (ACT/360 vs 30/360).
* **Recommendation:** Default to `parse_tenor_to_years_with_context` using the curve's own daycount and calendar where possible.
* **Test/Benchmark:** Compare "1M" simple (0.0833) vs Actual (varies).
* **Acceptance:** Tenors align with curve pillars.

## 4. Scorecard

| Category | Score (0-5) | Justification |
| :--- | :---: | :--- |
| **Conventions** | 3 | Generally good, but Inflation and some simple tenor parsing need polishing. |
| **Math** | 3 | Hazard bumping approximation is a weak point; otherwise adequate linear approaches. |
| **Algorithms** | 4 | Efficient, priority-based composition, logical phase separation. |
| **Numerical** | 4 | Explicit arb checks for Vols; protected division; deterministic. |
| **Performance** | 4 | Stack allocation for adapters; minimal copying; `IndexMap` for lookups. |
| **Safety** | 5 | Strong error typing, strict linting, no panics. |
| **API/Design** | 5 | Excellent separation of Spec/Engine; robust serialization support. |
| **Docs/Tests** | 4 | Clear doc comments; good unit coverage in adapters. |
