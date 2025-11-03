### 📎 Prompt: *Forward-Looking Code Review for Quant & Credit UX (New Functionality Only)*

**Context**

* **Bindings present:** Rust core + Python (PyO3) + WASM/TypeScript.
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

## Output Format (strict)

1. **Executive Summary** (bullets + impact/effort table)
2. **Feature Proposals** (sections with the fields above)
3. **Quick Wins**
4. **De-Dup Check**
5. **Appendix** (API snippets, schemas, notebook sketch)
