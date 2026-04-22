Here’s a solid **AGENT.md** you can drop at the repo root to guide an agent (or a human reviewer) doing a **documentation/docstring standards code review** for a **financial pricing library**.

````md
# AGENT.md — Documentation & Docstring Standards Review (Financial Pricing Library)

## Mission

Review and improve the library’s **docstrings and documentation standards** so the codebase is:
- **Understandable** to new users and maintainers
- **Correct** and **unambiguous** about financial conventions
- **Traceable** to **canonical math/algorithm sources** and **industry standards** (e.g., ISDA)
- **Consistent** across the entire public API

You are reviewing documentation quality, not implementing new features (unless a missing doc requires a tiny code change to make an example/testable snippet possible).

---

## Scope

### In scope
- Docstrings for **public API** (functions, classes, traits, modules)
- Docstrings for **semi-public**/internal APIs that encode key conventions (curves, calendars, day count, compounding)
- Top-level docs: README, docs site pages, module-level docs, reference sections
- Examples and doctests (where supported)
- Cross-references and links to canonical sources / standards

### Out of scope (unless directly necessary to clarify docs)
- Large refactors
- Re-implementing pricers/models
- Performance tuning unrelated to documentation correctness
- Rewriting the entire documentation site unless requested

---

## Quality Bar (What “Good” Looks Like)

### For every public symbol
Doc must answer, in plain language:
1. **What it does** (one sentence)
2. **When to use it** (and when not to)
3. **Inputs & outputs** (types + meaning + units)
4. **Conventions** (day count, calendar, compounding, settlement, accrual)
5. **Assumptions & limitations**
6. **Errors / edge cases**
7. **Examples** (copy/paste runnable)
8. **Sources** (math references + industry standards) when applicable

### Financial libraries: non-negotiables
Docs must be explicit about:
- **Units**: rates in decimals vs bps, vol in decimal, time in years, currency, notional scaling
- **Day count & calendars**: ACT/360 vs 30/360, holiday calendars, business-day adjustment
- **Compounding**: simple/annual/semi/continuous; OIS vs IBOR conventions
- **Curve building choices**: instruments used, interpolation method, bootstrapping assumptions
- **Discounting & forecasting**: single-curve vs multi-curve, collateral/OIS discounting
- **Quote conventions**: par rates, spreads, upfront points, clean/dirty price, yield conventions
- **Rounding**: where it matters (e.g., cashflows, PV, accrued interest)
- **Date handling**: timezone neutrality, valuation date vs settlement date vs payment date

---

## Documentation Style Guide (Enforced)

Pick one format and apply consistently. If the repo already has a standard, follow it.

### Recommended formats
- **Python**: Google-style docstrings (or NumPy-style if repo already uses it)
- **Rust**: rustdoc (`///` with `# Examples`, `# Panics`, `# Errors`, `# Safety` when relevant)
- **TypeScript**: TSDoc/JSDoc style with `@param`, `@returns`, `@example`

### Rules
- Prefer **short, direct sentences**
- Avoid undefined jargon; define once and link to glossary
- All parameter descriptions include: **meaning, units, domain constraints**
- Examples must compile/run (or be marked as pseudocode explicitly)
- Never bury critical conventions in footnotes—put them near the top

---

## Required Sections in Docstrings (by API type)

### Public function / method
Must include:
- Summary (1–2 lines)
- Parameters (meaning + units + constraints)
- Returns (meaning + units)
- Raises/Errors (what and when)
- Example(s)
- Notes: conventions + assumptions
- Sources (if algorithmic)

### Public type (class/struct/trait)
Must include:
- What the type represents in finance terms
- Invariants (e.g., curve must be monotone in DF space)
- Construction rules (required fields, defaults)
- Thread-safety / mutability expectations (esp. Rust)
- Example usage
- Sources/standards if it encodes market conventions

### Module-level docs
Must include:
- Conceptual overview
- Main types and entry points
- Diagram or flow (optional but useful): quote → curve → cashflows → PV/risk
- References to canonical sources
- Links to glossary pages

---

## Sources & References Policy (Canonical + Industry)

### When sources are required
Include sources for:
- Pricing models (Black(-Scholes), Bachelier, HJM/LMM, SABR, etc.)
- Calibration methods / solvers
- Bootstrapping and interpolation approaches
- Risk measures (DV01/CS01, bucketed risk, Greeks)
- Credit modeling (hazard rates, survival curves, CDS bootstrapping)
- Any method where users would ask: “Which paper or standard is this based on?”

### How to cite sources
- Prefer stable identifiers: DOI / ISBN / official spec name + version/date
- Put references in a consistent “Sources” or “References” section
- If a method is “market practice” rather than a single paper, cite:
  - An industry standard (ISDA, ICMA, etc.)
  - Or a widely accepted reference text
- Avoid random blogs as primary authority (okay as supplemental “Further reading”)

### Minimum reference set (suggested)
Keep a `/docs/references.md` or `REFERENCE.md` with canonical citations you can link to.

Examples of categories to include:
- ISDA definitions for interest rate derivatives / fallback conventions
- Curve construction references (multi-curve, OIS discounting)
- Day count / business day conventions references
- Standard texts: interest rate modeling, derivatives pricing, fixed income math
- Relevant model papers (SABR, Hagan et al.; etc.)

> If the repo cannot or does not want to vendor a references file, then each docstring should cite succinctly.

---

## Review Checklist

### A) Coverage
- [ ] Every public API has a docstring
- [ ] Every exported type/module has module-level docs
- [ ] “Tricky” internal functions (date logic, accrual, bootstraps) are documented

### B) Correctness & Unambiguity
- [ ] No ambiguous “rate”/“yield” without convention specified
- [ ] Units always stated (decimal vs bps, years vs days)
- [ ] Time basis and calendars explicit
- [ ] Quotes vs model parameters distinguished (e.g., par rate vs zero rate)

### C) Consistency
- [ ] Same terminology across codebase (“discount factor” vs “DF”)
- [ ] Same format and headings across modules
- [ ] Same naming rules for parameters and examples

### D) Examples
- [ ] At least one example for each public entry point
- [ ] Examples are minimal and runnable
- [ ] If examples rely on data, provide tiny deterministic fixtures

### E) Sources & Standards
- [ ] Algorithm docs cite canonical sources
- [ ] Conventions cite relevant industry standards where applicable
- [ ] Any deviations from market standard are documented explicitly

### F) “Docs as Contracts”
- [ ] Pre-conditions stated (e.g., curve points sorted by date)
- [ ] Post-conditions stated (e.g., returns PV in currency units)
- [ ] Error behavior documented (exceptions/results/panics)

---

## Output Requirements

Produce deliverables in three layers:

### 1) Repo-wide Standards Proposal (if missing or inconsistent)
- A short “Documentation Standard” doc describing the chosen format and headings
- A glossary file if terms are not already defined
- A references list (recommended)

### 2) Findings Report
Organize findings as:
- **Critical**: materially misleading docs or missing conventions (could cause wrong valuations)
- **High**: missing sources, missing units, unclear date handling
- **Medium**: inconsistent style, missing examples, weak phrasing
- **Low**: typos, minor formatting

Each finding must include:
- File + symbol name
- What’s wrong
- Proposed fix (specific text suggestions)

### 3) Patch Plan
A sequence of small PRs:
- PR1: Add/align repo doc standard + references/glossary
- PR2+: Fix docs module-by-module (keep PRs small, testable)

---

## What Not To Do
- Don’t invent conventions. If not obvious from code, mark as **TODO** and propose clarifying questions or default choices.
- Don’t cite sources you can’t name precisely (avoid “some ISDA doc”).
- Don’t add long essays in docstrings—put deep dives in docs pages and link.

---

## Quick Templates

### Template: Public function
Summary line.

**Parameters**
- `x`: meaning (units). Constraints.

**Returns**
- Meaning (units).

**Conventions**
- Day count: ...
- Compounding: ...
- Calendar: ...

**Errors**
- When/why.

**Example**
```text
<minimal example>
````

**Sources**

* Canonical paper/book/spec.
