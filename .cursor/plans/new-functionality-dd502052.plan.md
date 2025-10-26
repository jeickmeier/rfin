<!-- dd502052-be75-41ee-8184-28a2907fcd67 6276148d-646b-4a68-87d3-43c0ed6ab373 -->
# Forward-Looking Code Review for Quant & Credit UX (New Functionality)

## Overview

Identify missing capabilities across all domains that would improve workflows for buy-side quants, credit analysts (LBO/CRE/Private credit), and portfolio/risk teams. Analysis structured domain-by-domain with pragmatic, finance-native, API-ergonomic recommendations across Rust, Python, and WASM.

## Current State Summary (What Exists Today)

### Core & Market Data

- ✅ Comprehensive calendar system (19 calendars), day-count conventions, business-day logic
- ✅ Hazard curves with piecewise-constant λ(t), survival probability, default probability
- ✅ Discount, forward, inflation, base correlation curves with multiple interpolation styles
- ✅ FX matrix, vol surfaces, dividend curves
- ✅ Curve calibration: discount, forward, hazard (CDS spreads), inflation, base correlation, SABR surfaces

### Instruments & Pricing

- ✅ 20+ instrument types: bonds, swaps (IRS, basis, inflation), CDS/index/tranche/option, FX, equity options, swaptions, cap/floors, repos, TRS, variance swaps, convertibles, structured credit (ABS/RMBS/CMBS/CLO)
- ✅ Bond spreads: Z-spread, OAS, I-spread, discount margin, ASW (par/market, spot/forward)
- ✅ Risk metrics: DV01, CS01 (scalar + bucketed), duration (Macaulay/modified), convexity, Greeks (delta/gamma/vega/rho/theta)
- ✅ Bucketed DV01 (key-rate) with standard IR buckets
- ✅ Private markets fund with equity waterfall (European/American, IRR hurdles, catch-up, clawback)

### Statements & Modeling

- ✅ Full DSL with formulas, forecasting methods (growth, seasonal, stochastic), precedence (Value>Forecast>Formula)
- ✅ Extensions: corkscrew, credit scorecard
- ✅ Capital structure integration with instruments

### Scenarios

- ✅ Parallel/bucketed curve shocks, FX/equity/vol shocks, time roll-forward (theta/carry)
- ✅ Statement forecast adjustments

### Portfolio

- ✅ Entity-based positions, cross-currency aggregation (FxMatrix)
- ✅ Metrics aggregation (summable vs non-summable distinction)
- ✅ Attribute-based grouping, scenario application
- ✅ Polars DataFrame exports (positions, entities)

### Bindings

- ✅ Python (PyO3 + Pydantic v2 stubs), WASM (wasm-bindgen)

## Domain-by-Domain Analysis Plan

### Domain 1: Credit Term Structures & Spread Analytics

**Scope**: spread decomposition, credit risk tools
**Files to analyze**:

- `finstack/core/src/market_data/term_structures/hazard_curve.rs`
- `finstack/valuations/src/calibration/methods/hazard_curve.rs`
- `finstack/valuations/src/instruments/bond/metrics/` (spread metrics)
- `finstack/valuations/src/instruments/cds/` (credit derivatives)

**Missing Capabilities to Identify**:

1. Credit migration matrices (rating transition probabilities)
2. Spread decomposition (liquidity premium, credit risk premium)
3. Benchmark spread analytics (G-spread, T-spread vs existing Z/I/OAS)
4. Forward credit spreads and carry-roll-down attribution

### Domain 2: Waterfalls & Covenant Monitoring

**Scope**: Generalized waterfalls, covenant packs, breach detection
**Files to analyze**:

- `finstack/valuations/src/covenants/engine.rs`
- `finstack/valuations/src/instruments/private_markets_fund/waterfall.rs`
- `finstack/valuations/src/instruments/structured_credit/`

**Missing Capabilities to Identify**:

1. Generalized waterfall engine (beyond private equity) for CLOs, CMBS, operating company cash
2. Covenant test forward-projection with headroom/cushion analytics
3. Covenant breach consequence automation (beyond engine hooks)
4. Debt service coverage ratio (DSCR) with trailing/projected variants
5. LTV calculations with multiple collateral types and valuation policies

### Domain 3: Portfolio Analytics & Risk Aggregation

**Scope**: Attribution, concentration, limit checks, marginal risk
**Files to analyze**:

- `finstack/portfolio/src/metrics.rs`
- `finstack/portfolio/src/grouping.rs`
- `finstack/portfolio/src/results.rs`
- `finstack/scenarios/src/`

**Missing Capabilities to Identify**:

1. Performance attribution (carry, roll-down, spread, curve, selection/mix effects)
2. Concentration & limit framework (issuer, sector, rating, maturity buckets)
3. Marginal and incremental risk contributions (marginal DV01, incremental CS01)
4. Risk ladder views (maturity walls, KRD ladders, exposure ladders)
5. Scenario grid runner (sweep over multiple parameter combinations)
6. Top-N risk contributors (largest DV01, CS01, notional)

### Domain 4: Data I/O & Interop

**Scope**: Arrow/Parquet, importers, exporters, schema stability
**Files to analyze**:

- `finstack/io/src/lib.rs`
- `finstack/portfolio/src/dataframe.rs`
- `finstack-py/` (Python bindings)
- `finstack-wasm/` (WASM bindings)

**Missing Capabilities to Identify**:

1. First-class Parquet I/O with schema versioning
2. Term sheet / deal document parsers (structured YAML/JSON templates)
3. Rating history importers (S&P/Moody's/Fitch formats)
4. TRACE/EMMA schema adapters (user provides data, library provides schema)
5. Excel-friendly exporters with formatting/units/headers

### Domain 5: Bindings & DX

**Scope**: Python/WASM ergonomics, observability, repro
**Files to analyze**:

- `finstack-py/finstack/` (.pyi stubs)
- `finstack-wasm/src/`
- `finstack/*/src/` (core APIs)

**Missing Capabilities to Identify**:

1. Rich Python docstrings with examples (not just type stubs)
2. `py.typed` marker + stub validation
3. WASM JSON-Schema for configs and results
4. `explain()` infrastructure (curve calibration, pricing, waterfalls, covenant logic)
5. Run metadata stamping (versions, seeds, curve IDs, timestamps)
6. Progress reporting (tqdm-friendly for Python)

## Deliverable Structure (Per Domain)

For each domain, produce:

1. **Executive Summary**: 3-5 most impactful missing features
2. **Feature Proposals** (ranked by impact × effort):

- Title, Persona Pain, User Story
- Scope: Data/Models/APIs (Rust/Python/WASM signatures only)
- Explainability approach
- Impact/Effort (P0-P2, S/M/L), Dependencies, Risks
- Demo outline (notebook sketch)

3. **Quick Wins**: 3-5 low-effort enhancements
4. **De-Dup Check**: Evidence of absence (paths searched, partial implementations noted)

## Output Files

Create a single comprehensive document: `NEW_FUNCTIONALITY_REVIEW.md` with:

- Executive summary across all domains (10 bullets, impact/effort table)
- Domain 1: Credit Term Structures & Spread Analytics
- Domain 2: Waterfalls & Covenant Monitoring
- Domain 3: Portfolio Analytics & Risk Aggregation
- Domain 4: Data I/O & Interop
- Domain 5: Bindings & DX
- Cross-domain recommendations (features that span multiple areas)
- Final prioritized ranking using rubric (User Impact, Breadth, DX Quality, Perf, Cross-Bindings, Validation)

## Search Strategy

For each domain:

1. Read relevant implementation files to understand current capabilities
2. Search for partial implementations or TODOs
3. Check examples/tests for usage patterns and gaps
4. Cross-reference bindings to identify API surface gaps
5. Document evidence of absence (grep results, file listings)

- **User Impact**: saves analyst time or unlocks blocked workflow
- **Breadth**: useful across loans, bonds, CDS, CRE/LBO
- **DX Quality**: simple APIs, great errors, DataFrame-native, explain()
- **Perf & Scale**: batch-friendly; memory/time predictable
- **Cross-Bindings Fit**: Rust core + Python + TS minimal impedance
- **Validation Readiness**: testability vs known results

Total score = sum (max 30). Rank proposals top-down.

### To-dos

- [ ] Analyze Domain 1: Credit Term Structures & Spread Analytics (ECL/CECL, spread decomposition, migration matrices)
- [ ] Analyze Domain 2: Waterfalls & Covenant Monitoring (generalized waterfalls, covenant forward-testing, DSCR/LTV)
- [ ] Analyze Domain 3: Portfolio Analytics & Risk Aggregation (attribution, concentration, marginal risk)
- [ ] Analyze Domain 4: Data I/O & Interop (Parquet, importers, Excel exporters)
- [ ] Analyze Domain 5: Bindings & DX (Python docstrings, explain() infrastructure, metadata stamping)
- [ ] Synthesize cross-domain recommendations and create final prioritized ranking
- [ ] Create NEW_FUNCTIONALITY_REVIEW.md with all findings, impact/effort table, and prioritized recommendations