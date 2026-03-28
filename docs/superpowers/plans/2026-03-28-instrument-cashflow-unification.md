# Instrument Cashflow Unification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the cashflow architecture so every instrument emits a mandatory `CashFlowSchedule`, portfolio waterfalls consume one canonical schedule path, options return empty placeholder schedules, and the old optional bridge is fully removed.

**Architecture:** Make `CashflowProvider` the universal dated-cashflow contract and require all `Instrument` implementations to satisfy it. Rename the schedule APIs to semantic names, add explicit schedule metadata for empty-schedule meaning, collapse the portfolio layer onto one canonical waterfall builder, and migrate all instruments by family to one of four representations: `Contractual`, `Projected`, `Placeholder`, or `NoResidual`.

**Tech Stack:** Rust workspace (`finstack-cashflows`, `finstack-valuations`, `finstack-portfolio`, `finstack-scenarios`), PyO3 bindings, wasm-bindgen bindings, cargo-nextest, clippy, rustfmt

**Spec:** `docs/superpowers/specs/2026-03-28-instrument-cashflow-unification-design.md`

**Primary verification commands:**
- `make fmt`
- `make lint`
- `cargo nextest run -p finstack-cashflows --lib --no-fail-fast`
- `cargo nextest run -p finstack-portfolio --lib --no-fail-fast`
- `cargo nextest run -p finstack-scenarios --lib --no-fail-fast`
- `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --tests --no-fail-fast`
- `cargo check -p finstack-py`
- `cargo check -p finstack-wasm`

---

## File Structure

### Core trait and schedule metadata

| File | Responsibility |
|------|----------------|
| `finstack/cashflows/src/cashflow/traits.rs` | Rename the canonical trait surface to `cashflow_schedule()` / `dated_cashflows()` and remove `build_*` naming |
| `finstack/cashflows/src/cashflow/builder/schedule.rs` | Add `CashflowRepresentation` to `CashFlowMeta` and preserve it through constructors/serde |
| `finstack/valuations/src/instruments/common/traits.rs` | Make `Instrument` extend `CashflowProvider`; delete `as_cashflow_provider()` |
| `finstack/valuations/src/instruments/public_traits.rs` | Expose the cashflow methods on the public lean `Instrument` trait |
| `finstack/valuations/src/instruments/mod.rs` | Update instrument module docs to explain the new universal schedule contract |

### Canonical portfolio waterfall

| File | Responsibility |
|------|----------------|
| `finstack/portfolio/src/cashflows.rs` | Replace dual-path aggregation with one canonical waterfall builder and derived views |
| `finstack/portfolio/src/lib.rs` | Re-export any renamed portfolio cashflow types/functions |
| `finstack/portfolio/src/prelude.rs` | Keep prelude exports aligned with the new canonical pathway |

### Generic schedule consumers

| File | Responsibility |
|------|----------------|
| `finstack/scenarios/src/adapters/time_roll.rs` | Remove bridge lookup and use the universal schedule API directly |
| `finstack/valuations/src/metrics/sensitivities/theta.rs` | Replace optional-provider branching with direct instrument cashflow access |
| `finstack/valuations/src/instruments/common/period_pv.rs` | Rename helper usage to the new schedule API |
| `finstack/valuations/src/instruments/common/helpers.rs` | Rename shared schedule helper calls and preserve pricing semantics |
| `finstack/statements/src/capital_structure/integration.rs` | Update any direct schedule calls if required by the rename |
| `finstack/statements/src/evaluator/capital_structure_runtime.rs` | Update any direct schedule calls if required by the rename |

### Existing schedule-producing instruments

| File | Responsibility |
|------|----------------|
| `finstack/valuations/src/instruments/rates/xccy_swap/types.rs` | Rename and tag schedule representation |
| `finstack/valuations/src/instruments/rates/repo/types.rs` | Rename and preserve holder-view future-only semantics |
| `finstack/valuations/src/instruments/rates/inflation_swap/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/rates/fra/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/rates/deposit/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/rates/cms_swap/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/rates/irs/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/rates/basis_swap/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fx/ndf/types.rs` | Keep projected settlement schedule but move to the new naming/metadata |
| `finstack/valuations/src/instruments/fx/fx_spot/types.rs` | Classify as `Contractual` when settlement remains future-dated, otherwise `NoResidual` |
| `finstack/valuations/src/instruments/fx/fx_forward/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fx/fx_swap/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/commodity/commodity_forward/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/commodity/commodity_swap/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/bond/cashflows.rs` | Rename and preserve holder-view bond filtering |
| `finstack/valuations/src/instruments/fixed_income/bond/types.rs` | Remove bridge method and align helper calls/docs |
| `finstack/valuations/src/instruments/fixed_income/inflation_linked_bond/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/tba/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/mbs_passthrough/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/cmo/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/dollar_roll/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/convertible/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/bond_future/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/term_loan/types.rs` | Rename, preserve holder-view logic, tag representation |
| `finstack/valuations/src/instruments/fixed_income/revolving_credit/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/fi_trs/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/fixed_income/structured_credit/types/mod.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/equity/equity_trs/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/equity/pe_fund/types.rs` | Rename and tag representation |
| `finstack/valuations/src/instruments/credit_derivatives/cds/types.rs` | Keep projected premium/upfront policy and tag representation |
| `finstack/valuations/src/instruments/credit_derivatives/cds_index/types.rs` | Keep projected premium/upfront policy and tag representation |
| `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/types.rs` | Keep projected premium/upfront policy and tag representation |

### Empty `NoResidual` and `Placeholder` schedule types

| File | Responsibility |
|------|----------------|
| `finstack/valuations/src/instruments/equity/spot/types.rs` | Return empty `NoResidual` schedule |
| `finstack/valuations/src/instruments/rates/ir_future/types.rs` | Return empty `NoResidual` schedule |
| `finstack/valuations/src/instruments/equity/equity_index_future/types.rs` | Return empty `NoResidual` schedule |
| `finstack/valuations/src/instruments/equity/vol_index_future/types.rs` | Return empty `NoResidual` schedule |
| `finstack/valuations/src/instruments/equity/variance_swap/types.rs` | Replace zero placeholder flow with empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/fx_variance_swap/types.rs` | Replace zero placeholder flow with empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/ir_future_option/types.rs` | Return empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/equity/vol_index_option/types.rs` | Return empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/swaption/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/cms_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/cap_floor/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/inflation_cap_floor/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/rates/range_accrual/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/credit_derivatives/cds_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/equity/equity_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/equity/cliquet_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/equity/autocallable/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/quanto_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/fx_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/fx_digital_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/fx_barrier_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/fx/fx_touch_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/exotics/barrier_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/exotics/asian_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/exotics/lookback_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/exotics/basket/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/commodity/commodity_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/commodity/commodity_swaption/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/commodity/commodity_spread_option/types.rs` | Implement empty `Placeholder` schedule |
| `finstack/valuations/src/instruments/commodity/commodity_asian_option/types.rs` | Implement empty `Placeholder` schedule |

### Model-driven projected cashflow types

| File | Responsibility |
|------|----------------|
| `finstack/valuations/src/instruments/equity/dcf_equity/types.rs` | Emit projected explicit DCF flows via the universal schedule API |
| `finstack/valuations/src/instruments/equity/real_estate/types.rs` | Emit projected unlevered flows via the universal schedule API |
| `finstack/valuations/src/instruments/equity/real_estate/pricer.rs` | Reuse existing flow helpers to back the schedule implementation |
| `finstack/valuations/src/instruments/equity/real_estate/levered.rs` | Emit projected equity cashflows via the universal schedule API |
| `finstack/valuations/src/instruments/equity/real_estate/levered_pricer.rs` | Reuse levered flow helpers to back the schedule implementation |

### Bindings and coverage

| File | Responsibility |
|------|----------------|
| `finstack-py/src/valuations/instruments/fixed_income/revolving_credit.rs` | Rename exposed methods to the new semantic schedule names |
| `finstack-wasm/src/valuations/instruments/trs.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/repo.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/irs.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/inflation_linked_bond.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/fra.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/deposit.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/term_loan.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/structured_credit/mod.rs` | Rename exposed method calls |
| `finstack-wasm/src/valuations/instruments/revolving_credit.rs` | Rename exposed method calls |
| `finstack/valuations/tests/cashflows/instrument_bridge.rs` | Rewrite bridge coverage into universal schedule-surface coverage |
| `finstack/valuations/tests/cashflows/provider_contract.rs` | Update the contract to `cashflow_schedule()` / `dated_cashflows()` semantics |
| `finstack/portfolio/src/cashflows.rs` | Extend tests for empty schedule summaries and waterfall derivation |

---

## Task 1: Refactor the Core Cashflow Traits and Metadata

**Files:**
- Modify: `finstack/cashflows/src/cashflow/traits.rs`
- Modify: `finstack/cashflows/src/cashflow/builder/schedule.rs`
- Modify: `finstack/valuations/src/instruments/common/traits.rs`
- Modify: `finstack/valuations/src/instruments/public_traits.rs`
- Modify: `finstack/valuations/src/instruments/mod.rs`
- Test: `finstack/valuations/tests/cashflows/provider_contract.rs`

- [ ] Add failing tests that assert:
  - empty schedules preserve a non-default `CashflowRepresentation`
  - `dated_cashflows()` is derived from `cashflow_schedule()`
  - the public lean `Instrument` trait exposes the renamed cashflow methods

- [ ] Run targeted tests to confirm the new expectations fail before the refactor.
  - Run: `cargo nextest run -p finstack-cashflows --lib --no-fail-fast`
  - Run: `cargo nextest run -p finstack-valuations --test provider_contract --no-fail-fast`

- [ ] In `finstack/cashflows/src/cashflow/traits.rs`, rename:
  - `build_full_schedule()` -> `cashflow_schedule()`
  - `build_dated_flows()` -> `dated_cashflows()`
  - helper docs to describe the semantic API rather than the construction mechanics

- [ ] In `finstack/cashflows/src/cashflow/builder/schedule.rs`, add `CashflowRepresentation` to `CashFlowMeta`, thread it through constructors, and preserve it in serde/default behavior.

- [ ] In `finstack/valuations/src/instruments/common/traits.rs`, make the internal `Instrument` trait extend `CashflowProvider` and delete `as_cashflow_provider()`.

- [ ] In `finstack/valuations/src/instruments/public_traits.rs`, expose `cashflow_schedule()` and `dated_cashflows()` on the public lean trait.

- [ ] Update instrument-level docs in `finstack/valuations/src/instruments/mod.rs` so the new universal schedule contract is clearly documented.

- [ ] Run `cargo check --workspace --features mc,test-utils` to surface all compile fallout immediately.

---

## Task 2: Collapse the Portfolio Layer onto One Canonical Waterfall Path

**Files:**
- Modify: `finstack/portfolio/src/cashflows.rs`
- Modify: `finstack/portfolio/src/lib.rs`
- Modify: `finstack/portfolio/src/prelude.rs`
- Test: `finstack/portfolio/src/cashflows.rs`

- [ ] Add failing tests that assert:
  - empty placeholder instruments still appear in portfolio-level position summaries
  - `Unsupported` no longer exists as a cashflow issue category
  - all collapsed/bucketed views derive from the same canonical event set

- [ ] In `finstack/portfolio/src/cashflows.rs`, introduce the canonical waterfall object and add per-position summaries that preserve `CashflowRepresentation` even when a schedule is empty.

- [ ] Remove bridge lookup and call `instrument.cashflow_schedule(market, as_of)` directly.

- [ ] Delete or demote the parallel `aggregate_cashflows()` extraction logic so it becomes a pure projection of the canonical waterfall.

- [ ] Remove `CashflowExtractionIssueKind::Unsupported`; retain only build-failure reporting.

- [ ] Update `lib.rs` and `prelude.rs` re-exports so downstream crates have one obvious import path.

- [ ] Run: `cargo nextest run -p finstack-portfolio --lib --no-fail-fast`

---

## Task 3: Update Generic Consumers to the Mandatory Schedule API

**Files:**
- Modify: `finstack/scenarios/src/adapters/time_roll.rs`
- Modify: `finstack/valuations/src/metrics/sensitivities/theta.rs`
- Modify: `finstack/valuations/src/instruments/common/period_pv.rs`
- Modify: `finstack/valuations/src/instruments/common/helpers.rs`
- Modify: `finstack/statements/src/capital_structure/integration.rs`
- Modify: `finstack/statements/src/evaluator/capital_structure_runtime.rs`

- [ ] Replace all `as_cashflow_provider()` branches with direct `cashflow_schedule()` / `dated_cashflows()` calls.

- [ ] Keep the existing holder-view and future-only semantics in theta, time-roll, and periodized PV calculations.

- [ ] Where a generic consumer only needs flattened dated amounts, make it call `dated_cashflows()` rather than re-implementing schedule flattening.

- [ ] Re-run the most relevant targeted suites:
  - `cargo nextest run -p finstack-scenarios --lib --no-fail-fast`
  - `cargo nextest run -p finstack-valuations --lib -E 'test(theta)'`

---

## Task 4: Migrate Existing Rates, FX, and Commodity Schedule Producers

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/xccy_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/repo/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/inflation_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/fra/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/deposit/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/cms_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/irs/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/basis_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/ndf/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_spot/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_forward/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_forward/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_swap/types.rs`

- [ ] Rename each implementation to `cashflow_schedule()` / `dated_cashflows()`.

- [ ] Set representation metadata explicitly:
  - `Contractual` for deterministic scheduled legs
  - `Projected` for current-market projected settlement flows such as `Ndf`
  - `NoResidual` only when there are truly no future-dated flows left

- [ ] Preserve existing product-specific filtering rules:
  - repo holder-view exclusion of already-settled legs
  - FX spot future-settlement handling
  - floating-leg schedule projection behavior

- [ ] Run relevant valuation tests after each family lands:
  - `cargo nextest run -p finstack-valuations --features mc,test-utils --test '*' --no-fail-fast`

---

## Task 5: Migrate Fixed Income, Credit, TRS, PE Fund, and Structured Credit Producers

**Files:**
- Modify: `finstack/valuations/src/instruments/fixed_income/bond/cashflows.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/bond/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/inflation_linked_bond/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/tba/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/mbs_passthrough/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/cmo/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/dollar_roll/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/convertible/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/bond_future/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/term_loan/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/revolving_credit/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/fi_trs/types.rs`
- Modify: `finstack/valuations/src/instruments/fixed_income/structured_credit/types/mod.rs`
- Modify: `finstack/valuations/src/instruments/equity/equity_trs/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/pe_fund/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_index/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_tranche/types.rs`

- [ ] Rename all schedule methods and remove bridge overrides from the `Instrument` impls.

- [ ] Preserve existing holder-view logic for bonds, loans, and facilities while moving the representation tag into `CashFlowMeta`.

- [ ] Keep CDS-family waterfall policy intentionally narrow for now:
  - premium/upfront style cashflows remain exposed
  - contingent protection/default legs are still not projected into dated waterfall events in this refactor
  - representation should be `Projected`

- [ ] Ensure structured credit and TRS products continue emitting non-empty schedules and set `Projected` where flows are model-driven rather than fixed.

- [ ] Run the strongest existing regression suites for these families:
  - `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --tests --no-fail-fast`

---

## Task 6: Normalize Existing Empty and Zero-Placeholder Types

**Files:**
- Modify: `finstack/valuations/src/instruments/equity/spot/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/ir_future/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/equity_index_future/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/vol_index_future/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/variance_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_variance_swap/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/ir_future_option/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/vol_index_option/types.rs`

- [ ] Replace all remaining zero-amount synthetic placeholder events with genuinely empty schedules.

- [ ] Tag empty schedules as:
  - `NoResidual` for spot/futures products with no residual dated payments
  - `Placeholder` for contingent payoff products with no accepted dated-cashflow policy yet

- [ ] Add tests that assert the schedule is empty and that the representation is correct.

- [ ] Run targeted packages:
  - `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --tests --no-fail-fast`

---

## Task 7: Add Placeholder Schedules to Currently Unsupported Contingent Products

**Files:**
- Modify: `finstack/valuations/src/instruments/rates/swaption/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/cms_option/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/cap_floor/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/inflation_cap_floor/types.rs`
- Modify: `finstack/valuations/src/instruments/rates/range_accrual/types.rs`
- Modify: `finstack/valuations/src/instruments/credit_derivatives/cds_option/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/equity_option/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/cliquet_option/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/autocallable/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/quanto_option/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_option/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_digital_option/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_barrier_option/types.rs`
- Modify: `finstack/valuations/src/instruments/fx/fx_touch_option/types.rs`
- Modify: `finstack/valuations/src/instruments/exotics/barrier_option/types.rs`
- Modify: `finstack/valuations/src/instruments/exotics/asian_option/types.rs`
- Modify: `finstack/valuations/src/instruments/exotics/lookback_option/types.rs`
- Modify: `finstack/valuations/src/instruments/exotics/basket/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_option/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_swaption/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_spread_option/types.rs`
- Modify: `finstack/valuations/src/instruments/commodity/commodity_asian_option/types.rs`

- [ ] For each type, implement `CashflowProvider` directly.

- [ ] Return an empty schedule with:
  - the correct reporting currency/notional hint
  - `CashflowRepresentation::Placeholder`
  - no synthetic maturity cashflow

- [ ] Delete any remaining “unsupported in waterfalls” assumptions from tests or docs.

- [ ] Extend integration coverage so every formerly unsupported product now succeeds when asked for a schedule.

- [ ] Run: `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --tests --no-fail-fast`

---

## Task 8: Add Projected Schedule Support for DCF and Real Estate Products

**Files:**
- Modify: `finstack/valuations/src/instruments/equity/dcf_equity/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/real_estate/types.rs`
- Modify: `finstack/valuations/src/instruments/equity/real_estate/pricer.rs`
- Modify: `finstack/valuations/src/instruments/equity/real_estate/levered.rs`
- Modify: `finstack/valuations/src/instruments/equity/real_estate/levered_pricer.rs`

- [ ] Add failing tests proving these products emit non-empty projected schedules with the right dates and sign conventions.

- [ ] Implement `cashflow_schedule()` using the existing explicit flow helpers:
  - DCF -> explicit free cash flow schedule
  - Real estate -> unlevered future flows
  - Levered real estate -> projected equity cashflows

- [ ] Tag these schedules as `Projected`.

- [ ] Keep pricing logic unchanged; only expose the existing projected dated-flow surface through the unified schedule API.

- [ ] Re-run targeted suites for these products and then the full valuation test package.

---

## Task 9: Update Bindings, Rewrite Coverage Tests, and Finish Verification

**Files:**
- Modify: `finstack-py/src/valuations/instruments/fixed_income/revolving_credit.rs`
- Modify: `finstack-wasm/src/valuations/instruments/trs.rs`
- Modify: `finstack-wasm/src/valuations/instruments/repo.rs`
- Modify: `finstack-wasm/src/valuations/instruments/irs.rs`
- Modify: `finstack-wasm/src/valuations/instruments/inflation_linked_bond.rs`
- Modify: `finstack-wasm/src/valuations/instruments/fra.rs`
- Modify: `finstack-wasm/src/valuations/instruments/deposit.rs`
- Modify: `finstack-wasm/src/valuations/instruments/term_loan.rs`
- Modify: `finstack-wasm/src/valuations/instruments/structured_credit/mod.rs`
- Modify: `finstack-wasm/src/valuations/instruments/revolving_credit.rs`
- Modify: `finstack/valuations/tests/cashflows/instrument_bridge.rs`
- Modify: `finstack/valuations/tests/cashflows/provider_contract.rs`
- Modify: `finstack/valuations/tests/cashflows/mod.rs` if test names/modules change

- [ ] Rename any bound methods still exposing `build_full_schedule()` / `build_dated_flows()` to the new semantic names.

- [ ] Rewrite `instrument_bridge.rs` so it no longer tests bridge existence; instead test that representative instruments:
  - all produce a schedule successfully
  - emit the expected `CashflowRepresentation`
  - produce empty schedules only where policy says they should

- [ ] Update `provider_contract.rs` so the canonical contract becomes:
  - `cashflow_schedule()` succeeds
  - representation is set correctly
  - `dated_cashflows()` matches the schedule flattening
  - flows are sorted and currency-consistent

- [ ] Run the full verification sequence:
  - `make fmt`
  - `make lint`
  - `cargo nextest run -p finstack-cashflows --lib --no-fail-fast`
  - `cargo nextest run -p finstack-portfolio --lib --no-fail-fast`
  - `cargo nextest run -p finstack-scenarios --lib --no-fail-fast`
  - `cargo nextest run -p finstack-valuations --features mc,test-utils --lib --tests --no-fail-fast`
  - `cargo check -p finstack-py`
  - `cargo check -p finstack-wasm`

- [ ] Do a final grep-driven cleanup pass to confirm:
  - no `as_cashflow_provider(` calls remain
  - no `build_full_schedule(` calls remain in public-facing instrument APIs
  - no zero-amount synthetic placeholder flows remain

---

## Execution Notes

- Treat this as a breaking-change refactor. Do not preserve the bridge behind compatibility layers.
- Prefer rewriting tests to match the new semantics rather than adding shims that keep the old bridge alive.
- Keep `value()` and other pricer entry points untouched unless a rename fallout genuinely requires a local fix.
- If any contingent product resists classification, default to an empty `Placeholder` schedule rather than inventing a projected payoff policy during this refactor.

