# Cashflow Crate & Bindings — Documentation Review

**Date:** 2026-04-25
**Scope:** `finstack/cashflows/` (Rust crate), `finstack-py/src/bindings/cashflows.rs` + `.pyi` + `__init__.py` + `tests/test_cashflows.py` (Python binding), `finstack-wasm/src/api/cashflows.rs` + `exports/cashflows.js` + `index.d.ts` slice + `tests/wasm_cashflows.rs` (WASM binding).
**Method:** Manual file-by-file inspection of every source file, plus `cargo doc -p finstack-cashflows --no-deps -D warnings` and `cargo test -p finstack-cashflows --doc` to verify lint and doctest health.
**Reference:** Builds on findings in [DOCUMENTATION_REVIEW_2026-04-22.md](DOCUMENTATION_REVIEW_2026-04-22.md), which covered all workspace crates at a coarser granularity.

---

## TL;DR

The crate is in **good shape overall**. Doc-coverage is significantly improved over the 2026-04-22 baseline:
- All 6 previously flagged blocker items in `cashflows` (`AmortizationSpec`, `CouponType`, `FeeAccrualBasis`, `AccrualMethod`, plus two prepayment/default factory methods) now have docs.
- Every `ScheduleParams::*` market-convention helper now has full Arguments/Returns/Examples/References — addressing the largest single Major bucket from the prior review.
- `cargo doc -p finstack-cashflows --no-deps -D warnings` is **clean**.
- `cargo test -p finstack-cashflows --doc` passes **72 doctests, 0 failed**.

The remaining issues are mostly:
1. One **broken example in the crate README** (`FloatingRateSpec` literal missing the required `overnight_basis` field — would not compile).
2. **Stale field list** in two `dated_flows` binding docstrings (Python and WASM) that claim to return `currency` and `kind` separately, but the actual JSON shape is `{date, amount: {amount, currency}}` — no `kind`.
3. A handful of `pub(crate)` / private helpers in `dataframe.rs`, `compiler.rs`, and `rate_helpers.rs` with thin one-liner docs.
4. Module re-exports in `emission/mod.rs` carry no per-item docstring (low impact since the items are `pub(crate)` and not part of the published API).
5. README does not mention several new public surface items: `RecoveryTiming`, `ScheduleParams::jpy_tona_swap`, `CashFlowSchedule::weighted_average_life`, `CashFlowSchedule::coupons()`, `merge_cashflow_schedules`, `normalize_public`, `outstanding_path_per_flow`.

No item is unsafe, no doc actively misdescribes financial semantics, and the Rust → JSON → Python/WASM contract documentation is internally consistent except where flagged below.

---

## 1. Audit Scope and Coverage Numbers

| Surface | Files | Pub items | Docs running clean? |
|---|---|---|---|
| Cashflow Rust crate | 36 `.rs` files in [finstack/cashflows/src/](../finstack/cashflows/src/) | ~125 pub items | Yes — `RUSTDOCFLAGS='-D warnings' cargo doc` is clean |
| Doctests | 72 examples across the crate | n/a | Yes — `cargo test -p finstack-cashflows --doc` → 72 pass, 0 fail, 1 ignored |
| Python binding | 4 files (`cashflows.rs`, `__init__.py`, `__init__.pyi`, `test_cashflows.py`) | 5 pyfunctions | Yes — but two have stale `{date, amount, currency, kind}` docstrings |
| WASM binding | 4 files (`cashflows.rs`, `cashflows.js`, `index.d.ts` slice, `wasm_cashflows.rs`) | 5 wasm-bindgen fns | Yes — same docstring drift; TypeScript declarations have **no JSDoc at all** |
| Schemas | 7 JSON schemas in [finstack/valuations/schemas/cashflow/1/](../finstack/valuations/schemas/cashflow/1/) | derived via `schemars` | Descriptions flow through from Rust doc comments — improvements there propagate automatically |

The 2026-04-22 review reported `cashflows` had **6 zero-doc items + 115 incomplete-doc items + 28 affected files**. The diff against today:

| Class | 2026-04-22 count | 2026-04-25 status |
|---|---|---|
| Zero-doc blockers in cashflows | 6 | **0** — all addressed |
| `ScheduleParams::*` helpers missing Arguments/Returns/Examples | 9 (`quarterly_act360` … `gbp_sonia_swap`) | **0** — all now fully documented with `# References` to ISDA/ICMA/ARRC/ECB |
| Prepayment / default helper methods missing Examples | 9 (`smm`, `constant_cpr`, `psa`, `cmbs_with_lockout`, `mdr`, `constant_cdr`, `sda`, `cdr_2pct`, plus structs) | **0** — every constructor now has Arguments/Returns/Examples/References |

So the baseline has been substantially closed inside this crate. What remains is a long tail of stylistic/consistency issues, which are catalogued below.

---

## 2. Findings

Severity legend (matching project convention):
- **Blocker** — undocumented public API, broken example, or a docstring that misdescribes runtime behavior.
- **Major** — public API missing one of the required `# Arguments / # Returns / # Errors / # Examples` sections, or doc-style violation likely to confuse callers.
- **Minor** — internal `pub(crate)` items with thin docs, missing references on financial code, or stylistic gaps.

### 2.1 Crate root and module-level docs

#### [finstack/cashflows/src/lib.rs](../finstack/cashflows/src/lib.rs)

- **OK:** Module-level `//!` (lines 20–156) is comprehensive: covers core components, currency safety, conventions, and three full doctests (build, aggregate, periodized PV). All three compile.
- **OK:** `pub mod json;` at line 174 has no inline `///` comment, but the file `json.rs` carries a `//!` module doc, so `missing_docs` is satisfied. (Verified by clean `cargo doc -D warnings`.)
- **Minor:** The `pub use` re-exports at lines 182–194 inherit docs from the originals, so they too pass the lint. They do, however, surface in the rustdoc index without inline documentation, which means a reader on the crate's index page sees only the path. Not actionable; just noting that the rustdoc UX is "go follow the links."

#### [finstack/cashflows/src/accrual.rs](../finstack/cashflows/src/accrual.rs)

- **OK:** Module `//!` is rich; it lists supported coupon types, ex-coupon convention, and a callout that the engine is schedule-only (no instrument coupling).
- **OK:** `MAX_REASONABLE_ACCRUAL_FACTOR` constant has a thorough explanation of its 1.5 cap.
- **OK:** `AccrualMethod::Compounded` doc explicitly notes that ICMA Rule 251.1 prescribes *linear* accrual and that the compounded variant should not be cited as ICMA-style. This is exactly the kind of "non-obvious convention warning" the doc standard asks for.
- **OK:** `AccrualConfig::strict_issue_date` field doc (line 178) is detailed and includes a fallback warning. The `frequency` field doc (line 173) explicitly warns about the ACT/ACT ISMA fallback semantics. Good.
- **OK:** `accrued_interest_amount` (line 251) has Arguments/Returns/Errors/Examples.
- **Minor:** `advance_business_days` (private fn at line 52) has a long docstring but is private; could move the `# Performance` block into a regular `//` comment to keep noise out of dev docs.

#### [finstack/cashflows/src/aggregation.rs](../finstack/cashflows/src/aggregation.rs)

- **OK:** Module `//!` (lines 1–13) explicitly documents the per-flow rounding policy and the `Money::new` ingestion behavior. Reconciliation note about sum-then-round is present.
- **OK:** `aggregate_by_period` (line 169) has full sections including a `# Performance` block.
- **OK:** `aggregate_cashflows_checked` (line 229) has Arguments/Returns/Errors/Examples.
- **OK:** `RecoveryTiming` enum (line 465) has a clear description of `AtPaymentDate` vs `AtDefaultIntegrated`, including the ISDA "default at midpoint" closed-form formula.
- **OK:** `DateContext::new` (line 398) has full sections.
- **Minor:** `aggregate_cashflows_precise_checked` (line 248) — deprecated alias has a one-line doc, which is fine for a deprecation pointer.
- **Minor:** `pub(crate) fn pv_by_period_credit_adjusted_detailed` (line 579) and its `_with_timing` sibling (line 607) have full docstrings even though they're `pub(crate)`. That's actually great because they're the engine behind `CashFlowSchedule::pv_by_period`. Note: their `# Examples` section is `rust,ignore` because it references a `pub(crate)` symbol — that's the correct doc choice.

#### [finstack/cashflows/src/traits.rs](../finstack/cashflows/src/traits.rs)

- **Minor:** Module `//!` is just `Cashflow-related traits and aliases.` (line 1). For a module that owns the central `CashflowProvider` trait, a couple of sentences explaining where instruments plug in would be worth adding.
- **OK:** `ScheduleBuildOpts` struct (line 20) and all its fields are documented.
- **OK:** `CashflowProvider::notional` has Examples; `cashflow_schedule` and `dated_cashflows` document the future-filtering and PIK-omission contract clearly.
- **Major:** `CashflowProvider::dated_cashflows` (line 132) has no `# Errors` section even though it returns `Result`.
- **Minor:** `schedule_from_classified_flows` (line 218) is one-line documented and lacks Arguments/Returns/Errors/Examples. It is part of the public API (re-exported from the crate root). Should be brought up to the same standard as `schedule_from_dated_flows`.

#### [finstack/cashflows/src/json.rs](../finstack/cashflows/src/json.rs)

- **OK:** Module `//!` and every public function (`build_cashflow_schedule_json`, `validate_cashflow_schedule_json`, `dated_flows_json`, `accrued_interest_json`) have full Arguments/Returns/Errors/Examples.
- **OK:** `CashflowScheduleBuildSpec`, `PrincipalEventSpec`, and `DatedFlowJson` structs have field-level docs.
- **OK:** Doctests for all four functions pass.

#### [finstack/cashflows/src/serde_defaults.rs](../finstack/cashflows/src/serde_defaults.rs)

- **OK:** Module `//!` is one line; both helper fns are `pub(crate)` and one-line documented. Acceptable for trivially small file.

### 2.2 Builder root

#### [finstack/cashflows/src/builder/mod.rs](../finstack/cashflows/src/builder/mod.rs)

- **OK:** Module `//!` is well organized: lists Primary Types / Coupon Specifications / Amortization & Fees / Schedule Parameters / Credit Models, then a working doctest.
- **OK:** Clear `#[doc(hidden)]` boundary at line 102 on the `emit_*` re-exports — these are explicitly removed from the published surface.

#### [finstack/cashflows/src/builder/schedule.rs](../finstack/cashflows/src/builder/schedule.rs)

- **OK:** `kind_rank` (line 27) — has thorough variant ranking description.
- **Major:** `sort_flows` (line 49) is a one-line `Sort flows deterministically using schedule ordering semantics.` Should mention what the multi-key ordering is (date → kind → currency → amount → reset_date), since this is the canonical sort order callers reason about.
- **Major:** `coupons` iterator (line 350) has only a one-line description and no Examples.
- **Major:** `merge_cashflow_schedules` (line 488) has only a one-line description. Given that this is non-trivial (it merges metadata, dedups calendars, picks a representation, etc.) — needs Arguments/Returns/Errors and a one-line note that mismatched representations / facility limits / issue dates collapse to the default.
- **OK:** `normalize_public` (line 278) — has a 4-step description.
- **OK:** `outstanding_path_per_flow` (line 334) and `outstanding_by_date` (line 432) — both fully documented with the "When to Use Each Method" matrix.
- **OK:** `weighted_average_life` (line 381) — exemplary: includes the formula, the explicit Act/365F-only rationale, and SIFMA + Fabozzi references.
- **OK:** `PvCreditAdjustment` (line 615) and `PvDiscountSource` (line 628) variants are documented.
- **OK:** `pv_by_period` (line 696) — full Arguments/Returns/Errors/Examples.
- **Minor:** The three deprecated wrappers (`pv_by_period_with_ctx`, `pv_by_period_with_market_and_ctx`, `pv_by_period_with_survival_and_ctx`) have one-line docs pointing to the replacement. Adequate for `#[deprecated]` items.
- **Minor:** Both `IntoIterator` impls (lines 859–875) have no doc, but they're standard trait impls, so this is conventional.

### 2.3 Builder pipeline

#### [finstack/cashflows/src/builder/builder.rs](../finstack/cashflows/src/builder/builder.rs)

- **OK:** Module `//!` is comprehensive with a quick-start.
- **OK:** `PrincipalEvent` (line 109) and its fields are documented including the cash-vs-delta semantics for OID/fees.
- **OK:** `CashFlowBuilder` (line 565) and `PreparedCashFlow` (line 737) have struct-level docs explaining lifecycle, thread-safety, and caching pattern.

#### [finstack/cashflows/src/builder/builder/build.rs](../finstack/cashflows/src/builder/builder/build.rs)

- **OK:** `build_with_curves`, `prepared`, and `PreparedCashFlow::project` all have full docs including a "caching pattern" example.
- **Minor:** `build()` is `#[deprecated]` with a one-liner — fine.

#### [finstack/cashflows/src/builder/builder/principal.rs](../finstack/cashflows/src/builder/builder/principal.rs)

- **OK:** `principal` and `amortization` both have Arguments/Returns/Examples.
- **Minor:** `principal_events` (line 114) is `#[deprecated]` with brief docs — points to the replacement (`add_principal_event`). Adequate.
- **OK:** `add_principal_event` (line 138) has a clear Errors section.

#### [finstack/cashflows/src/builder/builder/coupons.rs](../finstack/cashflows/src/builder/builder/coupons.rs)

- **OK:** `fixed_cf` and `floating_cf` both fully documented with realistic examples that compile.
- **OK:** The `floating_cf` example correctly includes `overnight_basis: None`. (Contrast with the README — see §2.7.)

#### [finstack/cashflows/src/builder/builder/fees.rs](../finstack/cashflows/src/builder/builder/fees.rs), [builder/splits.rs](../finstack/cashflows/src/builder/builder/splits.rs), [builder/stepup.rs](../finstack/cashflows/src/builder/builder/stepup.rs)

(See §2.6 sub-agent findings.)

- **OK:** `fee()`, `step_up_cf()`, `fixed_stepup()`, `float_margin_stepup()`, `fixed_to_float()` all fully documented.
- **Minor:** `add_payment_window()` in `splits.rs:8` is `pub` but has only a one-line description. Either document fully or downgrade to `pub(crate)`. (Probably should be `pub(crate)` since the `payment_split_program` helper is the user-facing entry.)

### 2.4 Spec types

#### [finstack/cashflows/src/builder/specs/coupon.rs](../finstack/cashflows/src/builder/specs/coupon.rs)

- **OK:** `CouponType`, `FixedCouponSpec`, `OvernightCompoundingMethod`, `FloatingRateFallback`, `FloatingRateSpec`, `FloatingCouponSpec`, `StepUpCouponSpec` — all fully documented with field-level docs.
- **OK:** `OvernightCompoundingMethod` variants document SOFR / €STR / SONIA / TONA conventions in a market-conventions table.
- **OK:** `FloatingRateSpec` doctest at line 280 includes `overnight_basis: None` — compiles.
- **OK:** `StepUpCouponSpec` doctest at line 485 — compiles.
- **OK:** `FloatingRateSpec::validate()` documents validation rules.

#### [finstack/cashflows/src/builder/specs/amortization.rs](../finstack/cashflows/src/builder/specs/amortization.rs)

- **OK:** All `AmortizationSpec` variants documented (was previously a blocker).
- **OK:** `Notional::par` has Example. `Notional::validate` has Validation Rules + Errors.
- **Minor:** `Notional::currency()` (line 82) is one-line documented. Acceptable for a trivial accessor.

#### [finstack/cashflows/src/builder/specs/fees.rs](../finstack/cashflows/src/builder/specs/fees.rs)

- **OK:** `FeeSpec` variants and fields documented.
- **OK:** `FeeAccrualBasis` (was a blocker) is now documented with `is_default` helper.
- **OK:** `FeeBase`, `FeeTier`, `evaluate_fee_tiers` all documented.
- **Minor:** `FeeTier::from_bps` (line 93) documents `# Panics` clearly. Good.

#### [finstack/cashflows/src/builder/specs/schedule.rs](../finstack/cashflows/src/builder/specs/schedule.rs)

- **OK:** `ScheduleParams` struct + all field-level docs.
- **OK:** All 10 convention helpers (`quarterly_act360`, `semiannual_30360`, `annual_actact`, `usd_sofr_swap`, `usd_corporate_bond`, `usd_treasury`, `eur_estr_swap`, `eur_gov_bond`, `gbp_sonia_swap`, `jpy_tona_swap`) have Arguments/Returns/Examples/References. **All previously flagged Major items are now closed.**
- **OK:** `FixedWindow` documented.

#### [finstack/cashflows/src/builder/specs/prepayment.rs](../finstack/cashflows/src/builder/specs/prepayment.rs)

- **OK:** `PrepaymentCurve` variants documented.
- **OK:** `PrepaymentModelSpec` and all five constructors (`smm`, `constant_cpr`, `constant_cpr_pct`, `psa`, `psa_100`, `cmbs_with_lockout`) have full Arguments/Returns/Examples/References. Previously flagged Major items closed.

#### [finstack/cashflows/src/builder/specs/default.rs](../finstack/cashflows/src/builder/specs/default.rs)

- **OK:** `DefaultCurve`, `DefaultModelSpec`, `DefaultEvent` all documented.
- **OK:** `mdr`, `constant_cdr`, `sda`, `cdr_2pct` all have Arguments/Returns/Examples/References.

#### [finstack/cashflows/src/builder/specs/recovery.rs](../finstack/cashflows/src/builder/specs/recovery.rs)

- **OK:** `RecoveryModelSpec`, `with_lag`, `with_lag_pct`, `validate` all documented.

### 2.5 Builder helpers (delegated to sub-agent)

A sub-agent reviewed [builder/calendar.rs](../finstack/cashflows/src/builder/calendar.rs), [builder/compiler.rs](../finstack/cashflows/src/builder/compiler.rs), [builder/credit_rates.rs](../finstack/cashflows/src/builder/credit_rates.rs), [builder/dataframe.rs](../finstack/cashflows/src/builder/dataframe.rs), [builder/date_generation.rs](../finstack/cashflows/src/builder/date_generation.rs), [builder/periods.rs](../finstack/cashflows/src/builder/periods.rs), [builder/rate_helpers.rs](../finstack/cashflows/src/builder/rate_helpers.rs), and the `emission/*` files. Findings (filtered for accuracy and re-classified):

| File | Severity | Finding |
|---|---|---|
| [builder/calendar.rs](../finstack/cashflows/src/builder/calendar.rs) | — | Excellent — all public items fully documented with examples. |
| [builder/compiler.rs](../finstack/cashflows/src/builder/compiler.rs) | Minor | The `pub(super)` type aliases `FixedSchedule`, `FloatSchedule`, `PeriodicFees`, `FixedFees` (lines 70–108) have no one-line doc. Internal-only, but a one-liner each would speed onboarding. (The sub-agent's claim that doc comments are "inside function bodies" was a misread of contiguous block comments — the on-disk content is correctly placed.) |
| [builder/credit_rates.rs](../finstack/cashflows/src/builder/credit_rates.rs) | — | `cpr_to_smm` and `smm_to_cpr` have full Arguments/Returns/Errors/Examples. |
| [builder/dataframe.rs](../finstack/cashflows/src/builder/dataframe.rs) | Major | The four private helpers `init_optional_column` (32), `compute_discount_time` (49), `compute_notional_columns` (69), `compute_floating_decomposition` (110) have one-line docs only. They are private (no `pub`/`pub(crate)`), so this is technically Minor — but they implement non-trivial financial logic (signed year-fraction across base date, floating decomposition into base + spread). At least `# References` would help future maintainers. |
| [builder/dataframe.rs](../finstack/cashflows/src/builder/dataframe.rs) | — | `PeriodDataFrame` and `PeriodDataFrameOptions` have exemplary field-level documentation grouped by purpose. `to_period_dataframe` and `to_period_dataframe_into` documented. |
| [builder/date_generation.rs](../finstack/cashflows/src/builder/date_generation.rs) | — | `build_dates`, `SchedulePeriod`, `PeriodSchedule` all documented. The example uses `create_date()` which is a real export from `finstack_core::dates::mod` (`pub fn create_date` at [finstack/core/src/dates/mod.rs:175](../finstack/core/src/dates/mod.rs#L175)) and the doctest passes. (The sub-agent flagged this as a possible compile failure — it's actually fine.) |
| [builder/periods.rs](../finstack/cashflows/src/builder/periods.rs) | — | `BuildPeriodsParams`, `build_single_period`, `build_periods` fully documented. |
| [builder/rate_helpers.rs](../finstack/cashflows/src/builder/rate_helpers.rs) | — | All public functions documented with full Arguments/Returns/References, including the four overnight-rate helpers (`compute_compounded_rate`, `compute_simple_average_rate`, `compute_overnight_rate`, `project_floating_rate`). |
| [builder/rate_helpers.rs](../finstack/cashflows/src/builder/rate_helpers.rs) | Minor | `optional_decimal_to_f64` (line 202) is private with one-line doc. Acceptable. |
| [builder/emission/mod.rs](../finstack/cashflows/src/builder/emission/mod.rs) | Minor | Module `//!` is comprehensive. The re-exports at lines 44–62 inherit docs from the originals; `missing_docs` is satisfied. (The sub-agent flagged this as a Blocker; it isn't — the doc lint passes and the re-exported items are `pub(crate)` so they aren't part of the published surface. It's only a UX nit on the rustdoc index page, hence Minor.) |
| [builder/emission/coupons.rs](../finstack/cashflows/src/builder/emission/coupons.rs) | Major | `emit_fixed_coupons_on` and `emit_float_coupons_on` return `Result<f64>` where the `f64` is "PIK amount to capitalize." The current docstring describes this behaviorally but doesn't put it in a `# Returns` block. Worth adding for symmetry with the rest of the codebase. |
| [builder/emission/amortization.rs](../finstack/cashflows/src/builder/emission/amortization.rs) | — | `AmortizationParams` and `emit_amortization_on` documented. |
| [builder/emission/credit.rs](../finstack/cashflows/src/builder/emission/credit.rs) | — | `emit_default_on`, `emit_prepayment_on` documented. |
| [builder/emission/fees.rs](../finstack/cashflows/src/builder/emission/fees.rs) | Minor | `emit_fee_generic` (private) has detailed docstring but no `# Panics` block for its `debug_assert!` — `# Panics` is the right idiom for `debug_assert!` even on private fns. |
| [builder/emission/helpers.rs](../finstack/cashflows/src/builder/emission/helpers.rs) | — | `add_pik_flow_if_nonzero`, `compute_reset_date` documented. |

### 2.6 Crate README

[finstack/cashflows/README.md](../finstack/cashflows/README.md) is comprehensive: it covers Overview, Import Paths, Main Entry Points, Quick Start (3 examples), Common Workflows (3 more examples), Hidden Integration Helpers callout, `CFKind` guidance, Testing, References, and See Also.

#### Issues

- **Blocker:** Floating-rate example at [README.md:158–182](../finstack/cashflows/README.md#L158-L182) is missing `overnight_basis: None`. The actual struct ([builder/specs/coupon.rs:399](../finstack/cashflows/src/builder/specs/coupon.rs#L399)) requires this field; struct-literal construction without it will not compile. Note that the **doctest** version of this example (in `coupons.rs:113` and `coupon.rs:280`) does include the field — it's the README copy that's stale. The README is not run as a doctest, so CI didn't catch the drift.

- **Minor:** Several recently-added public surface items are not mentioned in the README's "Main Entry Points" or example sections:
  - `RecoveryTiming` enum (re-exported from crate root)
  - `ScheduleParams::jpy_tona_swap` (in the convention helpers list)
  - `CashFlowSchedule::weighted_average_life` (a primary user-facing method)
  - `CashFlowSchedule::coupons()` iterator
  - `CashFlowSchedule::merge_cashflow_schedules` (top-level function)
  - `CashFlowSchedule::normalize_public`
  - `CashFlowSchedule::outstanding_path_per_flow` (the simplified balance view)

- **Minor:** The README mentions `weighted_average_life` and `outstanding_by_date` indirectly but does not include WAL or two-method "When to Use" guidance that's already in the rustdoc on those methods. Worth a short Common Workflows entry.

### 2.7 Bindings

#### Python: [finstack-py/src/bindings/cashflows.rs](../finstack-py/src/bindings/cashflows.rs)

All five `#[pyfunction]`s have docstrings in NumPy/Sphinx format with Parameters / Returns sections. The `register()` function sets a clear module `__doc__`.

- **Blocker:** `dated_flows` docstring (line 47–62) says it returns *"JSON array of `{date, amount, currency, kind}` entries."* The actual JSON shape comes from [DatedFlowJson](../finstack/cashflows/src/json.rs#L60-L66):

  ```rust
  pub struct DatedFlowJson {
      pub date: Date,
      pub amount: Money,
  }
  ```

  Money serializes as `{amount, currency}`, so the actual shape is `{date, amount: {amount, currency}}`. There is no top-level `currency`, and there is no `kind` at all. The Rust-side docstring at [json.rs:254–265](../finstack/cashflows/src/json.rs#L254-L265) correctly describes this ("intentionally omits CFKind and accrual metadata"). The Python binding docstring is the one that's stale.

- **Minor:** `bond_from_cashflows` (line 89) lives in the cashflows binding but depends on `finstack_valuations` for `Bond::from_cashflows`. Worth a one-liner noting that it's a convenience wrapper that crosses crates.

#### Python: [finstack-py/finstack/cashflows/**init**.py](../finstack-py/finstack/cashflows/__init__.py) and [**init**.pyi](../finstack-py/finstack/cashflows/__init__.pyi)

- **Minor:** Both files have a one-line module docstring `"""Cashflow schedule JSON construction and validation."""`. Adequate for a thin re-export façade. The `.pyi` has no per-function docstrings, which is conventional for stub files.
- `__all__` ordering matches between `.py` and `.pyi` (both alphabetical). Good.

#### Python: [finstack-py/tests/test_cashflows.py](../finstack-py/tests/test_cashflows.py)

- **OK:** Has a module docstring and a test that exercises every binding function plus a downstream `price_instrument` call. Good integration coverage.

#### WASM: [finstack-wasm/src/api/cashflows.rs](../finstack-wasm/src/api/cashflows.rs)

All five `#[wasm_bindgen]` functions have JSDoc-style `///` docs with `@param`, `@returns`, `@throws`.

- **Blocker:** Same `dated_flows` docstring drift as the Python binding (line 31–34): claims `{date, amount, currency, kind}`. Same fix applies.

- **Minor:** Same `bond_from_cashflows` cross-crate note applies.

#### WASM: [finstack-wasm/exports/cashflows.js](../finstack-wasm/exports/cashflows.js)

- **OK:** Bare re-export façade; no docs needed.

#### WASM: [finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts) (`CashflowsNamespace` slice at lines 931–946)

- **Major:** **Zero JSDoc** on the `CashflowsNamespace` interface or any of its methods. Compare with the rest of `index.d.ts` — most other namespaces are similarly bare, so this is a workspace-wide pattern, not a cashflows-specific regression. Still, the function signatures have nothing pointing TypeScript users back to the rich `///` docs in `api/cashflows.rs`. Worth adding at minimum `@see` references:

  ```typescript
  export interface CashflowsNamespace {
    /**
     * Build a cashflow schedule from a JSON spec.
     * @see {@link https://docs.finstack.dev/cashflows#buildCashflowSchedule}
     */
    buildCashflowSchedule(specJson: string, marketJson?: string | null): string;
    // ...
  }
  ```

  If no docs URL is published, at minimum copy the @param / @returns from the Rust side.

#### WASM: [finstack-wasm/tests/wasm_cashflows.rs](../finstack-wasm/tests/wasm_cashflows.rs)

- **OK:** Module docstring present; test exercises every binding plus downstream `price_instrument`. Mirrors the Python integration test.

### 2.8 Tests and benches

- [tests/cashflows.rs](../finstack/cashflows/tests/cashflows.rs) — `//!` summarizes the four submodules and the `cargo test` invocation. Good.
- [tests/cashflows/schema_roundtrip.rs](../finstack/cashflows/tests/cashflows/schema_roundtrip.rs) — `//!` describes intent.
- [benches/cashflow_hot_paths.rs](../finstack/cashflows/benches/cashflow_hot_paths.rs) — `//!` enumerates 10 benchmarked hot paths and includes the run command. Excellent.

### 2.9 Schemas

[finstack/valuations/schemas/cashflow/1/](../finstack/valuations/schemas/cashflow/1/) contains 7 JSON schemas auto-generated via `schemars` from the Rust types. All schema descriptions are pulled from the corresponding Rust doc comments, so improvements made on the Rust side propagate automatically. No standalone schema-doc work needed.

---

## 3. Cross-cutting observations

### 3.1 The `dated_flows` docstring contract is broken in three places

Both `dated_flows_json` (Rust public fn) and the bindings agree that the output is `Vec<DatedFlowJson>` where each entry is `{date, amount}` and amount embeds currency. But:
- Python binding [cashflows.rs:47–62](../finstack-py/src/bindings/cashflows.rs#L47-L62) claims `{date, amount, currency, kind}`.
- WASM binding [cashflows.rs:31–34](../finstack-wasm/src/api/cashflows.rs#L31-L34) makes the same claim.
- Rust public fn [json.rs:254–306](../finstack/cashflows/src/json.rs#L254-L306) is correct.

This was likely copy-pasted between bindings and never validated against the actual `DatedFlowJson` shape.

### 3.2 Module-level docs are uniformly strong

Every `.rs` file that I audited has a `//!` module doc. None are missing. This is unusual and good — most Rust crates this size leak at least a few `mod.rs` files with no module doc.

### 3.3 References are well-placed where they exist

`weighted_average_life` cites SIFMA + Fabozzi. `OvernightCompoundingMethod` cites ISDA 2021 + ARRC. The convention helpers cite ISDA / ICMA / ARRC / ECB / BoE / BoJ. The accrual `Compounded` variant calls out the ICMA Rule 251.1 deviation explicitly. This is the right level of citation density for production financial code.

### 3.4 Doctest hygiene is strong

72 doctests, all pass. Examples use the public API correctly. The `#![doc(test(attr(allow(clippy::expect_used))))]` attribute lets doctests use `.expect()` without violating crate-level lint policy — that's the right setup.

### 3.5 Deprecated APIs are clean

Five deprecated items (three `pv_by_period_*` wrappers, `aggregate_cashflows_precise_checked`, `principal_events`, `build`) all have one-line doc + `#[deprecated(note = "...")]` pointing to the replacement. Standard idiom.

---

## 4. Recommendations

### Immediate (Blocker class — fix before next release)

1. **Fix the README floating-rate example.** Add `overnight_basis: None` to the `FloatingRateSpec` struct literal at [README.md:160–179](../finstack/cashflows/README.md#L160-L179). Even though it's not a doctest, it's the first thing copy-pasters land on.

2. **Fix the `dated_flows` docstring drift in both bindings.**
   - [finstack-py/src/bindings/cashflows.rs:47–62](../finstack-py/src/bindings/cashflows.rs#L47-L62)
   - [finstack-wasm/src/api/cashflows.rs:31–34](../finstack-wasm/src/api/cashflows.rs#L31-L34)
   Replace `{date, amount, currency, kind}` with the actual shape: `JSON array of {date, amount} where amount carries its own currency. CFKind is intentionally omitted; use the full schedule JSON if you need flow classification.`

### Short-term (Major class)

3. **`CashflowProvider::dated_cashflows` needs `# Errors`** ([traits.rs:132](../finstack/cashflows/src/traits.rs#L132)) — function returns `Result` but the docstring lacks the section.

4. **Bring `schedule_from_classified_flows` up to par with `schedule_from_dated_flows`** ([traits.rs:218](../finstack/cashflows/src/traits.rs#L218)). Add Arguments/Returns/Errors/Examples.

5. **Document `sort_flows`, `coupons`, and `merge_cashflow_schedules`** ([schedule.rs:49, 350, 488](../finstack/cashflows/src/builder/schedule.rs)). For `sort_flows` document the multi-key ordering (date → kind → currency → amount → reset_date). For `merge_cashflow_schedules` document the metadata-merge rules (mismatched fields collapse to default).

6. **Add `# Returns` to `emit_fixed_coupons_on` / `emit_float_coupons_on`** ([builder/emission/coupons.rs:151+](../finstack/cashflows/src/builder/emission/coupons.rs)). The `f64` they return is the PIK amount to capitalize — currently only inferable from context.

7. **Add JSDoc to `CashflowsNamespace` in `index.d.ts`** ([finstack-wasm/index.d.ts:931–946](../finstack-wasm/index.d.ts#L931-L946)). Mirror the `///` comments from `api/cashflows.rs`.

### Medium-term (Minor class — polish)

8. **README expansion.** Add a "Schedule Inspection" workflow section covering `weighted_average_life`, `coupons()`, `outstanding_path_per_flow` vs `outstanding_by_date`, and `merge_cashflow_schedules`. Add `RecoveryTiming` to the entry-points listing. Mention `jpy_tona_swap` in the convention helpers paragraph.

9. **Resolve `add_payment_window` visibility.** ([builder/splits.rs:8](../finstack/cashflows/src/builder/builder/splits.rs#L8)) Either downgrade to `pub(crate)` (preferred — `payment_split_program` is the public entry) or document fully.

10. **Note the cross-crate dependency in `bond_from_cashflows`** in both bindings: it's a convenience that pulls in `finstack-valuations`.

11. **Internal helper docstrings** in [builder/dataframe.rs](../finstack/cashflows/src/builder/dataframe.rs), [builder/compiler.rs](../finstack/cashflows/src/builder/compiler.rs), [builder/emission/fees.rs](../finstack/cashflows/src/builder/emission/fees.rs) — see file-by-file table above. None are public-API critical.

12. **Module `//!` for `traits.rs`** ([traits.rs:1](../finstack/cashflows/src/traits.rs#L1)) — expand from one line to a paragraph that frames `CashflowProvider` as the canonical instrument-to-cashflow contract.

### Nice-to-have

13. **Add `# Panics` block** to `emit_fee_generic` for the `debug_assert!` ([builder/emission/fees.rs:33](../finstack/cashflows/src/builder/emission/fees.rs#L33)) — even for private fns, `# Panics` is the right idiom.

14. **Consider documenting the JSON wire format** with one shared canonical reference (probably under `docs/`) and pointing the binding docstrings at it. Currently the JSON structure is implied by the Rust serde model and only fully understandable by reading the schemars-generated schemas.

---

## 5. Action Items Checklist

Blockers:
- [ ] README: add `overnight_basis: None` to FloatingRateSpec example
- [ ] Python `dated_flows` docstring: correct return shape
- [ ] WASM `dated_flows` docstring: correct return shape

Majors:
- [ ] `traits::CashflowProvider::dated_cashflows`: add `# Errors`
- [ ] `traits::schedule_from_classified_flows`: add Arguments/Returns/Errors/Examples
- [ ] `schedule::sort_flows`: document multi-key ordering
- [ ] `schedule::coupons`: add Examples (or `# See also`)
- [ ] `schedule::merge_cashflow_schedules`: add Arguments/Returns + metadata-merge rules
- [ ] `emit_fixed_coupons_on` / `emit_float_coupons_on`: add `# Returns` for the PIK `f64`
- [ ] `CashflowsNamespace` in `index.d.ts`: add JSDoc per method

Minors:
- [ ] README: add Schedule Inspection workflow section
- [ ] README: list `RecoveryTiming` and `jpy_tona_swap` in entry points
- [ ] `add_payment_window`: downgrade to `pub(crate)` or document fully
- [ ] `bond_from_cashflows` (Python + WASM): note cross-crate dependency
- [ ] `traits` module: expand module-level `//!` doc
- [ ] `dataframe.rs` private helpers: add `# References` where they implement non-trivial financial logic
- [ ] `emit_fee_generic`: add `# Panics`
- [ ] `compiler.rs`: add one-line docs to `pub(super)` type aliases

---

## 6. Files reviewed

Cashflow Rust crate (36 files):
- `Cargo.toml`, `README.md`
- `src/lib.rs`, `src/accrual.rs`, `src/aggregation.rs`, `src/json.rs`, `src/serde_defaults.rs`, `src/traits.rs`
- `src/builder/{mod,builder,calendar,compiler,credit_rates,dataframe,date_generation,periods,rate_helpers,schedule}.rs`
- `src/builder/builder/{build,coupons,fees,principal,splits,stepup}.rs`
- `src/builder/emission/{mod,amortization,coupons,credit,fees,helpers}.rs`
- `src/builder/specs/{mod,amortization,coupon,default,fees,prepayment,recovery,schedule}.rs`
- `tests/cashflows.rs`, `tests/cashflows/schema_roundtrip.rs`
- `benches/cashflow_hot_paths.rs`

Python binding (4 files):
- `finstack-py/src/bindings/cashflows.rs`
- `finstack-py/finstack/cashflows/__init__.py`
- `finstack-py/finstack/cashflows/__init__.pyi`
- `finstack-py/tests/test_cashflows.py`

WASM binding (4 files):
- `finstack-wasm/src/api/cashflows.rs`
- `finstack-wasm/exports/cashflows.js`
- `finstack-wasm/index.d.ts` (CashflowsNamespace slice, lines 931–946)
- `finstack-wasm/tests/wasm_cashflows.rs`

Verification commands run:
- `RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-cashflows --no-deps --all-features` → clean
- `cargo test -p finstack-cashflows --doc` → 72 passed, 0 failed, 1 ignored
