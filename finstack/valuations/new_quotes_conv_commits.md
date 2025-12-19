### Title
**V3 Quotes + Conventions + Calibration Pricing — Step-by-step commit plan**

### Goal
This document is a **commit-by-commit rollout** for implementing `finstack/valuations/new_quotes_conv.md` end-to-end, keeping the repo **buildable** throughout and converging to the state where the legacy modules are deleted.

### Guardrails
- **Always strict**: convention lookups must error (no silent defaults / fallbacks).
- **Calibration never prices**: no pricing formulae in calibration; only instruments do pricing.
- **Residuals are `f64`**: solver targets call `Instrument::value_raw`.
- **Prepared quotes own the time axis**: `PreparedQuote` exposes `pillar_time` (and optionally `pillar_date`) so adapters don’t inspect raw quote enums.
- **CI cadence**:
  - After each group of commits (or any non-trivial Rust change): run `make lint-rust` and `make test-rust`.
  - After binding-affecting commits: run `make python-dev` and/or `make wasm-build`, then `make test-python` / `make test-wasm` if applicable.

---

## Commit series

### Commit 01 — Scaffold `market/` module (no behavior change) - COMPLETE
- **Message**: `valuations: scaffold market module skeleton for V3`
- **Changes**:
  - Add `finstack/valuations/src/market/mod.rs`
  - Add `finstack/valuations/src/market/{quotes,conventions,build}/mod.rs`
  - Wire `finstack/valuations/src/lib.rs` to re-export `market` (feature-gated if needed, but simplest is always-on).
- **Notes**:
  - No existing code is modified besides module exports.
- **Run**: `make lint-rust && make test-rust`

### Commit 02 — Define stable IDs (typed keys) + `Pillar` - COMPLETE
- **Message**: `valuations(market): add QuoteId + Pillar + typed convention keys`
- **Changes**:
  - Create `market/quotes/ids.rs`:
    - `QuoteId` newtype (string) with serde stable shape.
    - `Pillar` enum: `Tenor | Date`.
  - Create `market/conventions/ids.rs`:
    - `IndexId` (if not already defined as a stable type elsewhere; otherwise reuse existing type).
    - `CdsConventionKey { currency, doc_clause }` and `CdsDocClause`.
    - `IrFutureContractId` (string/newtype).
- **Notes**:
  - Prefer **typed keys** for DB upserts; keep `QuoteId` as human/serialization id.
- **Run**: `make lint-rust && make test-rust`

### Commit 03 — ConventionRegistry API (in-memory + global)  - COMPLETE
- **Message**: `valuations(market): add ConventionRegistry with strict require() API`
- **Changes**:
  - Add `market/conventions/registry.rs`:
    - `ConventionRegistry { rate_index, cds, ir_futures, … }` (start with rates + cds).
    - `ConventionRegistry::new(...)` that accepts in-memory maps/records (DB-ready).
    - `ConventionRegistry::global()` (OnceLock) using embedded JSON loaders.
    - `require_*()` methods returning `Result<&'static …>` (or `&…` for non-static registry instances).
- **Notes**:
  - Keep error messages actionable and pointing to `valuations/data/conventions/*.json`.
- **Run**: `make lint-rust && make test-rust`

### Commit 04 — Port embedded JSON loaders into `market/conventions` (strict) - COMPLETE
- **Message**: `valuations(market): port embedded conventions JSON loaders (strict)`
- **Changes**:
  - Create `market/conventions/loaders/{rate_index,cds,ir_futures}.rs`
  - Move/duplicate the existing registry-file parsing helpers (currently under `calibration/quotes/json_registry.rs`) into `market/conventions` (or re-use via a small `pub(crate)` shared helper).
  - Ensure:
    - embedded JSON parse errors are `expect` in `global()` init (developer error)
    - missing keys are runtime `Error::NotFound` / `Error::Validation` (caller error)
- **Run**: `make lint-rust && make test-rust`

### Commit 05 — BuildCtx (wiring-only) + “prepared” envelope (generic) - COMPLETE
- **Message**: `valuations(market): add BuildCtx and PreparedQuote envelope`
- **Changes**:
  - Add `market/build/context.rs`: `BuildCtx { as_of, notional, curve_ids, attributes, … }`
  - Add `market/build/prepared.rs`: `PreparedQuote<Q> { quote: Arc<Q>, instrument: Arc<dyn Instrument>, pillar_date, pillar_time }`
  - Ensure `PreparedQuote` is plain data; no pricing.
- **Notes**:
  - `PreparedQuote` must be scoped to an `as_of` and rebuilt if `as_of` changes.
- **Run**: `make lint-rust && make test-rust`

### Commit 06 — V3 Rates quotes (serde-stable) - COMPLETE
- **Message**: `valuations(market): add RatesQuoteV3 schemas (ID-based)`
- **Changes**:
  - Add `market/quotes/rates.rs` defining the minimal set for curve calibration:
    - deposit rate
    - FRA rate
    - IR future price
    - swap par rate (OIS + term)
    - basis swap spread
  - Enforce `deny_unknown_fields`.
- **Run**: `make lint-rust && make test-rust`

### Commit 07 — V3 Credit quotes (CDS par + upfront)
- **Message**: `valuations(market): add CreditQuoteV3 schemas (CDS par + upfront)`
- **Changes**:
  - Add `market/quotes/credit.rs`
  - Include `CdsConventionKey` and explicit `recovery_rate`.
- **Run**: `make lint-rust && make test-rust`

### Commit 08 — Instruments: add precise `value_raw` for calibration instruments - COMPLETE
- **Message**: `valuations(instruments): implement value_raw without Money rounding for calibration instruments`
- **Changes** (start with rates instruments used in calibration):
  - `instruments/deposit/*`
  - `instruments/fra/*`
  - `instruments/irs/*`
  - `instruments/basis_swap/*`
  - `instruments/ir_future/*`
  - Add/override `Instrument::value_raw` to return unrounded `f64` directly from pricing engine internals.
- **Notes**:
  - This commit is purely about precision and solver stability.
- **Run**: `make lint-rust && make test-rust`

### Commit 09 — Instruments: strict futures convexity (remove fallback volatility) - IN PROGRESS
- **Message**: `valuations(ir_future): enforce strict convexity requirements (no fallback vol)`
- **Changes**:
  - Update `instruments/ir_future/*` so convexity is:
    - `Fixed { adj }` or
    - `Model { … }` requiring explicit vol surface ID / model inputs
  - If a model spec requires market data and it is missing: return error.
- **Run**: `make lint-rust && make test-rust`

### Commit 10 — Instruments: CDS upfront becomes part of the instrument - COMPLETE
- **Message**: `valuations(cds): add upfront payment support inside CreditDefaultSwap`
- **Changes**:
  - Update `instruments/cds/types.rs` to include an upfront cashflow/payment field.
  - Ensure `npv()` and `value_raw()` include the upfront in a sign-consistent way.
  - Remove any need for calibration-side “upfront subtraction”.
- **Run**: `make lint-rust && make test-rust`

### Commit 11 — Quote→Instrument builders: rates - COMPLETE
- **Message**: `valuations(market/build): implement RatesQuoteV3::to_instrument (strict)`
- **Changes**:
  - Implement `to_instrument()` for `RatesQuoteV3`:
    - Resolve conventions via `ConventionRegistry` (strict require).
    - Construct instruments with correct calendars/lags/day-counts.
    - Determine pillar date/time (incl. payment delay where appropriate) and populate `PreparedQuote`.
- **Notes**:
  - No solver logic here; only deterministic schedule/config construction.
- **Run**: `make lint-rust && make test-rust`

### Commit 12 — Quote→Instrument builders: CDS par + upfront - COMPLETE
- **Message**: `valuations(market/build): implement CreditQuoteV3::to_instrument (strict)`
- **Changes**:
  - Build CDS instruments using `CdsConventionKey` lookup.
  - For upfront quotes: set the instrument’s upfront field (no external adjustments).
- **Run**: `make lint-rust && make test-rust`

### Commit 13 — Calibration: introduce V3 prepared quote types (parallel path) - COMPLETE
- **Message**: `valuations(calibration): add V3 prepared quote types and adapters (parallel)`
- **Changes**:
  - Add `calibration/v3/` (or `calibration/market/`) containing:
    - conversion from plan input (quotes list) → `PreparedQuoteV3`
    - minimal glue to feed existing solvers
  - Keep legacy calibration untouched; V3 is additive.
- **Notes**:
  - Use the existing solvers (`BootstrapTarget`, `GlobalSolveTarget`) but adapt them to consume V3 prepared quotes.
- **Run**: `make lint-rust && make test-rust`

### Commit 14 — Calibration: refactor `DiscountCurveTarget` to use `pillar_time` from PreparedQuote - COMPLETE
- **Message**: `valuations(calibration): use prepared pillar_time in discount calibration`
- **Changes**:
  - Update discount adapter(s) to stop pattern-matching on raw quote variants for time mapping.
  - Use `prepared.pillar_time` and `instrument.value_raw()` for residuals.
- **Run**: `make lint-rust && make test-rust`

### Commit 15 — Calibration: same refactor for forward/hazard/inflation (as implemented in V3 scope) - COMPLETE
- **Message**: `valuations(calibration): use prepared pillar_time + value_raw across adapters`
- **Changes**:
  - Apply the same “prepared owns time axis” rule to the other calibration adapters you migrate in this rollout.
- **Run**: `make lint-rust && make test-rust`

### Commit 16 — Parity harness: run V2 and V3 side-by-side for selected cases - COMPLETE
- **Message**: `valuations(calibration): add V2/V3 parity tests for curve calibration`
- **Changes**:
  - Add tests that:
    - build identical market inputs
    - run V2 calibration and V3 calibration
    - assert residuals/curves match within tolerance
- **Run**: `make lint-rust && make test-rust`

### Commit 17 — Switch calibration engine to V3 (breaking change flip) - COMPLETE
- **Message**: `valuations(calibration): switch calibration engine to V3 quote→instrument path`
- **Changes**:
  - Route calibration plan execution to V3 pathway.
  - Keep V2 code temporarily but unused.
- **Run**: `make lint-rust && make test-rust`

### Commit 18 — Delete legacy calibration pricing stack - COMPLETE
- **Message**: `valuations(calibration): delete legacy quote/pricer/factory modules`
- **Deletes**:
  - `calibration/quotes/*`
  - `calibration/pricing/pricer/*`
  - `calibration/pricing/convention_resolution.rs`
  - `calibration/pricing/quote_factory.rs`
  - Update module exports and imports accordingly.
- **Run**: `make lint-rust && make test-rust`

### Commit 19 — Update public API surfaces (schemas / docs / examples)
- **Message**: `valuations: update calibration schema + docs to V3 quotes`
- **Changes**:
  - `calibration/api/schema.rs` (or replacement) to accept V3 quote shapes.
  - Update README/docs/examples referencing old quote types.
- **Run**: `make lint-rust && make test-rust`

### Commit 20 — Python bindings update (breaking)
- **Message**: `finstack-py: update bindings to V3 quote schemas`
- **Changes**:
  - Update `finstack-py/src/valuations/*` bindings for new quote types / registry access.
  - Update python tests expecting old quote enums.
- **Run**:
  - `make python-dev`
  - `make lint-python && make test-python`

### Commit 21 — WASM bindings update (breaking)
- **Message**: `finstack-wasm: update bindings to V3 quote schemas`
- **Changes**:
  - Update `finstack-wasm/src/valuations/*` and TS bindings if exported.
- **Run**:
  - `make wasm-build`
  - `make lint-wasm && make test-wasm`

### Commit 22 — Cleanup + stabilization
- **Message**: `valuations: cleanup, tighten errors, and stabilize serde shapes`
- **Changes**:
  - Remove any leftover deprecated exports.
  - Ensure all serde types have `deny_unknown_fields` and stable `rename_all`.
  - Improve error messages for missing conventions.
- **Run**: `make lint-rust && make test-rust`

---

## Optional follow-ups (separate PR / after V3 lands)
- Add a typed **market snapshot envelope** for DB storage (`MarketSnapshot { as_of, quotes, conventions_ref, metadata }`) in the IO crate and wire into `ConventionRegistry::new(...)`.
