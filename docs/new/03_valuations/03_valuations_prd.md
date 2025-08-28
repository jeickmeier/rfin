### Valuations (`/valuations`) — Product Requirements Document

**Version:** 1.0 (draft)
**Audience:** Product managers, analysts, quants, risk, data scientists, and app engineers (Rust/Python/WASM)
**Purpose:** Define user‑facing requirements for the Valuations crate that prices instruments, generates/aggregates cash flows, reports risk and performance, and integrates with scenarios and portfolio layers. This aligns with `03_valuations_tdd.md` while remaining accessible to non‑engineers.

---

## 1) Executive Summary

Valuations is the quantitative finance engine for finstack. It turns instruments and market data into trustworthy prices, risk, and tagged cash flows. It covers public markets (bonds, swaps, CDS, options, equities) and private credit (term loans, DDTL, revolvers, complex fees, cash/PIK/toggle coupons, amortization, calls/prepayment). It also provides first‑class real estate underwriting: property cash flows (rent rolls, opex, taxes, capex/reserves), construction loans with interest reserves and conversion to term, and a deterministic equity waterfall engine with audit‑ready allocation ledgers. Results are deterministic, currency‑safe, and portable across Rust, Python, and WASM, with transparent FX and rounding policies and stable, analytics‑friendly outputs.

Key outcomes:
- **Accurate pricing and risk** with market‑standard conventions and multi‑curve discounting.
- **Private credit readiness**: cash/PIK/toggle interest, fees, amortization, call/prepayment logic, covenants, and workout flows.
- **Real estate underwriting**: property cash flows, construction loans (interest reserve, capitalization, term conversion), and a deterministic equity waterfall with auditable ledgers.
- **Deterministic, auditable results**: explicit FX policies and rounding context stamped in outputs.
- **Analytics at scale**: currency‑preserving cashflow tags/rollups and DataFrame‑ready long‑format exports.

---

## 2) Goals and Non‑Goals

Goals:
- Provide a coherent instrument library spanning fixed income, derivatives, options, equities, and private credit.
- Deliver pricing, yield, spread, and risk measures that match desk expectations and vendor benchmarks.
- Generate tagged cash flows suitable for period aggregation and portfolio analysis, with currency safety.
- Support private credit workflows: covenants, enforcement toggles, rate step‑ups, cash sweeps, and workout/recovery.
- Expose deterministic “policy hooks” (e.g., grid margins, index fallbacks) with stable schemas across bindings.
- Support real‑estate underwriting: property cash flows, construction loans, and a deterministic equity waterfall.

Non‑Goals:
- Portfolio aggregation and reporting (lives in `portfolio`).
- Scenario orchestration and DSL (lives in `scenarios`, though valuations exposes selectors and knobs).
- Structured credit waterfalls (separate, feature‑gated crate).
- Real‑time market data connectors (market data is provided by the host).

---

## 3) Target Users & Personas

- **Credit/Fixed‑Income Analyst:** Prices loans/bonds, tests covenant outcomes, runs yield‑to‑worst, and compares quotes.
- **Quant/Risk:** Computes DV01/CS01/duration/convexity/Greeks, validates curves, and produces bucketed risk.
- **Data Scientist (Python):** Runs pricing at scale, exports results as DataFrames, and evaluates XIRR.
- **PM/Portfolio Manager:** Needs consistent PVs, cashflow rollups by period/currency, and scenario‑driven what‑ifs.
- **Web/App Engineer (WASM):** Embeds pricing and cashflow previews in browser with stable JSON IO.

---

## 4) Primary Use Cases

- **Price instruments** using discount/forward/credit/vol data; compute yields and spreads, including YTW.
- **Generate and tag cash flows** (interest, principal, fees, workout), then **aggregate by period** without losing currency identity.
- **Measure risk** (DV01/CS01/duration/convexity; Greeks for options) with bucketed reports.
- **Private credit workflows:** model PIK/cash/toggle structures, amortization and fees, call/prepayment, covenants, and workout/recovery.
- **Performance metrics:** compute XIRR for deals and portfolios.
- **Scenario knobs:** deterministically toggle enforcement, shock thresholds, and change pricing grid margins via selectors.

---

## 5) Scope (In/Out)

In‑Scope:
- Instruments: bonds (fixed/floating), inflation‑linked bonds, loans/term loans, DDTL, revolvers, FX spot, swaps (IRS), CDS, vanilla options, equities (spot).
- Pricing outputs: PV/NPV, clean/dirty price, accrued, yields (incl. YTW), spreads (G/I/Z/OAS), and standard measures.
- Risk outputs: DV01/CS01, duration, convexity, and option Greeks.
- Cashflow engine: deterministic schedules, tagging, and currency‑preserving period aggregation.
- Market data use: OIS discounting, projection curves for floating legs, credit curves, vol surfaces, inflation indices, and explicit IBOR→RFR fallbacks.
- Private credit: fees (origination/commitment/utilization/amendment/exit/prepayment), grids for margins, covenants, and workout state machine.
- Real estate: property cash flows (rent roll, opex, taxes, capex/reserves), construction loans (commitment/draws, interest reserve, capitalization, conversion to term), and a deterministic equity waterfall engine.

Out‑of‑Scope (now):
- Structured credit waterfalls (separate crate/feature).
- Portfolio rollups and reporting surfaces.
- Data connectors/feeds and GUIs.

---

## 6) Functional Requirements

### 6.1 Instrument Coverage
- Bonds: fixed/floating with standard calendars/day‑count, ex‑coupon support, call/put schedules.
- Inflation‑linked bonds: indexation via CPI/RPI series with lag/interpolation and deflation floors per policy.
- Loans & facilities: term loans with cash/PIK/toggle coupons; amortization (bullet, linear, fixed %/amount, custom); fees; prepayment and call logic.
- DDTL: commitment/ticking fees, draw rules/conditions, expiry, and post‑draw accruals.
- Revolving credit facilities: commitment/utilization fees, draw/repayment events, and covenant‑aware terms.
- Derivatives: interest rate swaps (fixed/float), CDS (ISDA conventions), and FX spot.
- Options: vanilla equity/FX/rate options with standard exercise types; Greeks for supported models.

### 6.2 Pricing & Measures
- Compute PV/NPV, clean/dirty prices, accrued interest, yields (YTM), spreads (G/I/Z/OAS), and YTW with street tie‑breakers.
- Support street quote adapters to echo desk conventions (settlement, ex‑coupon, format).
- Multi‑curve discounting: OIS for discounting; index‑specific forward curves for projection.
- Credit curve integration for risky PVs and CDS pricing per market standard.
- Option pricing using standard closed‑form models with Greeks where applicable.

### 6.3 Cash Flows & Aggregation
- Build deterministic schedules; tag flows (interest, principal, fees, protection, custom).
- Aggregate by period without mixing currencies; optional explicit conversion to a model/base currency with a documented FX policy.

### 6.4 Market Data & Policies
- Consume discount/forward/credit/vol/price/inflation series with clear identities.
- Explicit fallback policy for missing index fixings (e.g., last observed, policy rate).
- Deterministic margin grids (e.g., leverage/ICR buckets) that adjust spreads at reset.

### 6.5 Risk & Performance
- Produce DV01/CS01, duration, convexity for fixed income and swaps.
- Greeks (delta, gamma, vega, theta, rho) for vanilla options.
- Compute XIRR with robust convergence and clear error cases.

### 6.6 Private Credit: Covenants & Workout
- Evaluate common ratio tests (leverage, interest coverage, fixed‑charge coverage, asset coverage) per period.
- Apply consequences prospectively when breaches are uncured (rate step‑ups, cash sweeps, distribution blocks, default → workout).
- Model workout state machine: standstill, restructuring, default, recovery; emit penalty and recovery flows.

### 6.7 Scenarios & Attributes
- Instruments expose attributes/tags to enable selector‑based scenarios (e.g., rating, sector, seniority).
- Scenario adapters can toggle enforcement, shift thresholds, and change pricing grid buckets deterministically.

### 6.8 Outputs & Interop
- Each valuation returns: instrument id, as‑of date, value (with currency), named measures, optional cashflow schedule, optional covenant reports, and metadata.
- Metadata includes numeric mode, parallel flag, rounding context, and any FX policy applied.
- Provide long‑format, DataFrame‑friendly views for batch analysis; stable serde/JSON shapes across bindings.

### 6.9 Property Cash Flows (Real Estate)
- Rent roll with step‑ups and CPI/RPI indexation (lag/interp, caps/floors); free‑rent windows; renewal probabilities and expected cashflows.
- Operating expenses (fixed and % of rent or area), reimbursements/passthroughs, CAM recoveries with gross‑up policies.
- Property taxes (assessed value × mill rate), exemptions and phase‑ins; optional passthroughs per lease.
- Capex, TI/LC and reserves: dated outflows; reserve accrual/use; policy‑driven capitalization vs expense.
- Currency‑preserving period aggregation across property flows; optional explicit FX collapse stamped in metadata.

### 6.10 Construction Loans (Real Estate Debt)
- Commitment and staged draw schedules with notice/min/max draw rules; ticking/commitment fees on undrawn.
- Interest‑only during construction with interest reserve funding/usage; configurable capitalization of interest and fees.
- Conversion to term loan on substantial completion or a target date; DSCR‑based cash sweep policy hooks supported.
- Fees: origination, inspection, extension; covenants/tests as needed; deterministic schedule generation and tagging.

### 6.11 Deterministic Equity Waterfall Engine
- Serializable waterfall spec with pref IRR hurdles, catch‑up, promote splits, and clawback rules.
- Deterministic, auditable allocation ledger per period and tranche; IRR calculations use robust root finding.
- Stable serde shapes across bindings; DataFrame‑ready outputs for analysis and reporting.

---

## 7) Non‑Functional Requirements

- **Determinism:** Identical Decimal outputs across OS/hosts; parallel vs serial runs yield the same results by default.
- **Currency Safety:** No implicit cross‑currency math; FX requires explicit policy/provider, and policy is visible in outputs.
- **Performance:** Sub‑millisecond pricing for vanilla instruments on reference hardware; scalable vectorized/batch workflows.
- **Stability:** Semver‑governed public schemas; unknown fields rejected unless versioned.
- **Portability:** Rust core with first‑class Python/WASM parity and stable JSON IO.
- **Safety & Security:** No `unsafe` paths in core logic; closed set of deterministic policy hooks.
- **Observability:** Structured tracing around pricing/cashflow/risk; correlation fields for runs/scenarios.

---

## 8) User Experience Requirements

- **Python:** Simple constructors and pricing calls; Pydantic models mirror wire shapes; DataFrame outputs first‑class.
- **WASM:** Small bundles via features; JSON IO mirrors serde names; examples for cashflow previews and basic pricing.
- **Errors:** Clear, contextual messages for missing market data, invalid specs, and convergence failures.
- **Docs & Examples:** Quickstarts for bonds, loans, swaps, options, CDS, covenants/workout, period aggregation, property cash flows, construction loans, and equity waterfalls.

---

## 9) Acceptance Criteria (High‑Level)

- Pricing for core instruments matches trusted references within tight tolerances (e.g., Bloom/QuantLib parity where applicable).
- Currency‑preserving period aggregation validated by property tests; explicit FX collapse policies are stamped in results.
- Private credit features: time‑varying rates, cash/PIK/toggle, amortization types, fee accrual/payment, call/prepayment, DDTL and revolvers behave as specified.
- Yields & quotes: clean/dirty/accrued and YTW calculations match street conventions, including tie‑break rules.
- Risk measures (DV01/CS01/duration/convexity) and option Greeks align with market standards.
- Scenario selectors can toggle covenant enforcement, shift thresholds, and update grid margin buckets deterministically.
- Covenant engine evaluates ratio tests per period with grace/cure windows and applies consequences prospectively; workout flows are emitted correctly.
- Outputs include numeric mode, rounding context, and FX policy when used; long‑format exports work in Python/WASM.
- Deterministic policy hooks (grid margins, index fallbacks, FX policies) behave identically across Rust/Python/WASM.
- Performance targets met for reference workloads; CI covers unit/property/parity tests on core pricing logic.
- Real estate: Property cash flows compute rent, opex/taxes, and reserves correctly with indexation and policies; Construction loans accrue interest/reserve and convert to term as specified; Equity waterfall allocations match Excel/golden models within tolerance and produce an auditable ledger.

---

## 10) Success Metrics

- **Determinism:** Golden tests pass across CI matrices; serial vs parallel parity holds for Decimal mode.
- **Accuracy:** Parity within documented tolerances vs vendor/library benchmarks for bonds/swaps/CDS/options.
- **Throughput:** Median latency <1 ms per vanilla instrument; scalable batch pricing without numeric drift.
- **Adoption:** Majority of example notebooks/scripts run end‑to‑end in <15 minutes for new users.
- **Transparency:** 100% of valuation outputs include metadata for numeric mode, rounding, and FX policy when applicable.

---

## 11) Release Plan (Phased)

- **Phase A — Core pricing + aggregation:** Bonds, swaps, FX spot, equities, basic options; cashflow tagging and period aggregation; quote adapters; long‑format outputs.
- **Phase B — Private credit:** Loans, DDTL, revolvers; fees, amortization, call/prepayment; covenant engine and enforcement toggles.
- **Phase C — Real estate underwriting:** Property cash flows, construction loans (interest reserve, capitalization, conversion), equity waterfall engine with allocation ledgers.
- **Phase D — Risk & derivatives:** CDS pricing, option Greeks, bucketed risk; inflation‑linked bonds; index fallback and grid margin policies.
- **Phase E — Performance & parity:** Benchmarks, parallel portfolio pricing, and vendor/golden parity tests; UX polish and examples.

Each phase ships with examples, docs, and CI acceptance criteria aligned to the above.

---

## 12) Risks & Mitigations

- **Numeric drift across hosts:** Default to Decimal, stable ordering, and explicit rounding context in results.
- **Hidden FX assumptions:** Require explicit conversion APIs; stamp policy metadata; provide FX policy inspection utilities.
- **Scope creep (private credit):** Keep structured credit in a separate feature/crate; constrain policy hooks to deterministic registry functions.
- **Performance regressions:** Maintain Criterion benchmarks and CI guards on core kernels and pricing hot paths.

---

## 13) References

- Technical design: `docs/new/03_valuations/03_valuations_tdd.md`
- Overall requirements: `docs/new/01_overall/01_overall_prd.md`
- Core requirements: `docs/new/02_core/02_core_prd.md`


