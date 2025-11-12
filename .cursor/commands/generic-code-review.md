Here’s a reusable, fill-in-the-blanks prompt you can paste into your reviewer (human or LLM). It’s domain-agnostic and pushes for Rust-specific rigor in safety, correctness, performance, and API design.

---

# 🦀 Comprehensive Rust Code Review Prompt (Generic)

**Role**
You are a senior Rust engineer and code reviewer. Evaluate the codebase for **correctness, safety, concurrency, performance, API/UX, documentation, testing, security, and maintainability**. Favor **concrete, actionable** feedback with precise locations and minimal, reviewable diffs.

## Project Inputs (fill these)

* **Repo / path / artifact:** `{{ repo_or_zip_or_paths }}`
* **Rust edition & MSRV:** `{{ 2021 | 2024; MSRV=x.y }}`
* **Targets / runtime:** `{{ OS/arch; std or no_std; sync or async (Tokio/async-std) }}`
* **Build features:** `{{ feature flags, default features, optional deps }}`
* **Intended use:** `{{ library/binary/service/CLI; performance or correctness priorities }}`
* **Non-functional goals:** `{{ latency/throughput targets; determinism; memory limits }}`
* **Safety policy:** `{{ allow/forbid unsafe; FFI present? }}`
* **Versioning:** `{{ SemVer policy; public API stability expectations }}`

---

## What to Deliver

1. **Executive Summary (≤10 bullets)**

   * Top risks & wins across correctness, safety (incl. `unsafe`/FFI), concurrency, performance, API/UX, testing, docs, security, and ops/tooling.
   * One-week “quick wins” vs. multi-week refactors.

2. **Risk-Ordered Findings Table**
   Provide a table with: **ID · Severity(🔴/🟠/🟡/🟢) · File:Line · Issue · Why it matters · Fix (short) · Evidence/Refs**.

   * Use exact paths/lines and include brief code excerpts.
   * Show minimal diffs for each fix when helpful.

3. **Category Scores (0–5)**
   Score and justify each:

   * Correctness & Numerics · Memory Safety · Concurrency/Async · API Design/Ergonomics
   * Performance & Allocation · Readability · Testing Coverage & Quality
   * Documentation & Examples · Security & Supply Chain · Tooling/CI/CD & Observability

4. **Detailed Review Sections (Rust-Focused)**

   * **Correctness & Error Handling**

     * Misuse of `unwrap`/`expect`, panics in library code, error taxonomies (`thiserror` vs `anyhow`), sentinel values, error contexts (`.with_context()`), boundary conditions, arithmetic over/underflow, integer/time/float pitfalls.
   * **Ownership, Borrowing, and Lifetimes**

     * Unnecessary clones, hidden allocations, `Cow`, `Arc`/`Rc` vs references, interior mutability (`RefCell`, `Mutex`, `RwLock`), drop order & resource lifetimes, iterator invalidation.
   * **Concurrency & Async**

     * Blocking in async contexts, task leaks/cancellation, `select!` usage, timeouts/retries/backoff, lock granularity & deadlock risk, `Send`/`Sync` correctness, atomics & memory ordering, priority inversion, bounded queues.
   * **Unsafe & FFI (if any)**

     * Safety invariants documented near `unsafe` blocks; `repr(C)` and layout guarantees; aliasing, lifetimes, and pinning; UB vectors (dangling, uninit, out-of-bounds); unwind across FFI; thread-affinity; `transmute` alternatives; validation of foreign inputs.
   * **API Design & Public Surface**

     * Cohesion, minimal surface, sealed traits, visibility (`pub(crate)`), `Result` vs panic, builder patterns, `From`/`TryFrom`, `AsRef`/`Borrow`, `IntoIterator`, iterator ergonomics, feature-gating optional deps, `no_std` boundaries, prelude strategy, `docs.rs` metadata.
   * **Performance & Allocation**

     * Hot paths; algorithmic complexity; avoid `format!` in loops; `String` vs `&str`, `Vec` growth patterns, small-buffer optimizations, `SmallVec`/`arrayvec` suitability, zero-copy parsing, `serde` features, streaming I/O, cache locality, specialization/`#[inline]` (measured).
   * **Testing & Verification**

     * Unit, integration, doc tests; property-based tests (`proptest`); fuzzing (`cargo-fuzz`); deterministic seeds; snapshot tests; concurrency stress tests; test data realism; coverage (`cargo-llvm-cov`); `miri` UB checks; sanitizers.
   * **Documentation & Examples**

     * `rustdoc` completeness, doctest validity, examples that compile; README, architecture notes, safety & concurrency docs, migration guides, changelog.
   * **Tooling, CI/CD, and Lints**

     * `rustfmt` config, Clippy (pedantic/category denies), MSRV enforcement, `cargo-deny`/`cargo-audit`, `cargo-outdated`, reproducible builds, advisory DB checks, CI matrices and caching, `RUSTFLAGS` hardening, `docs.rs` build.
   * **Security & Supply Chain**

     * RUSTSEC advisories, `unsafe` usage policy, deserialization limits/guards, path and env handling, tempfiles, secrets in logs, cryptography best practices, TLS configuration, sandboxing, RBAC for ops tools.
   * **Packaging & Distribution**

     * Crate metadata, categories/keywords, example binaries, feature defaults minimal, SemVer adherence, MSRV policy in docs, license files, `build.rs` determinism.

5. **Proposed Patches (Minimal Diffs)**
   For high-impact items, provide unified diffs with minimal scope:

   ```diff
   diff --git a/src/foo.rs b/src/foo.rs
   @@
   - let data = data.clone();
   + let data = &data; // avoid allocation; borrow is sufficient
   ```

6. **Test Plan Additions**

   * Enumerate missing tests as a matrix: **Case | Why | Files Touched | Type (unit/integration/prop/fuzz) | Pass Criteria**.

7. **Roadmap & Risk Mitigation**

   * A short, ordered list of PRs (1–N), each with scope, owners, effort (S/M/L), and measurable acceptance criteria.

---

## Severity Rubric

* **🔴 Critical:** UB/unsoundness, memory safety, data races, privacy/key leaks, deterministic logic errors, panics in library APIs, FFI contract violations.
* **🟠 Major:** API foot-guns, blocking in async, deadlock hazards, significant perf regressions, incorrect error mapping, flaky or missing tests for critical paths.
* **🟡 Minor:** Readability, docs gaps, suboptimal patterns that don’t affect correctness.
* **🟢 Nit:** Style and micro-optimizations backed by measurements.

---

## Reviewer Method & Constraints

* **Cite precise locations**: `path/to/file.rs:L123-L145`. Include short code excerpts.
* **Prefer minimal changes** over sweeping rewrites. If a larger refactor is warranted, outline the migration steps and surface area affected.
* **No speculative claims**—ground findings in code, Rust language rules, and standard ecosystem practices.
* **Measure before optimizing**: call out where benchmarks are needed; recommend `criterion` for microbenchmarks.
* **Library vs Binary**: libraries must avoid panicking for recoverable errors; binaries may `expect` with clear messages.
* **Feature flags**: verify non-default features build & test; ensure feature combinations compile.

---

## Rust-Specific Checklist (Quick Scan)

* **Ownership & Borrows:** avoid needless `clone()`; prefer `&T`/`&mut T`; use `Cow` when applicable; correct lifetimes; no borrow of temporaries that outlive owners.
* **Errors:** use domain errors with `thiserror`; bubble context with `anyhow` only at app boundaries; no `unwrap`/`expect` in library code; map external errors faithfully.
* **Async:** never block in async; timeouts and cancellation respected; no shared mutable state without guards; structured concurrency (`JoinSet`, `select!`), graceful shutdown.
* **Locks & Atomics:** avoid coarse locks; prefer lock-free or sharded where appropriate; document memory ordering; no deadlock patterns (`lock()` while holding other locks).
* **Unsafe/FFI:** invariants commented at the `unsafe` site; `repr(C)` for FFI structs; no unwinding across FFI; validate inputs from foreign code; avoid `transmute`.
* **Performance:** avoid `format!`/`to_string()` in hot loops; preallocate (`with_capacity`); streaming deserialization; iterator fusion; consider `SmallVec`/`tinyvec` where measured beneficial.
* **API Design:** small, composable traits; avoid trait object leakage when generics work; sealed traits for extensibility control; minimal `pub` surface; consistent naming.
* **Docs & Examples:** every public item has `rustdoc`; examples compile; safety sections for `unsafe`; `# Panics`/`# Errors`/`# Safety` blocks where applicable.
* **Tooling:** `clippy --all-targets -D warnings` (or policy-appropriate denies); `rustfmt`; `cargo-deny`/`audit`; MSRV pinned; CI runs tests with all feature combos; docs build on `docs.rs`.

---

## Commands & Tools to Recommend (non-binding)

* Lints: `cargo clippy --all-targets --all-features -D warnings`
* Coverage: `cargo llvm-cov --ignore-filename-regex '(.cargo|tests?/fixtures)'`
* UB/Undefined behavior checks: `cargo +nightly miri test`
* Fuzzing: `cargo fuzz run fuzz_target_1`
* Audit: `cargo deny check` and `cargo audit`
* Benchmarks: `cargo bench` with `criterion`

---

## Output Format Template (copy for your response)

1. **Executive Summary**

* …

2. **Findings Table**
   | ID | Sev | File:Lines | Issue | Why it matters | Fix (short) | Refs |
   |---|---|---|---|---|---|---|
   | F-001 | 🔴 | `src/foo.rs:L120-L138` | … | … | … | … |

3. **Category Scores & Justification**

* Correctness: 3/5 — …
* Memory Safety: 4/5 — …
* …

4. **Detailed Review**

* Correctness & Error Handling: …
* Ownership & Lifetimes: …
* Concurrency & Async: …
* Unsafe & FFI: …
* API Design: …
* Performance: …
* Testing: …
* Docs: …
* Tooling/CI: …
* Security: …
* Packaging: …

5. **Proposed Patches (Diffs)**

```diff
…minimal diffs…
```

6. **Test Plan Additions (Matrix)**
   | Case | Why | Files | Type | Pass Criteria |
   |---|---|---|---|---|
   | … | … | … | unit | … |

7. **Roadmap (PR Order)**

1) … (owner, effort S/M/L, acceptance criteria)
2) …

---

**Notes**

* If evidence is ambiguous, state assumptions and suggest a targeted measurement or test.
* If recommending a dependency, justify the choice (maturity/MSRV/maintenance/cargo-deny status).
* Flag any **breaking changes** and outline a SemVer-compliant migration plan.
