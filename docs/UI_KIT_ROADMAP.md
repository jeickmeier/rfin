# Finstack UI Kit: Implementation Roadmap

## 4. Implementation Roadmap

> **Key Principles:**
>
> 1. **GenUI first** - The LLM integration dictates data structures across all domains
> 2. **Vertical slices** - Ship narrow but complete features before expanding breadth
> 3. **Prove patterns early** - Validate Handle Pattern, String Transport, and Schema Generation before building 30+ panels

### Phase 1: Core Infrastructure (Weeks 1–2)

- Initialize `finstack-ui` with Vite/React/TS.
- Setup Tailwind + Shadcn.
- **Implement Unified Finstack Engine Worker** with Handle-based architecture.
- Implement Rust panic hooks for graceful error recovery.
- Create `FinstackProvider` context with WASM initialization (singleton pattern).
- Implement core hooks: `useFinstack`, `useFinstackEngine`.
- Build primitives with **String Transport** (no JS math):
  - `AmountDisplay` (string-based, no float conversion)
  - `AmountInput`, `CurrencySelect`, `TenorInput`, `DatePicker`
- Configure Vitest + React Testing Library.
- Setup worker pool singleton.

### Phase 2: Schema Pipeline & GenUI Foundation (Weeks 3–4)

- **Setup `ts-rs` or `specta`** in Rust crates for auto-generated TypeScript types.
- Build Zod schema derivation from generated types with **schema versioning**.
- Define `DashboardDefinitionSchema v1` with layout templates.
- Implement `ComponentRegistry` with typed components.
- Build `DynamicRenderer` that reads hard-coded JSON dashboards.
- Create mutation action reducers (add/remove/update components).
- Build `toLLMContext()` semantic summary generation in Rust.
- Implement `schemaGenerator.ts` for OpenAI function schemas.
- **Test:** Validate end-to-end "JSON → UI" rendering with 3 sample dashboards.

### Phase 3: Vertical Slice #1 – Basic Rates (Weeks 5–7)

**Goal:** Prove all patterns with a narrow but complete feature set.

- **Instruments (2 only):**
  - `Bond` – via `GenericInstrumentPanel` + descriptor
  - `InterestRateSwap` – custom panel (complexity test)
- **Calibration (2 only):**
  - `DiscountCurveCalibration`
  - `ForwardCurveCalibration`
- **Charts:**
  - `CurveChart` (Recharts)
  - `VirtualDataTable` (TanStack)
  - `CashflowWaterfall` (virtualized)
- **GenUI integration:**
  - Register `CurveChart`, `BondPanel`, `SwapPanel` in `ComponentRegistry`.
  - Test LLM dashboard creation with these components.
- **Validation:**
  - Schema parity tests (Rust ↔ TS ↔ Zod).
  - Numeric parity golden tests.
  - LLM dashboard snapshot tests.

### Phase 4: Vertical Slice #2 – Portfolio (Weeks 8–10)

- **Portfolio core:**
  - `PositionGrid` with virtual scrolling.
  - `EntityTreeView` for entity hierarchy.
  - `PortfolioSummaryPanel` with aggregated metrics.
- **Risk:**
  - `InstrumentRiskTable` (DV01/CS01/Greeks).
  - Basic `RiskHeatmap` (table-based, not canvas yet).
- **GenUI integration:**
  - Register portfolio components.
  - Mutation actions for position selection/filtering.
  - Data binding DSL for portfolio paths.
- **LLM safety:**
  - Implement `mode` prop on portfolio editors.
  - Test `llm-assisted` mode with confirmation dialogs.

### Phase 5: Vertical Slice #3 – Statements (Weeks 11–13)

- **Statements core:**
  - `StatementViewer` (Matrix rendering).
  - `ForecastEditor` with method selection.
  - `FormulaBar` with autocomplete.
- **Corkscrew tracing:**
  - Build **Canvas Overlay** (virtualization-compatible).
  - Dependency graph from Rust DAG.
- **Analysis (2 only):**
  - `GoalSeekPanel`.
  - `SensitivityAnalyzer` + `TornadoChart`.
- **GenUI integration:**
  - Register statement components.
  - Data binding DSL for statement paths (node.period).

### Phase 6: Editors & Draft Mode (Weeks 14–15)

- Implement Draft Mode state management with `useDraftStore`.
- Build `EditableGrid` for quote entry with validation.
- Build `TradeEntryForm` for position entry.
- Implement `ConstraintEditor` for optimization.
- Add undo/redo middleware for editor state.
- Implement `useDeferredValue` for responsive editing.

### Phase 7: Breadth Expansion – All Instruments (Weeks 16–18)

- **Generic Instrument Form system:**
  - Generate `InstrumentDescriptor` for all instruments from Rust metadata.
  - Build remaining instrument panels via `GenericInstrumentPanel`.
- **Calibration breadth:**
  - `HazardCurveCalibration`, `InflationCurveCalibration`.
  - `VolSurfaceCalibration` (2D heatmap).
  - Lazy-load `VolSurface3D` (WebGL).
- **Custom panels (complex instruments only):**
  - `SwaptionPanel`, `ConvertibleBondPanel`, `AutocallablePanel`.

### Phase 8: Breadth Expansion – Analysis & Scenarios (Weeks 19–21)

- **Statements analysis:**
  - `MonteCarloConfigEditor` + `MonteCarloResultsView`.
  - `VarianceBridgeView` + `VarianceBridgeChart`.
  - `DependencyTreeViewer`, `FormulaExplainPanel`.
  - `CapitalStructurePanel`.
- **Portfolio analysis:**
  - `PortfolioAttributionView` with factor breakdown.
  - `PortfolioOptimizerView`, `EfficientFrontierChart`, `TradeProposalGrid`.
  - Canvas-based `RiskHeatmap` for large grids.
- **Scenarios domain:**
  - `ScenarioBuilder` with operation editors.
  - `ScenarioLibrary` with tagging.
  - `ScenarioExecutionPanel` with phase view.
  - `HorizonScenarioGrid`.

### Phase 9: Testing & Documentation (Weeks 22–23)

- **Unit tests:** >80% coverage.
- **Schema parity tests:** For each Rust type, assert JSON validates against Zod and vice versa.
- **LLM dashboard snapshots:** Library of canonical LLM outputs under test.
- **Performance budgets:** CI checks for rendering time and bundle size.
- **Integration tests:** Playwright for critical flows.
- **Storybook:** All components with examples.
- **API documentation.**

### Phase 10: Accessibility & Final Polish (Weeks 24–25)

- Accessibility audit with axe-core.
- Keyboard navigation testing.
- Screen reader testing (NVDA, VoiceOver).
- High contrast theme validation.
- Performance profiling and optimization.
- Bundle size optimization (target: <500KB core, lazy-load pro features).
- Memory leak testing for long-running sessions.
