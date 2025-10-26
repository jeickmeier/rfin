# Rust Package Code-Quality Review

## 0) Context (fill this in)

* **Crate(s):**
* **Scope of review:** (PR / module / entire workspace)
* **Edition / MSRV:** (e.g., 2021 / 1.74+)
* **Target(s):** (bin/lib, `no_std`, WASM, PyO3, etc.)
* **Key features/flags:**
* **Intended users:** (library consumers vs. app only)

---

## 1) Executive Summary (2–4 bullets)

* **Overall quality:**
* **Top risks:**
* **Quick wins:**
* **Recommended follow-ups (ranked):**

---

## 2) Quick Triage (10–15 minutes max)

### Build, Lints, Tests

```bash
cargo fetch
cargo check --all-targets
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

* No warnings, tests pass, no flaky tests.
* Review CI: same commands enforced on PRs (plus MSRV matrix).

### Formatting & Editions

```bash
cargo fmt --all -- --check
```

* Consistent formatting; `Cargo.toml` uses correct `edition = "2021"` or `2024` if applicable.

### Dependency Hygiene

```bash
cargo deny check               # licenses, advisories, bans, duplicates
cargo outdated -wR             # stale deps
cargo machete                  # unused deps in Cargo.toml
cargo udeps -all-targets       # unused deps in code
```

* No yanked/vulnerable crates; minimal duplicates; license policy is satisfied.

### Binary Size/Features (if lib)

* Default feature set is minimal; optional features are additive and documented.

---

## 3) Deep Dive Checklist

### A) Public API & SemVer

* **Stability:** Public types/functions intentional; sealed traits if needed.
* **Discoverability:** Clear module boundaries; logical names; re-exports for common paths.
* **SemVer discipline:** No breaking changes without major bump; MSRV pinned & documented.
* **Docs:** Every public item has doc comments, examples compile with `doc = true`.

### B) Error Handling

* Prefer typed errors (thiserror/eyre as appropriate).
* Include context via `anyhow::Context`/custom variants; never lose source error.
* **Guideline:** recoverable → `Result<T, E>`; programmer errors → `panic!` only when truly invariant-violating.
* No `unwrap`/`expect` in library code except in safe invariants with comments.

### C) Concurrency, Mutability, and Safety

* Concurrency primitives chosen for need (e.g., `Arc<Mutex<T>>` vs `RwLock`, atomics).
* No data races; `Send`/`Sync` bounds justified; avoid global mutable state.
* `unsafe` blocks minimized, justified, and documented with invariants & tests.
* Use **loom** for modeling concurrency where relevant.

### D) Performance & Allocations

* Hot paths avoid unnecessary clones/allocations; iterators over `Vec` building when possible.
* Zero-copy parsing/slicing where safe (`&[u8]`, `Cow`, `Bytes`).
* Benchmarks exist (`cargo bench` / criterion) with clear baseline & variance.
* Avoid needless `to_string()` / `format!` in inner loops.

### E) Correctness & Testing

* Unit tests for edge cases; property tests for invariants (proptest/quickcheck).
* Fuzz targets for parsers/decoders (`cargo fuzz`).
* Snapshot tests stable and well-scoped (insta).
* Coverage checked with llvm-cov or tarpaulin; critical modules ≥ 80% (quality > quantity).

### F) Security

* `cargo audit` clean; secrets not committed.
* Input validation for public/parsing APIs; length checks to prevent DoS.
* If deserializing untrusted data: deny unknown fields, cap sizes, fuzz + sanitizers.
* Avoid time-based side channels for sensitive data; use `subtle`/constant-time ops if needed.

### G) Packaging & CI/CD

* `Cargo.toml` metadata complete: `description`, `license`/`license-file`, `repository`, `documentation`, `readme`, `keywords`, `categories`.
* CI matrix: stable + MSRV + (optional) nightly; Linux/macOS/Windows as needed; feature matrix.
* Cache effectiveness; minimal build times; integration tests run in CI.
* Release automation: `cargo-release` or `git-cliff` for changelogs; tags = crate versions.

### H) Cross-Targets (only if relevant)

* **no_std:** feature-gated `std`; alloc checks; panics handled; `spin`/`critical-section` patterns.
* **WASM:** no `std::fs`, timers gated; `wasm-bindgen` or `wasm32-unknown-unknown` tested.
* **PyO3:** Python ref-count invariants respected; GIL handling; rich `#[pymethods]` docs & type hints.
* **FFI/C ABI:** `repr(C)`, stability of layouts, ownership of buffers explicit, safety docs.

---

## 4) Automated Commands (copy/paste block)

```bash
# Lints & style
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Tests (fast + deterministic order)
cargo nextest run --workspace --all-features

# Coverage (LLVM preferred)
cargo llvm-cov clean --workspace
cargo llvm-cov test --workspace --lcov --output-path lcov.info

# Security & deps
cargo deny check
cargo audit
cargo outdated -wR
cargo machete
cargo +nightly udeps --all-targets

# Fuzz
cargo fuzz run <target> -- -max_total_time=60

# Concurrency modeling
cargo test --features loom -- --nocapture
```

---

## 5) Structured Findings (use this table format)

| ID   | Area           | Severity | Finding                                                                 | Evidence              | Recommendation                                            | Effort |
| ---- | -------------- | -------- | ----------------------------------------------------------------------- | --------------------- | --------------------------------------------------------- | ------ |
| F-01 | Error Handling | High     | Public API returns `String` errors in `foo::parse`                      | `src/foo/parse.rs:88` | Introduce `thiserror` enum; preserve source via `#[from]` | M      |
| F-02 | Performance    | Medium   | Hot loop clones `Vec<u8>` each iteration                                | `bar/mod.rs:142`      | Iterate over slices; use `chunks_exact`; benchmark diff   | S      |
| F-03 | API            | Medium   | Breaking rename of `Config::timeout_ms` → `timeout` without deprecation | PR #214               | Add deprecated alias; bump major or gate via feature      | S      |
| F-04 | Safety         | High     | `unsafe` slice cast lacks length alignment check                        | `mem/cast.rs:37`      | Add precondition check + unit test; document invariants   | M      |
| F-05 | Docs           | Low      | Missing examples for `ClientBuilder`                                    | `lib.rs`              | Add `///` example; compile-tested doctest                 | S      |

**Severity rubric:** High (correctness/safety/security) • Medium (API/perf/foot-guns) • Low (style/docs/nits)
**Effort:** S (≤1h) • M (≤1d) • L (>1d)

---

## 6) Reviewer Playbook (what to look for, quickly)

* **Unwraps in library code?** Replace with typed errors or `expect` + invariant comment.
* **Excessive trait bounds?** Consider blanket impls or helper traits.
* **Feature creep?** Disable heavy deps by default; doc feature matrices.
* **Allocations in hot paths?** `#[inline]` judiciously; avoid `to_owned()` unless needed.
* **Iterator ergonomics:** Prefer iterators/combinators over index loops when clearer.
* **Borrow vs own:** Accept `impl AsRef<[u8]>`/`impl Into<String>` for edges; return borrowed where possible.
* **Panics in public API?** Replace with `Result` unless invariant break.
* **Docs:** First screen of `docs.rs` explains value prop, quickstart, and examples compile.

---

## 7) Suggested CI (GitHub Actions snippet)

```yaml
name: CI
on:
  pull_request:
  push:
    branches: [ main ]

jobs:
  build-test:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        toolchain: [ stable, 1.74.0 ]   # MSRV example
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with: { toolchain: ${{ matrix.toolchain }}, components: clippy,rustfmt }
      - run: cargo fmt --all -- --check
      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --workspace --all-features
      - run: cargo deny check
```

Optionally add jobs for `llvm-cov`, `cargo outdated`, `wasm32` build, and OS matrix.

---

## 8) Documentation & Publishing Quality

* **README:** 60-second overview, minimal example, feature flags, MSRV, safety notes.
* **Changelog:** human-readable; breaking changes highlighted.
* **Docs.rs badges:** MSRV, CI, coverage (codecov/coveralls/llvm-cov).
* **Examples folder:** runnable examples (`cargo run --example …`) that mirror README.

---

## 9) Follow-Up Plan (ranked)

1. (High) [e.g., Replace ad-hoc errors with `thiserror`, add integration tests for I/O failures.]
2. (High) [e.g., Remove unnecessary `unsafe` or document invariants + tests.]
3. (Medium) [e.g., Introduce feature flags to trim default deps by 40%.]
4. (Medium) [e.g., Add criterion benchmarks + regression gate in CI.]
5. (Low) [e.g., Fill missing docs + doctests for public builders.]
