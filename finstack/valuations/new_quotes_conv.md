### Title
**Quotes + Conventions + Calibration Pricing V3 (Replace `calibration/quotes` + `calibration/pricing/pricer`)**

### Status
Draft (breaking-change redesign)

### Motivation
The current design spreads “what is quoted” and “how to price it” across:
- quote enums with ad-hoc optional `InstrumentConventions`
- multiple JSON registries with implicit fallback (sometimes `"DEFAULT"`, sometimes `panic!`)
- pricer-side logic (`CalibrationPricer`, `convention_resolution`, futures convexity handling)
- a separate `quote_factory` that builds “temporary” instruments

This violates the desired invariants:
- **No hidden defaults**: missing market conventions must error.
- **All pricing logic in instruments**: calibration should not contain instrument pricing formulae or overrides.
- **Quote → (resolve conventions) → instrument** should be the only construction path used by calibration.

### Design goals (requirements)
- **Standard quote definitions** for traded instruments: deposits, FRAs, futures, swaps, basis swaps, CDS (par + upfront), inflation swaps.
- **Conventions come from embedded JSON registries** under `valuations/data/conventions/*.json`.
  - If convention lookup fails: **error** (no silent fallback).
- **Quote + conventions build a concrete `Instrument`** (or a calibratable wrapper) with no hard-coded parameters.
- **Calibration uses only instrument methods** (primarily `Instrument::value_raw`) to compute residuals.
- **Registries are accessible from instruments and calibration** through a single public API.
- **Convention access uses market IDs** (e.g., `IndexId` like `USD-SOFR-OIS`; CDS `(Currency, DocClause)`).
- **Extendable** to inflation, CDS tranches, base correlation, vol surfaces, etc.
- **Highly performant**: instruments pre-built; minimal allocations; avoid clones/copies in hot solver loops.
- **Breaking changes allowed** (no backwards compat).
- **Always strict**: No fallback to "DEFAULT" keys.
- **Pillar flexibility**: Standardization prefers `Tenor` (e.g., "5Y") for OTC instruments but explicitly allows `Date` for specific maturities (IMM dates, bespoke runs).

---

## Core principle: calibration never “prices”
Calibration is allowed to:
- build candidate curves/surfaces
- update a `MarketContext`
- call **instrument-level pricing methods** to obtain residuals

Calibration is **not allowed** to:
- decide day counts, calendars, payment lags, reset lags
- apply convexity adjustments / quote transformations
- special-case “upfront subtraction” outside the instrument

---

## Proposed architecture

### New top-level modules (inside `finstack/valuations`)
- `src/market/`
  - `market/quotes/` — **standard market quote schemas** (serde-stable)
  - `market/conventions/` — **JSON-backed convention registries** + typed IDs
  - `market/build/` — glue types for quote→instrument construction (thin, non-pricing)

### Removed / deprecated modules (replaced by V3)
- `calibration/quotes/*`
- `calibration/pricing/pricer/*`
- `calibration/pricing/convention_resolution.rs`
- `calibration/pricing/quote_factory.rs`

Calibration adapters keep their numerical logic, but will depend on:
- `market::quotes::*`
- `market::conventions::ConventionRegistry`
- quote-built instruments (prepared once)

---

## Data model

### 1) IDs and “pillars”
To avoid hardcoding dates in every quote and to standardize market inputs, introduce:

- `QuoteId`: stable human/serialization identifier (string, e.g. `"USD-OIS:SWAP:5Y"`).
- `Pillar`: how the market identifies maturity:
  - `Tenor(Tenor)` (preferred for swaps/deposits, enables rolling headers)
  - `Date(Date)` (allowed for IMM-specific, bespoke maturities, or fixed-end runs)

Calibration will compute:
- `pillar_date(as_of, conventions)` → `Date`
- `pillar_time(as_of, curve_day_count)` → `f64`

This allows:
- portability across valuation dates
- consistent time-axis derivation
- precomputation for performance

**Note**: `PreparedQuote` containing these precomputed values is ephemeral and scoped to a specific `as_of` date (or single calibration run). If the valuation date changes, prepared quotes must be rebuilt.

### 2) Convention IDs (typed, market-native)
- **Rates**: `IndexId` (e.g. `USD-SOFR-OIS`, `USD-SOFR-3M`)
- **IR futures**: `IrFutureContractId` (e.g. `CME:SR3` or `CME:SOFR-3M`) + `Pillar`/expiry
- **CDS**: `CdsConventionKey { currency: Currency, doc_clause: CdsDocClause }`
- **Inflation**: `InflationIndexId` (string/typed) + `region` if needed
- **Vol**: underlying id + quoting style id (later)

**Rule**: a quote must carry enough IDs to deterministically resolve conventions.

### 3) Quotes (schemas)
Replace the current “quotes with optional `InstrumentConventions` overrides” with **quotes that reference conventions by ID**.

Example (conceptual):
- `RatesQuoteV3::DepositRate { id, index: IndexId, pillar: Pillar, rate }`
- `RatesQuoteV3::SwapParRate { id, float_index: IndexId, pillar: Pillar, fixed_leg: FixedLegStyleId?, rate }`
- `CreditQuoteV3::CdsParSpread { id, entity, convention: CdsConventionKey, pillar: Pillar, spread_bp, recovery_rate }`
- `CreditQuoteV3::CdsUpfront { id, entity, convention: CdsConventionKey, pillar: Pillar, running_spread_bp, upfront_pct, recovery_rate }`

**No per-quote implicit defaults**. Any “override” facility (if we keep it) must be explicit and exhaustive:
- `Overrides` are optional, but if present must be complete for the overridden fields and must not cause fallback.

---

## Convention registry design

### 1) Single public entry point
`market::conventions::ConventionRegistry::global()` returns a `&'static ConventionRegistry` built once via `OnceLock`.

### 2) Strict lookup API (no fallback)
Every resolver is strict:
- `registry.rate_index.require(&IndexId) -> Result<&'static RateIndexConventions>`
- `registry.cds.require(&CdsConventionKey) -> Result<&'static CdsConventions>`
- etc.

**No “DEFAULT” fallback** inside resolvers.

### 3) JSON format and embedding
Continue using the existing `RegistryFile` pattern (ids + record), but:
- registries live under `market/conventions/*`
- parsing returns `Result` (no panics in runtime paths)
- embedded JSON errors are treated as “developer error” (can `expect` in `global()` init), but missing ID resolution is a normal `Error::NotFound`.

### 4) Registry scope and reuse
Registries must be callable from:
- instruments (during build)
- calibration (during plan validation/build)
- python/wasm bindings (read-only access, if desired)
- **Tests**: The builder API accepts `&ConventionRegistry` to allow tests to inject mock registries or modified conventions without polluting the global singleton.

### 5) Database & IO future-proofing
To ensure smooth integration with database storage and the future IO crate:

1.  **Registry construction**: `ConventionRegistry` must expose a constructor accepting raw data maps (e.g. `new(rates_map, cds_map, ...)`) in addition to the file-based loader. This allows the IO crate to populate the registry from database records without intermediate files.
2.  **Schema versioning**: All convention JSON structures and `MarketQuote` serialization schemas must be designed with forward compatibility in mind. Consider adding an optional `schema_version` field to top-level records.
3.  **Traceability**: `MarketQuote` and convention records should support an optional `metadata: HashMap<String, String>` field to store provenance data (e.g., "source_system", "snapshot_id", "last_updated") that is preserved but ignored by pricing logic.
4.  **Stable serialization**: Use `#[serde(rename_all = "snake_case")]` and explicit field renames to decouple Rust field names from database/JSON keys, ensuring persistent storage stability.

---

## Quote → Instrument construction

### 1) Build context (wiring, not pricing)
A small struct passed into builders:
- `as_of: Date`
- `curve_ids`: (discount/forward/hazard/inflation…) depending on calibration step
- `notional`: calibration notional (comes from plan/config; no hard-coded `1_000_000`)
- optional: `attributes`/tags

### 2) Builder trait
Each quote type implements:

- `fn to_instrument(&self, registry: &ConventionRegistry, ctx: &BuildCtx) -> Result<Arc<dyn Instrument>>`

This function:
- resolves conventions strictly
- constructs a concrete instrument configured so that **Residual ≈ 0 when curves match the quote**
- does not do any pricing math beyond deterministic date/schedule construction

### 3) Pricing logic lives in instruments
Any logic that affects valuation must be in the instrument implementation:
- OIS compounding conventions (already in IRS instrument)
- futures convexity adjustment model (move out of calibration pricer)
- **CDS Upfront**: Explicitly add `upfront_payment` support to the `CreditDefaultSwap` instrument struct and pricing engine. This allows calibration to treat it as a standard instrument rather than handling upfronts externally.
- calendar fallback policy (if allowed at all) must be an explicit convention field, not calibration behavior

---

## Performance plan
- **Prepare once**: calibration creates `PreparedQuote { quote: Arc<Quote>, instrument: Arc<dyn Instrument>, pillar_time: f64 }`.
  - **Scope**: `PreparedQuote` is valid only for the `as_of` date used during construction.
  - **Decoupling**: `PreparedQuote` MUST expose `pillar_time` (and optionally `pillar_date`) directly. Solver adapters (e.g., `DiscountCurveTarget`) must rely on this precomputed value instead of inspecting the raw quote type to derive times or lags.
- **No allocations in residual loop**:
  - no formatting IDs
  - no rebuilding instruments
  - no convention resolution
- **Target residuals**: Use raw `f64` (via `Instrument::value_raw`) instead of `Money`.
  - Solvers should converge on `f64` zero (or target quote). `Money` rounding introduces noise that can destabilize high-precision solvers.
- Use `Arc` for cheap clones across solver data structures.
- Precompute `pillar_time` so solvers don’t need to call date logic repeatedly.

---

## Rates: unify conventions on `RateIndexConventions`
Today rates conventions are split across:
- deposit conventions registry
- FRA conventions registry
- rate index conventions registry

V3 consolidates rates conventions under a single registry keyed by `IndexId`:
- `RateIndexConventions` becomes the source of truth for:
  - calendars, settlement, BDC
  - day count for floating index accrual
  - default fixed leg conventions for swaps
  - payment delays, reset lags
  - OIS compounding spec

This reduces duplication and makes “simple ID = index” the standard access path.

---

## Credit: CDS conventions as first-class market keys
- `cds_conventions.json` already uses `(Currency, DocClause)`-style ids like `"USD:IsdaNa"`.
- V3 formalizes this as a typed key and removes fallback behavior.
- CDS upfront is priced by a dedicated instrument (or a CDS instrument that includes upfront cashflow):
  - calibration residual is simply `instrument.value_raw(...) / notional` (no external subtraction)

---

## Futures: move convexity logic into the instrument
Current state: convexity is resolved in calibration pricer / factory.
V3:
- the futures contract convention record contains a convexity specification:
  - `None`
  - `Fixed { adj }`
  - `Model { model_params..., vol_source_id... }`
- the `InterestRateFuture` instrument reads the spec and:
  - either uses fixed adjustment
  - or pulls required market inputs from `MarketContext`
  - or errors if missing

Calibration never computes convexity.

---

## Calibration integration changes (high-level)
- Replace use of `CalibrationPricer` pricing entry points.
- Replace `quote_factory` with `quote.to_instrument(registry, ctx)`.
- `prepared` remains, but becomes generic across asset classes:
  - `PreparedQuote<Q>` where `Q: MarketQuote`
  - **Decoupling**: `PreparedQuote` should expose helper methods (e.g., `guess_df()` or `is_ois_suitable()`) to provide solver hints without forcing adapters to inspect raw quote enums, maintaining the "generic instrument" abstraction.
- Adapters call:
  - `prepared.instrument.value_raw(&ctx, as_of)`
  - scale/normalize only (if needed)

Anything like “swap pillar time uses payment delay” becomes:
- computed by quote builder (instrument exposes “pillar date”), or
- computed by a shared `Pillar` + conventions helper in `market/build`

---

## Validation rules (strict, deterministic)
- Every quote must resolve its convention IDs at plan build time.
- Any missing convention ID fails fast with actionable errors:
  - “Missing rate index conventions for ‘USD-SOFR-OIS’. Add to …/rate_index_conventions.json”
- Quotes must be internally consistent:
  - swap float index required
  - CDS doc clause required (or explicit “market standard” per currency encoded as an ID you must choose)
  - futures must have contract id and expiry/pillar sufficient to build schedule

---

## Migration plan (breaking changes)
- **Phase 0 (Prerequisite)**:
  - Update `CreditDefaultSwap` to support `upfront_payment` internally.
  - Implement explicit `value_raw` for all calibration instruments (avoid `Money` rounding).
  - Enforce strict futures convexity (remove hardcoded fallback vol and require explicit `Fixed` or `Model` spec).
  - (Optional) Add a `FairValue` trait for quoting-space residuals where it materially improves convergence.
  - Update `market::conventions::ConventionRegistry` and mirror existing JSONs there.
- **Phase 1**: Add `market::quotes::*V3` and implement quote→instrument builders for rates + CDS.
- **Phase 2 (Parity Check)**: Implement V3 logic in parallel with V2 in a test harness. Run a subset of calibration cases through both paths and assert `abs(v2_result - v3_result) < epsilon` to verify correctness before switching consumers.
- **Phase 3**: Update calibration adapters to accept `PreparedQuoteV3` and remove `CalibrationPricer` usage.
- **Phase 4**: Delete `calibration/quotes`, `calibration/pricing/pricer`, `convention_resolution`, `quote_factory`.
- **Phase 5**: Migrate inflation and futures; remove pricer-side convexity and settlement logic completely.
- **Phase 6**: Update python/wasm bindings to new quote schemas and registry accessors.

---

## Decisions made
- **Money vs scalar residuals**: Use scalar `f64` (via `value_raw`) to avoid `Money` rounding noise in solvers.
- **Pillar standardization**: Standardize on `Tenor` for OTC instruments to enable rolling headers, but allow `Date` for specific runs (IMM, etc.).
- **Calendar fallback**: Strict policy. No fallbacks. Missing calendars/conventions are errors.
- **Notional policy**: Store per-calibration-step notional in plan schema and pass via `BuildCtx`.

---

## Summary
V3 makes the system simple and strict:
- **Quotes reference market-standard convention IDs**
- **Conventions live in one strict registry**
- **Quotes build instruments once**
- **Instruments own all pricing logic**
- **Calibration just builds candidate curves and calls `instrument.value_raw`**
