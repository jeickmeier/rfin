### 📎 Prompt: *Forward-Looking Code Review for Quant & Credit UX (New Functionality Only)*

**Context**

* **Repo / path(s):** `<repo-or-paths>`
* **Branch / tag:** `<branch-or-tag>`
* **Bindings present:** Rust core + Python (PyO3) + WASM/TypeScript.
* **What exists today (one-liner):** `<brief summary>`
* **Personas:** (1) Buy-side quant, (2) Credit analyst (LBO/CRE/Private credit), (3) Portfolio/risk.

**Your Task**
Identify and **recommend new functionality that is *not* currently implemented** but would significantly improve the day-to-day experience of quants and credit analysts using this library. Be pragmatic, finance-native, and API-ergonomic across Rust, Python (notebooks), and the browser (WASM).

**Ground Rules**

1. **Do not propose what already exists.** Confirm absence by scanning `src/`, `bindings/`, `examples/`, `tests/`, and docs. If partially present, propose the delta only.
2. Prioritize features that:

   * Reduce analyst toil (fewer ad-hoc spreadsheets/one-off scripts).
   * Improve **explainability**, **reproducibility**, and **model governance**.
   * Integrate smoothly with **DataFrames** (Polars/Pandas), **Arrow/Parquet**, and notebook/web flows.
   * Fit credit markets (loans/bonds, covenants, waterfalls) and quant risk (sensitivities, scenarios).
3. Every proposal must include a **concrete user story**, a minimal **API sketch** for Rust & Python (and TS if relevant), and a **small demo/notebook outline**.

---

## Deliverables (in this exact order)

### 1) Executive Summary (≤ 10 bullets)

* The most impactful **missing capabilities** and why they matter to quants/credit analysts.
* Quick table: **Impact (P0–P2)** × **Effort (S/M/L)** × **Confidence (High/Med/Low)**.

### 2) High-Impact Feature Proposals (3–8 items, ranked)

For each item, provide:

* **Title**: short and specific (e.g., “Hazard-Rate Term Structures + CECL/IFRS9 ECL”).
* **Persona Pain**: what the analyst can’t do efficiently today.
* **User Story**: “As a `<quant|credit-analyst>`, I need `<capability>` so that `<result>`.”
* **Scope** *(what’s new)*:

  * **Data**: inputs/outputs; DataFrame/Arrow shapes; schema snippet.
  * **Models/Algos**: methods, formulas, references to widely used approaches.
  * **APIs**:

    * **Rust**: `pub struct/trait` + example call.
    * **Python**: function/class + notebook one-liner (`.to_polars()/.to_pandas()` expected).
    * **WASM/TS** (if applicable): minimal method signature.
  * **Explainability**: `explain()` or `trace()` output showing curve builds, spreads, cash-flow drivers, covenant breaches, etc.
  * **Validation**: test ideas vs. canonical cases or published examples; tolerance bands; property tests.
* **DX Enhancements**:

  * Batch-friendly, streaming-ready, and parallelizable (Rayon/async where sensible).
  * Deterministic numerics and rounding modes; day-count/holiday conventions.
  * Clear error messages and typed results with metadata (units, conventions).
* **Impact & Effort**: P0–P2; S/M/L; **Dependencies** (e.g., calendars, curve engines).
* **Risks**: perf, numerics, data availability; mitigations.
* **Demo Outline**: short notebook plan with sample I/O.
* **Why Now**: linkage to common workflows (pricing, monitoring, IC memos, risk packets).

### 3) Quick Wins (“Fast-Follow”, 6–12 items)

Low-effort enhancements that immediately smooth analyst workflows (bindings, docs, small helpers, exporters, error messages, presets). 1–2 lines each.

### 4) De-Dup Check

For every proposed feature, paste the **evidence of absence** (paths searched / brief note). If partially present, list the **delta**.

---

## Mandatory Hunting Grounds (scan and propose if missing)

**Credit & Fixed-Income Primitives**

* **Curve/Calendar Core**: robust calendars (US/UK/EU/CA), business-day conventions, day-count/frequency sets; OIS/IBOR/TSY bootstraps; FX curves.
* **Credit Term Structures**: hazard-rate curves from CDS or bond spreads; **CECL/IFRS 9** ECL tooling (PD/LGD/EAD, stage migration).
* **Loan/Bond Features**: callable/putable/make-whole, floors/caps, step-ups, **revolver & DDTL**, PIK toggles, amortization schedules.
* **Spread Measures & Risk**: Z-spread, **OAS**, discount margin; **DV01/PV01/CS01**, **key-rate DV01**, convexity; spread duration.
* **Prepay/Default Models**: CPR/SMM, stochastic default timing, recovery distributions.

**Waterfalls & Covenants**

* Generalized **waterfall engine** (priority rules, cash sweeps, leakage tests).
* Covenant pack: **Net leverage**, **Interest coverage**, **DSCR**, **LTV**, min liquidity; forward testing with breach flags & headroom.
* **Explain()** views: timeline of triggers, breaches, and cash routing.

**Portfolio Analytics**

* Performance attribution (carry/roll/spread/mix), **concentration & limit checks**, sector/issuer buckets, rating buckets.
* **Scenario framework**: preset macro/curve/credit-spread shocks; grid runs; report diffs.
* **Risk aggregation**: exposure ladders, maturity walls, top-N risk contributors, **KRD ladders**, **marginal/conditional contributions to risk**.

**Financial Statements & Pro Forma**

* Multi-scenario **FSA** module: adjustments, pro formas, covenants tied to modeled cash flows; standardized KPI registry; **audit log** of adjustments.

**Data, I/O, & Interop**

* First-class **Arrow/Parquet** I/O; `.to_polars()`/`.to_pandas()` on results; Polars expressions for metrics.
* Importers/parsers: term sheets, cap tables, basic XBRL, rating histories; adapters for TRACE/EMMA (schema only; user provides data).
* **Excel-friendly** exporters (clean headers, units, formats) + sample templates.

**Bindings & DX**

* Python: typed API, **py.typed** + stub generation, rich docstrings with examples; error classes; tqdm-friendly progress.
* WASM/TS: lightweight browser API for demos; JSON-schema for config & results; deterministic builds.
* **Config & Repro**: TOML/JSON scenario specs with JSON-Schema; run metadata (seeds, versions, curve IDs) stamped on outputs.
* **Observability**: `tracing` spans, timing, cache hits; `explain()` payloads serializable to JSON.
* **Testing**: property tests, numerical regression tests with fixtures/golden files.

---

## Prioritization Rubric (score each proposal 1–5)

* **User Impact**: saves real analyst time or unlocks a blocked workflow.
* **Breadth**: useful across loans, bonds, CDS, CRE/LBO.
* **DX Quality**: simple APIs, great errors, DataFrame-native, `explain()`.
* **Perf & Scale**: batch-friendly; memory/time predictable; WASM viability.
* **Cross-Bindings Fit**: Rust core + Python + TS with minimal impedance.
* **Validation Readiness**: testability against known results/identities.

Provide a final **ranked list** (top-down) using rubric totals.

---

## Output Format (strict)

1. **Executive Summary** (bullets + impact/effort table)
2. **Feature Proposals** (sections with the fields above)
3. **Quick Wins**
4. **De-Dup Check**
5. **Appendix** (API snippets, schemas, notebook sketch)

---

## Calibration Examples (types of features you might propose if missing)

> Use these only as inspiration—**include only if absent**, otherwise provide the delta.

* **Hazard-Rate Curves + ECL**: build PD/LGD/EAD pipelines, stage migration, lifetime ECL; `explain_calibration()`.
* **Z-Spread / OAS / Discount-Margin Suite** with **KRD ladder** and **CS01**.
* **General Waterfall Engine** with covenant hooks and `explain_waterfall()`.
* **Scenario Runner** (grid of shocks) producing a tidy DataFrame with deltas vs base.
* **Portfolio Attribution** (carry, roll, spread, selection) and concentration/limit checks.
* **Arrow/Parquet I/O + Polars/Pandas Bridges** with stable schemas and units.
* **Python Stubs & py.typed** + rich examples; WASM demo with JSON-schema configs.
* **Explainability Channels**: `explain()` on pricing, curves, waterfalls; JSON-serializable traces.
* **Repro & Audit**: run metadata stamping; hash of inputs/curves/cache keys.
