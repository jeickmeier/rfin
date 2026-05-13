# Credit Factor Hierarchy — Subagent Implementation Plan

> **Workflow:** `superpowers:subagent-driven-development`
> **Source spec:** [`docs/archive/plans/2026-04-26-credit-factor-hierarchy-design.md`](2026-04-26-credit-factor-hierarchy-design.md)
> **PR plan:** [`docs/archive/plans/2026-04-26-credit-factor-hierarchy-pr-plan.md`](2026-04-26-credit-factor-hierarchy-pr-plan.md)
> **Status:** Ready to execute
> **Date:** 2026-04-26

This document is the execution wrapper around the 12-PR plan. It does not restate the spec or the PR plan — it tells a controller agent **how to dispatch each PR to a subagent, what context to bundle, which model tier to use, and what each reviewer must check.**

When the controller executes, it pastes the relevant PR plan section verbatim into the implementer prompt (per the subagent-driven-development skill: "Make subagent read plan file" is a red flag — the controller curates context).

---

## 1. Strategy

### 1.1 Why subagents for this work

- 12 PRs span 5 crates (`finstack-core`, `finstack-valuations`, `finstack-portfolio`, `finstack-py`, `finstack-wasm`) and accumulate significant context. A single session would drift mid-stream.
- Each PR is a clean unit with explicit acceptance criteria already laid out in the PR plan — ideal for fresh-context dispatch.
- Reconciliation invariants (§7.4 carry, §6.3 attribution, §5.3 decomposition) are exactly the kind of acceptance check a spec reviewer can verify without having lived through the implementation.
- Two-stage review (spec compliance → code quality) is critical because **opt-in / non-breaking** is a load-bearing invariant; spec drift in PR 7/8 could silently break old serialized payloads. A spec reviewer with the PR text and nothing else is a strong defense.

### 1.2 Worktree and branch strategy

- **Stay in the current worktree.** Path: `/Users/jeickmeier/Projects/rfin/.claude/worktrees/naughty-mclean-6a0594`. Branch: `claude/naughty-mclean-6a0594`.
- **Stack one commit per PR** on this branch. Each PR's commit message starts with `pr-N:` so it can later be branched into a PR off `master`.
- **Do not push or open GitHub PRs from this session** unless the user requests it. Surface a final summary listing each PR's commit SHA and recommend a branching strategy.
- **One-PR-per-worktree** (using `superpowers:using-git-worktrees`) is overkill here — PRs are sequential and never run in parallel, so fresh worktrees would just add overhead.

### 1.3 Dispatch model

- **One implementer dispatch per PR by default.** A PR is the granularity that already passed brainstorming; splitting further fights the plan.
- **Split into two implementer dispatches only where the PR plan has clean internal seams** (PR 5 and PR 8 below). Each split dispatch still gets its own two-stage review.
- **Never dispatch implementers in parallel.** Same files, same workspace — guaranteed merge conflicts.
- **Two-stage review after every implementer dispatch:** spec compliance reviewer first, then code-quality reviewer. Loop both until ✅.
- **Final review after PR 12:** dispatch one more code-quality reviewer over the entire stack to catch cross-PR drift before recommending merge.

### 1.4 Always pass to every implementer subagent

Every implementer dispatch includes the same five context blocks:

1. **Full text of the relevant PR section** from the PR plan, copy-pasted (not a path).
2. **The relevant design-spec sections** (listed per PR in §5 below) copy-pasted.
3. **Worktree path:** `/Users/jeickmeier/Projects/rfin/.claude/worktrees/naughty-mclean-6a0594`.
4. **Verification commands** from the PR plan's "Run" block plus repo-wide `mise run all-fmt`.
5. **Cross-PR invariants** (§4 below) reproduced inline.

The design spec is large; do **not** paste the whole thing — paste only the sections each PR cares about. The mapping is in §5.

### 1.5 Model selection rationale

Cheap (Sonnet/Haiku tier) is fine for mechanical work where the spec is dense and unambiguous. Capable (Opus tier) is needed where multi-file integration, math correctness, or pattern recognition across an existing codebase is required. The mapping in §5 is the recommendation; the controller may upgrade if a subagent returns `BLOCKED` or `DONE_WITH_CONCERNS`.

---

## 2. Setup

Before dispatching PR 1, the controller must:

- [ ] Confirm working directory is the worktree path above (`pwd`).
- [ ] Confirm `git status` is clean.
- [ ] Confirm `git log -1` shows commit `50923bfe8 docs: add phased PR plan ...` or later.
- [ ] Read the design spec and PR plan once, end to end. Do not skim.
- [ ] Create the TodoWrite skeleton in §3.

The controller should **not** read the implementation code itself before dispatching — that's the subagent's job. Reading code into the controller's context defeats the purpose of fresh-context dispatch.

---

## 3. TodoWrite Skeleton

Create these todos at the start of execution. Each PR has three (impl, spec-review, quality-review). The `merge-prep` final entry covers the end-of-stack code review.

```text
[ ] PR-1: implementer — core artifact types
[ ] PR-1: spec compliance review
[ ] PR-1: code quality review
[ ] PR-2: implementer — credit hierarchy matcher
[ ] PR-2: spec compliance review
[ ] PR-2: code quality review
[ ] PR-3: implementer — per-period decomposition
[ ] PR-3: spec compliance review
[ ] PR-3: code quality review
[ ] PR-4: implementer — calibration MVP (diagonal Σ)
[ ] PR-4: spec compliance review
[ ] PR-4: code quality review
[ ] PR-5a: implementer — vol state + idio fallback chain
[ ] PR-5a: spec compliance review
[ ] PR-5a: code quality review
[ ] PR-5b: implementer — covariance strategies + golden artifact
[ ] PR-5b: spec compliance review
[ ] PR-5b: code quality review
[ ] PR-6: implementer — vol forecast + RiskDecomposition residuals
[ ] PR-6: spec compliance review
[ ] PR-6: code quality review
[ ] PR-7: implementer — attribution result plumbing + linear methods
[ ] PR-7: spec compliance review
[ ] PR-7: code quality review
[ ] PR-8a: implementer — waterfall + parallel attribution
[ ] PR-8a: spec compliance review
[ ] PR-8a: code quality review
[ ] PR-8b: implementer — carry decomposition
[ ] PR-8b: spec compliance review
[ ] PR-8b: code quality review
[ ] PR-9: implementer — JSON schemas + changelog
[ ] PR-9: spec compliance review
[ ] PR-9: code quality review
[ ] PR-10: implementer — Python bindings + parity
[ ] PR-10: spec compliance review
[ ] PR-10: code quality review
[ ] PR-11: implementer — WASM bindings + TS facade
[ ] PR-11: spec compliance review
[ ] PR-11: code quality review
[ ] PR-12: implementer — notebook + benches + final hardening
[ ] PR-12: spec compliance review
[ ] PR-12: code quality review
[ ] Final: end-of-stack code review across all 12 PR commits
[ ] Final: surface commit SHAs + branching plan to user
```

Mark each item complete the moment it's done — do not batch.

---

## 4. Cross-PR Invariants (paste into every reviewer prompt)

These come from the PR plan's "Shared Invariants" section. Reviewers must check each one explicitly:

1. **Opt-in.** Every consumer takes `Option<&CreditFactorModel>` (or equivalent). No-model behavior is byte-equivalent (or field-equivalent for `serde(default)` extensions) to pre-feature behavior. Old serialized JSON deserializes unchanged.
2. **Non-breaking serde.** New fields are `#[serde(default, skip_serializing_if = "Option::is_none")]` or equivalent. Stable artifact types use `#[serde(deny_unknown_fields)]`.
3. **Deterministic ordering.** `BTreeMap` for keyed maps; `Vec<T>` sorted by stable key for arrays. Factor IDs canonical-ordered. Same inputs → bit-identical artifact JSON.
4. **Reconciliation invariants.**
   - Decomposition (§5.3 of design): `ΔS_i ≡ β_i^PC × ΔF_PC + Σ_levels β_i^<level> × ΔF_<level>(g_i) + Δadder_i`.
   - Attribution (§6.3): `generic_pnl + Σ_levels(level.total) + adder_pnl_total ≡ credit_curves_pnl`.
   - Carry (§7.4): all five invariants.
5. **Naming triplet alignment.** Rust/Python `snake_case` identical; WASM `camelCase`. Public Rust fields and binding types have docs (clippy `-D missing_docs`).
6. **No new general utilities.** `analytics::benchmark::beta`, `core::math::stats::covariance`, `valuations::correlation::nearest_correlation` cover everything in v1. Any new utility introduction is a flag.
7. **Lint and format.** `mise run all-fmt` and the targeted `mise run rust-lint` (or python/wasm equivalent) must pass before the implementer reports DONE.

---

## 5. Per-PR Dispatch Cards

Each card below feeds **one** implementer dispatch (or two, where split). Each dispatch is followed by spec-compliance review then code-quality review per §1.3.

### PR 1 — Core Credit Hierarchy Artifact Types

- **Goal:** Add the canonical serde-first data model. Behavior unchanged.
- **Dispatches:** 1.
- **Model:** Sonnet (mechanical type definitions; no algorithmic content).
- **Design spec sections to bundle:** §3.1, §3.2, §3.3, §3.4, §9.1, §9.5.
- **PR plan section to paste:** "PR 1" entire block.
- **Files (preview for reviewer):** `finstack/core/src/factor_model/credit_hierarchy.rs` (new), `finstack/core/src/factor_model/mod.rs` (re-export), maybe `types.rs`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-core factor_model::credit_hierarchy`
- **Reviewer focus:**
  - Schema version constant is exactly `"finstack.credit_factor_model/1"`.
  - `IssuerTags` uses `BTreeMap` (deterministic serde).
  - Validation rejects duplicate issuer IDs and duplicate factor IDs.
  - Empty hierarchy `[]` is a valid configuration.
  - No runtime behavior change — grep for non-test changes outside the new module.
- **Risks / notes:** This PR establishes naming for everything downstream. If it ships with a wrong field name, every later PR pays. Spec reviewer must cross-check every field name against design §3.2/§3.3.

### PR 2 — Credit Hierarchy Matching and Factor Config Wiring

- **Goal:** Make `MatchingConfig` understand calibrated credit hierarchy metadata.
- **Dispatches:** 1.
- **Model:** Opus (multi-file integration with existing matching code; risk of duplicating mechanism).
- **Design spec sections to bundle:** §2.1 (the reuse table), §2.2, §3.4, design's note about `MatchingConfig::CreditHierarchical` only if existing `Hierarchical` cannot express it.
- **PR plan section to paste:** "PR 2" entire block.
- **Files:** `finstack/core/src/factor_model/{definition,matching/{config,matchers,filter},dependency}.rs`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-core factor_model::matching`
  - `cargo test -p finstack-portfolio factor_model::model`
- **Reviewer focus:**
  - **Did the implementer prefer extending existing `Hierarchical` over adding `CreditHierarchical`?** If they added a new variant, they must justify in the dispatch report. (PR plan says: extend only if existing cannot express beta lookup cleanly.)
  - Factor IDs follow `credit::generic` and `credit::level{idx}::{dim}::{val}` exactly.
  - All factor IDs referenced by the matching config are present in `FactorModelConfig.factors`.
  - Existing `MappingTable`, `Cascade`, `Hierarchical` tests still green.
- **Risks / notes:** Highest risk of "parallel pathway" smell flagged by `anthropic-skills:finstack-simplify`. Reviewer should explicitly check that the implementer didn't create a parallel matcher tree.

### PR 3 — Per-Period Decomposition Utility

- **Goal:** Pure math utility shared by attribution, carry, calibration. Reconciliation invariant must hold at 1e-10.
- **Dispatches:** 1.
- **Model:** Opus (math correctness; reconciliation invariant is load-bearing).
- **Design spec sections to bundle:** §5 (entire), §3.4, §6.3 (the invariant they must preserve later).
- **PR plan section to paste:** "PR 3" entire block.
- **Files:** `finstack/valuations/src/factor_model/credit_decomposition.rs` (new), `finstack/valuations/src/factor_model/mod.rs`, test fixtures.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_decomposition`
- **Reviewer focus:**
  - **Reconciliation test exists and passes at 1e-10.** This is the load-bearing test for everything downstream.
  - New issuer with full tags handled as `BucketOnly`, β=1, zero adder.
  - Missing tag returns a typed error (no panic, no silent zero).
  - Empty bucket at as-of degrades to zero level for affected issuers.
  - `decompose_period` is just `to - from` per component (not re-running level math on differences).
- **Risks / notes:** This is the most-tested function in the entire feature. PRs 4, 7, 8a, 8b all reuse it. Get it right here.

### PR 4 — Calibration MVP With Diagonal Covariance

- **Goal:** Produce a deterministic `CreditFactorModel` artifact from sparse history. Diagonal Σ only — no GARCH/EWMA yet.
- **Dispatches:** 1.
- **Model:** Opus (multi-step algorithm: mode classification → bucket inventory → PC → level peels → anchor levels → Σ assembly; many places for subtle bugs).
- **Design spec sections to bundle:** §4 (entire — algorithm), §3 (artifact shape), §5 (decomposition reuse).
- **PR plan section to paste:** "PR 4" entire block.
- **Files:** `finstack/valuations/src/factor_model/credit_calibration.rs` (new), test fixtures, `mod.rs`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_calibration`
  - `cargo test -p finstack-core factor_model`
- **Reviewer focus:**
  - **Sequential peel-the-onion order matches §4.2 exactly.** No re-ordering.
  - `analytics::benchmark::beta` is reused — no new OLS implementation.
  - `IssuerBetaPolicy::GloballyOff` produces betas all = 1.0 and zero per-issuer regressions.
  - Sparse-bucket fold-up is recorded in diagnostics (every fold-up).
  - Σ is diagonal, identity ρ, factors assumed orthogonal — no full sample covariance in this PR.
  - Determinism test: same inputs twice → bit-identical JSON.
- **Risks / notes:** This is the largest single PR in lines of code. If implementer reports `DONE_WITH_CONCERNS` about file size, accept and note — splitting was already considered and rejected because the algorithm is one cohesive flow.

### PR 5a — Vol State and Idiosyncratic Vol Fallback Chain

- **Goal:** Add `vol_state` (per-factor + per-issuer) with the caller→peer→parent→default fallback chain.
- **Dispatches:** 1 (this is the first half of the PR plan's "PR 5").
- **Model:** Standard.
- **Design spec sections to bundle:** §4.2 step 7, §3.3 (`AdderVolSource`), §8.1 (idio vol math).
- **PR plan section to paste:** "PR 5" — implementer is told to scope to vol-state additions only; defer covariance strategies and golden artifact to PR 5b.
- **Files:** `finstack/valuations/src/factor_model/credit_calibration.rs`, `finstack/core/src/factor_model/credit_hierarchy.rs`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_calibration`
- **Reviewer focus:**
  - Caller-supplied override always wins over computed vol.
  - Fallback chain is exact: `IssuerBeta history → BucketPeerProxy → parent bucket → global default`.
  - `AdderVolSource` enum is recorded per issuer for traceability.
  - `VolModelChoice::Sample` works first; only existing analytics APIs (no new GARCH module).

### PR 5b — Covariance Strategies and Golden Artifact

- **Goal:** Add `Ridge` and `FullSampleRepaired` covariance strategies; commit a golden JSON artifact.
- **Dispatches:** 1.
- **Model:** Standard.
- **Design spec sections to bundle:** §4.1 (`CovarianceStrategy`), §4.2 step 9, §2.1 (PSD repair via `valuations::correlation::nearest_correlation`).
- **PR plan section to paste:** "PR 5" — covariance + golden subset.
- **Files:** `finstack/valuations/src/factor_model/credit_calibration.rs`, `finstack/valuations/tests/golden/credit_factor_model_v1.json` (new), `finstack/valuations/benches/credit_factor_calibration.rs` (optional).
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_calibration`
  - `cargo test -p finstack-valuations --test factor_model`
- **Reviewer focus:**
  - **Golden JSON written only after determinism is proven** (PR 5a determinism test must pass before this golden is checked in).
  - PSD repair uses existing `valuations::correlation::nearest_correlation` — no new linear algebra.
  - Ridge `alpha` defaults reasonable (typical 1e-4 to 1e-2 range).
  - Bench file is non-gating unless CI already gates benches.

### PR 6 — Portfolio Risk and Credit Vol Forecast Integration

- **Goal:** Wire calibrated model into existing `FactorModel.analyze()` pipeline; emit grouped `CreditVolReport`.
- **Dispatches:** 1.
- **Model:** Opus (touches portfolio crate's existing `RiskDecomposition` and parametric decomposer; high risk of breaking serialized payloads).
- **Design spec sections to bundle:** §8 (entire), §2.1 reuse table, §3.4 dual-purpose note.
- **PR plan section to paste:** "PR 6" entire block.
- **Files:** `finstack/valuations/src/factor_model/credit_vol_forecast.rs` (new), `finstack/portfolio/src/factor_model/{types,parametric,simulation}.rs`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_vol_forecast`
  - `cargo test -p finstack-portfolio factor_model`
- **Reviewer focus:**
  - **`RiskDecomposition::position_residual_contributions` has `#[serde(default)]`.** Old serialized payloads must still deserialize. Test exists.
  - `VolHorizon::Custom` is Rust-only; not exposed to bindings (defer to PR 10/11).
  - `BucketOnly` issuer vol = cached scalar regardless of horizon.
  - `CreditVolReport` is pure aggregation by factor-ID prefix — no new risk math.
- **Risks / notes:** This PR is the most likely place to silently break existing portfolio risk tests. Reviewer must run `cargo test -p finstack-portfolio` in full, not just credit-related tests.

### PR 7 — Attribution Result Plumbing and Linear Methods

- **Goal:** Add `credit_factor_detail` to `PnlAttribution`; wire metrics-based and Taylor methods.
- **Dispatches:** 1.
- **Model:** Opus (touches result types — risk of breaking serialized attribution payloads — and two attribution methods).
- **Design spec sections to bundle:** §6 (entire), §5.3 (decomposition invariant they must preserve).
- **PR plan section to paste:** "PR 7" entire block.
- **Files:** `finstack/valuations/src/attribution/{types,spec,metrics_based,taylor,mod}.rs`, tests.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_factor_linear`
  - `cargo test -p finstack-valuations attribution::spec`
- **Reviewer focus:**
  - **Old attribution JSON deserializes with no `credit_factor_detail`.** Test exists. Run it.
  - `credit_curves_pnl` field is unchanged when no model supplied.
  - `include_per_issuer_adder` defaults to `false` (large-portfolio payload control).
  - Reconciliation invariant test passes at 1e-8 for both methods.
  - The shared linear helper lives at `valuations::attribution::credit_factor` (created in PR 8a, but PR 7 may need to bootstrap it — implementer should note if so).

### PR 8a — Waterfall and Parallel Attribution

- **Goal:** Cascade waterfall through `PC → levels → adder`; expand parallel factor set.
- **Dispatches:** 1.
- **Model:** Opus (waterfall is the hardest of the four methods; per-step bumping must reprice cleanly).
- **Design spec sections to bundle:** §6.1, §6.2, §6.3, §6.5 (perf note).
- **PR plan section to paste:** "PR 8" — waterfall + parallel subset.
- **Files:** `finstack/valuations/src/attribution/{credit_factor,waterfall,parallel,helpers,factors,types}.rs`, tests.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations credit_factor_waterfall_parallel`
  - `cargo test -p finstack-valuations attribution`
- **Reviewer focus:**
  - **No-model waterfall keeps `default_waterfall_order()` byte-identical.** Test exists.
  - Reconciliation invariant: waterfall `generic + Σ levels + adder ≡ credit_curves_pnl` at 1e-8.
  - Parallel cross-effect residual stays in **existing** `cross_factor_pnl` field — no new field for cross-effects.
  - Synthetic credit bumps reuse existing market-bump helpers — no new bump infra.
  - Same portfolio + same total + different hierarchies → different decomposition (test exists).

### PR 8b — Carry Decomposition

- **Goal:** Split `coupon_income` and `roll_down` into rates/credit; emit `CreditCarryDecomposition` with all five invariants.
- **Dispatches:** 1.
- **Model:** Opus (five reconciliation invariants — easy to break one while fixing another).
- **Design spec sections to bundle:** §7 (entire).
- **PR plan section to paste:** "PR 8" — carry subset.
- **Files:** `finstack/valuations/src/attribution/types.rs` (extend `CarryDetail`), `finstack/valuations/src/metrics/sensitivities/carry_decomposition.rs`, tests.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations carry_credit_factor`
- **Reviewer focus:**
  - **All five invariants from §7.4 have explicit tests.** Reviewer enumerates each.
  - `SourceLine.rates_part` and `credit_part` are `Option<SignedAmount>` — `None` when no model (preserves scalar collapse).
  - Roll-down credit allocation is **all to adder** in v1 (per design — level factors are scalar, no term structure).
  - Funding cost is never split (pure rates).
  - Theta unchanged (residual catch-all).

### PR 9 — JSON Schemas, Docs, Migration Notes

- **Goal:** Lock the wire format. Schemas are additive; old payloads valid.
- **Dispatches:** 1.
- **Model:** Sonnet (mechanical schema authoring; the Rust serde contract is the source of truth).
- **Design spec sections to bundle:** §9.1, §9.4, §9.5.
- **PR plan section to paste:** "PR 9" entire block.
- **Files:** `schemas/factor_model/*.schema.json` (new), existing attribution schemas (additive extension), `CHANGELOG.md`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run rust-lint`
  - `cargo test -p finstack-valuations schema`
- **Reviewer focus:**
  - Old attribution payload validates against new schema (test exists).
  - Wrong `schema_version` rejected (test exists).
  - Schemas live under the existing repo schema root — no second schema directory created.
  - Changelog says "opt-in, non-breaking" explicitly.

### PR 10 — Python Bindings and Parity

- **Goal:** Expose stable APIs to Python with PyO3.
- **Dispatches:** 1.
- **Model:** Sonnet (mechanical wrapping; the binding pattern is well-established in the repo).
- **Design spec sections to bundle:** §9.2, §9.5.
- **PR plan section to paste:** "PR 10" entire block.
- **Files:** `finstack-py/src/bindings/valuations/factor_model/*.rs`, `__init__.pyi`, `parity_contract.toml`, tests.
- **Verification:**
  - `mise run all-fmt`
  - `mise run python-build` (rebuild bindings before pytest)
  - `mise run python-lint`
  - `uv run pytest finstack-py/tests/test_valuations_new_bindings.py -q`
  - `uv run pytest finstack-py/tests/test_core_parity.py -q`
- **Reviewer focus:**
  - All business logic in Rust — bindings only convert JSON, construct wrappers, map errors via `core_to_py()`.
  - Wrapper pattern: `pub(crate) inner: RustType` + `from_inner()`.
  - `__all__` lists every new symbol.
  - `parity_contract.toml` has an entry for every public type and function.
  - `VolHorizon::Custom` is **not** exposed (Rust-only; closures don't cross FFI cleanly).

### PR 11 — WASM Bindings and TypeScript Facade

- **Goal:** Mirror Python surface in WASM with `camelCase` and a hand-written TS facade.
- **Dispatches:** 1.
- **Model:** Sonnet.
- **Design spec sections to bundle:** §9.3, §9.5.
- **PR plan section to paste:** "PR 11" entire block.
- **Files:** `finstack-wasm/src/api/valuations/factor_model.rs`, `finstack-wasm/exports/valuations.js`, `finstack-wasm/index.d.ts`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run wasm-build`
  - `mise run wasm-lint`
  - `npm --prefix finstack-wasm run test`
- **Reviewer focus:**
  - Every export uses `#[wasm_bindgen(js_name = ...)]` — names match §9.5 row exactly.
  - Inputs/outputs are JSON strings where wrapper ownership would add complexity.
  - `index.d.ts` and `exports/valuations.js` match generated wasm-bindgen names.
  - `VolHorizon::Custom` not exposed (same as PR 10).
  - All exports flow through `exports/valuations.js` facade — no raw `pkg/` imports.

### PR 12 — Notebook, Benchmarks, Final Hardening

- **Goal:** End-to-end synthetic notebook + perf benches + final compatibility sweep.
- **Dispatches:** 1.
- **Model:** Sonnet (mostly assembly).
- **Design spec sections to bundle:** §9.6, §10.6.
- **PR plan section to paste:** "PR 12" entire block.
- **Files:** `finstack-py/examples/notebooks/05_portfolio_and_scenarios/credit_factor_hierarchy.ipynb`, `finstack/valuations/benches/*.rs`, `CHANGELOG.md`.
- **Verification:**
  - `mise run all-fmt`
  - `mise run all-lint`
  - `uv run pytest finstack-py/tests/test_run_all_notebooks.py -q`
  - Targeted Rust tests from PRs 1–8
  - Binding smoke tests from PRs 10–11
- **Reviewer focus:**
  - Notebook outputs are deterministic (set seeds in synthetic data).
  - Notebook runs in CI under reasonable wall time.
  - Old-payload compatibility sweep: load a pre-feature attribution JSON and a pre-feature `RiskDecomposition` JSON, confirm both still deserialize.
  - No-model fallback sweep: run all four attribution methods on a sample portfolio with no model — totals match pre-feature totals.
  - Changelog finalized with shipped API list and explicit v2 deferred-items list.

---

## 6. Final End-of-Stack Review

After PR 12's two-stage review passes, dispatch one more reviewer over the **entire commit stack**:

- **Subagent type:** `superpowers:code-reviewer` (or `quant-code-reviewer:quant-audit` for the math-heavy PRs 3/4/5/6/8 — see notes below).
- **Scope:** All 12 PR commits as a single diff against `master`.
- **Focus:**
  - Cross-PR drift — did later PRs accidentally undo invariants from earlier PRs?
  - Reconciliation invariants still hold across the full feature.
  - Naming triplet still aligned (Rust ↔ Python ↔ WASM).
  - No new general utilities crept in.
  - No file became a god-module across multiple PRs.

Optionally split into two final reviews:

1. `quant-code-reviewer:quant-audit` over the math (PRs 3, 4, 5a, 5b, 6, 7, 8a, 8b).
2. `superpowers:code-reviewer` over the surface (PRs 1, 2, 9, 10, 11, 12).

---

## 7. Stop Conditions / Escalate to Human

Pause execution and surface to the user when:

- **Any implementer subagent returns `BLOCKED` after one model upgrade.** The plan itself may be wrong.
- **Any spec reviewer finds an extra-feature flag** (implementer added something not in the PR plan) and the implementer pushes back on removing it.
- **PR 2 implementer wants to add `MatchingConfig::CreditHierarchical`** rather than extending existing `Hierarchical`. The user should approve the parallel-pathway risk before this happens.
- **Old serialized payloads fail to deserialize** at any reviewer checkpoint. This is a hard non-breaking violation.
- **A reconciliation invariant test fails at the boundary tolerance** (1e-10 for decomposition, 1e-8 for attribution). Do not loosen tolerance to make it green.
- **Performance benches in PR 5 or 12 miss the §10.6 targets by >2x.** Surface measured numbers; the targets in the design spec are "to be confirmed during implementation," so a real number that misses by a small margin is fine — only escalate on large misses.

---

## 8. Deferred V2 Items (do not let scope creep in)

From the design spec §11 and the PR plan's deferred list. Reviewers should flag any of these if a PR tries to ship them:

- Term-structure level factors.
- PCA-derived generic factor.
- Multivariate or DCC GARCH.
- Online covariance updating.
- Joint loadings calibration with regularization.
- Ledoit-Wolf covariance shrinkage.
- General `core::math::regression`.
- FRTB or regulatory adapters.
- Per-issuer adder term structure.
- Stress-scenario consistency across methods.

If a subagent reports it added one of these "for completeness," that's a spec-compliance ❌ — instruct it to remove and re-review.

---

## 9. Quick-Reference Dispatch Checklist

Before every implementer dispatch:

- [ ] Branch is current worktree branch; no uncommitted changes from prior PR.
- [ ] Prior PR's commit SHA recorded for the spec reviewer's `BASE_SHA`.
- [ ] PR plan section copy-pasted into prompt (no file path indirection).
- [ ] Design spec section(s) per §5 above copy-pasted into prompt.
- [ ] Worktree path included.
- [ ] Verification commands included.
- [ ] Cross-PR invariants (§4) included.
- [ ] Model selected per §5.

After implementer returns DONE:

- [ ] Commit exists; SHA recorded.
- [ ] Verification commands passed locally (or implementer reports they passed).
- [ ] Spec reviewer dispatched with PR plan section + implementer report + commit SHA.

After spec reviewer returns ✅:

- [ ] Code-quality reviewer dispatched with `BASE_SHA` and `HEAD_SHA`.

After code-quality reviewer returns ✅:

- [ ] TodoWrite items for this PR all marked complete.
- [ ] Move to next PR.
