# Analytics Crate Documentation Review

**Date:** 2026-04-25
**Scope:** `finstack/analytics` (Rust canonical), `finstack-py/src/bindings/analytics` + `finstack-py/finstack/analytics` (Python), `finstack-wasm/src/api/analytics` + `finstack-wasm/index.{js,d.ts}` + `finstack-wasm/exports/analytics.js` (WASM)
**Method:** Five parallel deep-read passes (one per surface) plus cross-cutting triplet/parity inspection. Each public item examined for: doc presence, completeness (Args/Returns/Errors/Panics/Examples/References), accuracy vs. implementation, convention surfacing, and cross-surface naming.

---

## Summary

| Surface | Lines | Findings | Critical | High | Medium | Low |
|---|---:|---:|---:|---:|---:|---:|
| Rust core (top-level: `lib`, `aggregation`, `benchmark`, `drawdown`, `lookback`, `performance`, `returns`, README) | ~5,000 | 44 | 2 | 8 | 19 | 15 |
| Rust core (submodules: `risk_metrics/*`, `timeseries/*`, `backtesting/*`) | ~9,400 | 60 | 0 | 12 | 26 | 22 |
| Python bindings (Rust side, `finstack-py/src/bindings/analytics`) | ~3,400 | 78 | 1 | 13 | 31 | 33 |
| Python wrapper (`finstack-py/finstack/analytics/{__init__.py,__init__.pyi}`) | ~2,900 | 23 | 0 | 4 | 9 | 10 |
| WASM bindings (`finstack-wasm/src/api/analytics` + `index.d.ts` + `exports/`) | ~1,400 + 1,626 d.ts | ~30 | 1 | 8 | 13 | ~9 |
| **Total** | **~22,000 + 1,626** | **~235** | **4** | **45** | **98** | **~89** |

### Coverage by surface (high level)

| Surface | Public items inspected | Documented | With full sections (Args/Returns) | With Examples |
|---|---:|---:|---:|---:|
| Rust core (top-level) | ~155 | ~99% | ~65% | ~33% (sparse on `Performance`) |
| Rust core (submodules) | ~120 | ~98% | ~88% | ~25% |
| Python bindings (Rust side) | ~277 (PyO3 items) | ~99% (one-line summaries dominate) | low | **0%** |
| Python `.pyi` stub | 82 free fns + 25 classes | 100% | 89% (free fns) / 0% (class properties) | 89% (free fns) / 0% (class properties) |
| WASM Rust bindings | 91 `#[wasm_bindgen]` items | 77% | low | **0%** |
| WASM `index.d.ts` | ~80 declarations in `AnalyticsNamespace` | ~5% (1 JSDoc on the namespace itself) | <5% | **0%** |

### Headline takeaways

1. **Rust core is in excellent shape**: ~98–99% public items documented, citations consistent, conventions clearly stated in README. Main gaps are sparse `# Examples` on `Performance` methods, missing Jorion (2006) reference across all VaR-family functions, and one **broken intra-doc link** that fails `cargo doc -D warnings`.
2. **Python `.pyi` is the strongest binding surface**: 89% of free functions have full Args/Returns/Examples; 100% structural parity with PyO3 bindings. Critical gap is the **6 minimal docstrings at the bottom of the file** (`compare_var_backtests`, `pnl_explanation`, `mtd_select`/`qtd_select`/`ytd_select`/`fytd_select`) and **loose `object` typing** for ~13 date parameters.
3. **PyO3 binding `///` comments are uniformly thin**: most are one-line summaries that PyO3 forwards verbatim into Python `__doc__`. Rust-style `# Arguments`/`# Returns` headings render as literal text in `help()`. **Zero PyO3 docstrings carry Python examples.**
4. **WASM is the weakest documented surface**: `finstack-wasm/src/lib.rs` lacks `#![warn(missing_docs)]`; whole files (`backtesting.rs`, `lookback.rs`, `timeseries.rs`) have zero `///` on exports; `index.d.ts` violates the project's own `DOCS_STYLE.md` which mandates JSDoc `@param`/`@returns`/`@throws`/`@example`.
5. **Triplet naming is clean**: zero Rust↔Python↔WASM name mismatches across 91 WASM exports + 82 Python free functions. Two style mismatches noted (`Performance.dates()` vs Rust `active_dates()`, and `BenchmarkAlignmentPolicy.zero_on_missing` vs Rust `ZeroReturnOnMissingDates`) — both are intentional but undocumented.
6. **Cross-surface coverage gap**: 4 Rust public functions (`compare_garch_models`, `auto_garch`, `vol_term_structure`, `period_stats_from_grouped`) are not exposed in Python or WASM. Either bind or document the omission.
7. **Stale parity contract**: `parity_contract.toml:67` lists a `comps` analytics submodule that does not exist anywhere in the codebase.
8. **Stale `AGENTS.md` paths**: claims `parity_contract.toml` at "repo root" (actually at `finstack-py/parity_contract.toml`) and `parity tests under finstack-py/tests/parity` (directory does not exist; tests are flat at `finstack-py/tests/test_*.py`).

---

## Critical Findings

| # | Severity | Location | Issue |
|---|---|---|---|
| 1 | **Critical** | `finstack/analytics/src/lib.rs:29` (+ `:39`) | Broken intra-doc link `[\`crate::dates::PeriodKind\`]` — `dates` is `pub(crate) use` (line 39), so rustdoc cannot resolve it from the public surface. With the README's own verification command `RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-analytics --no-deps --all-features`, this is a hard error. **Fix:** make the re-export`pub` (preferred) or rewrite as `[\`finstack_core::dates::PeriodKind\`]`. |
| 2 | **Critical** | `finstack/analytics/src/benchmark.rs:362–405` | `beta` doc claims "for `n ≥ 40` the normal critical value 1.96 is used" but the implementation uses the t-critical at df=40 (~2.021) until df ≥ 240. **Either the doc or the code is wrong.** Verify against intent and fix. |
| 3 | **Critical** | `finstack-py/src/bindings/analytics/mod.rs:32–148` | `__all__` omits `GarchFit` and `GarchParams`. Both classes are registered via `timeseries::register()` (lines 479–480) and visible on the module, but `from finstack.analytics import *` and IDE auto-import won't see them. **Fix:** add `"GarchFit"`, `"GarchParams"` to the types block. |
| 4 | **Critical** | `finstack-wasm/index.d.ts:269–282` and `:434–447` | `interface PeriodStats` is **declared twice** with identical bodies. TS allows it via declaration merging, but a future edit to one block without the other silently breaks the contract. **Fix:** delete one. |

---

## High Findings (45)

### Rust core — top-level (8)

- **High** `lib.rs:38–39` — `pub(crate) use finstack_core::{dates, error, math};` with a `///` doc that documents an item users never see. Either make it `pub` (consistent with intra-doc links), or change `///` to `//`.
- **High** `aggregation.rs:36` — `kelly_criterion` formula `win_rate − loss_rate / payoff_ratio` is operator-precedence-ambiguous. Add explicit parentheses; cite Kelly (1956) or Thorp.
- **High** `aggregation.rs:11–38` — `PeriodStats::cpc_ratio` ("Common Sense Ratio") has no source citation despite non-standard terminology.
- **High** `benchmark.rs:1–4` — module doc fails to enumerate the 19+ public items in this 1542-line file. Add a "Key types"/"Key functions" section.
- **High** `drawdown.rs:691–733` — `martin_ratio` doc says "Returns 0.0 if Ulcer Index is zero" but the doctest example asserts `INFINITY`. Doc/example contradiction.
- **High** `drawdown.rs:1–9` — module doc lists 4 levels of granularity but the file actually exposes a 5th (drawdown-derived ratios at lines 475–805). Update enumeration.
- **High** `returns.rs:1–3` — module doc says "Delegates to `math::stats::log_returns` for log variants" but this module **does not** expose log-return functions. Misleads users.
- **High** `performance.rs:1–4` — module doc opens with "Mirrors the Python `Performance` class 1:1 (minus plotting)" — internal-history language that confuses external Rust users.

### Rust core — submodules (12)

- **High** `risk_metrics/return_based.rs:488–506` — `RuinDefinition` enum-level doc is one line; users must read both `WealthFloor` and `TerminalFloor` variants to learn the semantic difference (intra-path vs end-of-path). Add a "Choosing a definition" paragraph.
- **High** `risk_metrics/return_based.rs:138–151` — `cagr` does not enumerate sentinel returns: `(empty → NaN), (Dates with end<=start → 0.0), (Factor with bad ann_factor → NaN)`. Currently merged into one sentence.
- **High** `risk_metrics/return_based.rs:604–673` — `estimate_ruin` cites "Press et al." for circular block bootstrap (incorrect reference). Should cite Politis & Romano (1992) for block bootstrap and Wilson (1927) for the Wilson-score interval.
- **High** `risk_metrics/tail_risk.rs:14–66` — `value_at_risk` cites only J.P. Morgan RiskMetrics (1996); audit guidance requires Jorion (2006) reference. Same issue at `parametric_var` (`tail_risk.rs:342–394`) and `cornish_fisher_var` (`tail_risk.rs:395–464`).
- **High** `timeseries/garch.rs:75–141` — `GarchParams::half_life` doc does not state units (returns are in *period* units — daily/monthly/etc — depending on input).
- **High** `timeseries/garch.rs:201–242` — `FitConfig::tol` doc says "Function-value convergence tolerance" but does not specify that it's a **relative** tolerance: `(f_worst - f_best) < tol * (1 + |f_best|)`. Critical for users tuning the fit.
- **High** `timeseries/forecast.rs:36` — `STANDARD_HORIZONS = &[1, 5, 21, 63, 252]` doc mentions trading days but doesn't say the choice assumes a 252-day year and breaks for 24/7 markets.
- **High** `backtesting/metrics.rs:57–137` (kupiec), `:138–236` (christoffersen), `:273–332` (traffic_light), `:334–410` (pnl_explanation) — all four primary user-facing backtesting functions lack `# Examples` despite being canonical entry points.
- **High** `backtesting/types.rs:221–272` — `VarBacktestConfig::significance` is documented as configurable, but `KupiecResult::reject_h0_5pct` and `ChristoffersenResult::reject_h0_5pct` are hard-coded at 0.05, **ignoring** the field. Either wire it through (real bug) or document the inconsistency.
- **High (cross-cutting)** Multiple modules — Sentinel conventions (`NaN` vs `0.0` vs `±∞`) vary by function but no per-module summary documents the policy. Rolled up under "Cross-Cutting Issues" below.
- **High (cross-cutting)** GARCH `pub` field doc comments do not state per-family parameter constraints (e.g. `omega > 0` for symmetric GARCH/GJR but unrestricted for EGARCH).
- **High (cross-cutting)** Doctest examples use `.unwrap()` despite the crate's `#![deny(clippy::unwrap_used)]`; the `#![doc(test(attr(allow(clippy::expect_used))))]` (lib.rs:18) covers `expect`, not `unwrap`. Several doctests at e.g. `risk_metrics/return_based.rs:131`, `timeseries/forecast.rs:139` will fire under `RUSTDOCFLAGS='-D warnings' cargo test --doc`.

### Python bindings — Rust side (13)

- **High** `analytics/types.rs:17, 99, 140, 181, 227, 273, 324, 385, 421, 457, 510, 560, 606, 669, 716, 749, 808, 869, 921` — `pub(super) inner` fields lack `///` comments. Only the last two (lines 974, 1028) carry docs. If `inner` should be `pub(crate)` per `AGENTS.md`, this becomes a `-D missing_docs` build error.
- **High** Cross-cutting — Docstring style mixes Rust-flavored markup that renders awkwardly in Python `help()`:
  - `[\`PyPerformance\`]` intra-doc-links appear as literal `[\`PyPerformance\`]`
  - `:class:\`KupiecResult\`` Sphinx role markers (e.g. `backtesting.rs:66, 90`) only render under Sphinx; in plain`help()` they're raw text
  - Rust `# Arguments` / `# Returns` headings (~30 occurrences across `backtesting.rs`, `timeseries.rs`) are not idiomatic Python. Convert to NumPy/Google style or unify on Rust style if Sphinx is the target.
- **High** Cross-cutting — Conventions from Rust README (decimal returns, non-positive drawdowns, non-negative CDaR, right-labeled rolling, explicit annualization) **not surfaced anywhere visible to Python users**. Module `__doc__` (`mod.rs:19–21`) is one sentence.
- **High** Cross-cutting — Default-value documentation missing in PyO3 docstrings. `volatility(returns, annualize=True, ann_factor=252.0)` — defaults emitted via `text_signature` but the `///` body never explains the 252 convention.
- **High** Cross-cutting — Errors mapping not documented anywhere. Every `PyResult` method routes through `core_to_py()` but no docstring states which Python exception type the user should catch.
- **High** Cross-cutting — No examples for primary user-facing APIs. **Zero** of ~115 PyO3 binding items carries a Python usage example.
- **High** `analytics/types.rs:1` — file-level `//!` is a single sentence ("Result structs and enums for the analytics domain.") — doesn't tell Python users that these are *frozen* result wrappers (constructed by analytic functions, not by users; `KupiecResult(...)` from Python errors opaquely).
- **High** `analytics/performance.rs:87–90` — `PyPerformance` class docstring doesn't describe the date-grid convention (must be sorted; gaps?), thread-safety, immutability, or provide a Python example.
- **High** `analytics/performance.rs:98–101` — `__new__` says nothing about what happens if prices contain NaN, what exception is raised, or which `freq` strings are valid.
- **High** `analytics/performance.rs:123` — `from_arrays` doesn't explain `prices` shape: is it `prices[ticker_idx][date_idx]` or `prices[date_idx][ticker_idx]`? Critical because Python users will guess.
- **High** `analytics/backtesting.rs:7, 22, 28, 49, 55, 71, 107, 132` — eight functions (`classify_breaches`, `kupiec_test`, `christoffersen_test`, `traffic_light`, `run_backtest`, `rolling_var_forecasts`, `compare_var_backtests`, `pnl_explanation`) all have one-line docstrings; none enumerate valid `method`/`distribution` strings or input shapes.
- **High** `analytics/timeseries.rs:8` — `fit_garch11` accepts `distribution: &str` silently. Allowed values (`"gaussian"`/`"student_t"` per `parse_dist`) are not documented anywhere visible to Python users.
- **High** `analytics/functions/{aggregation,benchmark,drawdown,returns,risk_metrics}.rs:1` — none of these five files has a file-level `//!` doc.

### Python wrapper / `.pyi` (4)

- **High** `__init__.pyi:1–6` — no module-level CONVENTIONS section. The Rust README documents 7 invariants none of which surface to Python users from `help(finstack.analytics)`.
- **High** `__init__.pyi:407, 764, 790, 1020, 1088, 1106, 1147, 1207, 1419, 1486, 1934, 1961, 1985` — 13 occurrences of `start: object` / `end: object` / `Sequence[object]` for date parameters. Bindings accept `datetime.date | pd.Timestamp` (and tz-naive ISO strings) — type checkers can't help users. Tighten to the Union type.
- **High** `__init__.pyi:734` — `PyPerformance` lacks a `__repr__` declaration in `.pyi` because `performance.rs` doesn't implement one (every other class does). At runtime Python users see `<finstack.analytics.Performance object at 0x...>`.
- **High** `__init__.pyi:2595, 2603, 2610, 2617, 2624, 2631` — six functions at the end of file have one-line docstrings only (`compare_var_backtests`, `pnl_explanation`, `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select`). Bindings have full `# Arguments`/`# Returns` blocks; these need to be ported.

### WASM (8)

- **High** `finstack-wasm/src/lib.rs:7–10` — lacks `#![warn(missing_docs)]` (Python bindings have it). This is the structural cause of WASM doc gaps.
- **High** `src/api/analytics/backtesting.rs:7, 22, 28, 49, 55, 71, 107, 132` — eight `#[wasm_bindgen]` exports with **zero** `///` docs (only `rollingVarBatch` at `:84–91` has prose).
- **High** `src/api/analytics/lookback.rs:7, 16, 25, 34` — all four `#[wasm_bindgen]` exports (`mtdSelect`, `qtdSelect`, `ytdSelect`, `fytdSelect`) have **zero** `///` docs.
- **High** `src/api/analytics/timeseries.rs:8, 18, 28, 38, 56, 63, 70, 75, 80` — nine exports (`fitGarch11`, `fitEgarch11`, `fitGjrGarch11`, `forecastGarchFit`, `ljungBox`, `archLm`, `aic`, `bic`, `hqic`) have **zero** `///` docs.
- **High** `index.d.ts:541–727` — `AnalyticsNamespace` is the canonical TS contract (per `package.json:7`'s `"types"` field). 187 lines, only one JSDoc (line 542–547 on the namespace itself). DOCS_STYLE.md mandates JSDoc on every export — materially unmet.
- **High** `index.d.ts:665, 693, 700, 713` — TS parameters `periodKind`, `method`, `distribution` typed as `string` instead of literal unions. `VarMethod` exists at line 286 but isn't reused. Loses type safety.
- **High** `index.d.ts:501–532` — `CagrBasis`, `BenchmarkAlignmentPolicy`, `RuinDefinition`, `RuinModel` are declared as opaque interfaces (`{}`) with separate `…Constructor` types but no `declare const X: XConstructor`. Construction path opaque to TS users; defaults invisible.
- **High** `index.d.ts:573` — `estimateRuin` returns `RuinEstimateJson` and never throws (Rust returns NaN for invalid inputs per README), but `@throws` documentation is absent. TS users may try-catch needlessly.

---

## Medium Findings (98)

The full enumeration spans the per-file findings from the five agent reports; below is the consolidated list of **cross-cutting medium-severity themes**. For specific file:line items, see the per-surface breakdowns in [Per-Surface Detail](#per-surface-detail) below.

### M1. Convention surfacing — uneven across surfaces

The Rust README documents 7+ conventions (decimal returns, non-positive drawdowns, non-negative CDaR, right-labeled rolling, NaN-padded `*_values` helpers, explicit annualization, `Performance::new` derives simple returns internally). Surfacing:

| Surface | Where exposed | Quality |
|---|---|---|
| Rust README | Centralized "Core Conventions" section | Excellent |
| Rust per-function docs | Scattered; some functions repeat, others omit | Inconsistent |
| Python `__init__.py:1` | One 8-word fragment | Poor |
| Python `__init__.pyi` | A few function-level callouts (`value_at_risk` Returns line, etc.) | Fragmentary |
| Python PyO3 docstrings (Rust side) | None | Absent |
| WASM `index.d.ts` | None | Absent |
| WASM Rust source | None | Absent |
| WASM `index.js` facade | None | Absent |

**Recommendation:** add a `CONVENTIONS.md` to `finstack/analytics/` (or expand the README's "Core Conventions" section) and reference it from:
- Each module-level `//!` block
- `__init__.pyi` module docstring
- `mod.rs::register()` `__doc__` setter
- `index.d.ts` AnalyticsNamespace JSDoc
- `index.js` facade top-of-file comment

### M2. Annualization parameter terminology — inconsistent

The same parameter goes by `ann_factor` (most pure functions), "annualization factor" (some narrative), "periods per year" (most `# Arguments`), `ann` (private method on `Performance`). Pick one canonical form ("annualization factor (periods per year)") and apply everywhere, including the `.pyi` and TS declarations.

### M3. Sign / sentinel conventions on tail-risk methods

Functions return `f64::NAN`, `0.0`, or `±∞` as sentinels for invalid/degenerate inputs but the policy varies by function and is not summarized at module level. Examples:
- `value_at_risk` returns `0.0` for empty input
- `parametric_var` and `cornish_fisher_var` return `NaN` for invalid `ann_factor`
- Sharpe / Sortino return `0.0` for `0/0`, `±∞` for nonzero/0
- Calmar / Martin / Sterling / Burke / Pain / Recovery — all use `ratio_or_sign_infinity` (`drawdown.rs:588`) consistent semantics
- Treynor and m_squared (benchmark.rs) use a *different* convention (m_squared returns the risk-free rate for zero-vol)

**Recommendation:** add a "Sentinel conventions" sub-section to each module-level `//!`, or factor a single `crate::conventions` doc page that everyone cross-references.

### M4. Rust `# References` coverage uneven

| Citation present | Citation missing |
|---|---|
| `tracking_error`, `information_ratio`, `multi_factor_greeks`, `treynor`, `m_squared` (Grinold & Kahn) | `r_squared`, `up_capture`, `down_capture`, `capture_ratio`, `batting_average` |
| `cdar`, `ulcer_index`, `calmar`, `martin_ratio`, `sterling_ratio`, `burke_ratio` | `pain_index`, `pain_ratio`, `recovery_factor`, `mean_episode_drawdown` |
| `value_at_risk`, `expected_shortfall` (J.P. Morgan, Artzner et al.) | All VaR-family missing **Jorion (2006)** |
| `omega_ratio` (Keating & Shadwick) | `aggregation::PeriodStats` (Kelly/CPC) |
| `parametric_var`, `cornish_fisher_var` (Cornish & Fisher 1937) | All four VaR functions need Jorion (2006), Hull |
| `kupiec_test`, `christoffersen_test`, `traffic_light`, `pnl_explanation` (Kupiec, Christoffersen, Basel) | — |
| GARCH module-level (Bollerslev, Nelson, Glosten et al.) | — |

### M5. PyO3 `///` docstrings are systematically thin

Across all PyO3 binding files, ~70% of docstrings are single-line summaries that don't surface units, defaults, sign conventions, error types, or examples. Examples:

```rust
/// Sortino ratio.
#[pyfunction]
fn sortino(...) -> f64 { ... }
```

renders in Python as just `"Sortino ratio."` — no Args, no Returns, no Examples, no Raises. Defaults (`annualize=True, ann_factor=252.0, mar=0.0`) are visible in `text_signature` but never explained.

**Recommendation:** establish a `finstack-py/DOCS_STYLE.md` mirroring `finstack-wasm/DOCS_STYLE.md`. Decide on NumPy- or Google-style docstrings (compatible with Sphinx and rendered well in `help()`). Port the Rust core's full doc comments where they add value, translating Rust types to Python types in the prose.

### M6. WASM `#[wasm_bindgen]` exports lack JSDoc per project's own DOCS_STYLE

`finstack-wasm/DOCS_STYLE.md` mandates: Summary, `@param` (with units + constraints), `@returns` (with units), `@throws`, optional Conventions block, mandatory `@example`. **Not a single analytics export satisfies this.** Best-documented file (`returns.rs`) has one-line summaries; worst (`backtesting.rs`, `lookback.rs`, `timeseries.rs`) have no docs at all.

### M7. `.pyi` `object` typing too loose

13 occurrences of `object` / `Sequence[object]` for date parameters (see High Findings — Python wrapper). Type checkers can't validate; IDE auto-completion can't suggest `pd.Timestamp` constructor. Tighten to `datetime.date | pd.Timestamp` consistently.

### M8. WASM JSON result types use `snake_case` keys

`KupiecResultJson`, `ChristoffersenResultJson`, `TrafficLightResultJson`, `BacktestResultJson`, `PnlExplanationJson`, `GarchParamsJson`, `GarchFitJson`, `VarianceForecastJson`, `RuinEstimateJson`, `BetaResult`, `GreeksResult`, `RollingSharpe`, `RollingSortino`, `RollingVolatility` all use `snake_case` field names (`lr_statistic`, `p_value`, `n_obs`, `r_squared`, `std_err`, etc.). This is consistent with `serde_wasm_bindgen::to_value` preserving Rust field names but breaks the camelCase JS convention used everywhere else in the namespace. Either:

- Add `#[serde(rename_all = "camelCase")]` to upstream Rust types, OR
- Document the parity intent (snake_case kept for serde-stability with Python/Rust)

### M9. Python wrapper `Performance.dates()` chose less-recommended Rust name

The Rust core has both `Performance::dates()` (`performance.rs:1293`) and `Performance::active_dates()` (`performance.rs:329`). The Rust README (line 194) flags `active_dates()` as the preferred name. The Python binding (`performance.rs:176`) wraps `dates()`, not `active_dates()`. Acceptable but should be documented. Same pattern: `BenchmarkAlignmentPolicy.zero_on_missing` (Python) vs `ZeroReturnOnMissingDates` (Rust).

### M10. Doctest examples mix `.unwrap()` and `?`

Some Rust doctests use `?` with `# Ok::<(), finstack_core::Error>(())`; others use `.unwrap()`. The crate's `#![doc(test(attr(allow(clippy::expect_used))))]` (lib.rs:18) covers `expect` but not `unwrap`. Several `.unwrap()` doctests (e.g. `risk_metrics/return_based.rs:131`, `timeseries/forecast.rs:139`) will fire under `RUSTDOCFLAGS='-D warnings' cargo test --doc`.

### M11. Cross-surface omitted Rust functions

Four Rust public functions are not exposed in either Python or WASM bindings:

| Rust function | Location | Recommendation |
|---|---|---|
| `compare_garch_models` | `timeseries/mod.rs:96` | Bind or document the omission in `__init__.pyi` and `index.d.ts` |
| `auto_garch` | `timeseries/mod.rs:160` | Bind or document |
| `vol_term_structure` | `timeseries/forecast.rs:144` | Bind or document |
| `period_stats_from_grouped` | `aggregation.rs:186` | Already documented as Rust-only in parity contract; surface that note in user-facing docs |

### M12. Stale parity contract entry

`finstack-py/parity_contract.toml:67` lists a `comps` analytics submodule:

```toml
comps = { python = "finstack.analytics", status = "flattened", note = "Comparable-company analysis exposed flat on finstack.analytics" }
```

No `comps*` files exist in `finstack/analytics/src/`, `finstack-py/src/bindings/analytics/`, or anywhere in the workspace. The 2026-04-22 review also called out `analytics:comps/types.rs::new, as_str` as zero-doc items but those have since been removed/moved. **Fix:** either restore the comps module or remove from parity contract.

### M13. AGENTS.md path drift

- `AGENTS.md` says "Parity contract at repo root: `parity_contract.toml`" — actually at `finstack-py/parity_contract.toml`.
- `AGENTS.md` says "parity tests under `finstack-py/tests/parity`" — directory does not exist; parity tests are flat at `finstack-py/tests/test_*_parity.py` (e.g. `test_core_parity.py`).
- These are outside the analytics scope but affect anyone reading AGENTS.md to understand the analytics binding contract.

### Other Medium themes

- **`functions/aggregation.rs`, `functions/benchmark.rs`, `functions/drawdown.rs`, `functions/returns.rs`, `functions/risk_metrics.rs`** — all five PyO3 binding files lack file-level `//!` docs.
- **`PyPerformance` lacks `__repr__`** (binding-side gap; every other class has one).
- **`__init__.py:1`** — module docstring is 8 words; no `Performance`-vs-free-function guidance, no notebook references.
- **`finstack-py/Cargo.toml:14`** — `doctest = false` means PyO3 `///` doc comments don't run as doctests. Acceptable (they're not Rust API docs, just Python docstring sources) but worth documenting.
- **WASM `tests.rs`** — 19 of 36 tests bypass the WASM layer and exercise the Rust core directly. The file is a slim parity smoke test, not a usage-example resource.
- **WASM `Breach` type** — `index.d.ts:285` declares `Breach = 'Hit' | 'Miss'` but `classifyBreaches` returns `boolean[]` (binding collapses enum at `backtesting.rs:15–19`). Type alias is unused.
- **`PeerStatsJson`, `RegressionResultJson`, `DimensionScoreJson`, `RelativeValueResultJson`** — declared in analytics block of `index.d.ts:382–421` but not referenced by `AnalyticsNamespace`. Likely copy-paste leak from `statements_analytics`.

---

## Per-Surface Detail

The following per-surface sections enumerate findings beyond the cross-cutting themes above. They are condensed; see the parallel agent reports for full text per item.

### A. Rust core — top-level files

**`lib.rs`** (Critical, 1 High, 4 Medium, 4 Low) — see Critical #1; missing crate-level `# Examples`; no "see also docs/REFERENCES.md" pointer at module root; `type Result<T> = ...` aliasing not explained.

**`aggregation.rs`** (3 High, 6 Medium, 3 Low) — `kelly_criterion` formula ambiguity; `cpc_ratio` no citation; `PeriodStats` lacks edge-case docs for `avg_loss == 0`; `period_stats` only one-line vs `period_stats_from_grouped` full docs.

**`benchmark.rs`** (Critical, 5 High, 11 Medium, 3 Low) — module doc lists no public items; `ROLLING_GREEKS_RECOMPUTE_INTERVAL = 64` magic number undocumented; `beta_ci_critical_value` 50-line lookup with no rationale; `beta` doc/code mismatch on critical value crossover; `BenchmarkAlignmentPolicy` `Default = ErrorOnMissingDates` is a behavior change not flagged prominently; multi-factor regression error variants not enumerated.

**`drawdown.rs`** (4 High, 8 Medium, 4 Low) — module doc enumerates 4 levels but file has 5; `DrawdownEpisode::near_recovery_threshold` formula ambiguity; `drawdown_details` truncates dates silently; `for_each_episode` magic threshold `< -1e-15` unexplained; `martin_ratio` doc contradicts example; `calmar` Returns description misleading on sign convention.

**`lookback.rs`** (3 High, 4 Medium, 3 Low) — module doc doesn't document boundary behavior (`ref_date` before/after `dates`); silent fallbacks (`unwrap_or`); MTD `offset_days` semantics confusing; `fytd_select` calendar-year inference unexplained.

**`performance.rs`** (5 High, 17 Medium, 6 Low) — `Performance::new` enumerates 5 error conditions but actual implementation has more; `reset_bench_ticker` may not refresh active-window drawdown cache (potential bug); only 3 of ~70 methods have `# Examples`; many methods omit "active window" note; method-vs-property inconsistency (`dates()` is callable, others are property-style accessors).

**`returns.rs`** (3 High, 4 Medium, 2 Low) — module doc misleading delegation note re log returns; `clean_returns` "rows" terminology confusing; `MIN_GROWTH_FACTOR = 1e-18` rationale undocumented; `simple_returns` NaN-cascade behavior misdescribed.

**README.md** (3 High, 4 Medium, 3 Low) — Main Modules table mismatches lib.rs export structure; one example uses `expect("...")` despite crate's `#![deny(clippy::expect_used)]`; verification commands instruct to run `RUSTDOCFLAGS='-D warnings' cargo doc` which currently fails (Critical #1).

### B. Rust core — submodules

**`risk_metrics/mod.rs`** (1 Medium, 1 Low) — `require_finite` doctest doesn't show negative path.

**`risk_metrics/return_based.rs`** (3 High, 7 Medium, 3 Low) — `RuinDefinition` enum-level doc terse; `cagr` sentinel enumeration incomplete; `RuinEstimate.std_err` semantics inconsistent with Wilson-score CI; `estimate_ruin` has wrong "Press et al." citation; `sortino` lacks NaN documentation for invalid `ann_factor`.

**`risk_metrics/rolling.rs`** (4 Medium, 3 Low) — `DatedSeries.values` doesn't document possible NaN values; `to_nan_padded` truncation behavior surprising; `nan_pad` private with doc but no module visibility note; `rolling_mean_m2_kernel` Welford-style update without citation.

**`risk_metrics/tail_risk.rs`** (2 High, 8 Medium, 1 Low) — module doc enumeration incomplete; `value_at_risk`/`expected_shortfall`/`parametric_var`/`cornish_fisher_var` missing Jorion (2006); `skewness`/`kurtosis` reference Joanes & Gill but don't specify estimator type (G_1/G_2/Type-2); `moments4` is `pub` but minimally documented; `tail_ratio` no `# References` despite citation in `gain_to_pain`.

**`timeseries/mod.rs`** (2 Medium, 1 Low) — `compare_garch_models` tiebreaker order undocumented; `auto_garch` has `unwrap_or_else(|| unreachable!())` worth a `# Panics` note.

**`timeseries/garch.rs`** (3 High, 7 Medium, 1 Low) — `GarchParams` field constraints not documented per-family; `GarchParams::half_life` units not stated; `to_vec`/`from_vec` parameter order not enumerated; `FitConfig::tol` relativity not stated; `GarchModel::filter` precondition (`sigma2_out` mutated in place) undocumented; `log_likelihood` `f64::NEG_INFINITY` sentinel undocumented; `GarchFit::std_errors = None` semantics not stated.

**`timeseries/garch11.rs`/`gjr_garch11.rs`/`egarch11.rs`** (3 Medium, 1 Low total) — unit struct one-line docs; parameter bounds in trait impls undocumented; EGARCH gamma sign convention worth cross-tool note (some packages use opposite sign).

**`timeseries/optimizer.rs`** (3 Medium, 5 Low) — `NelderMead::minimize` missing `# Arguments`/`# Panics`; `finite_diff_hessian` is `pub` with terse doc; `halton_perturb` private with terse doc.

**`timeseries/forecast.rs`** (1 High, 1 Medium, 5 Low) — `STANDARD_HORIZONS` doc doesn't note 252-day-year assumption; `vol_term_structure` deduplication behavior undocumented.

**`timeseries/innovations.rs`** (3 Medium, 2 Low) — `InnovationDist::StudentT` standardization convention not stated; behavior at `nu ≤ 2` undocumented; `log_pdf`/`expected_abs` parameterization conventions implicit.

**`timeseries/diagnostics.rs`** (2 Medium, 4 Low) — `ljung_box` and `arch_lm` degenerate-input fallbacks (`(0.0, 1.0)`) silent; chi-squared series helpers cite Numerical Recipes in prose but not in `# References`.

**`backtesting/mod.rs`** (1 Low) — minor cross-reference cleanup.

**`backtesting/metrics.rs`** (4 High, 3 Medium, 1 Low) — `kupiec_test`, `christoffersen_test`, `traffic_light`, `pnl_explanation` lack `# Examples`; `pnl_explanation` "degenerate result" vague; `traffic_light` `# Returns` incomplete.

**`backtesting/orchestrator.rs`** (3 Medium, 1 Low) — `run_backtest` NaN handling silent; `rolling_var_forecasts` no example; `compare_var_backtests` slice-conversion idiom not shown.

**`backtesting/types.rs`** (1 High, 4 Medium, 5 Low) — `VarBacktestConfig::significance` is hard-coded-ignored at 0.05 (real bug or doc gap); `confidence` valid range not stated.

### C. Python bindings — Rust side

**`mod.rs`** (1 Critical, 4 Medium, 3 Low) — `__all__` missing `GarchFit`, `GarchParams`; module `__doc__` terse; ordering rationale absent.

**`types.rs`** (1 High cluster, 13 Medium, 8 Low) — undocumented `pub(super) inner` fields; `__repr__` undocumented across all classes; `RuinDefinition.drawdown_breach` "positive threshold" contrary to README's negative-fraction convention; valid `convention` strings for `CagrBasis.dates` not listed; `RuinModel.__new__` parameter units not stated; `BenchmarkAlignmentPolicy` variant naming abbreviated vs Rust enum.

**`performance.rs`** (3 High, 18 Medium, 3 Low) — `__new__`, `from_arrays`, `reset_date_range`, `reset_bench_ticker` all missing exception-type and shape documentation; `Performance.lookback_returns`/`period_stats` raise behavior implicit; getter docs (`benchmark_idx`, `dates`, `freq`, `ticker_names`) sparse on shape; `summary_to_dataframe` doesn't list 22 columns; `cumulative_returns_outperformance` / `drawdown_difference` shape (per-ticker) implicit.

**`backtesting.rs`** (8 High, 4 Medium, 2 Low) — see High findings; `parse_var_method` private helper undocumented; `# Arguments` blocks render literally in Python `help()`.

**`timeseries.rs`** (1 High, 12 Medium, 4 Low) — `fit_garch11`/`fit_egarch11`/`fit_gjr_garch11` `distribution: &str` valid values undocumented; `forecast_garch_fit.fit` parameter shape (must be JSON output of fit_*) implicit; `aic`/`bic`/`hqic` formulas in single-line docs; `auto_garch`/`compare_garch_models` not exposed.

**`functions/aggregation.rs`** (1 High, 1 Medium, 1 Low) — file `//!` missing; `group_by_period` valid `freq` strings undocumented; `period_stats` is the **best-documented** file in the audit (use as template).

**`functions/benchmark.rs`** (1 High, 12 Medium, 1 Low) — file `//!` missing; all 14 functions have one-line docs; `multi_factor_greeks` factor-orientation (`factors[i][t]` vs `factors[t][i]`) implicit; pre-computed-input vs returns-input distinction (`treynor`, `m_squared`) not separated from runtime-input variants in `Performance`.

**`functions/drawdown.rs`** (1 High, 13 Medium, 1 Low) — file `//!` missing; `drawdown` parameter expectations (drawdown series, not raw returns) implicit; `burke_ratio`'s `dd_episodes` shape (flat array of negative depths) not stated.

**`functions/returns.rs`** (1 High, 6 Medium, 1 Low) — file `//!` missing; `convert_to_prices` parameter named `base` here but `startPrice` in WASM TS — minor drift; `excess_returns.nperiods` semantics implicit.

**`functions/risk_metrics.rs`** (1 High, 22 Medium, 1 Low) — file `//!` missing; defaults differ across Performance methods vs free functions in unclear ways (`mean_return.annualize` defaults `False` here vs `True` in Performance); `confidence` units (decimal vs percent) implicit; `mar` (minimum acceptable return) in `downside_deviation`/`sortino` not explained.

### D. Python wrapper / `.pyi`

**`__init__.py`** (2 Medium, 3 Low) — module docstring fragment; `finstack.finstack` indirection undocumented; comparable-company aliases imported from `_statements_analytics` need explanation.

**`__init__.pyi`** (4 High, 7 Medium, 5 Low) — see High findings; result-type properties have one-line docs only; `DrawdownEpisode.start`/`valley`/`end` are methods (not properties), and `RollingGreeks.dates`/`RollingSharpe.dates`/etc. similarly are methods — unmarked in docstrings; Performance's free-function-vs-method differences (e.g., `excess_returns` returns `list[float]` vs `list[list[float]]`) unexplained.

**Notebook coverage**: 5 analytics notebooks at `examples/notebooks/03_analytics/` plus 2 cross-domain at `06_advanced_quant/` and `07_capstone/`. Notebooks exercise the full top-level API but **no `.pyi` docstring cross-references a notebook**.

### E. WASM bindings

**`src/api/analytics/mod.rs`** (2 Low) — module doc misses convention surfacing.

**`src/api/analytics/aggregation.rs`** (4 Medium) — `groupByPeriod` valid `period_kind` strings undocumented; output shape `[string, number][]` implicit; `periodStats` intended pairing with `groupByPeriod` not stated.

**`src/api/analytics/backtesting.rs`** (8 High, 1 Medium) — see High findings.

**`src/api/analytics/benchmark.rs`** (8 Medium, 2 Low) — `WasmBenchmarkAlignmentPolicy` opaque construction undocumented; `alignBenchmark` ISO-date requirement implicit; `multiFactorGreeks` throws-conditions implicit.

**`src/api/analytics/drawdown.rs`** (3 Medium, 5 Low) — `toDrawdownSeries` non-positive convention not stated; `cdar` non-negative convention not stated; `burkeRatio.dd_episodes` shape implicit; `drawdownDetails` ISO-date input implicit.

**`src/api/analytics/lookback.rs`** (3 High, 1 Medium) — see High findings; `[start_idx, end_idx]` half-open vs closed undocumented.

**`src/api/analytics/returns.rs`** (4 Low) — best-documented file but still violates DOCS_STYLE.md (no `@param`/`@returns`/`@example`); `cleanReturns` non-mutating semantics implicit; `convertToPrices.base` vs WASM TS `startPrice` drift; `excessReturns.nperiods` semantics implicit.

**`src/api/analytics/risk_metrics.rs`** (5 Medium, 5 Low) — see Medium findings.

**`src/api/analytics/support.rs`** (2 Low) — internal helpers, file lacks `//!`.

**`src/api/analytics/timeseries.rs`** (4 High, 2 Medium) — see High findings.

**`src/api/analytics/tests.rs`** (2 Low) — 19 of 36 tests bypass WASM layer; missing tests for ~25 of 75 exports.

**`index.js`** (1 Medium, 1 Low) — facade has 4 lines of comment; no per-namespace JSDoc.

**`index.d.ts`** (Critical #4 + 4 High + 4 Medium + 1 Low) — `PeriodStats` duplicated; loose string typing; opaque-class construction path; snake_case JSON result keys; `Breach` type unused; `PeerStatsJson`-family leaked from statements_analytics.

---

## Triplet Consistency Analysis

### Naming parity (Rust ↔ Python ↔ WASM)

**Result: zero critical mismatches across 91 WASM exports + 82 Python free functions.**

Spot-verified:

| Rust | Python | WASM | Match? |
|---|---|---|---|
| `value_at_risk` | `value_at_risk` | `valueAtRisk` | ✓ |
| `comp_total` | `comp_total` | `compTotal` | ✓ |
| `simple_returns` | `simple_returns` | `simpleReturns` | ✓ |
| `tracking_error` | `tracking_error` | `trackingError` | ✓ |
| `rolling_sharpe` | `rolling_sharpe` | `rollingSharpe` | ✓ |
| `period_stats` | `period_stats` | `periodStats` | ✓ |
| `group_by_period` | `group_by_period` | `groupByPeriod` | ✓ |
| `fit_garch11` | `fit_garch11` | `fitGarch11` | ✓ |
| `r_squared` | `r_squared` | `rSquared` | ✓ |
| `m_squared` | `m_squared` | `mSquared` | ✓ |
| `max_drawdown` | `max_drawdown` | `maxDrawdown` | ✓ |
| `cagr` | `cagr` | `cagr` | ✓ |
| `sharpe` | `sharpe` | `sharpe` | ✓ |
| `cornish_fisher_var` | `cornish_fisher_var` | `cornishFisherVar` | ✓ |

### Style mismatches (intentional, but undocumented)

| Issue | Rust | Python | WASM |
|---|---|---|---|
| `Performance` accessor | `active_dates()` (recommended) and `dates()` (alias) | `dates()` only | n/a (Performance not in WASM) |
| `BenchmarkAlignmentPolicy` variant | `ZeroReturnOnMissingDates` | `zero_on_missing` | `zeroOnMissing` |
| `BenchmarkAlignmentPolicy` variant | `ErrorOnMissingDates` | `error_on_missing` | `errorOnMissing` |
| `RuinDefinition` factories | enum variants `WealthFloor`/`TerminalFloor`/`DrawdownBreach` | classmethods `wealth_floor`/`terminal_floor`/`drawdown_breach` | factories `wealthFloor`/`terminalFloor`/`drawdownBreach` |

The Python/WASM abbreviations are reasonable host-language adaptations. None are documented. Add a "Naming differences from Rust" note in `__init__.pyi` and `index.d.ts`.

### Coverage gaps

**Python-only (intentional, mostly stateful Performance facade):**
- `Performance` class with ~64 methods + `LookbackReturns` opaque class
- All 22 result-type wrapper classes (PyO3 wrappers; WASM uses serde-JSON instead)
- DataFrame-returning helpers: `summary_to_dataframe`, `cumulative_returns_to_dataframe`, `drawdown_series_to_dataframe`, `correlation_to_dataframe`, `drawdown_details_to_dataframe`, `lookback_returns_to_dataframe`

**WASM-only (perf optimization):**
- `rollingVarBatch` — batched form of `rollingVarForecasts`, amortizes JS↔WASM serde cost. Not in Python.

**Rust-only (not bound):**

| Function | Rust location | Bind, document, or remove? |
|---|---|---|
| `compare_garch_models` | `timeseries/mod.rs:96` | Bind (model selection useful in both Python and JS) |
| `auto_garch` | `timeseries/mod.rs:160` | Bind |
| `vol_term_structure` | `timeseries/forecast.rs:144` | Bind (already implied by `forecastGarchFit` workflow) |
| `period_stats_from_grouped` | `aggregation.rs:186` | Document as Rust-only (parity contract already notes this) |

### Stale parity contract

`finstack-py/parity_contract.toml:67`:

```toml
comps = { python = "finstack.analytics", status = "flattened", note = "Comparable-company analysis exposed flat on finstack.analytics" }
```

No `comps` Rust source, no `comps` Python bindings, no `comps` WASM bindings. **Remove** unless the comps module is actively planned to return.

### `index.d.ts` cross-domain leak

`index.d.ts:382–421` declares `PeerStatsJson`, `RegressionResultJson`, `DimensionScoreJson`, `RelativeValueResultJson` in the analytics block but `AnalyticsNamespace` (lines 541–727) does not reference them. They appear to belong to `statements_analytics`. Move or delete.

---

## Coverage Tables

### Rust core (per file)

| File | Public items | Documented | Full sections | Examples | References (where applicable) |
|------|---:|---:|---:|---:|---:|
| `lib.rs` | 1 module + 9 sub-mods + 4 re-exports | 100% | 0 (no `# Examples`) | 0 | 0 |
| `aggregation.rs` | 1 struct (12 fields), 3 fns | 100% | 67% | 67% | 0% (target: PeriodStats Kelly/CPC) |
| `benchmark.rs` | 5 structs, 1 enum, 13 fns | 100% | 92% | 92% | 46% |
| `drawdown.rs` | 1 struct (6 fields), 12 fns | 100% | 83% | 83% | 42% |
| `lookback.rs` | 4 fns | 100% | 100% | 100% | 0% |
| `performance.rs` | 2 structs, ~70 methods | 100% | 71% | **4%** | 0% (delegated) |
| `returns.rs` | 6 fns | 100% | 100% | 100% | 0% |
| `risk_metrics/mod.rs` | 1 fn + 19 re-exports | 100% | 100% | 100% | n/a |
| `risk_metrics/return_based.rs` | 14 (10 fns, 4 types) | 100% | 93% | 79% | 88% |
| `risk_metrics/rolling.rs` | 7 (3 fns, 1 struct, 3 type aliases) | 100% | 100% | 100% (fns) | 0% |
| `risk_metrics/tail_risk.rs` | 9 (8 fns, 1 helper) | 100% | 100% | 89% | 88% (target: Jorion across the board) |
| `timeseries/mod.rs` | 2 fns + re-exports | 100% | 100% | 100% | n/a |
| `timeseries/garch.rs` | 4 structs, 1 trait, 7 fields/methods | 100% | 83% | **8%** | 25% |
| `timeseries/{garch11,gjr_garch11,egarch11}.rs` | 1 struct each + trait impl | 100% | n/a (inherits) | 0% | inherits module ref |
| `timeseries/optimizer.rs` | 4 fns, 2 structs, 1 type | 100% | 71% | 0% | 100% |
| `timeseries/forecast.rs` | 4 (1 const, 2 structs, 2 fns) | 100% | 100% | 100% (fns) | inherits module ref |
| `timeseries/innovations.rs` | 1 enum, 4 methods | 100% | 80% | 0% | n/a |
| `timeseries/diagnostics.rs` | 5 fns | 100% | 60% | 0% | 100% (module-level) |
| `backtesting/mod.rs` | 13 re-exports | n/a | n/a | 1 module-level | n/a |
| `backtesting/metrics.rs` | 5 fns | 100% | 100% | **0%** | 100% |
| `backtesting/orchestrator.rs` | 3 fns | 100% | 100% | 33% | 0% (delegate) |
| `backtesting/types.rs` | 8 types + builder methods | 100% | 100% | 0% | 100% |

### Python (PyO3 bindings + .pyi)

| File | PyO3 items | Documented | With Examples | With Error notes |
|---|---:|---:|---:|---:|
| `bindings/analytics/mod.rs` | 1 fn (register) | 100% | 0% | 0% |
| `bindings/analytics/types.rs` | 21 classes; 88 methods/getters | 99% | 0% | 0% |
| `bindings/analytics/performance.rs` | 1 class; 53 methods | 100% | 0% | 0% |
| `bindings/analytics/backtesting.rs` | 12 fns | 100% | 0% | 0% |
| `bindings/analytics/timeseries.rs` | 3 classes; ~30 methods; 8 fns | 100% | 0% | 0% |
| `bindings/analytics/functions.rs` | 1 fn (register) | 0% | 0% | 0% |
| `bindings/analytics/functions/aggregation.rs` | 2 fns | 100% | 0% | 0% |
| `bindings/analytics/functions/benchmark.rs` | 14 fns | 100% | 0% | 0% |
| `bindings/analytics/functions/drawdown.rs` | 15 fns | 100% | 0% | 0% |
| `bindings/analytics/functions/returns.rs` | 7 fns | 100% | 0% | 0% |
| `bindings/analytics/functions/risk_metrics.rs` | 23 fns | 100% | 0% | 0% |
| **PyO3 total** | **~277 items** | **~99%** | **0%** | **0%** |

| `.pyi` section | Count | Documented | Args/Params | Returns | Examples |
|---|---:|---:|---:|---:|---:|
| Top-level free functions | 82 | 100% | 93% | 93% | 89% |
| Result-type classes | 25 | 100% | n/a | n/a | 20% (constructors only) |
| Performance class methods | 64 | 100% | 8% | 8% | 6% |
| Result-type properties | 95 | 100% | 0% | 0% | 0% |
| GARCH class properties | 31 | 100% | 0% | 0% | 0% |
| GARCH free functions | 9 | 100% | 100% | 100% | 100% |
| VaR backtesting free functions | 12 | 100% | 50% | 50% | 42% |

### WASM

| File | `#[wasm_bindgen]` items | Documented (`///`) | With Examples | With Error notes |
|---|---:|---:|---:|---:|
| `mod.rs` | 0 (re-exports) | n/a | 0/0 | 0/0 |
| `aggregation.rs` | 2 | 100% | 0% | 0% |
| `backtesting.rs` | 9 | **11%** (1/9) | 0% | 0% |
| `benchmark.rs` | 16 | 100% | 0% | 0% |
| `drawdown.rs` | 16 | 100% | 0% | 0% |
| `lookback.rs` | 4 | **0%** | 0% | 0% |
| `returns.rs` | 7 | 100% | 0% | 0% |
| `risk_metrics.rs` | 28 | 100% | 0% | 0% |
| `timeseries.rs` | 9 | **0%** | 0% | 0% |
| **WASM Rust total** | **91** | **77%** | **0%** | **0%** |

| `index.d.ts` | Declarations | With JSDoc |
|---|---:|---:|
| Type aliases / interfaces | ~30 | ~13% |
| `AnalyticsNamespace` methods | ~80 | **~1%** (1 namespace-level JSDoc only) |

---

## Prioritized Recommendations

### Immediate (Blocking — Fix before next release)

1. **Fix the broken intra-doc link in `lib.rs:29`/`:39`** — `RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-analytics` currently fails. Either change `pub(crate) use` to `pub use`, or rewrite intra-doc links to fully-qualified paths.
2. **Fix `benchmark.rs:362–405` doc/code mismatch** in `beta`'s critical-value crossover. Verify intent; correct either doc or code.
3. **Fix `drawdown.rs::martin_ratio` doc/example contradiction** (doc says "0.0 if Ulcer Index is zero"; example asserts INFINITY).
4. **Add `GarchFit` and `GarchParams` to `__all__`** in `finstack-py/src/bindings/analytics/mod.rs:32–148`.
5. **Delete duplicate `PeriodStats` interface** in `finstack-wasm/index.d.ts:269–282` or `:434–447`.
6. **Resolve `VarBacktestConfig::significance` wiring** at `backtesting/types.rs:221–272` — either honor the field or document the 5%-hardcode in `KupiecResult.reject_h0_5pct`/`ChristoffersenResult.reject_h0_5pct`.

### Short-term (Next sprint)

7. **Add `#![warn(missing_docs)]` to `finstack-wasm/src/lib.rs`** and fix the resulting cascade — start with the 22 undocumented exports in `backtesting.rs`, `lookback.rs`, `timeseries.rs`.
8. **Backfill the 6 minimal `.pyi` docstrings** at `__init__.pyi:2595, 2603, 2610, 2617, 2624, 2631` (`compare_var_backtests`, `pnl_explanation`, `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select`). Bindings already have full `# Arguments`/`# Returns` text.
9. **Tighten `.pyi` `object` typing** — replace 13 occurrences with `datetime.date | pd.Timestamp`.
10. **Tighten `index.d.ts` string-typed parameters** — `periodKind` (line 665), `method` (lines 693, 700), `distribution` (line 713) → literal unions or existing `VarMethod` type.
11. **Add a CONVENTIONS section** at the top of `__init__.pyi:1` and create or expand a `finstack/analytics/CONVENTIONS.md` referenced from each surface (`mod.rs::register()` `__doc__`, `index.d.ts` AnalyticsNamespace JSDoc, `index.js` facade).
12. **Add Jorion (2006) reference** to `value_at_risk`, `expected_shortfall`, `parametric_var`, `cornish_fisher_var` (and corresponding entries in `docs/REFERENCES.md`).
13. **Document GARCH parameter constraints per family** on `GarchParams` field docs (`omega > 0` for symmetric, unrestricted for EGARCH, etc.).
14. **Decide and document the WASM JSON-key casing policy** — either `#[serde(rename_all = "camelCase")]` upstream or document why snake_case is preserved.

### Medium-term

15. **Establish `finstack-py/DOCS_STYLE.md`** mirroring `finstack-wasm/DOCS_STYLE.md`. Decide on NumPy vs Google vs Rust-style docstrings; apply consistently.
16. **Translate Rust `# Arguments` / `# Returns` blocks** in PyO3 bindings (`backtesting.rs`, `timeseries.rs`) to chosen Python style.
17. **Add Python examples to high-traffic PyO3 functions** — `value_at_risk`, `sharpe`, `to_drawdown_series`, `Performance.__new__`, `fit_garch11`, etc.
18. **Add JSDoc per `DOCS_STYLE.md`** to high-traffic WASM exports — every export should have `@param` (with units), `@returns`, `@throws`, runnable `@example`.
19. **Add `# Examples` to ~10 high-traffic `Performance` methods**: `beta`, `greeks`, `rolling_sharpe`, `rolling_greeks`, `multi_factor_greeks`, `lookback_returns`, `period_stats`, `correlation_matrix`, `cdar`, `estimate_ruin`.
20. **Standardize annualization parameter terminology** across all surfaces ("annualization factor (periods per year)").
21. **Document sentinel/sign conventions** at module level for `risk_metrics::return_based`, `risk_metrics::tail_risk`, `timeseries::diagnostics`.
22. **Bind or document the omitted Rust functions**: `compare_garch_models`, `auto_garch`, `vol_term_structure`. (`period_stats_from_grouped` is already documented as Rust-only.)
23. **Implement `__repr__` for `PyPerformance`** in `bindings/analytics/performance.rs`.
24. **Cross-reference notebooks** from key class docstrings (`Performance`, `GarchFit`, `RuinModel`).

### Long-term

25. **Remove the stale `comps` entry** from `finstack-py/parity_contract.toml:67`.
26. **Update `AGENTS.md`** to fix the `parity_contract.toml` and `tests/parity` path references.
27. **Establish a per-crate documentation coverage gate** — once blockers cleared, upgrade `missing_docs` from `warn` to `deny` (Rust core) and add a parity check that asserts `.pyi` matches PyO3 binding signatures.
28. **Add wasm-bindgen-test-based usage examples** to a new `finstack-wasm/tests/analytics_usage.rs` so WASM has the same "tests-as-examples" surface that Rust core has.
29. **Verify TS literal types match `FromStr` contracts** — add a unit test asserting that every string literal in `VarMethod`, eventual `PeriodKind` literal, eventual `Distribution` literal, etc., is a valid `parse_*` input.
30. **Document the `Performance.dates()` vs `active_dates()` choice** and the `BenchmarkAlignmentPolicy.{zero_on_missing, error_on_missing}` abbreviation in a "Naming differences from Rust" subsection of `__init__.pyi` and `index.d.ts`.

---

## Action Items Checklist

### Blockers

- [x] Fix `finstack/analytics/src/lib.rs:29` broken intra-doc link — **resolved 2026-04-25**: changed `[\`crate::dates::PeriodKind\`]` to `[\`finstack_core::dates::PeriodKind\`]`; demoted misleading`///` on `pub(crate) use` re-export to `//`.`RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-analytics` now passes.
- [x] Fix `finstack/analytics/src/benchmark.rs:362–405` `beta` doc/code mismatch — **resolved 2026-04-25**: replaced the inaccurate "`n ≥ 40` uses 1.96" claim with the actual step-down table (df 38–59 → 2.021, 60–119 → 2.000, 120–239 → 1.980, ≥ 240 → 1.96).
- [x] Fix `finstack/analytics/src/drawdown.rs:691–733` `martin_ratio` doc/example contradiction — **resolved 2026-04-25**: replaced "Returns 0.0 if Ulcer Index is zero" with the full sentinel convention (`±∞` for nonzero/0, `0.0` only for `0/0`); added matching example.
- [x] Fix `finstack-py/src/bindings/analytics/mod.rs` `__all__` to include `GarchFit`, `GarchParams` — **resolved 2026-04-25**: added to the types block. (Verified `__init__.py` and `__init__.pyi` already had them.)
- [x] Fix duplicate `PeriodStats` in `finstack-wasm/index.d.ts` — **resolved 2026-04-25**: deleted the un-documented duplicate at the head of the analytics section; kept the single documented version. TypeScript `tsc --noEmit` clean; `periodStats(returns): PeriodStats` reference still resolves.
- [x] Resolve `VarBacktestConfig::significance` wiring vs `reject_h0_5pct` hard-coding — **doc-only resolution 2026-04-25**: documented the limitation on `VarBacktestConfig::significance`, `with_significance`, and both `reject_h0_5pct` fields. The full API fix (threading `significance` through `kupiec_test`/`christoffersen_test` and renaming `reject_h0_5pct` → `reject_h0`) is API-breaking across Rust/Python/WASM and is left as a deliberate follow-up.
- [x] Verify `Performance::reset_bench_ticker` interaction with active-window drawdown cache — **resolved 2026-04-25**: traced cache logic; `active_window_drawdowns` is pre-populated for *every* ticker by `refresh_active_drawdown_cache`, so flipping `benchmark_idx` correctly retargets the windowed bench drawdown without recomputation. **No bug.** Added a clarifying paragraph to `reset_bench_ticker`'s doc comment explaining the invariant.

**Verification**: `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test` (76/76 doctests + unit tests pass), and `RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-analytics --no-deps --all-features` all clean. `cargo check -p finstack-py` and `cargo check -p finstack-wasm` clean.

### Documentation Coverage

- [ ] Add `#![warn(missing_docs)]` to `finstack-wasm/src/lib.rs`
- [ ] Document 8 `#[wasm_bindgen]` exports in `src/api/analytics/backtesting.rs`
- [ ] Document 4 `#[wasm_bindgen]` exports in `src/api/analytics/lookback.rs`
- [ ] Document 9 `#[wasm_bindgen]` exports in `src/api/analytics/timeseries.rs`
- [ ] Add file-level `//!` to `finstack-py/src/bindings/analytics/functions/{aggregation,benchmark,drawdown,returns,risk_metrics}.rs`
- [ ] Backfill 6 minimal `.pyi` docstrings (`compare_var_backtests`, `pnl_explanation`, `mtd_select`, `qtd_select`, `ytd_select`, `fytd_select`)
- [ ] Implement `__repr__` for `PyPerformance` in `bindings/analytics/performance.rs`

### Convention Surfacing

- [ ] Add CONVENTIONS section to `finstack-py/finstack/analytics/__init__.pyi:1`
- [ ] Create or expand `finstack/analytics/CONVENTIONS.md`
- [ ] Add JSDoc convention block to `finstack-wasm/index.d.ts` AnalyticsNamespace
- [ ] Add convention pointer in `finstack-wasm/index.js` facade
- [ ] Add convention pointer in `finstack-py/src/bindings/analytics/mod.rs::register()` `__doc__`

### References

- [ ] Add Jorion (2006) to `docs/REFERENCES.md` and to `value_at_risk`, `parametric_var`, `cornish_fisher_var`
- [ ] Add Acerbi & Tasche (2002) to `expected_shortfall`
- [ ] Add Wilson (1927), Politis & Romano (1992) to `estimate_ruin` (replacing generic Press et al. cite)
- [ ] Add Joanes & Gill estimator-type qualifier to `skewness`/`kurtosis`
- [ ] Add `# References` to `r_squared`, `up_capture`, `down_capture`, `pain_index`, `pain_ratio`, `recovery_factor`, `mean_episode_drawdown`, `aggregation::PeriodStats`

### Type Tightening

- [ ] Tighten 13 `.pyi` `object` / `Sequence[object]` to `datetime.date | pd.Timestamp`
- [ ] Tighten `index.d.ts` `periodKind: string` (line 665) to literal union
- [ ] Tighten `index.d.ts` `method: string` (lines 693, 700) to `VarMethod`
- [ ] Tighten `index.d.ts` `distribution: string` (line 713) to literal union (verify against Rust `InnovationDist::FromStr`)
- [ ] Decide on `Breach` type fate (`index.d.ts:285`) — use it in `classifyBreaches` or remove

### Per-Module Docs

- [ ] Document GARCH parameter constraints per family in `GarchParams` field docs
- [ ] Document `STANDARD_HORIZONS` 252-day-year assumption
- [ ] Document `FitConfig::tol` relativity
- [ ] Document `RuinDefinition` "choosing a definition" guidance
- [ ] Document `cagr` sentinel return enumeration
- [ ] Enumerate Main Modules table in `benchmark.rs` module doc

### Cross-Surface Cleanup

- [ ] Bind or document `compare_garch_models`, `auto_garch`, `vol_term_structure`
- [ ] Remove stale `comps` entry from `finstack-py/parity_contract.toml:67`
- [ ] Move or delete `PeerStatsJson` family from `finstack-wasm/index.d.ts:382–421`
- [ ] Document `Performance.dates()` vs Rust `active_dates()` naming choice
- [ ] Document `BenchmarkAlignmentPolicy.zero_on_missing` vs Rust `ZeroReturnOnMissingDates` abbreviation
- [ ] Update `AGENTS.md` `parity_contract.toml` path (`finstack-py/parity_contract.toml`, not repo root)
- [ ] Update `AGENTS.md` `parity tests` path (no `finstack-py/tests/parity` directory exists)

### Tooling / Process

- [ ] Establish `finstack-py/DOCS_STYLE.md` mirroring `finstack-wasm/DOCS_STYLE.md`
- [ ] Add wasm-bindgen-test usage examples to `finstack-wasm/tests/`
- [ ] Add `RUSTDOCFLAGS='-D warnings' cargo test --doc` to CI for `finstack-analytics`
- [ ] Add a parity check that `.pyi` signatures match PyO3 binding signatures (catches future drift)
