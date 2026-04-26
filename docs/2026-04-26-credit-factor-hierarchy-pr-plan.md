# Credit Factor Hierarchy Phased PR Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the approved hierarchical credit factor decomposition spec as a sequence of small, reviewable PRs that each leaves the Rust workspace green and preserves opt-in behavior.

**Architecture:** Build the canonical domain model in `finstack_core::factor_model`, implement calibration/decomposition/forecasting in `finstack_valuations`, reuse the existing `finstack_portfolio::factor_model` runtime, then expose the stable surface through schemas, Python, WASM, parity, and notebooks. Every runtime consumer takes `Option<&CreditFactorModel>` or an equivalent reference and falls back to current behavior when absent.

**Tech Stack:** Rust workspace crates (`finstack-core`, `finstack-valuations`, `finstack-portfolio`), serde JSON schemas, PyO3 bindings, wasm-bindgen bindings, `mise` task runner, `uv` for Python tests.

---

## Source Spec

Primary design spec:

- `docs/2026-04-26-credit-factor-hierarchy-design.md`

The checked-out worktree containing this plan is sparse for some valuation and binding code. For implementation, use the full repo paths below as the canonical targets when a file is absent from the sparse worktree.

## PR Dependency Graph

1. PR 1: Core data model and serde contract.
2. PR 2: Credit hierarchy matching and factor-model config wiring.
3. PR 3: Per-period decomposition utility.
4. PR 4: Calibration MVP with deterministic diagonal covariance.
5. PR 5: Calibration robustness, diagnostics, vol state, and golden artifacts.
6. PR 6: Portfolio risk and credit vol forecast integration.
7. PR 7: Attribution result plumbing and linear methods.
8. PR 8: Waterfall, parallel attribution, and carry decomposition.
9. PR 9: JSON schemas, docs, and migration notes.
10. PR 10: Python bindings and parity.
11. PR 11: WASM bindings and TypeScript facade.
12. PR 12: Notebook, benchmarks, and final hardening.

Each PR should merge independently. Do not start a later PR until the previous PR is green on its targeted lint/test set, except for draft branches used to prove API shape.

## Shared Invariants For Every PR

- Keep the feature opt-in. No model supplied means existing attribution, risk, carry, and serialization behavior remains unchanged.
- Keep Rust canonical names aligned with Python and WASM names: Rust/Python `snake_case`; WASM `camelCase`.
- Preserve deterministic ordering with `BTreeMap` or sorted vectors anywhere serialization or factor ordering is observable.
- Avoid new general utilities unless a PR proves an existing crate function cannot handle the need.
- Public Rust fields and public binding types need docs because clippy runs with `-D missing_docs`.
- Run `mise run all-fmt` before every PR is marked ready.
- For Rust-only PRs, run `mise run rust-lint` plus targeted Rust tests.
- For binding PRs, rebuild bindings with `mise run python-build` or `mise run wasm-build` before running host-language tests.

---

## PR 1: Core Credit Hierarchy Artifact Types

**Purpose:** Add the canonical serde-first credit hierarchy data model without changing runtime matching or valuation behavior.

**Files:**

- Create: `finstack/core/src/factor_model/credit_hierarchy.rs`
- Modify: `finstack/core/src/factor_model/mod.rs`
- Modify: `finstack/core/src/factor_model/types.rs` if new strongly typed aliases are needed.
- Test: inline tests in `finstack/core/src/factor_model/credit_hierarchy.rs`

**Public API to introduce:**

- `CreditFactorModel`
- `CreditHierarchySpec`
- `HierarchyDimension`
- `IssuerTags`
- `IssuerBetaPolicy`
- `IssuerBetaMode`
- `IssuerBetaRow`
- `IssuerBetas`
- `LevelsAtAnchor`
- `LevelAnchor`
- `VolState`
- `FactorHistories`
- `CalibrationDiagnostics`
- `GenericFactorSpec`
- `AdderVolSource`
- `FitQuality`
- `FactorCorrelationMatrix`

**Implementation steps:**

- [ ] Add `credit_hierarchy.rs` with serde types only. Use `#[serde(deny_unknown_fields)]` on stable artifact structs unless backward-compatible extension is explicitly planned.
- [ ] Set `CreditFactorModel::SCHEMA_VERSION` to `"finstack.credit_factor_model/1"`.
- [ ] Store issuer rows and level anchors in deterministic order. Prefer `Vec<IssuerBetaRow>` sorted by `issuer_id` for wire compatibility and `BTreeMap` inside helper-only APIs.
- [ ] Use existing `FactorModelConfig`, `FactorId`, `FactorType`, and date types rather than redefining them.
- [ ] Add constructors or validation helpers that check `schema_version`, duplicate issuer IDs, duplicate factor IDs, and hierarchy dimension names.
- [ ] Re-export the new module from `finstack/core/src/factor_model/mod.rs`.

**Tests:**

- [ ] `credit_factor_model_round_trips_json`
- [ ] `credit_factor_model_rejects_duplicate_issuers`
- [ ] `credit_hierarchy_custom_dimensions_serialize_deterministically`
- [ ] `credit_factor_ids_are_stable_for_same_hierarchy`
- [ ] `empty_hierarchy_is_valid`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-core factor_model::credit_hierarchy`

**Ready to merge when:**

- The new artifact serializes/deserializes without touching valuation or portfolio behavior.
- No consumer has a new required field or behavior change.

---

## PR 2: Credit Hierarchy Matching And Factor Config Wiring

**Purpose:** Make the core factor-model matcher understand calibrated credit hierarchy metadata while preserving existing `MatchingConfig` behavior.

**Files:**

- Modify: `finstack/core/src/factor_model/definition.rs`
- Modify: `finstack/core/src/factor_model/matching/config.rs`
- Modify: `finstack/core/src/factor_model/matching/matchers.rs`
- Modify: `finstack/core/src/factor_model/matching/filter.rs` only if issuer tags need a small helper.
- Modify: `finstack/core/src/factor_model/dependency.rs`
- Test: inline tests in the modified matching files.

**Implementation choices to lock:**

- Prefer extending `MatchingConfig` with `CreditHierarchical` only if existing `Hierarchical(HierarchicalConfig)` cannot express beta lookup cleanly.
- If `CreditHierarchical` is added, make it a thin config wrapper around existing matcher mechanics plus a beta lookup. Avoid a parallel matching tree.
- Factor IDs follow the spec convention:
  - `credit::generic`
  - `credit::level{idx}::{dimension_path}::{value_path}`
  - `credit::adder::{issuer_id}` only if adder needs a factor identity in a downstream report.

**Implementation steps:**

- [ ] Write a failing matcher test that maps a `MarketDependency::CreditCurve` plus issuer attributes into `credit::generic` and bucket factors in deterministic order.
- [ ] Add the minimal `MatchingConfig` and matcher support needed for calibrated issuer beta lookup.
- [ ] Add validation that all factor IDs referenced by the matching config exist in `FactorModelConfig.factors`.
- [ ] Keep `FactorType::Credit` as the factor type for all generated common factors.
- [ ] Ensure `FactorModelBuilder::build()` still catches factor/covariance ID misalignment through existing validation.

**Tests:**

- [ ] `credit_hierarchical_matcher_returns_generic_and_bucket_factors`
- [ ] `credit_hierarchical_matcher_errors_on_missing_required_tag`
- [ ] `credit_hierarchical_matcher_treats_unknown_issuer_as_bucket_only_when_tags_exist`
- [ ] `credit_hierarchical_config_rejects_unknown_factor_id`
- [ ] Existing `matching` tests still pass unchanged.

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-core factor_model::matching`
- `cargo test -p finstack-portfolio factor_model::model`

**Ready to merge when:**

- Existing `MappingTable`, `Cascade`, and `Hierarchical` behavior is unchanged.
- The credit hierarchy path is usable by `FactorModelConfig` but not yet required by valuation consumers.

---

## PR 3: Per-Period Credit Decomposition Utility

**Purpose:** Implement the pure decomposition functions that all attribution, carry, and calibration tests can share.

**Files:**

- Create: `finstack/valuations/src/factor_model/credit_decomposition.rs`
- Modify: `finstack/valuations/src/factor_model/mod.rs`
- Test: `finstack/valuations/tests/factor_model/credit_decomposition.rs`
- Modify: `finstack/valuations/tests/factor_model/mod.rs` if the test tree uses explicit module registration.

**Public API to introduce:**

- `CreditFactorModel::decompose_levels(...)`
- `CreditFactorModel::decompose_period(...)`
- Free functions `decompose_levels(...)` and `decompose_period(...)` if the crate already favors free-function exports in `factor_model`.
- `LevelsAtDate`
- `PeriodDecomposition`
- `LevelDecomposition`

**Implementation steps:**

- [ ] Build a tiny synthetic two-date, two-level fixture in the test module with known `beta` and known spreads.
- [ ] Write the reconciliation invariant test first:
  `observed_delta_spread_i = beta_pc * delta_generic + sum(beta_level * delta_level) + delta_adder`.
- [ ] Implement level decomposition as sequential residual peeling:
  generic residual, level residuals in hierarchy order, final adder.
- [ ] Validate missing tags before calculating bucket means and return a typed valuation/core error.
- [ ] Treat new issuers with complete tags as `BucketOnly` with beta `1.0` and zero historical adder.
- [ ] Skip issuers absent from observed spreads.
- [ ] Set empty current bucket level values to `0.0` and beta at that level to `0.0` for affected issuers.

**Tests:**

- [ ] `decompose_period_reconciles_each_issuer`
- [ ] `new_issuer_with_tags_is_bucket_only`
- [ ] `missing_required_tag_returns_error`
- [ ] `empty_bucket_at_as_of_degrades_to_zero_level`
- [ ] `empty_hierarchy_decomposes_to_generic_and_adder`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_decomposition`

**Ready to merge when:**

- The decomposition utility is deterministic, pure, and independent of pricing.
- All later PRs can reuse the same reconciliation fixture.

---

## PR 4: Calibration MVP With Diagonal Covariance

**Purpose:** Produce a deterministic `CreditFactorModel` artifact from sparse issuer spread history using the sequential peel-the-onion algorithm and identity correlation.

**Files:**

- Create: `finstack/valuations/src/factor_model/credit_calibration.rs`
- Modify: `finstack/valuations/src/factor_model/mod.rs`
- Test: `finstack/valuations/tests/factor_model/credit_calibration.rs`
- Optional test fixture: `finstack/valuations/tests/fixtures/credit_factor_panel.rs`

**Public API to introduce:**

- `CreditCalibrator`
- `CreditCalibrationInputs`
- `CreditCalibrationConfig`
- `HistoryPanel`
- `IssuerTagPanel`
- `GenericFactorSeries`
- `BucketSizeThresholds`
- `CovarianceStrategy::Diagonal`
- `BetaShrinkage`
- `PanelSpace`

**Implementation steps:**

- [ ] Add tests for mode classification before adding regression code.
- [ ] Implement `IssuerBetaPolicy::GloballyOff` and `IssuerBetaPolicy::Dynamic { min_history, overrides }`.
- [ ] Use `analytics::benchmark::beta` for single-factor OLS; do not add a general regression module.
- [ ] Compute bucket inventory by hierarchy level and fold sparse buckets into the parent level.
- [ ] Generate factor histories in canonical factor ID order.
- [ ] Assemble `FactorModelConfig` with `FactorType::Credit`, diagonal `FactorCovarianceMatrix`, and credit hierarchy matching config from PR 2.
- [ ] Compute anchor levels from `asof_spreads` using the same decomposition utility from PR 3.
- [ ] Add calibration diagnostics for mode counts, bucket sizes, fold-ups, and missing observations.

**Tests:**

- [ ] `calibration_is_bit_identical_for_same_inputs`
- [ ] `globally_off_sets_all_betas_to_one`
- [ ] `dynamic_policy_classifies_short_history_as_bucket_only`
- [ ] `override_force_issuer_beta_wins`
- [ ] `sparse_bucket_folds_to_parent`
- [ ] `single_level_hierarchy_builds_expected_factor_ids`
- [ ] `all_bucket_only_calibration_succeeds`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_calibration`
- `cargo test -p finstack-core factor_model`

**Ready to merge when:**

- A caller can calibrate a self-contained artifact for generic plus hierarchy factors.
- Only diagonal covariance is supported in this PR; no GARCH/EWMA or full sample covariance yet.

---

## PR 5: Calibration Robustness, Vol State, Diagnostics, Golden Artifacts

**Purpose:** Complete the calibration artifact so it contains idiosyncratic vol state, optional covariance strategies, factor histories, and structured diagnostics.

**Files:**

- Modify: `finstack/valuations/src/factor_model/credit_calibration.rs`
- Modify: `finstack/core/src/factor_model/credit_hierarchy.rs`
- Test: `finstack/valuations/tests/factor_model/credit_calibration.rs`
- Create: `finstack/valuations/tests/golden/credit_factor_model_v1.json`
- Optional bench: `finstack/valuations/benches/credit_factor_calibration.rs`

**Implementation steps:**

- [ ] Add caller-supplied idiosyncratic vol overrides and test override precedence.
- [ ] Implement peer proxy fallback chain: exact bucket, parent bucket, global default.
- [ ] Implement `VolModelChoice::Sample` first, then wire existing EWMA/GARCH choices only where existing analytics APIs already expose them cleanly.
- [ ] Add `CovarianceStrategy::Ridge { alpha }` and `CovarianceStrategy::FullSampleRepaired` using existing covariance and nearest-correlation utilities.
- [ ] Add PSD repair tests for full sample covariance.
- [ ] Write golden JSON only after deterministic ordering is proven.
- [ ] Add the performance bench as non-gating unless the repo already gates benches in CI.

**Tests:**

- [ ] `idiosyncratic_override_wins_over_history`
- [ ] `bucket_peer_proxy_falls_back_to_parent_then_global`
- [ ] `full_sample_repaired_covariance_is_psd`
- [ ] `golden_credit_factor_model_matches_checked_in_json`
- [ ] `diagnostics_include_mode_counts_foldups_and_fallbacks`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_calibration`
- `cargo test -p finstack-valuations --test factor_model`

**Ready to merge when:**

- The calibrated artifact is complete enough for runtime consumers and stable enough for JSON schema work.
- Any EWMA/GARCH support uses existing analytics code; missing advanced variants remain v2 candidates.

---

## PR 6: Portfolio Risk And Credit Vol Forecast Integration

**Purpose:** Use the calibrated credit model to generate horizon-specific factor covariance and grouped volatility contribution reports through existing portfolio factor-model decomposers.

**Files:**

- Create: `finstack/valuations/src/factor_model/credit_vol_forecast.rs`
- Modify: `finstack/valuations/src/factor_model/mod.rs`
- Modify: `finstack/portfolio/src/factor_model/types.rs`
- Modify: `finstack/portfolio/src/factor_model/parametric.rs`
- Modify: `finstack/portfolio/src/factor_model/simulation.rs` if residual contributions are supported there.
- Test: `finstack/valuations/tests/factor_model/credit_vol_forecast.rs`
- Test: existing portfolio factor-model tests in `finstack/portfolio/src/factor_model/`

**Public API to introduce:**

- `FactorCovarianceForecast`
- `VolHorizon`
- `CreditVolReport`
- `LevelVolContribution`
- `PositionVolContribution`
- `PositionResidualContribution`

**Implementation steps:**

- [ ] Add `position_residual_contributions: Vec<PositionResidualContribution>` to `RiskDecomposition` with serde default for backward-compatible deserialization.
- [ ] Extend parametric decomposition to emit residual/idiosyncratic contribution when sensitivity rows carry issuer residual metadata.
- [ ] Implement `FactorCovarianceForecast::covariance_at`.
- [ ] Implement `FactorCovarianceForecast::idiosyncratic_vol`.
- [ ] Implement `FactorCovarianceForecast::factor_model_at` by cloning the calibrated config and replacing covariance/risk measure.
- [ ] Build `CreditVolReport` by grouping factor IDs by credit hierarchy prefix.
- [ ] Keep `VolHorizon::Custom` out of bindings unless it can be expressed as data; closures are Rust-only.

**Tests:**

- [ ] `forecast_covariance_is_psd`
- [ ] `one_step_and_unconditional_sample_vol_are_consistent`
- [ ] `bucket_only_issuer_vol_uses_cached_scalar_for_all_horizons`
- [ ] `credit_vol_report_groups_by_level_prefix`
- [ ] `risk_decomposition_deserializes_without_residual_contributions`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_vol_forecast`
- `cargo test -p finstack-portfolio factor_model`

**Ready to merge when:**

- Existing portfolio factor-model APIs still work with old serialized `RiskDecomposition`.
- Credit vol reporting is a pure aggregation on top of existing risk decomposition.

---

## PR 7: Attribution Result Plumbing And Linear Methods

**Purpose:** Add opt-in credit factor detail to attribution outputs and wire the low-cost Taylor and metrics-based methods first.

**Files:**

- Modify: `finstack/valuations/src/attribution/types.rs`
- Modify: `finstack/valuations/src/attribution/spec.rs`
- Modify: `finstack/valuations/src/attribution/metrics_based.rs`
- Modify: `finstack/valuations/src/attribution/taylor.rs`
- Modify: `finstack/valuations/src/attribution/mod.rs`
- Test: `finstack/valuations/tests/attribution/credit_factor_linear.rs`
- Test: `finstack/valuations/tests/attribution/spec_tests.rs`
- Test: `finstack/valuations/tests/attribution/serialization_roundtrip.rs`

**Public API to introduce:**

- `CreditFactorAttribution`
- `LevelPnl`
- `CreditFactorDetailOptions`
- `CreditFactorModelRef`
- Add `credit_factor_detail: Option<CreditFactorAttribution>` to `PnlAttribution`.
- Add `credit_factor_model` and `credit_factor_detail_options` to `AttributionSpec`.

**Implementation steps:**

- [ ] Add serde-defaulted optional fields to result/spec types so old JSON continues to deserialize.
- [ ] Add no-model fallback tests before wiring model logic.
- [ ] Implement a shared helper in `valuations::attribution::credit_factor` for linear credit PnL allocation:
  `-CS01_i * beta_i^level * delta_factor_level`.
- [ ] Wire `attribute_pnl_metrics_based` to populate `credit_factor_detail` when a model is supplied.
- [ ] Wire `attribute_pnl_taylor` to populate the same structure.
- [ ] Respect `include_per_issuer_adder` and `include_per_bucket_breakdown`.
- [ ] Keep `credit_curves_pnl` unchanged and reconcile detail totals to it within tolerance.

**Tests:**

- [ ] `metrics_based_no_model_matches_existing_credit_total`
- [ ] `metrics_based_credit_detail_reconciles_to_credit_curves_pnl`
- [ ] `taylor_credit_detail_reconciles_to_credit_curves_pnl`
- [ ] `per_issuer_adder_is_omitted_by_default`
- [ ] `per_bucket_breakdown_can_be_disabled`
- [ ] `old_attribution_json_deserializes_with_no_credit_detail`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_factor_linear`
- `cargo test -p finstack-valuations attribution::spec`

**Ready to merge when:**

- Linear attribution methods produce the new detail without changing totals.
- Serialized outputs remain backward compatible.

---

## PR 8: Waterfall, Parallel Attribution, And Carry Decomposition

**Purpose:** Complete the runtime consumption story for all attribution methods and the carry split.

**Files:**

- Create: `finstack/valuations/src/attribution/credit_factor.rs`
- Modify: `finstack/valuations/src/attribution/waterfall.rs`
- Modify: `finstack/valuations/src/attribution/parallel.rs`
- Modify: `finstack/valuations/src/attribution/helpers.rs`
- Modify: `finstack/valuations/src/attribution/factors.rs`
- Modify: `finstack/valuations/src/attribution/types.rs`
- Modify: `finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs`
- Test: `finstack/valuations/tests/attribution/credit_factor_waterfall_parallel.rs`
- Test: `finstack/valuations/tests/attribution/carry_credit_factor.rs`
- Test: `finstack/valuations/benches/attribution.rs` if benchmark scenarios need new options.

**Public API to introduce or complete:**

- `SourceLine`
- `CreditCarryDecomposition`
- `CreditCarryByLevel`
- `LevelCarry`
- Extend `CarryDetail` with source-line rates/credit split while preserving old behavior through serde defaults or compatibility helpers.

**Implementation steps:**

- [ ] Refactor credit market bump helpers so a caller can bump PC, each hierarchy level, and adder as separate synthetic credit moves.
- [ ] Replace the single waterfall credit step with `PC -> hierarchy levels -> adder` only when a model is supplied.
- [ ] Keep the original `default_waterfall_order()` output unchanged when no model is supplied.
- [ ] Add each credit hierarchy level to the parallel factor set when a model is supplied.
- [ ] Leave parallel cross-effects in existing `cross_factor_pnl`.
- [ ] Split `coupon_income` into rates and credit parts using the existing carry engine's yield/spread information.
- [ ] Split `roll_down` into rates and credit parts by separating old/new rates curves and spread curves already used internally.
- [ ] Allocate v1 credit roll-down to adder as specified.
- [ ] Add carry reconciliation checks for the five invariants from the spec.

**Tests:**

- [ ] `waterfall_credit_factor_detail_reconciles_to_credit_curves_pnl`
- [ ] `parallel_credit_detail_plus_cross_effects_preserves_total`
- [ ] `waterfall_no_model_keeps_default_credit_step`
- [ ] `same_credit_total_different_hierarchy_different_detail`
- [ ] `carry_coupon_total_equals_rates_plus_credit`
- [ ] `carry_roll_down_total_equals_rates_plus_credit`
- [ ] `credit_carry_total_equals_sum_of_credit_source_lines`
- [ ] `credit_carry_total_equals_generic_levels_and_adder`
- [ ] `rates_carry_total_matches_rates_source_lines_minus_funding`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- `cargo test -p finstack-valuations credit_factor_waterfall_parallel`
- `cargo test -p finstack-valuations carry_credit_factor`
- `cargo test -p finstack-valuations attribution`

**Ready to merge when:**

- All four attribution methods support credit factor detail.
- Carry decomposition is opt-in and reconciles without changing no-model outputs.

---

## PR 9: JSON Schemas, Docs, And Migration Notes

**Purpose:** Lock the wire format for the credit factor model and additive attribution/carry fields.

**Files:**

- Create: `schemas/factor_model/credit_factor_model.schema.json` or the existing repo schema directory equivalent.
- Create: `schemas/factor_model/credit_calibration_inputs.schema.json`
- Create: `schemas/factor_model/credit_calibration_config.schema.json`
- Modify: existing attribution result schema files for additive `credit_factor_detail` and carry extensions.
- Modify: `finstack/valuations/src/schema.rs` if schema registry is crate-local.
- Modify: `CHANGELOG.md`
- Modify: `INVARIANTS.md` only if new schema-version policy language is needed.
- Test: schema parity or audit tests under `finstack/valuations/tests/`.

**Implementation steps:**

- [ ] Locate the repo's active schema directory before adding files; do not create a second schema root.
- [ ] Generate or hand-author schema files from the Rust serde contract.
- [ ] Add schema-version validation for `"finstack.credit_factor_model/1"`.
- [ ] Extend attribution schemas additively. Keep old attribution payloads valid.
- [ ] Add a changelog entry that states the feature is opt-in and non-breaking for existing attribution consumers.
- [ ] Document `CreditFactorModelRef` behavior after the implementation choice is final: inline JSON, file path, or both.

**Tests:**

- [ ] `credit_factor_model_schema_accepts_golden_artifact`
- [ ] `credit_factor_model_schema_rejects_wrong_schema_version`
- [ ] `old_attribution_result_schema_payload_still_valid`
- [ ] `new_attribution_result_schema_accepts_credit_factor_detail`

**Run:**

- `mise run all-fmt`
- `mise run rust-lint`
- Targeted schema tests, for example `cargo test -p finstack-valuations schema`

**Ready to merge when:**

- The artifact and result schemas match the Rust wire format.
- Documentation reflects the actual behavior shipped in PRs 1-8.

---

## PR 10: Python Bindings And Parity

**Purpose:** Expose the stable credit hierarchy APIs to Python with thin PyO3 wrappers and parity entries.

**Files:**

- Modify or split: `finstack-py/src/bindings/valuations/factor_model.rs`
- Create: `finstack-py/src/bindings/valuations/factor_model/credit_factor_model.rs` if the existing file is too large.
- Create: `finstack-py/src/bindings/valuations/factor_model/credit_calibrator.rs`
- Create: `finstack-py/src/bindings/valuations/factor_model/credit_decomposition.rs`
- Create: `finstack-py/src/bindings/valuations/factor_model/credit_vol_forecast.rs`
- Modify: `finstack-py/src/bindings/valuations/mod.rs`
- Modify: `finstack-py/finstack/valuations/__init__.pyi`
- Modify: `finstack-py/parity_contract.toml`
- Test: `finstack-py/tests/test_valuations_new_bindings.py`
- Test: `finstack-py/tests/test_core_parity.py` if any core-level types are exposed directly.

**Binding surface:**

- `CreditFactorModel.from_json`
- `CreditFactorModel.to_json`
- `CreditCalibrator.calibrate`
- `decompose_levels`
- `decompose_period`
- `FactorCovarianceForecast.covariance_at`
- `FactorCovarianceForecast.idiosyncratic_vol`
- `FactorCovarianceForecast.factor_model_at` if a Python `FactorModel` wrapper exists.

**Implementation steps:**

- [ ] Keep all business logic in Rust; bindings perform JSON conversion, wrapper construction, and error mapping only.
- [ ] Use `pub(crate) inner: RustType` plus `from_inner()` wrapper pattern.
- [ ] Register every new symbol explicitly in `__all__`.
- [ ] Add `.pyi` stubs using canonical Rust/Python names.
- [ ] Update `parity_contract.toml` for every public type and function.
- [ ] Add Python smoke tests that calibrate from synthetic JSON, serialize the artifact, decompose a period, and request a vol forecast.

**Run:**

- `mise run all-fmt`
- `mise run python-build`
- `mise run python-lint`
- `uv run pytest finstack-py/tests/test_valuations_new_bindings.py -q`
- `uv run pytest finstack-py/tests/test_core_parity.py -q`

**Ready to merge when:**

- Python users can complete the synthetic flow without touching private Rust internals.
- Parity entries document every new public binding.

---

## PR 11: WASM Bindings And TypeScript Facade

**Purpose:** Mirror the Python binding surface in WASM with JSON in/out APIs and a hand-written JS/TS facade.

**Files:**

- Modify: `finstack-wasm/src/api/valuations/factor_model.rs`
- Create sibling WASM modules if the existing file is too large.
- Modify: `finstack-wasm/src/api/valuations/mod.rs`
- Modify: `finstack-wasm/exports/valuations.js`
- Modify: `finstack-wasm/index.d.ts`
- Modify: `finstack-wasm/index.js` only if new top-level exports are needed.
- Test: existing WASM tests under `finstack-wasm/`.

**WASM names:**

- `creditFactorModel`
- `creditCalibrator`
- `decomposeLevels`
- `decomposePeriod`
- `factorCovarianceForecast`

**Implementation steps:**

- [ ] Use `#[wasm_bindgen(js_name = ...)]` for every exported function.
- [ ] Keep input/output as JSON strings where wrapper object ownership would add complexity.
- [ ] Add explicit TypeScript declarations to `index.d.ts`.
- [ ] Wire exports through `exports/valuations.js`; do not rely on raw `pkg/` exports.
- [ ] Keep `VolHorizon::Custom` unavailable in WASM unless represented as a serializable enum variant.

**Tests:**

- [ ] WASM smoke test for calibrate -> serialize -> decompose.
- [ ] WASM smoke test for `creditFactorModel` JSON round-trip.
- [ ] TypeScript declaration test if the package has one.

**Run:**

- `mise run all-fmt`
- `mise run wasm-build`
- `mise run wasm-lint`
- `npm --prefix finstack-wasm run test`

**Ready to merge when:**

- WASM exposes the same stable functionality as Python with camelCase names.
- `index.d.ts` and `exports/valuations.js` match generated wasm-bindgen names.

---

## PR 12: Notebook, Benchmarks, End-To-End Hardening

**Purpose:** Prove the full feature works for a user-facing synthetic workflow and add final production-readiness checks.

**Files:**

- Create: `finstack-py/examples/notebooks/05_portfolio_and_scenarios/credit_factor_hierarchy.ipynb`
- Modify: `finstack-py/examples/notebooks/run_all_notebooks.py` only if notebook discovery requires it.
- Test: `finstack-py/tests/test_run_all_notebooks.py`
- Bench: `finstack/valuations/benches/credit_factor_calibration.rs`
- Bench: `finstack/valuations/benches/attribution_scale.rs`
- Modify: `CHANGELOG.md`

**Notebook flow:**

- Build synthetic spread history.
- Build issuer tags and hierarchy spec.
- Calibrate a `CreditFactorModel`.
- Save and reload JSON.
- Decompose a two-date period.
- Run attribution on a small sample portfolio.
- Run a credit vol forecast report.

**Implementation steps:**

- [ ] Create the notebook with small synthetic data so it runs quickly in CI.
- [ ] Keep notebook outputs deterministic and lightweight.
- [ ] Add or update benchmark scenarios for `500 issuers x 60 months x 3 levels` calibration and `200-position` attribution/vol forecast.
- [ ] Run a final old-payload compatibility sweep for attribution JSON and `RiskDecomposition`.
- [ ] Run a final no-model fallback sweep for all consumers touched by PRs 7-8.
- [ ] Update the changelog with the final shipped API list and any deferred v2 items.

**Run:**

- `mise run all-fmt`
- `mise run all-lint`
- `uv run pytest finstack-py/tests/test_run_all_notebooks.py -q`
- Targeted Rust tests from PRs 1-8
- Binding smoke tests from PRs 10-11

**Ready to merge when:**

- The notebook demonstrates the full workflow without hidden setup.
- Performance targets are either met or documented with measured numbers and follow-up issues.
- The feature is ready for a release candidate.

---

## Cross-PR Review Checklist

- [ ] Does every new public Rust field have documentation?
- [ ] Are all serde additions optional or backward-compatible where the design requires non-breaking behavior?
- [ ] Does every level of PnL/carry/vol decomposition reconcile to the existing total?
- [ ] Does no-model behavior produce byte-equivalent or field-equivalent results to pre-feature behavior?
- [ ] Are factor IDs deterministic and stable across calibration runs?
- [ ] Are Python and WASM names aligned with the canonical Rust names?
- [ ] Did the PR avoid adding a general utility that already exists in `analytics`, `core`, `valuations`, or `portfolio`?

## Deferred V2 Items

Keep these out of the above PRs unless a reviewer explicitly expands scope:

- Term-structure level factors.
- PCA-derived generic factor.
- Multivariate or DCC GARCH.
- Online covariance updating.
- Joint loadings calibration with regularization.
- Ledoit-Wolf covariance shrinkage.
- General `core::math::regression`.
- FRTB or regulatory adapters.
