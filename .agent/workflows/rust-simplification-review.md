---
description: Rust Simplification Review
---

# 🧹 Rust Code Hygiene Review — Duplication, Legacy, Dead & Unnecessary Code

**Role:** You are a senior Rust reviewer focused on **removing duplication, legacy shims, dead code, and unnecessary complexity** without changing public behavior.

**Scope Inputs (fill these):**

* **Repo/Path:** `{{repo_or_path}}`
* **Primary crates/modules:** `{{crates_or_paths}}`
* **Edition & MSRV:** `{{edition}} / {{msrv}}`
* **Target behavior guarantees:** `{{tests_must_pass | API stable?}}`
* **Perf & size constraints (optional):** `{{p99 | binary_kb}}`

---

## What to Deliver (in order)

1. **Executive Summary (≤10 bullets)**: Biggest deletion/refactor wins, estimated LOC removed, risk level.
2. **Findings Table** (CSV-style in Markdown):
   `type | location | evidence | impact | risk | fix | est-LOC | quick-check`

   * `type` ∈ {duplicate, dead, legacy, unnecessary, config-drift, overengineering}
3. **Actionable Diffs/Edits**: Minimal invasive edits (per file) or PR plan with commit grouping.
4. **Safety Net**: Tests that must be added/adjusted to keep behavior stable (list files & test names).
5. **Metrics Before/After (est.)**: LOC saved, build graph simplifications, binary size/compile time notes.
6. **PR Checklist**: Ticked items showing you ran the tools below and verified no observable behavior change.

---

## Review Heuristics & Checks

### A. Duplication (copy/paste & structural)

* Detect near-duplicates in `src/**` and `crates/**` (including test utils).
  Look for: identical/near-identical functions, repeated impl blocks, copy-pasted error enums, duplicated feature-gated versions of same code.
* Prefer **extract-fn/trait**, generic over type param bounds, or local helper modules.
* Flag duplicated business rules hidden in tests/fixtures.

### B. Dead/Unused Code

* Unreachable or **never called** functions, private items, modules, `pub(crate)` not referenced.
* Unused type params/lifetimes, phantom generics, `#[allow(dead_code)]` that can be removed.
* Feature-gated islands: items behind features never enabled in workspace CI or `Cargo.toml`.

### C. Legacy / Drift

* Shims for old APIs, deprecated types, or versioned modules not referenced by current public API.
* Conversions/traits maintained for pre-Rust-2021 idioms (implicit elisions, old error traits).
* `anyhow` plus custom error types both present—standardize.
* Parallel utils replaced by `rayon`, manual arc/mutex patterns replaced by channels/async.

### D. Unnecessary Code / Overengineering

* Redundant `clone()`/`to_owned()`; pass `&T`/`&str` where possible.
* Over-generic traits without multiple impl sites; unnecessary lifetimes on non-ref fields.
* Wrapper newtypes that add no invariants; error enums that mirror upstream.
* Hand-rolled parsing/serde where `serde`/`toml`/`serde_with` suffices.
* Inlined large constants or lookup tables duplicated across crates (move to one module).

### E. Build/Config Bloat

* Unused deps, features, dev-deps; redundant workspace members; example/bin targets never used.
* Overlapping `cfg`s producing multiple identical variants.
* Lints suppressed globally that should be local.

---

## Concrete Commands to Run (and report results)

**Dependency & dead-code sweep**

* `cargo udeps --all-targets --workspace` (report by crate)
* `RUSTFLAGS="-Zunused-deps" cargo +nightly build -Z unstable-options` (if allowed)
* `cargo machete` (compare with udeps)

**Duplicate / size / call-site evidence**

* `cargo llvm-lines` (heavy offenders; list top 20)
* `cargo bloat --release --crates` (binary/code bloat before/after)
* `ripgrep -n "(unsafe|clone\(\)|to_owned\(\)|unwrap\(\))" src` (hotspots)
* `cargo tree -i {{crate}}` (reverse-deps for removable crates)

**Linting / static**

* `cargo clippy --all-targets -- -W clippy::pedantic -W clippy::nursery -W clippy::redundant_clone -W clippy::needless_pass_by_value -W clippy::large_enum_variant`
* Enable and report `dead_code`, `unused_imports`, `unused_mut`, `unreachable_code`.

**Feature & cfg audit**

* `cargo hack check --each-feature --no-dev-deps` (report orphan features)
* `rg -n "#\[cfg\(feature"` + compare with `Cargo.toml` features actually used in CI.

**Build graph & bins**

* `cargo metadata --no-deps -q | jq '.workspace_members | length'`
* `fd . ./crates -td | wc -l` for crates/bins/examples count; mark unused.

> Include tool outputs or summarized excerpts in the Findings Table’s `evidence` field.

---

## Fix Patterns (recommend the minimal viable change)

* **Duplicate logic** → Extract helper fn/trait; prefer internal `mod` over new crate unless reused widely.
* **Dead items** → Delete; if exported, deprecate with `#[deprecated]` then remove.
* **Legacy shims** → Replace call sites; delete shim; ensure re-exports preserve public API if needed.
* **Redundant clones** → Use references; implement `AsRef`/`Borrow` as needed.
* **Over-generic APIs** → Concretize where there is a single use; remove unused type params/lifetimes.
* **Unused deps/features** → Remove and re-lock; document in PR.
* **Error types** → Consolidate on one style (`thiserror` or `anyhow`), not both.

---

## Output Format

### A) Findings Table (Markdown)

```
type | location | evidence | impact | risk | fix | est-LOC | quick-check
duplicate | crates/alpha/src/lib.rs:120-210 | 87% similar to beta::utils::normalize | medium | low | extract normalize() to shared utils | -120 | tests: utils_normalize_ok
dead | crates/beta/src/legacy.rs | never referenced; behind unused feature `legacy` | low | none | delete module + feature | -420 | cargo hack passes
unnecessary | src/foo.rs:33 | to_owned() on &str in hot loop | perf | low | borrow instead; adjust signature | n/a | benches stable
...
```

### B) Patch Plan (Bulleted)

* Commit 1: remove unused features & deps (`udeps`, `hack` outputs).
* Commit 2: dedupe normalize flow across `alpha`/`beta`.
* Commit 3: reduce clones & needless allocations in `foo`.
* Commit 4: delete `legacy.rs` module; update exports.
* Commit 5: tighten clippy lint set; remove global `allow`s.

### C) Optional JSON Summary

```json
{
  "loc_removed_est": 950,
  "findings": [
    {"type":"duplicate","path":"crates/alpha/src/lib.rs","lines":"120-210","impact":"medium","fix":"extract helper"},
    {"type":"dead","path":"crates/beta/src/legacy.rs","impact":"low","fix":"delete"}
  ],
  "tool_runs": {
    "udeps": "OK",
    "clippy": {"new_warnings": 0},
    "hack_each_feature": "OK"
  }
}
```

---

## Acceptance Criteria

* All tests pass; public API behavior unchanged unless noted.
* `cargo udeps` returns **no unused deps**.
* `cargo clippy` with pedantic/nursery has **no new warnings**; removed global `allow`s where feasible.
* **Binary size and/or compile time not worse**; note improvements if achieved.
* Findings & PR plan are **reproducible** from listed commands.

---

## Constraints & Notes

* Prefer deletion and consolidation over adding new crates.
* If a deletion risks behavior change, propose a deprecation path (one minor version).
* Keep diffs small and localized; group high-churn files last.
