# PR-07: Breadth Expansion – Instruments & Calibration

## Summary

Expand coverage from the initial Basic Rates slice to **all major instruments and calibration modules** in the Valuations domain, leveraging the generic instrument form system, calibration views, and charts designed earlier.

## Background & Motivation

- Implements **Phase 7: Breadth Expansion – All Instruments** from `UI_KIT_ROADMAP.md`.
- Uses Valuations and Calibration domain designs from `UI_KIT_DOMAINS.md`.
- Builds on the generic instrument form system and descriptors introduced in PR-03 and schema pipeline from PR-02.

## Scope

### In Scope

- Generic Instrument Form System:
  - Finalize descriptor DSL in `schemas/instrumentDescriptor.ts`.
  - Ensure coverage for all instrument fields needed across `valuations::instruments`.
- Instrument Panels:
  - Generate or hand-author panels and viewers for all instruments exported by `finstack-wasm/src/valuations/instruments`.
  - Provide panel/viewer variants for complex instruments where necessary.
- Calibration breadth:
  - `HazardCurveCalibration`.
  - `InflationCurveCalibration`.
  - `VolSurfaceCalibration` (2D heatmap with future 3D hooks).
  - `BaseCorrelationCalibration`.
  - Additional calibrators like `SabrCalibration` and `HullWhiteCalibration` where bindings exist.

### Out of Scope

- Portfolio-level margin and covenant analytics (handled later as part of portfolio breadth).
- Scenario and analysis breadth (PR-08).

## Design & Implementation Details

### 1. Descriptor DSL Finalization

- Complete `schemas/instrumentDescriptor.ts` as outlined in `UI_KIT_GENUI_AND_SCHEMAS.md`:
  - Field kinds: `money`, `rate`, `tenor`, `date`, `enum`, `boolean`, `string`, `curveId`, `surfaceId`.
  - Support grouping fields into logical sections (e.g., core terms, schedule, day-count).
- Add descriptors for every instrument type using TS/Zod types generated from Rust:
  - `Bond`, `InterestRateSwap`, `FxForward`, `FxOption`, `Swaption`, `ConvertibleBond`, `VarianceSwap`, etc.

### 2. Generic Instrument Panels & Viewers

- Extend `GenericInstrumentPanel` to support:
  - Section headers and collapsible groups.
  - Conditional fields based on enums (e.g., pay/receive, leg types).
- For each instrument:
  - Create `InstrumentPanel` component that composes the generic panel with descriptor and domain-specific layout tweaks.
  - Create `InstrumentViewer` variant for read-only views used in portfolio and statements.
- Ensure all panels:
  - Use string transport for monetary values.
  - Integrate with `useValuation` and unified worker.
  - Display cashflows and key risk metrics when available.

### 3. Calibration Views

- Implement additional calibration views under `domains/valuations/calibration/` as defined in `UI_KIT_DOMAINS.md`:
  - `HazardCurveCalibration` – CDS quotes, recovery rate config.
  - `InflationCurveCalibration` – inflation swap quotes and seasonality.
  - `VolSurfaceCalibration` – strike/expiry grids, SABR/SVI params, 2D heatmap.
  - `BaseCorrelationCalibration` – tranche quotes and detachment points.
  - Optional extras (`SabrCalibration`, `HullWhiteCalibration`) where bindings exist.
- Use `EditableGrid` for quote entry and `CurveChart`/`SurfaceViewer` for visualization.

### 4. GenUI & Registry

- Register all new instrument panels, viewers, and calibration views in `ComponentRegistry`.
- Provide example dashboards for key instrument families:
  - Rates, credit, FX, equity/derivatives.
- Extend OpenAI function schemas to include new components and instrument descriptors so LLMs can target them reliably.

### 5. Testing & Validation

- Schema parity:
  - For each instrument type, ensure Rust-generated JSON validates against TS/Zod and vice versa.
- Numeric parity:
  - Golden tests for representative instruments across categories (e.g., CDS, volatility options) using existing Rust test fixtures.
- Integration tests:
  - Example dashboards for several instrument types that can be loaded, edited, and revalued without errors.

## Dependencies

- Builds directly on PR-03 (Basic Rates) and PR-02 (schema pipeline, descriptors).
- Requires valuations Rust crate to expose necessary types and calibration APIs.

## Acceptance Criteria

- Panels and viewers exist for all instruments exported by `finstack-wasm/src/valuations/instruments`.
- Calibration views for hazard, inflation, vol, and base correlation operate end-to-end on example data.
- LLMs can construct dashboards for any supported instrument using documented schemas and descriptors.
