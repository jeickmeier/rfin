# Market Bootstrap Phase 3 — IDE Autocomplete

**Status:** Draft
**Date:** 2026-05-08
**Owner:** finstack/valuations + finstack-wasm
**Phase:** 3 of 5 (autocomplete)
**Depends on:** Phase 1 foundation (independent of Phase 2)
**Related specs:**
- Phase 1 — Canonical-path foundation: [2026-05-08-market-bootstrap-phase-1-foundation-design.md](2026-05-08-market-bootstrap-phase-1-foundation-design.md)
- Phase 2 — Reference catalog: [2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md](2026-05-08-market-bootstrap-phase-2-reference-catalog-design.md)
- Phase 4 — Diagnostics: [2026-05-08-market-bootstrap-phase-4-diagnostics-design.md](2026-05-08-market-bootstrap-phase-4-diagnostics-design.md)
- Phase 5 — Python TypedDict (fast-follow): [2026-05-08-market-bootstrap-phase-5-fast-follow-design.md](2026-05-08-market-bootstrap-phase-5-fast-follow-design.md)

## 1. Motivation

`CalibrationEnvelope` is JSON-as-string at every binding boundary. Without external schema or type definitions, users get no autocomplete or validation in their editors when building envelopes. This phase ships JSON Schema for universal editor support and TypeScript types for JavaScript users — both derived from the same Rust source of truth.

Python TypedDicts are deferred to Phase 5 fast-follow.

## 2. Goals

- A JSON Schema is generated from the Rust `CalibrationEnvelope` type and shipped in the repo. Reference envelopes opt into it via `$schema`.
- TypeScript declarations for `CalibrationEnvelope` and `CalibrationResultEnvelope` are generated and re-exported from `finstack-wasm/index.d.ts`.
- The WASM JS facade exposes a typed `calibrate(envelope: CalibrationEnvelope | string): CalibrationResultEnvelope` wrapper.
- Drift between the Rust types and generated artifacts is caught in CI.

## 3. Non-Goals

- Python TypedDicts (Phase 5).
- Pydantic models (out of scope; users can layer on top if needed).
- Schema versioning automation. The schema URL includes a version segment (`/calibration/1/`), but bumping it is manual.

## 4. Architectural Baseline

The workspace already depends on `schemars 1.2` ([Cargo.toml:56](../Cargo.toml)) with `rust_decimal1` and `indexmap2` features, and uses `ts_rs` behind a `ts_export` feature ([finstack/valuations/src/market/quotes/market_quote.rs:15-16](../finstack/valuations/src/market/quotes/market_quote.rs)). Both are existing tooling; this phase extends their reach to the calibration envelope types. The repo also ships JSON schemas under [finstack/valuations/schemas/instruments/1/](../finstack/valuations/schemas/instruments/) — Phase 3 adds a peer `calibration/1/` subtree.

## 5. Scope — file-by-file

### 5.1 Layer 1 — JSON Schema generation

Verify and extend `schemars::JsonSchema` derives across:

- `CalibrationEnvelope`, `CalibrationPlan`, `CalibrationStep`, `StepParams` (and all variants), `MarketQuote` (and all variants — `MarketQuote` already derives), `CalibrationConfig`, `MarketContextState` (already may have it).
- `CalibrationResultEnvelope`, `CalibrationResult`, `CalibrationReport` for completeness.

Generation pipeline. Two acceptable forms; pick one during implementation:

- **Option A: cargo bin** — new `finstack/valuations/src/bin/gen-calibration-schemas.rs` runs `schemars::schema_for!(CalibrationEnvelope)` and writes JSON to disk. Invoked by `cargo run --bin gen-calibration-schemas`.
- **Option B: snapshot test** — a `#[test]` in `tests/schemas.rs` reads the committed schema, regenerates, and asserts equal. Failure prints a one-line fix command.

Either way, output paths:

- `finstack/valuations/schemas/calibration/1/envelope.schema.json`
- `finstack/valuations/schemas/calibration/1/result_envelope.schema.json`

The schemas have stable `$id` URLs (e.g., `"$id": "finstack.calibration/1/envelope.schema.json"`) so they can be referenced cross-project.

Reference envelopes (Phases 1 and 2) gain a `"$schema": "../../../schemas/calibration/1/envelope.schema.json"` field at the top.

[`.vscode/settings.json.example`](../.vscode/settings.json.example) (or README documentation):

```json
{
  "json.schemas": [
    {
      "fileMatch": ["**/examples/market_bootstrap/*.json", "**/*.calibration.json"],
      "url": "./finstack/valuations/schemas/calibration/1/envelope.schema.json"
    }
  ]
}
```

### 5.2 Layer 2 — TypeScript types

Extend the `ts_export` feature in [finstack/valuations/Cargo.toml](../finstack/valuations/Cargo.toml) and the relevant types:

- Add `#[cfg_attr(feature = "ts_export", derive(TS))]` and `#[cfg_attr(feature = "ts_export", ts(export))]` to `CalibrationEnvelope`, `CalibrationPlan`, `CalibrationStep`, `StepParams`, etc., matching the existing pattern in `MarketQuote` ([finstack/valuations/src/market/quotes/market_quote.rs:45-49](../finstack/valuations/src/market/quotes/market_quote.rs)).
- Generate TS bindings to `finstack-wasm/types/calibration/`.
- Re-export from [finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts):

  ```ts
  export type {
    CalibrationEnvelope,
    CalibrationResultEnvelope,
    CalibrationStep,
    StepParams,
    MarketQuote,
  } from './types/calibration';
  ```

- Update existing function declarations:

  ```ts
  export function calibrate(envelope: CalibrationEnvelope | string): CalibrationResultEnvelope;
  export function validateCalibrationJson(json: string): string;
  ```

- Implement the typed wrapper in [finstack-wasm/exports/valuations.js](../finstack-wasm/exports/valuations.js):

  ```js
  import { calibrate as rawCalibrate, validateCalibrationJson } from '../pkg/finstack_wasm';

  export function calibrate(envelope) {
    const json = typeof envelope === 'string' ? envelope : JSON.stringify(envelope);
    return JSON.parse(rawCalibrate(json));
  }

  export { validateCalibrationJson };
  ```

  (Object-in / object-out is purely a JS-facade convenience; the wasm-bindgen boundary is still string-typed.)

### 5.3 CI drift checks

Add to the CI pipeline:

- A test or pre-commit hook that regenerates the JSON Schema and `ts_rs` bindings, then asserts no diff against committed versions. Failure prints the diff with a one-line fix command (e.g., `cargo run --bin gen-calibration-schemas` or `cargo test --features ts_export schemas -- --ignored`).
- Wiring details depend on the project's CI conventions (likely [mise.toml](../mise.toml) tasks); verify during implementation.

## 6. Acceptance Criteria

- [ ] `finstack/valuations/schemas/calibration/1/envelope.schema.json` and `result_envelope.schema.json` exist and validate a hand-written envelope correctly when checked with a standard JSON Schema validator (e.g., `ajv` or Python `jsonschema`).
- [ ] All twelve reference envelopes from Phases 1-2 include a `$schema` reference.
- [ ] VS Code (with default JSON LSP) autocompletes step `kind` values when editing a reference envelope.
- [ ] [finstack-wasm/index.d.ts](../finstack-wasm/index.d.ts) exports `CalibrationEnvelope` and related types.
- [ ] JS users can call `calibrate({ schema: 'finstack.calibration', plan: { ... } })` with a typed object and receive a typed result; `tsc --noEmit` passes on a sample TypeScript file using the API.
- [ ] CI fails on uncommitted schema/TS-type drift with a clear "run `<command>`" message.

## 7. Verification Commands

- `cargo run --bin gen-calibration-schemas` (or equivalent test)
- `cargo test --features ts_export -p finstack-valuations`
- `npx ajv validate -s finstack/valuations/schemas/calibration/1/envelope.schema.json -d finstack/valuations/examples/market_bootstrap/01_usd_discount.json`
- `npm --prefix finstack-wasm run typecheck` (or equivalent)
- `mise run all-fmt && mise run all-lint && mise run all-test`

## 8. Risks

- **`ts_rs` coverage gap.** If `ts_rs` doesn't yet derive cleanly across some calibration types (e.g., `Decimal` newtypes, complex generic parameters, lifetimes), the build fails until those are addressed. Likely small but verify during implementation. Workaround: `#[ts(type = "string")]` overrides for stubborn types.
- **Schema URL stability.** Once published, the `$id` becomes a contract for external consumers. Use the project's existing schema versioning convention (`/1/`); bump only on breaking changes.
- **Editor configuration friction.** Not all users have JSON LSP enabled by default. Document the setup in README clearly. The committed `$schema` references in example files are the most reliable signal — modern editors pick them up automatically.
