# PR-05: Vertical Slice #3 – Statements

## Summary

Deliver the third vertical slice focused on **Statements**, including statement viewing, forecasting, formula editing, corkscrew tracing overlays, and initial analysis components (goal seek, sensitivity). This slice exercises heavy-grid virtualization, dependency visualization, and GenUI integration with the Statements domain.

## Background & Motivation

- Implements **Phase 5: Vertical Slice #3 – Statements** from `UI_KIT_ROADMAP.md`.
- Implements the Statements domain designs in `UI_KIT_DOMAINS.md` (Section C: Statements).
- Validates the Canvas overlay pattern for corkscrew tracing from `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md` and ADR-003.

## Scope

### In Scope

- Core components:
  - `StatementViewer` (matrix-style financial statement grid).
  - `ForecastEditor` (node-level forecast editor).
  - `FormulaBar` (Excel-like formula editing).
- Corkscrew tracing overlays and dependency visualization:
  - `CorkscrewOverlay` canvas overlay compatible with virtualization.
  - Basic `DependencyTreeViewer` and `FormulaExplainPanel` skeletons.
- Initial analysis tools (limited set):
  - `GoalSeekPanel`.
  - `SensitivityAnalyzer` + `TornadoChart` (basic implementation).
- GenUI integration with Statements components and data bindings.

### Out of Scope

- Full Monte Carlo, variance analysis, and backtesting breadth (covered in PR-08).
- Capital structure panels and extended templates (later statements breadth expansion).

## Design & Implementation Details

### 1. Domain Layout

- Under `src/domains/statements/` add:
  - `components/`
    - `StatementViewer.tsx`.
    - `ForecastEditor.tsx`.
    - `FormulaBar.tsx`.
    - `CorkscrewOverlay.tsx`.
    - `GoalSeekPanel.tsx`.
    - `SensitivityAnalyzer.tsx`.
    - `TornadoChart.tsx`.
  - `hooks/`
    - `useStatement.ts` – per `UI_KIT_HOOKS_AND_WORKERS.md`.
  - `schemas/`
    - Zod schemas for `StatementModelWire`, `StatementResultsWire`, `ForecastMethod`, etc.

### 2. Statement Viewer & Virtualization

- `StatementViewer`:
  - Uses TanStack Table + Virtual for rows/columns per `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md`.
  - Displays Income Statement, Balance Sheet, and Cashflow statements as matrix views.
  - Keyboard navigation, ARIA labels, and consistent tabular number formatting (per `UI_KIT_DEVELOPMENT_AND_A11Y.md`).
- Integrate with `useStatement` hook:
  - Evaluate models in WASM via unified worker.
  - Expose helpers like `getValue(nodeId, period)` and `getNodeSeries(nodeId)`.

### 3. Corkscrew Tracing & Canvas Overlay

- Implement `CorkscrewOverlay` following the Canvas overlay pattern in `UI_KIT_PERFORMANCE_AND_VIRTUALIZATION.md` and ADR-003:
  - Derive cell coordinates from row/column indices rather than DOM elements.
  - Use scroll offsets and viewport size from the virtualizer to compute visible edges.
  - Draw arrows and off-screen indicators on a transparent `<canvas>` overlay.
  - Retrieve dependency relations from Rust DAG data instead of DOM inspection.
- Integrate overlay with `StatementViewer` so users can select a cell and view its corkscrew and dependency relationships.

### 4. Forecast Editor & Formula Bar

- `ForecastEditor`:
  - Allows editing of `ForecastMethod` and explicit values per node and period.
  - Writes back to `StatementModelWire` and triggers re-evaluation via `useStatement`.
- `FormulaBar`:
  - Provides a single-line editor for formulas associated with the selected node.
  - Integrates with syntax highlighting and simple autocomplete for node references.
  - Shows error messages when formulas are invalid, based on WASM-provided validation.

### 5. Initial Analysis Tools

- `GoalSeekPanel`:
  - UI to specify target metric, driver variable, and bounds.
  - Calls into WASM goal-seek APIs and displays solution and convergence diagnostics.
- `SensitivityAnalyzer` + `TornadoChart`:
  - Allow selecting key drivers and ranges.
  - Visualize effect on a target metric using a simplified Tornado chart.

### 6. GenUI Integration

- Register Statements components in `ComponentRegistry`:
  - `StatementViewer`, `ForecastEditor`, `FormulaBar`, `GoalSeekPanel`, `SensitivityAnalyzer`, `TornadoChart`.
- Extend `DataBindingsSchema` usage:
  - Enable binding props such as `modelId`, `nodeId`, and `targetMetricPath` to engine state paths (`statements.*`).
- Provide a few sample dashboards combining statements with valuations or portfolio views.

### 7. Testing & Validation

- Unit tests:
  - `useStatement` hook behavior with mocked WASM APIs.
  - `CorkscrewOverlay` coordinate math and clipping logic.
- Integration tests:
  - Example page that loads a small statement model and allows:
    - Navigating cells.
    - Triggering corkscrew overlays.
    - Running a simple goal-seek scenario.
- Snapshot tests:
  - Ensure dashboards containing statements components validate against GenUI schemas and render without errors.

## Dependencies

- Requires PR-02 (schemas, GenUI) and PR-03 (valuation infrastructure) to be in place.
- Requires statements-related Rust types and WASM bindings to be exposed and integrated with the unified worker.

## Acceptance Criteria

- Example Statements UI demonstrates:
  - Viewing at least one full financial statement model.
  - Editing forecasts and formulas with immediate updates.
  - Corkscrew traces visible and performant with virtualization enabled.
- Goal seek and basic sensitivity tools are usable end-to-end on a small model.
- All new components are registered for GenUI usage and pass schema validation.
