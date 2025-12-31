# PR-03: Vertical Slice #1 – Basic Rates

## Summary

Deliver the first full vertical slice for **Basic Rates**, covering a narrow but complete feature set:

- Bond and Interest Rate Swap instrument panels.
- Discount and Forward curve calibration views.
- Curve charting and cashflow visualization.
- Initial GenUI integration and LLM dashboard flows.

## Background & Motivation

- Implements **Phase 3: Vertical Slice #1 – Basic Rates** from `UI_KIT_ROADMAP.md`.
- Uses domain designs from `UI_KIT_DOMAINS.md` (Valuations section) and architectural patterns from `UI_KIT_ARCHITECTURE.md`.
- Validates the handle-based worker pattern, schema pipeline, and GenUI renderer under realistic but bounded complexity.

## Scope

### In Scope

- **Instruments (2 only):**
  - `Bond` – via `GenericInstrumentPanel` + descriptor.
  - `InterestRateSwap` – custom, hand-crafted panel (complexity test).
- **Calibration Views (2 only):**
  - `DiscountCurveCalibration`.
  - `ForwardCurveCalibration`.
- **Charts & Tables:**
  - `CurveChart` (Recharts-based curve viewer).
  - `VirtualDataTable` for cashflows and quotes (TanStack Table + Virtual).
  - `CashflowWaterfall` view for swaps and bonds.
- **GenUI Integration:**
  - Register above components in `ComponentRegistry` with full props schemas and LLM documentation metadata.
  - Provide 3 sample dashboards rendered from JSON definitions.
- **Validation & Testing:**
  - Schema parity tests for instrument/calibration types (Rust ↔ TS ↔ Zod).
  - Numeric parity golden tests for a small set of bond/swap cases.
  - LLM dashboard snapshot tests.

### Out of Scope

- Other instruments, calibrations, or risk/attribution surfaces (added in PR-07 and beyond).
- Full portfolio/stress/scenario orchestration (later phases).

## Design & Implementation Details

### 1. Domain Directory Structure (Valuations)

- Under `packages/finstack-ui/src/domains/valuations/` add:
  - `instruments/`
    - `BondPanel.tsx` (using generic descriptor-based system).
    - `InterestRateSwapPanel.tsx` (custom layout).
  - `calibration/`
    - `DiscountCurveCalibration.tsx`.
    - `ForwardCurveCalibration.tsx`.
  - `views/`
    - `CashflowWaterfall.tsx`.
- Ensure components use hooks from PR-01 (`useFinstack`, `useValuation`) and schemas from PR-02.

### 2. Instrument Panels

- Implement `GenericInstrumentPanel` infrastructure (as per `UI_KIT_GENUI_AND_SCHEMAS.md`):
  - Descriptor describing fields (kind: `money`, `rate`, `tenor`, `date`, `curveId`, `enum`, etc.).
  - Auto-generated forms using primitives (`AmountInput`, `CurrencySelect`, `TenorInput`, `DatePicker`, `RateInput`).
  - Shared layout for **Inputs**, **Market Data**, **Cashflows**, and **Metrics**.
- `BondPanel`:
  - Built entirely via `GenericInstrumentPanel` using a `BondDescriptor` derived from `BondSpec` TS/Zod types.
  - Displays PV and a small metrics table (e.g., DV01).
  - Embeds `CashflowWaterfall` when discountable cashflows are available.
- `InterestRateSwapPanel`:
  - Custom panel UI to stress-test flexibility (multiple legs, pay/receive, floating index details).
  - Still delegates to `useValuation` and reuses `CashflowWaterfall` and `CurveChart` sections.

### 3. Calibration Views

- Implement `DiscountCurveCalibration` and `ForwardCurveCalibration` views as in `UI_KIT_DOMAINS.md`:
  - Quote grids using `VirtualDataTable` with strongly typed columns.
  - Configuration editors for calibration parameters (curve IDs, interpolation, solver config) using React Hook Form + Zod.
  - Curve preview using `CurveChart` (zero and forward curves).
  - "Calibrate" action that calls into the unified worker (calibration API) and renders diagnostics.

### 4. Charts & Tables

- Implement `components/charts/CurveChart.tsx`:
  - Recharts-based line chart for yield curves with axes, tooltips, and basic theming.
  - Accepts structured data from WASM (`{ tenor: string; rate: string }`), using string values for rates to preserve precision.
- Implement `components/tables/VirtualDataTable.tsx` per `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`:
  - Uses `@tanstack/react-virtual` under the hood for large tables.
  - Stable row height and overscan configuration.
- Implement `CashflowWaterfall` view:
  - Tabular representation of cashflows with virtualized rows.
  - Columns: Period, Fix/Float, Rate, Notional, Discount Factor, PV (per `UI_KIT_DOMAINS.md`).

### 5. GenUI Integration

- Register new components in `ComponentRegistry`:
  - `BondPanel` (allowedModes: `['viewer', 'editor', 'llm-assisted']`).
  - `InterestRateSwapPanel` (same as above).
  - `DiscountCurveCalibration`, `ForwardCurveCalibration`.
  - `CurveChart`, `CashflowWaterfall`, `VirtualDataTable`.
- For each, provide:
  - `propsSchema` in Zod.
  - Human-readable `description`.
  - `exampleProps` for documentation and Storybook.
- Add sample `DashboardDefinition` JSON fixtures under `fixtures/dashboards/basic-rates/` with 3 dashboards that demonstrate:
  - Simple bond pricing + curve chart.
  - Swap pricing + cashflow view.
  - Calibration-focused dashboard combining quotes + chart.

### 6. Testing & Validation

- **Schema Parity:**
  - Rust golden values for bond/swap pricing to compare with WASM calls used by the UI.
  - TS/Zod schemas for `BondSpec`, `SwapSpec`, `CalibrationConfig` validated against sample JSON from Rust tests.
- **Numeric Parity:**
  - Vitest-based tests loading golden JSON from `finstack` Rust tests (or a JS copy) and asserting PV and key metrics match to decimal precision.
- **LLM Dashboard Snapshots:**
  - For each sample dashboard, create a snapshot test that:
    - Validates the JSON against `DashboardDefinitionSchema`.
    - Renders via `DynamicRenderer` and asserts key components render without error.

## Dependencies

- Requires `PR-01` and `PR-02` to be merged.
- Assumes initial worker APIs for single-instrument pricing and curve calibration are available in `finstack-wasm`.

## Acceptance Criteria

- User can load a Basic Rates example app that:
  - Initializes WASM and unified worker.
  - Calibrates a simple discount and forward curve from quotes.
  - Prices a Bond and Interest Rate Swap and displays PV and cashflows.
- All components are registered in `ComponentRegistry` and can be referenced from dashboard JSON.
- Golden tests confirm numeric parity with Rust benchmarks for at least 3–5 bond/swap cases.
