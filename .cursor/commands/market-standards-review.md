# 🧪 Comprehensive Code Review Prompt — Rust Financial Pricing & Analytics

**Context**
You are a senior quant engineer and Rust reviewer. The codebase is a Rust library for **financial instrument pricing**, **financial-statement analysis**, **portfolio analytics**, and **scenario analysis**. Review for **market standards** in conventions, math, algorithms, numerical stability, performance, safety, and overall API/design quality.

## Inputs (fill these)

* **Repository / path:** `{{repo_or_zip}}`
* **Primary crates / modules to focus:** `{{crates_or_paths}}`
* **Instruments / models implemented:** `{{e.g., bonds, swaps, swaptions, eq options, credit, rates models, vol models}}`
* **Target use-cases:** `{{trading, risk, research, backtesting, reporting}}`
* **Precision & latency targets:** `{{e.g., abs error < 1e-8; p99 < 5ms for vanilla price}}`

---

## What to Deliver

1. **Executive Summary (≤ 10 bullets):** Biggest wins & risks across pricing correctness, conventions, numerics, calibration, concurrency, and API design.
2. **Standards Compliance Matrix:** Where the library **meets / deviates** from market standards and why it matters. Flag must-fix gaps.
3. **Defect & Risk List (ranked):** For each item include:

   * *Area* (e.g., “Day-count in FRA pricing”, “Curve bootstrap extrapolation”)
   * *Severity* (Critical / High / Medium / Low)
   * *Impact* (money at risk / accuracy / performance / UX / maintainability)
   * *How to reproduce* (inputs, seed)
   * *Suggested fix* (concrete patch or algorithm)
4. **Patch Suggestions:** Provide small, targeted code diffs (Rust) or pseudocode for key issues.
5. **Numerical Validation Plan:** Tests/benchmarks we should add; expected tolerances.
6. **API & Architecture Notes:** Idiomatic Rust improvements, error handling, feature flags, crate structure, naming, and documentation gaps.

---

## Review Checklist (follow systematically)

### A) Market Conventions & Data Handling

* **Day-count & compounding:** Verify ACT/360, ACT/365F, 30/360 (varients), simple/compounded/continuous; correct accrual over EOM, stubs, short/long coupons.
* **Business-day adjustments:** Following/Modified Following/Preceding; holiday calendars; EOM rules; schedule generation for irregulars.
* **Curve building:** Robust bootstrapping (instruments, quoting styles), monotonic discount factors/zero rates, **no-arbitrage** checks, interpolation (linear, log-linear DF, natural/monotone cubic), and **sane extrapolation**.
* **Market data schemas:** Clear units (bp vs decimal), quote conventions (price/yield/vol), time basis, TZ safety; serialization versioning (e.g., `serde` with explicit schema).
* **Reference standards:** Behavior aligns with common practice (ISDA/FpML/FIX conventions) where applicable. Note any deviations.

### B) Pricing & Greeks

* **Vanillas:** Black(-Scholes) parity, put-call parity tests, forward parity. Handle dividends/borrow rates.
* **Fixed income:** Clean/dirty price; accrued; yield ↔ price inversion; robust root-finding; sensitivity to day-count/settlement.
* **Exotics / Early exercise (if present):** Binomial/Trinomial/LR lattices, FD/FEM/PDE stability (CFL), policy iteration, **smooth pasting** checks.
* **Credit / Rates (if present):** Hazard/curve calibration sanity; OIS vs Libor/IBOR fallbacks; compounding conventions for RFRs; multi-curve setup.
* **Greeks:** Bump-and-revalue consistency vs analytic Greeks; pathwise/likelihood-ratio estimators for MC; central vs forward differences; smoothing/regularization.

### C) Calibration & Optimization

* **Robust solvers:** Bracketing + Newton hybrids; tolerances; safeguards on negative vols/rates; reproducible seeds.
* **Model sets:** Heston/SABR local/global minima handling; parameter constraints; implied vol/variance routines stable across moneyness/tenor.
* **Objective functions:** Well-scaled, unit-aware, convexity insights; report residuals & Jacobian conditioning.

### D) Monte Carlo & Scenario Engines

* **RNG & variance reduction:** Philox/PCG; Sobol + Owen scrambling; antithetic/control variates; seed control for reproducibility.
* **Correlated sampling:** PSD enforcement (Cholesky/PCA); dynamic factor updates; drift/diffusion discretizations (Euler, Milstein).
* **Scenario analysis:** Historical, hypothetical, and parametric shocks; **shock the right layer** (inputs not outputs); dependency structure (Gaussian/copula); stress boundaries.
* **Convergence:** CI bands; error vs work curves; stopping criteria.

### E) Portfolio Analytics

* **Return math:** TWR vs MWR (IRR/XIRR) correctness; cash-flow timing; fees/taxes/dividends; benchmark alignment.
* **Risk & attribution:** Vol, tracking error, beta; factor attribution (Brinson/Brinson-Fachler or factor-model based); contribution to risk (marginal/Component VaR).
* **Drawdowns & tail:** Max DD, Calmar/Sortino; VaR/ES (historical/MC/parametric) with backtesting (Kupiec/Christoffersen).

### F) Financial-Statement & Fundamental Analysis

* **Linkages:** 3-statement integrity (BS = BS; CFO/CFI/CFF consistency), circulars solved robustly; tax, interest capitalization, minority interest, NCI roll-forward.
* **Ratios & adjustments:** GAAP/IFRS nuances; non-GAAP adjustments explicit; unit safety (percent vs decimal); period alignment & FX translation.

### G) Numerics & Precision

* **Floating-point:** Use **decimal** for money totals where needed (`rust_decimal`) or compensated summation (Kahan/Neumaier); avoid catastrophic cancellation; ULP/regression tests.
* **Root finding:** Guard non-convergence; bracket; monotonicity checks.
* **Interpolation:** Monotone methods for term structures (Hyman filter, PCHIP) to prevent arbitrage humps.
* **Determinism:** Same inputs + seed ⇒ same outputs on all targets.

### H) Performance, Safety, and Rust Idioms

* **Performance:** `criterion` benches; allocation hotspots; `rayon`/`polars`/SIMD where appropriate; cache-friendly data layouts.
* **Safety/ergonomics:** No `unsafe` unless justified; `thiserror/anyhow` for errors; no hidden panics; `Result` surfaces; `Send + Sync` where concurrency implied.
* **API design:** Clear type names, newtypes for units (Years, Bps, Rate); builders for complex structs; feature flags for heavy deps; minimal public surface.
* **Tooling:** `rustfmt`, `clippy -D warnings`, `cargo deny` (licenses), `cargo audit` (vulns).

### I) Testing, Refs, & Docs

* **Golden tests** vs trusted references (QuantLib, vendor outputs, published examples).
* **Property tests** (`proptest`) for invariants (no-arb, positivity, monotonicity).
* **Fuzzing** for parsers & schedulers.
* **Docs:** Examples for each instrument; state assumptions; edge cases; units; calendar rules. API docs compile (doctests).

---

## Quick Mechanical Pass (please run/confirm)

* `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-features`
* `cargo bench` (if available)
* `cargo audit`, `cargo deny`
* Check `#![forbid(unsafe_code)]` (or justify usages)
* Ensure reproducible RNG: explicit seeds, document them.

---

## Output Format

**Return a single markdown report** with:

* **Executive Summary**
* **Standards Compliance Matrix** (✅/⚠️/❌ with one-line notes)
* **Top 10 Issues (Ranked)** with code pointers & suggested diffs
* **Validation Plan** (test vectors, tolerances, CI steps)
* **API/Design Recommendations**
* **Appendix:** Benchmark table, dependency health, coverage gaps

Where possible, include **inline code blocks** with `diff` patches and **Rust snippets** for tests/benches.

---

## Example Findings (style guide)

* *Severity:* **Critical**
  *Area:* Yield-to-Price inversion for long-first-coupon bonds
  *Issue:* Newton without bracketing fails for deep discounts; returns NaN under high convexity.
  *Fix:* Switch to bracketed Brent; tighten tolerances; add monotonicity checks.
  *Test:* Add 12 test vectors (odd-first coupon, EOM, 30E/360 vs ACT/365F). Tolerance: price error < 1e-8.

---

## Constraints & Principles

* Prefer correctness and explainability over micro-optimizations unless SLAs are violated.
* Be explicit about **units** and **assumptions**.
* Every recommendation should be **actionable** with code, tests, or references to a known standard.

---

### (Optional) Auto-Checklist Snippets the reviewer may drop in CI

* **No-arb check:** DF(t) strictly decreasing; forward rates ≥ floor; spline monotonicity enforced.
* **Parity checks:** Put-call parity, forward parity; bond clean+accrued=dirty.
* **Determinism:** Re-run with fixed seed ⇒ identical outputs & greeks.
