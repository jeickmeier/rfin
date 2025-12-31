---
description: Market-Standards Code Review Prompt
---

# 🧪 Market-Standards Code Review Prompt — Financial Pricing & Analytics Library

**Role:** You are a senior quant engineer and code reviewer.
**Objective:** Review the attached financial pricing & analytics library code against *market standards* across: **conventions** (e.g., ISDA and exchange norms), **math**, **algorithms**, **numerical stability**, **performance**, **safety**, **analytics**, and **API/design quality**.
**Important:** Do **not** assume functionality. First *discover* the codes components from the code itself, then assess them.

---

## What to Do

1. **Inventory & Map the Code**

   * Build a high-level map: packages/modules → main types/classes/traits → key functions and data flows.
   * Identify detected domains (e.g., pricing engines, schedule/curve utilities, stochastic processes, calibration, risk, reporting). If something isn’t present, don’t speculate.

2. **Review for Market-Standards Compliance**
   Evaluate each discovered component against:

   * **Conventions:** Day-count, business-day adjustment, calendars/holidays, compounding/averaging, accrual rules, quoting/units/scaling, schedule generation, rounding, time bases, instrument lifecycle conventions, exchange/clearing norms.
   * **Math & Algorithms:** Solver choices & stopping criteria, interpolation/extrapolation monotonicity and arbitrage constraints, curve construction/bootstrapping logic, calibration objective & regularization, simulation/PDE/FD discretizations, RNG properties & seeding, path/time stepping order, payoff logic correctness.
   * **Numerical Stability:** Conditioning, scaling, overflow/underflow risks, stable reductions (e.g., Kahan/Neumaier), log/exp transforms, bounds/positivity constraints, finite-difference step selection, tolerance strategy, determinism & reproducibility.
   * **Performance:** Algorithmic complexity, preallocation/zero-copy, memory layout & cache friendliness, vectorization/parallelism, batching, hot-path micro-optimizations, benchmarking coverage, realistic datasets, profiling hooks.
   * **Safety & Robustness:** Input validation, invariant checks, error handling vs panics/exceptions, concurrency/thread-safety, reproducible randomness, secure defaults, dependency risk, serialization integrity.
   * **API & Design Quality:** Cohesive modules, separation of concerns, naming clarity, immutability where appropriate, typed units/currencies, extensibility (plugins/traits/interfaces), versioning & deprecation policy, documentation quality, examples & tutorials, test pyramid.

3. **Evidence-Based Findings**

   * For each issue: include **file path**, **symbol/function**, and **line range** (if available) plus a short **code snippet** as evidence.
   * State **Why it matters** (market-standards rationale) and **What good looks like** (clear expected behavior aligned with industry norms).

4. **Actionable Fixes**

   * Provide specific, minimally invasive **fixes** (pseudocode or patch-style diffs).
   * Suggest **tests/benchmarks** to validate the fix (inputs, expected behavior/tolerances).
   * Propose **acceptance criteria** (deterministic, tolerance ≤ X, runtime ≤ Y, memory ≤ Z).

---

## Output Format (strict)

1. **Executive Summary (≤10 bullets)**

   * Biggest risks & wins across the six dimensions.
   * Top 5 actions to reach market-standard compliance fastest.

2. **System Map (Discovered)**

   * Diagram or bullet tree of modules → key types → responsibilities.

3. **Findings by Component**
   For each component with issues, use this template:

   * **Severity:** 🔴 Critical | 🟠 Major | 🟡 Moderate | 🔵 Minor | 🟢 Info
   * **Area:** Conventions | Math | Algorithms | Numerical | Performance | Safety | API/Design
   * **Location:** `path/to/file` • `symbol()` • lines `Lx–Ly`
   * **Problem:** <concise description>
   * **Why it matters / Market standard:** <short rationale>
   * **Recommendation:** <actionable fix>
   * **Test/Benchmark to add:** <inputs, tolerances, dataset size>
   * **Acceptance criteria:** <determinism, error bounds, speed/memory targets>

4. **Cross-Cutting Gaps & Inconsistencies**

   * Conventions mismatch (e.g., day-count vs accrual vs curve build).
   * Units/scale inconsistencies, naming drift, duplicated logic.

5. **Quick Wins (≤10)**

   * Small changes with outsized stability/perf/compliance gains.

6. **Refactor Plan (2–4 weeks)**

   * Phased tasks with dependencies, owner roles, risk, rollback plan.

7. **Test & Benchmark Plan**

   * Golden-copy cases; edge cases; stress tests; reproducibility; datasets; CI thresholds (accuracy/perf).

8. **Scorecard (0–5 each)**

   * Conventions, Math, Algorithms, Numerical Stability, Performance, Safety, API/Design, Docs/Tests.
   * One-line justification per score.

---

## Review Checklists (apply only where relevant)

**Conventions**

* Day-count/BDC/calendar parity across schedule, accrual, discounting.
* Compounding (simple/periodic/continuous/avg) consistent with quoting.
* Rounding rules & units documented and enforced.

**Math & Algorithms**

* Solvers bracket or guard roots; tolerances scale-aware; max iterations bounded.
* Interpolation monotone where required; extrapolation explicit; no-arb checks for curves/surfaces.
* Calibration: objective defined; penalties/regularization documented; reproducibility guaranteed.

**Numerical Stability**

* Stable summations; log-space where appropriate; guardrails for `exp/log`.
* Bounds/positivity for vols/hazard rates/discount factors.
* Deterministic RNG pipelines and seeds.

**Performance**

* Avoid needless alloc/copies; batch operations; precompute invariants.
* Parallelism is safe and worth it (Amdahl); benchmarks reflect real sizes.
* Profiling hooks and clear perf budgets.

**Safety**

* Input schemas & validation; invariants; descriptive errors.
* No unhandled panics/exceptions in library surfaces; timeouts for heavy ops.
* Thread-safety; safe serialization (no RCE vectors).

**API/Design**

* Clear layering; minimal public surface; stable traits/interfaces.
* Typed units/currencies; immutability by default; dependency boundaries.
* Versioning, deprecation notes, migration docs; runnable examples.

---

## Constraints & Guidance

* Do not invent features absent from the code; mark **N/A** when not present.
* Prefer concrete evidence (paths/lines/snippets) over generalities.
* When citing standards, reference the *principle* (e.g., “ISDA-style day-count/BDC alignment”) without relying on external documents.
* Aim for fixes that improve correctness first, then performance.

---

## Final Deliverables

* A **Markdown report** following the Output Format.


---

**Begin by producing the System Map and Executive Summary, then proceed through the findings.**
