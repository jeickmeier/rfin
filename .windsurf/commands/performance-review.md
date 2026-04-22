```md
# AGENT.md — Performance Standards Code Review (Financial Pricing Library)

## Purpose

This guide defines how to review **performance standards** for a **financial pricing library** while preserving a second core objective: **avoid over-engineering**.
Performance matters (latency, throughput, determinism, scaling), but the default posture is **simple first**: optimize only where it’s measurable and meaningful.

## Scope

This review focuses on:
- Algorithmic complexity and scaling behavior
- Hot paths and allocation patterns
- Numerical performance (vectorization opportunities, stable math)
- Concurrency and parallelism safety
- I/O and serialization costs (if applicable)
- API design decisions that impact performance (without expanding surface area unnecessarily)

Out of scope (unless they directly affect performance):
- New features
- Large refactors that change architecture without measurements
- Style-only changes
- Extensive rewrites “for speed” without evidence

## Review Mindset

### Performance Principles
1. **Measure > Assume**
   - Don’t accept “this is faster” without a benchmark, profile, or clear complexity win.
2. **Optimize the 5%**
   - Identify the real hot paths (pricing loop, curve bootstrapping, calibration, scenario evaluation).
3. **Keep correctness and determinism**
   - Especially in finance: reproducibility matters as much as speed.
4. **Prefer algorithmic wins over micro-optimizations**
   - O(n²) → O(n log n) beats clever caching tricks.
5. **Avoid performance myths**
   - Don’t add caches, pools, or concurrency “just in case.”

### Anti Over-Engineering Guardrails
- No new abstraction layers unless they:
  - remove duplication **and**
  - maintain readability **and**
  - have a measured or well-reasoned performance impact.
- No speculative parallelism or memoization without:
  - a clear workload model
  - correctness constraints (thread safety, determinism)
  - benchmarks showing a win.

## What “Good Performance” Means Here

A performance-healthy pricing library generally exhibits:
- **Predictable scaling** with instrument count, curve nodes, time steps, scenarios
- **Low allocation rates** in tight loops
- **Stable numerical routines** with appropriate tolerances
- **Clear separation** between:
  - “setup / precompute” work
  - “per-price / per-scenario” work
- **Benchmarks** that reflect real workloads (not toy examples)

## Review Checklist

### 1) Identify Hot Paths & Workloads
- What are the main workflows?
  - single price
  - batch price
  - calibration
  - risk (bump & reprice)
  - scenario grids / Monte Carlo (if present)
- Is there evidence of profiling or benchmark coverage?
- Are there obvious tight loops (per cashflow, per time step, per instrument)?

**Red flags**
- Heavy work inside per-instrument loops that could be moved to setup
- “Hidden” work triggered by getters, formatting, logging, or debug checks

### 2) Complexity & Data-Flow
- Time complexity per major operation:
  - curve build: O(n), O(n²), etc.
  - pricing: O(cashflows), O(steps), etc.
  - calibration: iterations × cost(per iteration)
- Avoid repeated scans, nested loops, repeated sorting, repeated parsing.

**Red flags**
- Recomputing schedules / year fractions repeatedly
- Rebuilding interpolation structures per call instead of reusing

### 3) Allocation & Memory Behavior
- Are allocations occurring in tight loops?
- Are intermediate containers created repeatedly?
- Is data layout friendly for batch processing?

**Prefer**
- preallocation when sizes are known
- reusing buffers in internal hot paths (carefully)
- streaming where appropriate

**Red flags**
- cloning large structures unnecessarily
- building strings/log objects in hot paths
- converting between representations repeatedly (e.g., list → map → list)

### 4) Numerical Performance & Stability
- Are expensive math functions used in loops (exp/log/pow/sqrt)?
- Are there avoidable transcendental calls?
- Are tolerances explicit and consistent?
- Are there unstable formulations (catastrophic cancellation) that cause extra iterations?

**Prefer**
- stable formulas and consistent tolerance strategy
- compute once, reuse (discount factors, accrual factors, fixed schedule terms)

### 5) Concurrency & Parallelism
- Is parallelism used where it helps (large batch pricing)?
- Does it preserve determinism and reproducibility?
- Is contention introduced (locks, shared mutable caches)?

**Red flags**
- global caches with locks
- nondeterministic iteration ordering affecting floats
- parallelism for small workloads where overhead dominates

### 6) API & Abstractions Impacting Performance
- Is the API encouraging efficient usage patterns?
  - batch functions available?
  - ability to precompute and reuse curves/schedules?
- Are there “convenience” APIs that hide big costs?
- Are types too dynamic (stringly-typed, runtime dispatch) in hot paths?

**Avoid**
- adding complex builder hierarchies
- adding “smart caches” that complicate correctness

### 7) Benchmarks, Profiling, and Regression Safety
- Are there benchmarks for:
  - representative instruments
  - realistic curve sizes
  - realistic scenario counts
- Do benchmarks include warmup / multiple runs?
- Are there performance regression tests (at least smoke-level)?

**Minimum expectation**
- A small set of stable micro-benchmarks + at least one end-to-end benchmark.

## Required Evidence for Performance Claims

Any PR claiming performance improvement should include at least one:
- benchmark before/after (preferred)
- profile capture summary (acceptable)
- complexity analysis with justification (sometimes acceptable)

And must specify:
- workload scenario
- input sizes
- environment notes (CPU, build flags, runtime config)

## Output Expectations for the Reviewer

Your review should include:

### A) Performance Inventory (What you found)
- top hotspots (suspected or measured)
- allocation-heavy areas
- scaling risks
- hidden costs (conversions, repeated precompute)

### B) Recommendations (Prioritized)
Each recommendation must include:
- **impact** (high/med/low)
- **effort** (small/med/large)
- **risk** (correctness / determinism / maintainability)
- **evidence needed** (benchmark/profile)

Use this priority order:
1. algorithmic improvements
2. eliminate repeated work
3. reduce allocations
4. micro-optimizations last

### C) Non-Recommendations (What NOT to do)
Explicitly call out optimizations you reject as over-engineering, and why:
- unclear win
- adds complexity
- correctness risk
- no benchmark evidence

## Standard Review Comments (Copy/Paste)

**Request benchmark**
- “This might be faster, but can you add a benchmark that reflects the expected workload (N instruments, curve nodes, scenarios) and show before/after numbers?”

**Avoid over-engineering**
- “This adds a cache/pool/abstraction that increases complexity. Can we first confirm it’s a hotspot via profiling, and consider a simpler change (precompute/reuse) before adding machinery?”

**Move work out of hot loop**
- “This looks like setup work happening per-price/per-scenario. Can we precompute it once and reuse across calls?”

**Allocation concern**
- “This allocates inside a tight loop. Can we reuse a buffer / preallocate / avoid intermediate containers?”

**Determinism**
- “Parallelization or map iteration order could change floating-point summation ordering. Is output deterministic across runs?”

## Definition of Done (Performance Review)

A performance review is complete when:
- hot paths are identified (measured or clearly argued)
- major scaling risks are documented
- at least minimal benchmarks exist for key workflows
- recommendations are prioritized and avoid unnecessary complexity
- any performance claims in the change set are supported by evidence

## Final Note

This library is a pricing engine: performance matters, but the best performance feature is often **a simpler design** that does less work.
Optimize with data, keep the API ergonomic, and never trade away correctness or determinism.
```
