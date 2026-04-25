# Documentation Review — Core Crate & Bindings

**Date:** 2026-04-25
**Scope:** `finstack/core`, `finstack-py` (PyO3), `finstack-wasm` (wasm-bindgen)
**Method:** Manual + grep audit across module docs, item docs, examples, references, builders, and generated artefacts; cross-checked against `finstack-wasm/DOCS_STYLE.md` and `docs/REFERENCES.md`. Doctests run; `cargo doc` run with `-D warnings` per crate.
**Predecessor:** `docs/DOCUMENTATION_REVIEW_2026-04-22.md` (workspace-wide; this review is narrower and verifies progress on `core`).

---

## Executive Summary

| Crate | Module docs | Item docs | Examples | References | `cargo doc` clean |
|-------|-------------|-----------|----------|------------|-------------------|
| `finstack-core` | Strong | Strong (was 32 zero-doc → ~3 truly missing) | Mixed (XIRR/dates/config strong; credit scoring weak) | Strong on financial code; thin in math/surfaces | ✅ clean (398 doctests pass) |
| `finstack-py` | Strong (28/28 binding files have `//!`) | Strong on user-facing methods; dunders inconsistent | Strong for primitives; thin on `ScheduleBuilder` | Inherits from core | ✅ clean |
| `finstack-wasm` | Adequate | **Below standard** vs `DOCS_STYLE.md` | **Zero `@example` blocks across all 134+ exports** | n/a | ⚠️ 1 broken intra-doc link |

**Top 3 cross-cutting issues** (drive almost everything else):

1. **`docs/DOCUMENTATION_STANDARD.md` is referenced but does not exist.** `finstack/core/src/lib.rs:81` instructs readers to follow it, but the file is absent. There is also no Python equivalent of `finstack-wasm/DOCS_STYLE.md`. This is the single biggest leverage point — without a written standard, future contributions will continue to drift.
2. **`finstack-wasm` JSDoc is structurally absent.** Zero `@example` blocks anywhere under `finstack-wasm/src/api/`; zero `@param` / `@example` in the hand-written `index.d.ts` (1626 lines, 0 examples); zero JSDoc in the 10 facade `exports/*.js` files. Compliance with the existing `finstack-wasm/DOCS_STYLE.md` is ~22%.
3. **Examples are missing on the canonical credit-scoring API in core.** `altman_z_score`, `altman_z_prime`, `altman_z_double_prime`, `ohlson_o_score` have references and zone tables but no `# Examples`. Same gap on `PdTermStructure` (the struct — `PdTermStructureBuilder` does have one).

---

## Methodology

- Read prior review (`docs/DOCUMENTATION_REVIEW_2026-04-22.md`) and verified the 32 `core` zero-doc items individually.
- Built each crate's rustdoc in isolation: `cargo doc -p <crate> --no-deps` after `cargo clean -p <crate>`. Captured warnings.
- Ran `cargo test -p finstack-core --doc` to confirm doctests still pass.
- Grep'd for `@example`, `@param`, `@returns`, `@throws` across `finstack-wasm`.
- Read `finstack-wasm/DOCS_STYLE.md` and applied its checklist to a sampled set of exports.
- Spot-checked subagent claims against the actual files; corrections noted inline below.

Coverage caveat: this is qualitative on the larger crates. The 2026-04-22 review remains the authoritative quantitative baseline (~7,400 pub items). This review focuses depth over breadth on core + bindings.

---

## 1. `finstack-core`

### 1.1 Build / lints status

- `cargo doc -p finstack-core --no-deps` builds clean with `RUSTDOCFLAGS="-D warnings"` (verified).
- `cargo test -p finstack-core --doc`: **398 passed, 0 failed, 15 ignored** (verified).
- `lib.rs` declares `#![warn(missing_docs)]` (line 2). AGENTS.md states `-D missing_docs` is enabled at workspace level — the crate currently produces no `missing_docs` output, so the public surface is fully covered.

### 1.2 Status of the 32 zero-doc items from the 2026-04-22 review

Verified by reading each file. The list below corrects the in-flight subagent report on a few items:

| Item | File | Status (2026-04-25) |
|------|------|---------------------|
| `CFKind` | `core/src/cashflow/primitives.rs` | ✅ Now documented (variant docs + reference table) |
| `CashFlow` | `core/src/cashflow/primitives.rs` | ✅ Now documented (validation rules + cross-refs) |
| `RoundingMode` | `core/src/config.rs:43-71` | ✅ Now documented with examples |
| `SeniorityClass` | `core/src/credit/lgd/seniority.rs:16-40` | ✅ Now documented with references |
| `CollateralType` | `core/src/credit/lgd/workout.rs:42-67` | ✅ Now documented with haircut guidance |
| `CalendarMetadata` | `core/src/dates/calendar/business_days.rs:137-150` | ✅ Documented (one-line struct doc + per-field docs). **Note:** subagent reported "still missing" — verified incorrect; line 137 has `/// Basic metadata describing a holiday calendar.` |
| `BusinessDayConvention` | `core/src/dates/calendar/business_days.rs:152-199` | ✅ Now documented with ISDA/FpML/ISO 20022 references and `# Examples` |
| `WeekendRule` | `core/src/dates/calendar/types.rs` | ⚠️ Variant-level docs only; enum-level prose minimal |
| `DayCount` | `core/src/dates/daycount.rs` | ✅ Module + enum docs |
| `SifmaSettlementClass` | `core/src/dates/imm.rs` | ⚠️ Variant docs present; enum-level summary thin |
| `PeriodKind` | `core/src/dates/periods.rs` | ⚠️ Brief enum comment |
| `TenorUnit`, `Tenor` | `core/src/dates/tenor.rs` | ✅ Documented |
| `ArbitrageSeverity` | `core/src/market_data/arbitrage/types.rs` | ⚠️ Categorical comment; could expand |
| `Seniority`, `ParInterp` | `core/src/market_data/term_structures/hazard_curve.rs` | ⚠️ Brief enum comments |
| `ExtrapolationPolicy` | `core/src/math/interp/types.rs` | ⚠️ Brief enum doc |
| `recover_fx_wing_vols`, `fx_forward`, `fx_atm_dns_strike` | `core/src/market_data/surfaces/mod.rs` | n/a — these are `pub(crate)`, not part of public API |
| `id_bump_bp`, `id_spread_bp`, `id_bump_pct` | `core/src/market_data/bumps.rs` | n/a — these are `pub(crate)` helpers |

**Net:** of 32 originally-listed items, ~13 now have full docs, ~13 have partial/terse docs, 6 are not part of the public API (the workspace's `missing_docs` lint already enforces public-API coverage, so the crate compiles clean).

### 1.3 Module-level docs (`//!`)

All 21 top-level `pub mod` declarations in `core/src/lib.rs` have non-trivial module-level docs, and the same is true for nested `mod.rs` files. Highlights:

- `cashflow`, `config`, `dates`, `market_data`, `math`, `types`, `credit` all have substantive multi-paragraph module docs with sub-area enumeration.
- `lib.rs` itself has a strong crate-level doc (lines 21-90) with quick-start, API layers, cargo features, doc conventions, and MSRV.
- No empty or single-sentence `//!` was found on any public module.

### 1.4 Examples — the gap is concentrated in `credit/`

Doctest count by file (sampled):

| File | Doctests | Quality |
|------|----------|---------|
| `core/src/cashflow/xirr.rs` | 2 | Canonical (`irr([-100,60,60])`, `xirr` with dates) |
| `core/src/config.rs` | 3 | Config / RoundingMode / results_meta |
| `core/src/credit/lgd/seniority.rs` | 1+ | Recovery distribution example |
| `core/src/credit/lgd/workout.rs` | 1+ | LGD calc example |
| `core/src/credit/scoring/altman.rs` | **0** | ❌ Has formula tables and references but no example |
| `core/src/credit/scoring/ohlson.rs` | **0** | ❌ Same gap |
| `core/src/credit/pd/term_structure.rs` | 1 | `PdTermStructureBuilder` has example; `PdTermStructure` itself does not |
| `core/src/dates/calendar/business_days.rs` | 1+ | ISDA convention example |
| `core/src/types/{rates,ratings,id}.rs` | many | Strong (Rate/Bps/Percentage/Id all exemplified) |
| `core/src/money/...` | 1+ | Quick-start in lib.rs covers basics |

**Concrete missing-Examples list** (these are the highest-traffic public APIs without a doctest):

- `core/src/credit/scoring/altman.rs:104` — `altman_z_score`
- `core/src/credit/scoring/altman.rs:149` — `altman_z_prime`
- `core/src/credit/scoring/altman.rs:~190` — `altman_z_double_prime`
- `core/src/credit/scoring/ohlson.rs` — `ohlson_o_score`
- `core/src/credit/pd/term_structure.rs:~17` — `PdTermStructure` (struct itself; the *builder* has one)

### 1.5 References

`grep -r "# References"` finds ~101 files with reference sections. Strong on:

- `cashflow/xirr.rs` (Brent 1973, Hull)
- `credit/lgd/seniority.rs`, `workout.rs` (Altman/Resti/Sironi, Schuermann, Basel)
- `credit/scoring/altman.rs`, `ohlson.rs` (original 1968 / 1980 papers)
- `dates/daycount.rs` (ISDA 2006, ICMA)

Thin on:

- `core/src/math/interp/*` — interpolation algorithms (de Boor, Fritsch-Carlson, Hagan-West) not cited at function level. `docs/REFERENCES.md` already has a `hagan-west-monotone-convex` anchor.
- `core/src/market_data/term_structures/*` — hazard / inflation curves lack canonical citations on the model side.
- `core/src/market_data/surfaces/*` — FX wing/SVI/SABR fits lack citations to the smile literature.

### 1.6 Builder pattern

Builders are well-covered at the method level: `WorkoutLgd::builder()…build()`, `PdTermStructureBuilder`, `ScheduleBuilder`, `FinstackConfig::builder()`. Most have method-level docs and the chain is exemplified in either the type's own `# Examples` or in the module-level doc.

One gap: `MarketContext::builder()` does not have a module-level chained example showing realistic curve attachment. That's a high-value addition because `MarketContext` is the primary input to every pricer.

### 1.7 Type aliases / re-exports

`lib.rs` lines 150-160 re-export `HashMap`, `HashSet`, `Error`, `InputError`, `NonFiniteKind`, `Result`. The aliases have one-line docs at the re-export site. Acceptable.

### 1.8 Things to fix in `core` (concrete, prioritized)

| Priority | File:Line | Action |
|----------|-----------|--------|
| P0 | `docs/DOCUMENTATION_STANDARD.md` | Create the file (or remove the reference at `core/src/lib.rs:81`). See section 4 below for proposed content. |
| P1 | `core/src/credit/scoring/altman.rs` | Add `# Examples` doctest to each of `altman_z_score`, `altman_z_prime`, `altman_z_double_prime` (canonical inputs from Altman 1968 produce known scores). |
| P1 | `core/src/credit/scoring/ohlson.rs` | Add `# Examples` doctest to `ohlson_o_score` with a known case. |
| P1 | `core/src/credit/pd/term_structure.rs` | Add struct-level `# Examples` to `PdTermStructure` (constructed via the builder, then `cumulative_pd` / `marginal_pd` shown). |
| P2 | `core/src/dates/calendar/types.rs` | Expand `WeekendRule` enum-level doc with one-line summary and reference. |
| P2 | `core/src/dates/imm.rs` | Add enum-level doc to `SifmaSettlementClass`. |
| P2 | `core/src/dates/periods.rs` | Expand `PeriodKind` enum doc. |
| P2 | `core/src/market_data/arbitrage/types.rs` | Expand `ArbitrageSeverity` doc with severity meanings (what triggers each). |
| P2 | `core/src/market_data/term_structures/hazard_curve.rs` | Expand `Seniority` and `ParInterp` enum docs; cite hazard-curve references. |
| P2 | `core/src/math/interp/types.rs` | Expand `ExtrapolationPolicy` with citation to standard interpolation literature. |
| P2 | `core/src/math/interp/*` | Add `# References` linking to `docs/REFERENCES.md#hagan-west-monotone-convex` and de Boor for splines. |
| P3 | `core/src/market_data/MarketContext` (wherever its `builder()` lives) | Add a module-level chained example. |
| P3 | `core/src/market_data/surfaces/*.rs` | Add references to SABR / SVI literature on the FX surface fit helpers. |

---

## 2. `finstack-py` (PyO3 bindings)

### 2.1 Build / lints status

- `cargo doc -p finstack-py --no-deps` builds clean (verified, no warnings).
- AGENTS.md lints (`#![deny(clippy::unwrap_used)]`, `#![deny(clippy::panic)]`, `#![forbid(unsafe_code)]`, `-D missing_docs`) apply.

### 2.2 Module-level docs

All 28 binding files under `finstack-py/src/bindings/core/` and its subdirectories have a `//!` module doc (audited, 100%). Includes `dates/`, `credit/`, `market_data/`, `math/`, `money.rs`, `currency.rs`, `types.rs`, `config.rs`, `mod.rs`.

### 2.3 PyO3 docstring coverage (`///` → Python `__doc__`)

User-facing items (`__new__`, classmethods, properties, fallible methods, JSON serialization helpers): essentially fully covered.

Inconsistent coverage on dunder methods (`__repr__`, `__str__`, `__hash__`, `__richcmp__`):

| Class | File | User-facing | Dunders documented? |
|-------|------|-------------|---------------------|
| `Money` | `bindings/core/money.rs` | ✅ all | Mixed: arithmetic dunders documented; `__repr__`/`__str__`/`__hash__`/`__richcmp__` not |
| `Currency` | `bindings/core/currency.rs` | ✅ all | ✅ all |
| `Rate`, `Bps` | `bindings/core/types.rs` | ✅ all | ❌ `__repr__`/`__str__`/`__hash__` missing |
| `Percentage` | `bindings/core/types.rs` | ✅ all | ❌ `__repr__`/`__str__` missing |
| `CreditRating` | `bindings/core/types.rs` | ✅ all (~21/22) | One gap |

This isn't a big deal — Python users intuit `__repr__`/`__str__` — but the inconsistency is a style-guide gap. Pick one rule: either always document dunders one-liner-style ("Pretty string representation"), or systematically skip them. Right now it's per-class authorial choice.

### 2.4 `.pyi` stub coverage

Audited `finstack-py/finstack/core/*.pyi`: `money.pyi`, `currency.pyi`, `types.pyi`, `config.pyi`, `dates.pyi` (1500+ lines). All major classes/functions have triple-quoted docstrings with `Parameters` / `Returns` / `Raises` sections, and full type annotations. `__all__` is set on every module. Naming matches Rust 1:1. **This is the strongest part of the Python documentation.**

### 2.5 Examples

For five core types, examples are present and runnable in the `.pyi` docstring or class-level doc:

| Type | Module-level example | Class-level example | Verdict |
|------|----------------------|---------------------|---------|
| `Money` | ✅ | ✅ | Excellent |
| `Currency` | ✅ | ✅ | Excellent |
| `Rate` / `Bps` / `Percentage` | ✅ | ✅ | Excellent |
| `Tenor` | ✅ | ✅ | Excellent |
| `ScheduleBuilder` | ⚠️ no chain example | ⚠️ class doc lacks fluent chain | **Gap** |

`ScheduleBuilder` is the only chain-style API in the user-visible Python surface, and there is no doctest or `.pyi` example showing the full `builder.frequency(...).stub_rule(...).build()` flow. Compounding the gap: the Python builder uses **in-place mutation** (methods return `None`), not Rust-style fluent self-return — and that difference is nowhere documented. New users will bounce off it.

### 2.6 Error mapping

`finstack-py/src/errors.rs` is the model file in the entire workspace for documentation quality. Module-level `//!` explains the chain-flattening contract; `core_to_py`, `display_to_py`, `error_to_py` are each documented with intent and trade-offs; tests show the chain behaviour. **Keep this file as the reference template** for any future docs-style work.

### 2.7 README quality

`finstack-py/README.md` covers install, modules, quick-start, package structure, parity, testing, and Rust/WASM relationships. It does **not** cover:

- What `parity_contract.toml` is and how a contributor uses it (mentioned by name only).
- Decimal vs `f64` boundary — no warning that core uses Decimal at boundaries while bindings expose `f64` (per `INVARIANTS.md` §1).
- Error mapping convention (what Python exception type is raised; how chains are flattened).
- "Where do I find type X?" navigation aid.
- Builder-pattern quirk (in-place mutation vs Rust fluent self-return).

### 2.8 Things to fix in `finstack-py` (concrete, prioritized)

| Priority | File / Path | Action |
|----------|-------------|--------|
| P0 | `finstack-py/DOCS_STYLE.md` (new) | Create a Python-side doc style mirroring `finstack-wasm/DOCS_STYLE.md`. Cover: PyO3 `///` → `__doc__` mapping, `.pyi` docstring format (NumPy-style: Parameters/Returns/Raises/Examples), financial conventions (rates units, date roles, curve IDs, Decimal vs f64), and the dunder-documentation rule. |
| P1 | `finstack-py/src/bindings/core/dates/schedule.rs` and `finstack-py/finstack/core/dates.pyi` | Add a complete chained-builder example to `ScheduleBuilder` showing `frequency`, `stub_rule`, `adjust_with`, `build` — and call out in the class doc that the methods mutate in-place rather than returning `Self`. |
| P1 | `finstack-py/README.md` | Add: "Common pitfalls" section (Decimal vs f64, error chain flattening, builder mutation), "Type discovery" section (where each type lives), short paragraph explaining `parity_contract.toml`. |
| P2 | `finstack-py/src/bindings/core/types.rs`, `money.rs` | Pick the dunder-docstring rule and apply it consistently. Recommend: short one-liner for `__repr__`/`__str__`/`__hash__`/`__richcmp__` (mirrors `currency.rs` which is the class with full coverage). |
| P3 | `finstack-py/examples/notebooks/README.md` | Add brief "what you'll learn" bullets per notebook so users can scan for the right starting point. |

---

## 3. `finstack-wasm` (wasm-bindgen bindings)

### 3.1 Build / lints status

- `cargo doc -p finstack-wasm --no-deps` produces **1 warning** (verified):

  ```
  warning: unresolved link to `Portfolio`
   --> finstack-wasm/src/api/portfolio/mod.rs:179:8
  179 | /// [`Portfolio`] handle.
      |       ^^^^^^^^^ no item named `Portfolio` in scope
  ```

  `Portfolio` here is `finstack_portfolio::Portfolio` from another crate; it is referenced in the doc but not in scope. Either fully qualify (` [`finstack_portfolio::Portfolio`] `) or drop the link.

- AGENTS.md cites `#![forbid(unsafe_code)]` and `-D missing_docs` for the binding crate. The crate compiles, so `missing_docs` itself isn't tripping — but the broken-link warning slips through the gate.

### 3.2 Compliance with `finstack-wasm/DOCS_STYLE.md`

The style guide (63 lines) mandates, **per exported item**: 1-2 line Summary, `@param`, `@returns`, `@throws`, `@example`, plus a Conventions section when applicable. Verified compliance across `src/api/`:

- **`@example` blocks across all of `finstack-wasm/src/api/`: 0** (verified by `grep -rE '@example' finstack-wasm/src/api/`).
- **`@example` blocks in `finstack-wasm/index.d.ts`: 0** (1626 lines, 0 `@example`, 0 `@param` — verified).
- **`@example` blocks in `finstack-wasm/exports/*.js` (10 facade files): 0**.

Module-by-module compliance (sampled):

| Module | Summary | `@param` | `@returns` | `@throws` | `@example` | Conventions |
|--------|---------|----------|------------|-----------|------------|-------------|
| `src/api/cashflows.rs` | ✅ | ✅ | ✅ | ✅ | ❌ | Partial |
| `src/api/core_ns/types.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | Partial |
| `src/api/core_ns/money.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| `src/api/core_ns/dates.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | Partial |
| `src/api/core_ns/market_data.rs` | ✅ | Partial (uses `# Arguments` Rust style, not `@param`) | ❌ | ❌ | ❌ | Partial |
| `src/api/margin/mod.rs` | ✅ | ❌ | Partial | ❌ | ❌ | ❌ |
| `src/api/valuations/pricing.rs` | ✅ | ❌ | Partial | ❌ | ❌ | ❌ |
| `src/api/valuations/analytic.rs` | ✅ | ❌ | ❌ | ❌ | ❌ | Partial (Greeks scaling explained inline) |
| `src/api/portfolio/mod.rs` | ✅ | ❌ | Partial | ❌ | ❌ | ❌ |
| `index.d.ts` | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

`cashflows.rs` is the only file where `@param` / `@returns` / `@throws` are systematically applied. It is the canonical pattern to copy.

### 3.3 The hand-written JS facade (`index.js` and `index.d.ts`)

- `finstack-wasm/index.js` (18 lines): has a top-level header comment ("Namespaced facade for finstack-wasm…") — agent report incorrectly stated this was missing. The file itself is essentially a re-export skeleton, which is fine.
- `finstack-wasm/index.d.ts` (1626 lines): contains the user-facing TypeScript type contract. **Has 0 `@example` and 0 `@param` JSDoc tags** across all interfaces and constructors. JSDoc continuation lines (`*`) appear 35 times — most are inside the few JSDoc blocks for `MonteCarloEstimateJson` and `VariationMarginJson`. Almost every other constructor / function signature is bare.
- `finstack-wasm/exports/*.js` (10 files): 0 JSDoc anywhere. These are the user-facing facade files reachable via `import { core } from "finstack-wasm"` — they should at minimum have a header comment per file describing the namespace contents, and ideally JSDoc on each exported member (which can be forwarded from the underlying wasm binding).
- `finstack-wasm/types/generated/*.ts`: auto-generated by ts-rs. These **do** include the original Rust doc comments (verified on `MarketQuote.ts` — full multi-line JSDoc block forwarded from Rust). This is the working channel; it just doesn't reach `index.d.ts` because `index.d.ts` is hand-written.

### 3.4 Naming triplet consistency

Spot-checked 10 exports (`Currency`, `Money`, `Rate`, `DayCount`, `Tenor`, `DiscountCurve`, `Money::add`, `DayCount::yearFraction`, `Portfolio::fromSpec`, `FxConversionPolicy::cashflowDate`). All match the AGENTS.md triplet rule (Rust `snake_case` ↔ `#[wasm_bindgen(js_name = ...)]` camelCase ↔ `index.d.ts` camelCase). This is the cleanest part of the WASM crate.

### 3.5 Financial conventions surfacing

The WASM crate has the most-stringent financial-doc requirements (per `DOCS_STYLE.md` §"Financial documentation rules"), but those rules are honoured *unevenly*:

- ✅ Rate units: `core_ns/types.rs` is explicit (`0.05` decimal, `500.0` bps).
- ✅ Continuously-compounded rate semantics: `core_ns/market_data.rs` lines 105-106.
- ✅ Greeks scaling: `valuations/analytic.rs` line 8 ("vega/rho per 1% move, theta per-day").
- ❌ Calendar codes / available calendars: not listed in `index.d.ts`.
- ❌ Curve-ID format and required entries in `MarketContext`: not documented anywhere TS users can find.
- ❌ CSA JSON schema for `margin.calculateVm`: not documented.
- ❌ Portfolio spec schema for `Portfolio.fromSpec`: not documented.
- ❌ Quote convention (clean vs dirty) on bond pricing: not documented in TS.

### 3.6 README

`finstack-wasm/README.md` covers install, namespaces, quick-start, and the facade-vs-pkg distinction. It does **not** link to `DOCS_STYLE.md`, name the generated `pkg/finstack_wasm.d.ts` path, or point readers at the JSON-schema documentation channels (CSA, Portfolio, MarketContext).

### 3.7 Things to fix in `finstack-wasm` (concrete, prioritized)

| Priority | File:Line | Action |
|----------|-----------|--------|
| P0 | `finstack-wasm/src/api/portfolio/mod.rs:179` | Fix the broken intra-doc link. Either ` [`finstack_portfolio::Portfolio`] ` or remove the brackets. This is the only thing standing between `cargo doc -p finstack-wasm` and a clean `RUSTDOCFLAGS="-D warnings"` gate. |
| P0 | `finstack-wasm/src/api/**/*.rs` | Backfill `@example` blocks per `DOCS_STYLE.md`. **Start with the 10-15 highest-traffic exports** (priority list below in §5.2). Use `cashflows.rs` as the template for `@param`/`@returns`/`@throws` shape. |
| P1 | `finstack-wasm/src/api/**/*.rs` | Bring all sampled modules to the `cashflows.rs` baseline for `@param`/`@returns`/`@throws`. The `# Arguments` style in `market_data.rs` should be migrated to `@param` for JSDoc to render. |
| P1 | `finstack-wasm/index.d.ts` | Add JSDoc per export with at least Summary + `@example`. Where the underlying Rust binding has `@param`/`@returns`/`@throws`, mirror them. The file is hand-written so it can't rely on wasm-pack to forward; this is manual work but high-leverage (TypeScript IntelliSense is the main consumer experience per `DOCS_STYLE.md`). |
| P1 | `finstack-wasm/exports/*.js` | Add at minimum a top-of-file JSDoc block per facade describing the namespace contents and one usage example. |
| P1 | `finstack-wasm/index.d.ts` and bindings | Document JSON-schema inputs that today are opaque: CSA JSON for margin, MarketContext required curves, Portfolio spec, Instrument JSON envelope. Either inline JSDoc or a `docs/WASM_SCHEMAS.md` companion. |
| P2 | `finstack-wasm/README.md` | Link to `DOCS_STYLE.md`; name the generated `.d.ts` path; add a "Common errors" section listing the failure modes of `init()` and JSON-parse errors. |
| P2 | `finstack-wasm/index.d.ts` | Add a top-of-file overview JSDoc block (Summary, namespace map, link to README). |
| P3 | `finstack-wasm/lib.rs` (or `cargo doc`) | Consider upgrading the `cargo doc` job in CI to `RUSTDOCFLAGS="-D warnings"` so future broken intra-doc links fail the build (matches the `finstack-core` posture). |

---

## 4. Cross-cutting recommendations

### 4.1 Create `docs/DOCUMENTATION_STANDARD.md`

`finstack/core/src/lib.rs:81` cites it as the canonical standard. It does not exist. Either write it or remove the cite. Writing it is the higher-leverage move, because it's also where the *Python* equivalent of `finstack-wasm/DOCS_STYLE.md` should live (or be linked from).

Suggested structure:

1. Scope: applies to all `pub` items in `finstack/*` crates plus the binding crates.
2. Required sections per item (Summary, `# Arguments`, `# Returns`, `# Errors`, `# Examples`, `# References`).
3. When `# References` is mandatory: any item that encodes a market convention, pricing model, numerical method, or risk calculation. Cite via `docs/REFERENCES.md` anchor.
4. Financial conventions block (rates units, date roles, curve IDs, quote convention) — copy verbatim from `finstack-wasm/DOCS_STYLE.md` so the rules are identical across Rust / Python / WASM.
5. Crate-specific deltas: link out to `finstack-wasm/DOCS_STYLE.md` and a new `finstack-py/DOCS_STYLE.md`.
6. Doctest policy: at least one runnable example for any item whose intended use isn't obvious from its signature.
7. Workflow: how the lints (`-D missing_docs`, `-D warnings` on `cargo doc`) plug into CI.

### 4.2 Create `finstack-py/DOCS_STYLE.md`

Mirrors `finstack-wasm/DOCS_STYLE.md` but with PyO3 specifics:

- `///` Rust doc comment on PyO3 exports → Python `__doc__`. (Confirm with one-line test.)
- `.pyi` docstring format: NumPy style (Parameters / Returns / Raises / Examples).
- Naming-strategy note: snake_case = snake_case (no rename overrides except where AGENTS.md forces them).
- Builder-pattern note: Python builders mutate in-place; chained calls return `None`. Document this loudly so users don't try Rust-style chaining.
- `Decimal` vs `f64`: per `INVARIANTS.md`, Decimal at money/accounting boundaries; `f64` everywhere else. Bindings expose `f64`; users converting back to Decimal in Python application code is their responsibility.

### 4.3 Tighten the rustdoc gate per crate

- `finstack-core`: already clean under `-D warnings`. Lock it in CI.
- `finstack-py`: also clean. Lock it in CI.
- `finstack-wasm`: 1 broken link today. Fix and lock.

A single `cargo doc --workspace --no-deps` with `RUSTDOCFLAGS="-D warnings"` in CI would catch future intra-doc-link breakage automatically.

### 4.4 Don't promote `missing_docs` from `warn` to `deny` until the binding gaps are closed

The 2026-04-22 review proposed promoting `missing_docs` from `warn` to `deny` once blockers are clear. For `core`, that's almost feasible today. For `finstack-wasm`, promotion would require backfilling the `@example` blocks first (otherwise the lint passes but the JSDoc contract still fails — the Rust lint doesn't enforce JSDoc semantics). Sequence: fix WASM JSDoc gaps → promote `missing_docs` to `deny` → add a JSDoc-coverage check (e.g., a small script counting `@example` per `#[wasm_bindgen]` export).

---

## 5. Action plan

### 5.1 Immediate (this week)

1. **[core]** Create `docs/DOCUMENTATION_STANDARD.md` (or remove the reference). 1-2 hours.
2. **[wasm]** Fix the broken intra-doc link at `finstack-wasm/src/api/portfolio/mod.rs:179`. 5 minutes.
3. **[core]** Add `# Examples` doctests to the four credit-scoring functions. 30-60 minutes.

### 5.2 Short-term (next sprint)

4. **[wasm]** Backfill `@example` and full JSDoc on the 10-15 highest-traffic exports. Suggested order:
   - `core_ns.Money` (constructor + `add`/`sub`/`mulScalar`/`divScalar`)
   - `core_ns.Currency` (constructor + `fromJson`)
   - `core_ns.Rate.fromPercent`, `fromBps`, `asPercent`, `asBps`
   - `core_ns.Tenor.parse` and `core_ns.Tenor` constructor
   - `core_ns.DayCount.yearFraction`
   - `core_ns.DiscountCurve` constructor
   - `core_ns.adjustBusinessDay`
   - `valuations.bsPrice`, `valuations.bsGreeks`, `valuations.bsImpliedVol`
   - `margin.calculateVm` (with CSA JSON shape inline)
   - `portfolio.Portfolio.fromSpec` (with spec JSON shape inline)
   - `valuations.priceInstrument`
5. **[wasm]** Add JSDoc Summary + `@example` to every export in `finstack-wasm/index.d.ts`.
6. **[py]** Write `finstack-py/DOCS_STYLE.md`. 1-2 hours.
7. **[py]** Add chained `ScheduleBuilder` example in `dates.pyi` and `bindings/core/dates/schedule.rs`; call out the in-place-mutation contract. 30 minutes.
8. **[py]** Patch README with "Common pitfalls" + "Type discovery" + parity-contract paragraph.
9. **[core]** Add `# Examples` to `PdTermStructure` (struct), `# References` to `core/src/math/interp/*` and `core/src/market_data/term_structures/*`.

### 5.3 Medium-term

10. **[core]** Tighten the partial enum docs (`WeekendRule`, `SifmaSettlementClass`, `PeriodKind`, `ArbitrageSeverity`, `Seniority`, `ParInterp`, `ExtrapolationPolicy`).
11. **[py]** Standardize dunder-method docstring policy across `Money`, `Rate`, `Bps`, `Percentage`.
12. **[wasm]** Document JSON schemas (CSA, Portfolio spec, Instrument envelope, MarketContext requirements) — either inline JSDoc or `docs/WASM_SCHEMAS.md`.
13. **[ci]** Add workspace-wide `cargo doc --workspace --no-deps` job under `RUSTDOCFLAGS="-D warnings"`.
14. **[ci]** Once WASM gaps are closed, add a coverage check for `@example` density per `#[wasm_bindgen]` export.

### 5.4 Action items checklist

- [ ] Create `docs/DOCUMENTATION_STANDARD.md` (or remove cite at `core/src/lib.rs:81`)
- [ ] Create `finstack-py/DOCS_STYLE.md`
- [ ] Fix `finstack-wasm/src/api/portfolio/mod.rs:179` broken intra-doc link
- [ ] Add `# Examples` to `core/src/credit/scoring/altman.rs` (3 functions)
- [ ] Add `# Examples` to `core/src/credit/scoring/ohlson.rs::ohlson_o_score`
- [ ] Add `# Examples` to `core/src/credit/pd/term_structure.rs::PdTermStructure`
- [ ] Backfill `@example` on the 10-15 priority WASM exports listed above
- [ ] Bring WASM `src/api/**/*.rs` to `cashflows.rs` JSDoc baseline
- [ ] Add JSDoc to `finstack-wasm/index.d.ts` and `finstack-wasm/exports/*.js`
- [ ] Document JSON schemas (CSA, Portfolio, MarketContext)
- [ ] Add chained `ScheduleBuilder` example to `finstack-py` (Rust + `.pyi`)
- [ ] Patch `finstack-py/README.md` (Pitfalls + Type discovery + parity contract)
- [ ] Standardize Python dunder docstring policy
- [ ] Tighten partial enum docs in `core` (WeekendRule, SifmaSettlementClass, PeriodKind, ArbitrageSeverity, Seniority, ParInterp, ExtrapolationPolicy)
- [ ] Add `# References` to `core/src/math/interp/*` and `core/src/market_data/term_structures/*`
- [ ] CI: `cargo doc --workspace --no-deps` under `RUSTDOCFLAGS="-D warnings"`
- [ ] CI: JSDoc coverage check (post-backfill)

---

## Appendix A — Verified vs unverified subagent claims

This review used three exploration subagents in parallel. A few of their claims were checked directly and adjusted:

| Claim | Verified? | Correction |
|-------|-----------|------------|
| `CalendarMetadata` is "still missing" docs (core agent) | ❌ Incorrect | Has `/// Basic metadata describing a holiday calendar.` at `business_days.rs:137` plus per-field docs |
| `index.js` "no top-level JSDoc overview comment" (wasm agent) | ❌ Incorrect | Has top-level header comment lines 1-4 |
| `index.d.ts` "350+ lines" (wasm agent) | ❌ Incorrect | Actually 1626 lines |
| Zero `@example` blocks across `finstack-wasm/src/api/` (wasm agent) | ✅ Verified | `grep -rE '@example' finstack-wasm/src/api/` → 0 hits |
| `finstack-wasm` rustdoc clean (wasm agent implicit) | ❌ Not verified clean | 1 broken intra-doc link warning |
| `docs/DOCUMENTATION_STANDARD.md` referenced but missing | ✅ Verified | `ls` confirms absent; `core/src/lib.rs:81` confirms reference |
| `finstack-core` rustdoc clean under `-D warnings` | ✅ Verified | Clean rebuild succeeds |
| 398 doctests pass in `finstack-core` | ✅ Verified | `cargo test -p finstack-core --doc` |
| WASM `cashflows.rs` is the only file with full `@param`/`@returns`/`@throws` | ✅ Verified | Lines 6-65 show consistent JSDoc tags |
| `PdTermStructureBuilder` has an example, `PdTermStructure` does not | ✅ Verified | Builder `# Examples` at term_structure.rs ~17, struct lacks one |

## Appendix B — Files referenced

- `docs/DOCUMENTATION_REVIEW_2026-04-22.md` — predecessor review
- `docs/REFERENCES.md` — citation anchors for `# References` sections
- `docs/SERDE_STABILITY.md` — wire-format stability guarantees
- `finstack-wasm/DOCS_STYLE.md` — WASM-specific JSDoc style
- `AGENTS.md` — project-level conventions including naming triplets and lints
- `INVARIANTS.md` — workspace invariants including Decimal-vs-f64 rules
