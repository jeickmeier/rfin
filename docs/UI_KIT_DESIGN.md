# Finstack UI Kit: PRD & Technical Design Document

**Version:** 2.2
**Status:** Draft
**Date:** December 9, 2025

---

## 1. Executive Summary

The **Finstack UI Kit** is a specialized React component library designed to serve as the "visual frontend" for the `finstack-wasm` financial engine. It solves a unique architectural challenge: **bridging strict, deterministic financial computation (Rust/WASM) with dynamic, probabilistic orchestration (LLMs/AI Agents).**

This library will empower developers and AI agents to build high-performance financial applications—including pricing dashboards, risk reports, and interactive financial statement models—where the UI state is a serializable artifact that both humans and LLMs can read, write, and reason about.

---

## 2. Product Requirements (PRD)

### 2.1 Product Vision

To create a "Lego set" for financial engineering that is:

1. **Mathematically Correct:** Enforces the same precision, rounding, and currency safety as the core Rust engine.
2. **AI-Native:** Designed from the ground up to be controlled by LLMs via structured JSON, allowing agents to "render" answers (e.g., *"Here is the risk heatmap you asked for"*).
3. **High Performance:** Capable of rendering large cashflow trees and volatility surfaces without main-thread blocking.
4. **Accessible:** Full keyboard navigation, screen reader support, and high-contrast modes for professional trading environments.

### 2.2 Target Audience

1. **Financial App Developers:** Building internal tools for trading desks, risk management, or FP&A.
2. **Quants/Analysts:** Using "Notebook-like" interfaces to interactively explore models.
3. **AI Agents:** LLMs that need a standard output format to visualize complex financial data instead of just text.

### 2.3 Core Capabilities

#### A. The Financial Primitives (Foundation)

* **Strict Inputs:** Specialized form controls for Currency (ISO-4217), Tenors (1M, 10Y), Dates (Business Day adjustment), and Rates (Bps vs %).
* **Precision Display:** `AmountDisplay` components that respect the global `finstack` `RoundingContext`.

#### B. The Visualization Layer (Components)

* **Market Data:** Interactive Yield Curves (Zero/Forward), Volatility Surfaces (3D/Heatmap).
* **Valuations:** Cashflow Waterfalls, Risk/Greeks Heatmaps, PnL Attribution Waterfalls.
* **Statements:** "Corkscrew" financial models, Balance Sheet projections, Forecast Editors.
* **Portfolio:** Position grids, Book hierarchy trees.

#### C. The GenUI Bridge (Orchestration)

* **Dynamic Renderer:** A system that accepts a JSON "View Definition" and renders the corresponding interactive component tree.
* **State Serialization:** Ability to snapshot the entire UI context (Market + Portfolio + User Edits) into JSON for LLM analysis.
* **Scenario Orchestrator:** High-level control plane that composes and applies scenarios (via the `scenarios` crate) across Market, Valuations, and Statements, with deterministic reports for each run.

---

## 3. Technical Design Document (TDD)

### 3.1 Technology Stack

#### Frontend

* **Framework:** React 19 (leveraging Hooks, Suspense, and `useDeferredValue`).
* **Language:** TypeScript (Strict Mode).
* **Build System:** Vite (Library Mode).
* **Styling:** Tailwind CSS + `clsx` + `tailwind-merge` + Shadcn/UI (Base primitives).
* **State Management:** **Zustand** (Chosen for its ability to work outside components and ease of JSON hydration/snapshotting).
* **Data Grids:** **TanStack Table** + **TanStack Virtual** (Headless, high-performance, critical for financial statements).
* **Charts (Complex):** **Apache ECharts** (WebGL support for Vol Surfaces and dense scatters).
* **Charts (Simple):** **Recharts** (SVG-based for standard time-series).
* **Forms:** **React Hook Form** + **Zod Resolver** (Schema-driven validation).

#### WASM Integration

* **Worker Communication:** **Comlink** (Type-safe Web Worker RPC).
* **Schema Validation:** **Zod** (Runtime validation + OpenAI function schema generation).
* **Schema Generation:** **`zod-to-json-schema`** (Generate OpenAI function schemas from Zod).
* **WASM Bridge:** `finstack-wasm` (Direct dependency).

#### Rust-Side (Schema Generation)

* **TypeScript Generation:** **`ts-rs`** or **`specta`** (Auto-generate TypeScript types from Rust structs).
* **Panic Handling:** Custom panic hook for graceful JS exception conversion.

### 3.2 Architecture: Schema-First Design

To enable LLM control, every major component ("Organism") is defined by a Zod Schema. This allows us to generate OpenAI Function Calling definitions automatically.

#### 3.2.1 Type Sources of Truth

The system has four type layers that must stay synchronized:

1. **Rust structs/enums** (`finstack-wasm` / core engine)
2. **WASM bindings** (wasm-bindgen / JS class wrappers)
3. **TypeScript interfaces/types**
4. **Zod schemas** → JSON Schema → OpenAI function specs

**Strategy: Dual Source of Truth**

| Type Category | Source of Truth | Generated From |
|--------------|-----------------|----------------|
| Engine types (instruments, curves, etc.) | **Rust-first** | `ts-rs`/`specta` → TypeScript → Zod wrappers |
| UI-facing types (dashboards, layouts, etc.) | **Zod-first** | Zod → JSON Schema for LLMs |

This hybrid approach ensures:
* Engine types match Rust's serde representation exactly
* UI types are optimized for LLM consumption and validation

#### 3.2.2 Schema Versioning

All LLM-facing schemas include a version field to support backwards compatibility:

```typescript
// schemas/versioned.ts
export const VersionedDashboardDefinition = DashboardDefinitionSchema.extend({
  schemaVersion: z.literal('1'),
});

// Migration function for schema upgrades
export function migrateDashboard(
  data: unknown,
  fromVersion: string,
  toVersion: string
): DashboardDefinition {
  // Handle version migrations
  if (fromVersion === '1' && toVersion === '2') {
    return migrateV1toV2(data as DashboardDefinitionV1);
  }
  throw new Error(`Unknown migration: ${fromVersion} → ${toVersion}`);
}
```

This prevents "LLM built a dashboard two months ago that no longer validates."

#### 3.2.3 The State Engine (Engine/UI Separation)

We **hard-separate** engine state from UI state. This makes serialization, snapshots, and migrations tractable:

```typescript
// store/types.ts
import { z } from 'zod';

// ============================================
// ENGINE STATE (Protocol-like, versioned)
// ============================================
export const EngineStateSchema = z.object({
  schemaVersion: z.literal('1'),
  marketContext: MarketContextWireSchema,
  portfolios: z.record(PortfolioWireSchema),
  statements: z.record(StatementModelWireSchema),
  // Results cache keyed by computation hash
  computationCache: z.record(ValuationResultWireSchema).optional(),
});

export type EngineState = z.infer<typeof EngineStateSchema>;

// ============================================
// UI STATE (Transient, not versioned)
// ============================================
export const UIStateSchema = z.object({
  activeView: DashboardDefinitionSchema,
  panelState: z.record(z.unknown()),  // Per-component local UI bits
  selections: z.object({
    selectedPositionIds: z.array(z.string()),
    selectedNodeId: z.string().nullable(),
    selectedPeriod: z.string().nullable(),
  }),
  filters: z.record(z.unknown()),
});

export type UIState = z.infer<typeof UIStateSchema>;

// ============================================
// ROOT STATE
// ============================================
export const RootStateSchema = z.object({
  engine: EngineStateSchema,
  ui: UIStateSchema,
  history: HistoryStateSchema,
});

export type RootState = z.infer<typeof RootStateSchema>;
```

**Why separation matters:**
* **Serializing everything** for LLMs or history is heavy. Typically you want snapshots of `EngineState + DashboardDefinition`, not every transient UI bit.
* It's easier to version protocol-like `EngineState` than "was the 3rd accordion panel open".

**Critical:** The Zustand store must be **100% JSON-serializable** (no class instances, Maps, etc.). For WASM objects, store the **wire form** (`toJSON()` output) and re-hydrate at the boundary.

#### 3.2.4 Data Binding DSL

Instead of loose `z.record(z.string())`, we define a structured binding grammar:

```typescript
// schemas/bindings.ts
import { z } from 'zod';

// Binding sources correspond to engine state domains
const BindingSourceSchema = z.enum([
  'market',
  'portfolio',
  'statements',
  'scenarios',
]);

// Structured path with documented grammar
const BindingPathSchema = z.object({
  source: BindingSourceSchema,
  // Dot-notation path within the source
  // Examples:
  //   "curves.USD-SOFR.rate(1)"
  //   "entities.Desk1.positions.Trade123.pv"
  //   "results.revenue.2025Q1"
  path: z.string().regex(/^[a-zA-Z0-9_\-\.]+(\([^)]*\))?$/),
});

export type BindingPath = z.infer<typeof BindingPathSchema>;

// Data bindings map component prop names to data sources
export const DataBindingsSchema = z.record(BindingPathSchema);

// Examples for LLM documentation:
// {
//   "portfolioValue": { "source": "portfolio", "path": "totalPV" },
//   "curve": { "source": "market", "path": "curves.USD-SOFR" },
//   "revenue": { "source": "statements", "path": "results.revenue.2025Q1" }
// }
```

#### 3.2.5 Dashboard Definition (Complete)

```typescript
// schemas/dashboard.ts
import { z } from 'zod';

export const DashboardDefinitionSchema = z.object({
  schemaVersion: z.literal('1'),
  id: z.string().uuid(),
  name: z.string(),

  // Layout template (not arbitrary component soup)
  layout: LayoutTemplateSchema,

  // Components within the layout
  components: z.array(ComponentInstanceSchema),

  // Structured data bindings
  bindings: DataBindingsSchema,

  // LLM context
  userIntent: z.string().optional(),

  // Metadata
  createdAt: z.string().datetime(),
  updatedAt: z.string().datetime(),
});

// Layout templates constrain LLM output to sensible structures
export const LayoutTemplateSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('Single'),
    mainComponentId: z.string(),
  }),
  z.object({
    kind: z.literal('TwoColumn'),
    leftComponentIds: z.array(z.string()),
    rightComponentIds: z.array(z.string()),
    splitRatio: z.number().min(0.2).max(0.8).default(0.5),
  }),
  z.object({
    kind: z.literal('Grid'),
    columns: z.number().min(1).max(4),
    componentIds: z.array(z.string()),
  }),
  z.object({
    kind: z.literal('TabSet'),
    tabs: z.array(z.object({
      label: z.string(),
      componentIds: z.array(z.string()),
    })),
  }),
  z.object({
    kind: z.literal('Report'),
    sections: z.array(z.object({
      title: z.string(),
      componentIds: z.array(z.string()),
    })),
  }),
]);

export const ComponentInstanceSchema = z.object({
  id: z.string().uuid(),
  type: z.string(),  // Validated against ComponentRegistry at runtime
  props: z.record(z.unknown()),  // Validated by component's propsSchema
  // Component-specific mode for LLM safety
  mode: z.enum(['viewer', 'editor', 'llm-assisted']).default('viewer'),
});

// Auto-generate OpenAI function schema from Zod
import { zodToJsonSchema } from 'zod-to-json-schema';

export const generateFunctionSchema = (schema: z.ZodSchema) => {
  return zodToJsonSchema(schema, { target: 'openApi3' });
};
```

### 3.3 Directory Structure

We adopt a **Domain-Driven** structure, moving away from generic Atomic Design to terms that match the financial domain.

```text
packages/finstack-ui/
├── src/
│   ├── components/             # Generic UI Infrastructure
│   │   ├── primitives/        # Financial input primitives
│   │   │   ├── AmountDisplay.tsx
│   │   │   ├── AmountInput.tsx
│   │   │   ├── CurrencySelect.tsx
│   │   │   ├── TenorInput.tsx
│   │   │   ├── RateInput.tsx         # Handles bps vs % automatically
│   │   │   ├── DatePicker.tsx        # Business day aware
│   │   │   ├── PeriodRangeInput.tsx  # "2025Q1..Q4" style
│   │   │   └── DayCountSelect.tsx
│   │   ├── ui/                # Shadcn Primitives (Button, Card, Select)
│   │   ├── charts/            # Base Chart Wrappers
│   │   │   ├── CurveChart.tsx
│   │   │   ├── SurfaceViewer.tsx     # WebGL with Canvas fallback
│   │   │   ├── WaterfallChart.tsx
│   │   │   ├── HeatmapGrid.tsx
│   │   │   ├── FanChart.tsx          # Monte Carlo percentiles
│   │   │   └── HistogramChart.tsx
│   │   ├── tables/            # Base Table Wrappers
│   │   │   ├── VirtualDataTable.tsx  # TanStack Table + Virtual
│   │   │   ├── EditableGrid.tsx      # For quote entry
│   │   │   └── TreeGrid.tsx          # Hierarchical data
│   │   └── common/            # Cross-domain components
│   │       ├── MetadataPanel.tsx     # Show RoundingContext, FX policy
│   │       ├── ExportButtons.tsx     # CSV/JSON/DataFrame
│   │       ├── ErrorFallback.tsx     # WASM panic recovery UI
│   │       ├── LoadingOverlay.tsx    # Suspense fallback
│   │       └── EmptyState.tsx
│   │
│   ├── domains/                # FINANCIAL LOGIC MODULES
│   │   ├── market/             # Market Data Domain
│   │   │   ├── CurveEditor.tsx
│   │   │   ├── VolSurfaceViewer.tsx
│   │   │   ├── VolSurface3D.tsx      # WebGL 3D surface
│   │   │   ├── QuoteTicker.tsx
│   │   │   ├── FxMatrixViewer.tsx
│   │   │   └── InflationIndexViewer.tsx
│   │   │
│   │   ├── portfolio/          # Portfolio Domain
│   │   │   ├── TradeEntryForm.tsx
│   │   │   ├── EntityTreeView.tsx
│   │   │   ├── PositionGrid.tsx
│   │   │   ├── TagPivotGrid.tsx
│   │   │   ├── PortfolioSummaryPanel.tsx
│   │   │   ├── PortfolioMetricsPanel.tsx
│   │   │   ├── PortfolioAttributionView.tsx
│   │   │   ├── PortfolioCashflowViewer.tsx
│   │   │   ├── scenarios/
│   │   │   │   └── PortfolioScenarioImpactView.tsx
│   │   │   ├── margin/
│   │   │   │   ├── MarginSummaryPanel.tsx
│   │   │   │   ├── NettingSetView.tsx
│   │   │   │   └── SensitivityBreakdown.tsx
│   │   │   ├── optimization/
│   │   │   │   ├── PortfolioOptimizerView.tsx
│   │   │   │   ├── ConstraintEditor.tsx
│   │   │   │   ├── ConstraintViolationsPanel.tsx
│   │   │   │   ├── EfficientFrontierChart.tsx
│   │   │   │   ├── TradeProposalGrid.tsx
│   │   │   │   └── OptimizationResultView.tsx
│   │   │   ├── cashflows/
│   │   │   │   └── PortfolioCashflowAggregate.tsx
│   │   │   └── exports/
│   │   │       └── DataFramePreview.tsx
│   │   │
│   │   ├── valuations/         # Pricing & Risk Domain
│   │   │   ├── instruments/    # One component per instrument
│   │   │   │   ├── BondPanel.tsx
│   │   │   │   ├── DepositPanel.tsx
│   │   │   │   ├── InterestRateSwapPanel.tsx
│   │   │   │   ├── FraPanel.tsx
│   │   │   │   ├── SwaptionPanel.tsx
│   │   │   │   ├── CapFloorPanel.tsx
│   │   │   │   ├── BasisSwapPanel.tsx
│   │   │   │   ├── FxSpotPanel.tsx
│   │   │   │   ├── FxOptionPanel.tsx
│   │   │   │   ├── FxSwapPanel.tsx
│   │   │   │   ├── FxBarrierOptionPanel.tsx
│   │   │   │   ├── EquityPanel.tsx
│   │   │   │   ├── EquityOptionPanel.tsx
│   │   │   │   ├── AsianOptionPanel.tsx
│   │   │   │   ├── BarrierOptionPanel.tsx
│   │   │   │   ├── LookbackOptionPanel.tsx
│   │   │   │   ├── CliquetOptionPanel.tsx
│   │   │   │   ├── QuantoOptionPanel.tsx
│   │   │   │   ├── AutocallablePanel.tsx
│   │   │   │   ├── CdsPanel.tsx
│   │   │   │   ├── CdsIndexPanel.tsx
│   │   │   │   ├── CdsOptionPanel.tsx
│   │   │   │   ├── CdsTranchePanel.tsx
│   │   │   │   ├── InflationLinkedBondPanel.tsx
│   │   │   │   ├── InflationSwapPanel.tsx
│   │   │   │   ├── ConvertibleBondPanel.tsx
│   │   │   │   ├── VarianceSwapPanel.tsx
│   │   │   │   ├── TrsPanel.tsx
│   │   │   │   ├── RepoPanel.tsx
│   │   │   │   ├── TermLoanPanel.tsx
│   │   │   │   ├── RevolvingCreditPanel.tsx
│   │   │   │   ├── PrivateMarketsFundPanel.tsx
│   │   │   │   ├── RangeAccrualPanel.tsx
│   │   │   │   ├── CmsOptionPanel.tsx
│   │   │   │   └── StructuredCreditPanel.tsx
│   │   │   ├── calibration/    # One component per calibration type
│   │   │   │   ├── DiscountCurveCalibration.tsx
│   │   │   │   ├── ForwardCurveCalibration.tsx
│   │   │   │   ├── HazardCurveCalibration.tsx
│   │   │   │   ├── InflationCurveCalibration.tsx
│   │   │   │   ├── VolSurfaceCalibration.tsx
│   │   │   │   ├── BaseCorrelationCalibration.tsx
│   │   │   │   ├── SabrCalibration.tsx
│   │   │   │   ├── HullWhiteCalibration.tsx
│   │   │   │   └── QuoteEditor.tsx
│   │   │   ├── metrics/
│   │   │   │   ├── InstrumentRiskTable.tsx
│   │   │   │   ├── BucketedRiskGrid.tsx
│   │   │   │   └── MetricsRegistryBrowser.tsx
│   │   │   ├── margin/
│   │   │   │   ├── InstrumentMarginPanel.tsx
│   │   │   │   └── SimmBreakdownView.tsx
│   │   │   ├── covenants/
│   │   │   │   ├── InstrumentCovenantPanel.tsx
│   │   │   │   └── CovenantTimelineView.tsx
│   │   │   ├── attribution/
│   │   │   │   ├── PnLAttributionBridge.tsx
│   │   │   │   ├── FactorBreakdownPanel.tsx
│   │   │   │   └── BucketedAttributionGrid.tsx
│   │   │   ├── RiskHeatmap.tsx
│   │   │   ├── CashflowWaterfall.tsx
│   │   │   └── ValuationRunViewer.tsx
│   │   │
│   │   ├── statements/         # Financial Reporting Domain
│   │   │   ├── StatementViewer.tsx         # Core matrix + corkscrew
│   │   │   ├── ForecastEditor.tsx          # Node-level forecast config
│   │   │   ├── CapitalStructurePanel.tsx   # Debt schedule & cs.* metrics
│   │   │   ├── EbitdaAdjustmentsPanel.tsx  # EBITDA normalization bridge
│   │   │   ├── RegistryBrowser.tsx         # Metric registry explorer
│   │   │   ├── FormulaBar.tsx              # Excel-like formula editing
│   │   │   ├── capital_structure/
│   │   │   │   ├── DebtScheduleGrid.tsx
│   │   │   │   ├── WaterfallViewer.tsx
│   │   │   │   └── instruments/
│   │   │   │       ├── BondInstrumentForm.tsx
│   │   │   │       ├── LoanInstrumentForm.tsx
│   │   │   │       └── SwapInstrumentForm.tsx
│   │   │   ├── templates/
│   │   │   │   ├── RollForwardBuilder.tsx
│   │   │   │   ├── VintageAnalysisView.tsx
│   │   │   │   └── VintageWaterfallView.tsx
│   │   │   ├── analysis/
│   │   │   │   ├── ScenarioSetManager.tsx
│   │   │   │   ├── ScenarioComparisonView.tsx
│   │   │   │   ├── SensitivityAnalyzer.tsx
│   │   │   │   ├── TornadoChart.tsx
│   │   │   │   ├── GoalSeekPanel.tsx
│   │   │   │   ├── BacktestDashboard.tsx
│   │   │   │   ├── BacktestAccuracyView.tsx
│   │   │   │   ├── MonteCarloConfigEditor.tsx
│   │   │   │   ├── MonteCarloResultsView.tsx
│   │   │   │   ├── VarianceBridgeView.tsx
│   │   │   │   ├── VarianceBridgeChart.tsx
│   │   │   │   ├── DependencyTreeViewer.tsx
│   │   │   │   ├── DependencyGraphViewer.tsx  # Interactive DAG
│   │   │   │   ├── FormulaExplainPanel.tsx
│   │   │   │   ├── CovenantMonitor.tsx
│   │   │   │   └── CreditScorecardViewer.tsx
│   │   │   └── extensions/
│   │   │       └── ExtensionsConsole.tsx
│   │   │
│   │   └── scenarios/          # Cross-domain scenarios
│   │       ├── ScenarioBuilder.tsx
│   │       ├── ScenarioLibrary.tsx
│   │       ├── ScenarioExecutionPanel.tsx
│   │       ├── HorizonScenarioGrid.tsx
│   │       └── OperationEditor/
│   │           ├── CurveShockEditor.tsx
│   │           ├── EquityShockEditor.tsx
│   │           ├── VolShockEditor.tsx
│   │           ├── FxShockEditor.tsx
│   │           ├── StatementShockEditor.tsx
│   │           └── TimeRollEditor.tsx
│   │
│   ├── hooks/                  # WASM Integration Layer
│   │   ├── useFinstack.tsx     # Core context provider
│   │   ├── useFinstackEngine.ts # Unified engine worker access
│   │   ├── useValuation.ts     # Instrument pricing (via engine)
│   │   ├── useStatement.ts     # Statement evaluation (via engine)
│   │   ├── useMarketData.ts    # Market context management
│   │   ├── useScenario.ts      # Scenario application
│   │   ├── usePortfolio.ts     # Portfolio operations
│   │   ├── useCalibration.ts   # Curve/surface calibration
│   │   ├── useMetrics.ts       # Metrics registry access
│   │   ├── useDraftMode.ts     # Transactional editing state
│   │   └── useExport.ts        # DataFrame/CSV exports
│   │
│   ├── workers/                # Web Worker (UNIFIED)
│   │   ├── finstackEngine.ts   # Single stateful engine (Handle pattern)
│   │   └── types.ts            # Worker API types
│   │
│   ├── store/                  # Global State (Zustand)
│   │   ├── financial.ts        # Domain data slice
│   │   ├── ui.ts               # UI state slice
│   │   ├── history.ts          # Undo/redo state
│   │   └── middleware/
│   │       ├── persist.ts      # localStorage/IndexedDB
│   │       ├── undo.ts         # Undo/redo middleware
│   │       └── sync.ts         # Cross-tab synchronization
│   │
│   ├── engine/                 # GenUI Renderer
│   │   ├── DynamicRenderer.tsx
│   │   ├── ComponentRegistry.ts
│   │   ├── schemaGenerator.ts  # Zod → OpenAI function schema
│   │   └── llmAdapter.ts       # LLM response → component mapping
│   │
│   ├── schemas/                # Zod Definitions (The "LLM Interface")
│   │   ├── generated/         # AUTO-GENERATED from Rust (ts-rs/specta)
│   │   │   ├── BondSpec.ts
│   │   │   ├── SwapSpec.ts
│   │   │   ├── MoneySpec.ts
│   │   │   ├── DateSpec.ts
│   │   │   └── ...            # Generated during build
│   │   ├── instruments/       # Zod wrappers around generated types
│   │   │   ├── bond.ts
│   │   │   ├── swap.ts
│   │   │   └── ...
│   │   ├── market/
│   │   │   ├── curves.ts
│   │   │   ├── surfaces.ts
│   │   │   └── scalars.ts
│   │   ├── statements/
│   │   │   ├── model.ts
│   │   │   ├── forecast.ts
│   │   │   └── analysis.ts
│   │   ├── portfolio/
│   │   │   ├── position.ts
│   │   │   ├── entity.ts
│   │   │   └── metrics.ts
│   │   ├── scenarios/
│   │   │   ├── spec.ts
│   │   │   └── operations.ts
│   │   ├── dynamic.ts         # Runtime schema injection for LLM
│   │   └── dashboard.ts
│   │
│   └── utils/
│       ├── formatting.ts       # Number/currency formatting
│       ├── validation.ts       # Input validation helpers
│       └── accessibility.ts    # a11y utilities
```

### 3.4 Hooks Layer Specification

The hooks layer provides a clean abstraction between React components and WASM bindings, handling async initialization, error boundaries, and memoization.

#### 3.4.1 Singleton WASM Initialization

Ensure `init()` is called **once globally**, not once per provider if nested:

```typescript
// lib/wasmSingleton.ts
let wasmInitPromise: Promise<void> | null = null;

export function ensureWasmInit(): Promise<void> {
  if (!wasmInitPromise) {
    wasmInitPromise = init();
  }
  return wasmInitPromise;
}

// Gate for SSR environments
export function canInitWasm(): boolean {
  return typeof window !== 'undefined';
}
```

#### 3.4.2 Core Context Provider

```typescript
// hooks/useFinstack.tsx
import { createContext, useContext, useEffect, useState, ReactNode } from 'react';
import { ensureWasmInit, canInitWasm } from '../lib/wasmSingleton';
import { FinstackConfig, MarketContext } from 'finstack-wasm';

interface FinstackContextValue {
  isReady: boolean;
  isLoading: boolean;
  error: Error | null;
  config: FinstackConfig;
  market: MarketContext;
  setMarket: (market: MarketContext) => void;
  // Rounding context for display
  roundingContext: RoundingContextInfo;
}

const FinstackContext = createContext<FinstackContextValue | null>(null);

export function FinstackProvider({ children }: { children: ReactNode }) {
  const [isReady, setIsReady] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [market, setMarket] = useState<MarketContext | null>(null);

  useEffect(() => {
    // SSR guard
    if (!canInitWasm()) {
      setIsLoading(false);
      return;
    }

    let mounted = true;

    // Use singleton to prevent double initialization
    ensureWasmInit()
      .then(() => {
        if (mounted) {
          setIsReady(true);
          setMarket(new MarketContext());
        }
      })
      .catch((err) => {
        if (mounted) setError(err);
      })
      .finally(() => {
        if (mounted) setIsLoading(false);
      });

    return () => { mounted = false; };
  }, []);

  // ... context value
}

export function useFinstack() {
  const context = useContext(FinstackContext);
  if (!context) {
    throw new Error('useFinstack must be used within FinstackProvider');
  }
  return context;
}
```

#### 3.4.3 Valuation Hook

**Key considerations:**
* `options.instrument` object identity may change often → use stable ID for deps
* Avoid recompute storms with proper memoization
* Implement automatic caching for identical inputs

```typescript
// hooks/useValuation.ts
import { useMemo, useCallback, useState, useEffect, useRef } from 'react';
import { useFinstack } from './useFinstack';
import { useFinstackEngine } from './useFinstackEngine';

interface UseValuationOptions {
  // Use stable ID to prevent recompute storms
  instrumentId: string;
  instrumentData: InstrumentWire;
  enabledMetrics?: string[];
  // Auto-determined based on thresholds if not specified
  useWorker?: boolean;
}

interface ValuationResult {
  // String values - no float precision loss
  pv: string | null;
  pvCurrency: string | null;
  metrics: Record<string, string>;  // All values as strings
  cashflows: CashflowWire[];
  isLoading: boolean;
  error: Error | null;
  refetch: () => void;
}

// Simple LRU cache for recent computations
const computationCache = new Map<string, ValuationResult>();
const MAX_CACHE_SIZE = 50;

function getCacheKey(instrumentId: string, marketHash: string): string {
  return `${instrumentId}:${marketHash}`;
}

export function useValuation(options: UseValuationOptions): ValuationResult {
  const { isReady, marketHash } = useFinstack();
  const engine = useFinstackEngine();

  const [result, setResult] = useState<ValuationResult>({
    pv: null,
    pvCurrency: null,
    metrics: {},
    cashflows: [],
    isLoading: false,
    error: null,
    refetch: () => {},
  });

  // Memoize instrument JSON to prevent unnecessary recomputation
  const instrumentJson = useMemo(
    () => JSON.stringify(options.instrumentData),
    [options.instrumentId, options.instrumentData]  // Use stable ID
  );

  const compute = useCallback(async () => {
    if (!isReady || !engine || !options.instrumentData) return;

    // Check cache first
    const cacheKey = getCacheKey(options.instrumentId, marketHash);
    const cached = computationCache.get(cacheKey);
    if (cached) {
      setResult({ ...cached, refetch: compute });
      return;
    }

    setResult(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      // Engine uses Handle Pattern - market already loaded
      const workerResult = await engine.priceInstrument(instrumentJson);

      const newResult: ValuationResult = {
        pv: workerResult.pv,  // Already a string from Rust
        pvCurrency: workerResult.pvCurrency,
        metrics: workerResult.metrics,  // All strings
        cashflows: workerResult.cashflows,
        isLoading: false,
        error: null,
        refetch: compute,
      };

      // Update cache (with size limit)
      if (computationCache.size >= MAX_CACHE_SIZE) {
        const firstKey = computationCache.keys().next().value;
        computationCache.delete(firstKey);
      }
      computationCache.set(cacheKey, newResult);

      setResult(newResult);
    } catch (err) {
      setResult(prev => ({
        ...prev,
        isLoading: false,
        error: err as Error,
      }));
    }
  }, [isReady, engine, instrumentJson, marketHash, options.instrumentId]);

  // Initial computation - use instrumentId as stable dependency
  useEffect(() => {
    compute();
  }, [options.instrumentId, marketHash]);

  return result;
}
```

**Note:** The `refetch` function is returned directly, not stored in state. The cache prevents redundant WASM calls for unchanged inputs.

#### Statement Evaluation Hook

```typescript
// hooks/useStatement.ts
import { useMemo, useCallback, useState } from 'react';
import { useFinstack } from './useFinstack';

interface UseStatementOptions {
  model: StatementModel;
  scenarios?: ScenarioSpec[];
}

interface StatementResult {
  results: EvaluationResults | null;
  meta: EvaluationMeta | null;
  isLoading: boolean;
  error: Error | null;
  // Get value for specific node/period
  getValue: (nodeId: string, period: string) => number | null;
  // Get all values for a node
  getNodeSeries: (nodeId: string) => Record<string, number>;
  refetch: () => void;
}

export function useStatement(options: UseStatementOptions): StatementResult {
  const { isReady } = useFinstack();
  const [result, setResult] = useState<StatementResult>({
    results: null,
    meta: null,
    isLoading: false,
    error: null,
    getValue: () => null,
    getNodeSeries: () => ({}),
    refetch: () => {},
  });

  const evaluate = useCallback(async () => {
    if (!isReady || !options.model) return;

    setResult(prev => ({ ...prev, isLoading: true, error: null }));

    try {
      const evaluator = new JsEvaluator();
      const evalResults = evaluator.evaluate(options.model);

      setResult({
        results: evalResults,
        meta: evalResults.meta,
        isLoading: false,
        error: null,
        getValue: (nodeId, period) => evalResults.get(nodeId, period),
        getNodeSeries: (nodeId) => evalResults.getNodeSeries(nodeId),
        refetch: evaluate,
      });
    } catch (err) {
      setResult(prev => ({
        ...prev,
        isLoading: false,
        error: err as Error,
      }));
    }
  }, [isReady, options.model]);

  useEffect(() => { evaluate(); }, [evaluate]);

  return result;
}
```

#### Web Worker Hook

```typescript
// hooks/useWasmWorker.ts
import { useEffect, useRef, useCallback } from 'react';
import { wrap, Remote } from 'comlink';
import type { ValuationWorkerAPI } from '../workers/valuationWorker';

export function useWasmWorker<T>(
  workerPath: string
): { worker: Remote<T> | null; terminate: () => void } {
  const workerRef = useRef<Worker | null>(null);
  const apiRef = useRef<Remote<T> | null>(null);

  useEffect(() => {
    workerRef.current = new Worker(
      new URL(workerPath, import.meta.url),
      { type: 'module' }
    );
    apiRef.current = wrap<T>(workerRef.current);

    return () => {
      workerRef.current?.terminate();
    };
  }, [workerPath]);

  const terminate = useCallback(() => {
    workerRef.current?.terminate();
    workerRef.current = null;
    apiRef.current = null;
  }, []);

  return { worker: apiRef.current, terminate };
}
```

### 3.5 Detailed Domain Designs

#### A. Domain: Valuations (`finstack-wasm/valuations`)

Focus: Visualizing the output of `pricer.rs`, `metrics/`, and `attribution/` across all instruments, with full access to calibration, cashflows, margin, covenants, and result envelopes.

1. **`RiskHeatmap`**
    * **Purpose:** Display Greeks (Delta, Gamma, Vega) across a portfolio.
    * **Tech:** TanStack Table with dynamic cell coloring.
    * **Features:** Grouping by Currency/Sector; Drill-down to individual trades.
    * **Accessibility:** Color-blind safe palette, keyboard navigation.

2. **`CashflowWaterfall`**
    * **Purpose:** Inspect intermediate cashflows of a trade (e.g., Swaps).
    * **Tech:** TanStack Table + TanStack Virtual (Virtualization required for 30Y swaps with 120+ rows).
    * **Columns:** Period | Fix/Float | Rate | Notional | Discount Factor | PV.

3. **`PnLAttributionBridge`**
    * **Purpose:** Explain PnL changes (e.g., "Why did we lose money?").
    * **Tech:** ECharts Waterfall.
    * **Data:** Visualizes the `AttributionResult` struct from `attribution.rs`.

4. **Instrument Panels (1:1 with `valuations::instruments`)**
    * **Purpose:** Provide instrument-specific pricing and inspection surfaces that mirror each WASM instrument binding (e.g., `Bond`, `InterestRateSwap`, `FxOption`, `ConvertibleBond`, `VarianceSwap`).
    * **Components:** For every exported instrument in `finstack-wasm/src/valuations/instruments` there will be:
        * **Inputs section:** A typed form component (e.g., `BondPanel`, `InterestRateSwapPanel`, `FxOptionPanel`) that wraps form controls for instrument attributes and calls `useValuation` under the hood.
        * **Cashflows section (discountable instruments only):** A reusable `CashflowWaterfall` (or related cashflow table component) embedded inside the instrument UI to show projected cashflows and discount factors.
        * **Market data section:** Read-only viewers for the relevant curves/surfaces used in pricing (e.g., `CurveChart`, `VolSurfaceViewer`, quote tickers) so users and LLMs can see exactly which market data is driving the valuation.
        * **Outputs section:** A metrics table showing PV and risk measures (DV01/CS01, bucketed risk, Greeks, etc.) in a consistent, schema-backed layout.
        * **Viewer variant:** A read-only viewer (e.g., `BondViewer`, `SwaptionViewer`) for displaying key terms, cashflows and metrics without editing.
    * **Invariants:** Adding a new instrument to `valuations::instruments` requires adding/auto-generating the corresponding panel + viewer components and their Zod schemas, so LLMs can reliably target them via the GenUI bridge.

5. **Calibration Views (1:1 with `valuations::calibration`)**
    * **Purpose:** Provide dedicated UIs for curve and surface calibration that match each WASM calibrator.
    * **Components:**
        * `DiscountCurveCalibration` - Quote grid, config editor, curve visualization
        * `ForwardCurveCalibration` - Deposits/FRAs/Swaps quote entry, tenor selection
        * `HazardCurveCalibration` - CDS quote entry, recovery rate config
        * `InflationCurveCalibration` - ZC inflation swap quotes, seasonality
        * `VolSurfaceCalibration` - Strike/expiry grid, SABR/SVI params
        * `BaseCorrelationCalibration` - Detachment points, tranche quotes
        * `SabrCalibration` - α, β, ρ, ν parameter fitting
        * `HullWhiteCalibration` - a, σ parameter calibration from swaptions
    * **Behavior:** Each view owns quote entry grids (typed to `RatesQuote`, `CreditQuote`, `VolQuote`, `InflationQuote`), configuration editors (`CalibrationConfig`, `SolverKind`), and visualization of calibration reports (error diagnostics, convergence plots), with state fully serializable for LLM control.

6. **Metrics & Risk Registry**
    * **Components:** `InstrumentRiskTable`, `BucketedRiskGrid`, and `MetricsRegistryBrowser` mapped to `metrics/` and the global metrics registry.
    * **Features:** Per-instrument and per-bucket risk tables (DV01/CS01/Delta/Gamma/Vega/Theta), bucketed risk grids (by tenor/strike), and a browser for discovering which metrics are available for each instrument type.

7. **Margin Analytics**
    * **Components:** `InstrumentMarginPanel`, `SimmBreakdownView` backed by the valuations `margin` module (IM/VM calculators, CSA specs).
    * **Features:** Per-instrument and CSA-level margin details (IM, VM, total), SIMM risk-class breakdowns, and links into the portfolio-level margin views for aggregated context.

8. **Covenant Engine Surfaces**
    * **Components:** `InstrumentCovenantPanel`, `CovenantTimelineView` wired to the valuations `covenants` engine.
    * **Features:** Configure and visualize covenants attached to instruments (e.g., coverage ratios, leverage tests), see pass/fail status over time, and expose covenant breach timelines that can be reused by portfolio and statements domains.

9. **Attribution Module**
    * **Components:** `PnLAttributionBridge`, `FactorBreakdownPanel`, `BucketedAttributionGrid`.
    * **Features:** Waterfall visualization of PnL attribution factors (rates, credit, vol, FX, carry, residual), drill-down to factor-level detail (e.g., rates bucket by tenor), percentage breakdown display.

10. **Valuation Runs & Result Envelopes**
    * **Components:** `ValuationRunViewer` for inspecting `results/` envelopes (PV, risk vectors, metadata, FX policy, numeric mode).
    * **Features:** Show raw and FX-collapsed results, period aggregation details, and provide export hooks for downstream consumers (DataFrame/CSV) in a way that stays aligned with the valuations `results` module.

#### B. Domain: Portfolio (`finstack-wasm/portfolio`)

Focus: Entity-based position tracking, cross-currency aggregation, portfolio metrics, P&L attribution, margin, optimization, and scenario integration on top of valuations.

1. **Entity & Position Management**
    * **Components:** `TradeEntryForm`, `EntityTreeView`, and `PositionGrid` backed by `Portfolio`, `Entity`, `Position`, and `PositionUnit`.
    * **Features:** Entity-centric views (including dummy entity for standalone instruments), unit-aware quantity handling, tag editing (rating, sector, strategy), and per-position drill-down to the underlying instrument panels in the Valuations domain.

2. **Valuation & Metrics Dashboards**
    * **Components:** `PortfolioSummaryPanel` and `PortfolioMetricsPanel` mapped to `value_portfolio`, `PortfolioValuation`, and `PortfolioMetrics`.
    * **Features:** Totals by portfolio and entity, cross-currency aggregation to base currency with explicit FX policies, and display of aggregated DV01/CS01/Greeks alongside non-summable metrics at position level.

3. **Attribute-Based Grouping & Pivots**
    * **Components:** `TagPivotGrid` using grouping and aggregation helpers from `grouping.rs`.
    * **Features:** Interactive rollups by arbitrary tags (e.g., asset_class, rating, desk, strategy), with pivot-like controls and exportable grouped views.

4. **Scenario & What-If at Portfolio Level**
    * **Components:** `PortfolioScenarioImpactView` under `domains/portfolio/scenarios`, layered on top of the Scenarios domain and portfolio `apply_and_revalue` helpers.
    * **Features:** Before/after portfolio value and metric deltas by scenario, entity, and tag, reusing `ScenarioExecutionPanel` runs but focused on portfolio-level impacts.

5. **P&L Attribution**
    * **Components:** `PortfolioAttributionView` mapped to `PortfolioAttribution` and `attribute_portfolio_pnl`.
    * **Features:** Factor breakdown (rates, credit, inflation, vol, FX, carry, residual) with percentages of total, plus drill-down to per-position attribution and optional detailed factor views (e.g., rates curve buckets).

6. **Margin Aggregation**
    * **Components:** `MarginSummaryPanel`, `NettingSetView`, `SensitivityBreakdown` mirroring `margin::PortfolioMarginResult` and `NettingSetMargin`.
    * **Features:** IM/VM/Total margin by netting set, cleared vs bilateral splits, SIMM risk-class breakdowns, and optional views of aggregated sensitivities; designed to sit alongside `PortfolioSummaryPanel` for funding/risk oversight.

7. **Optimization Surface**
    * **Components:**
        * `PortfolioOptimizerView` - Main optimization interface
        * `ConstraintEditor` - Define constraints (position limits, sector caps, etc.)
        * `ConstraintViolationsPanel` - Show which constraints bind
        * `EfficientFrontierChart` - Risk/return scatter plot (Recharts)
        * `TradeProposalGrid` - Proposed buy/sell actions
        * `OptimizationResultView` - Pre/post comparison
    * **Backed by:** `PortfolioOptimizationProblem`, `PortfolioOptimizer`, `PortfolioOptimizationResult`, and `TradeSpec`.
    * **Features:** Express objectives and constraints over portfolio metrics (e.g., max yield with CCC limit), visualize proposed trades (buy/sell, size, direction), and compare pre/post-optimization portfolios.

8. **Cashflow Aggregation**
    * **Components:** `PortfolioCashflowAggregate` displaying aggregated cashflows across all positions.
    * **Features:** Timeline view of all portfolio cashflows, grouped by date/type, with currency aggregation.

9. **Data Exports**
    * **Components:** `DataFramePreview` with export actions backed by `positions_to_dataframe` and `entities_to_dataframe`.
    * **Features:** Preview before export, one-click DataFrame/CSV/Parquet exports of position- and entity-level results for external analysis, preserving stable schema from the portfolio crate.

#### C. Domain: Statements (`finstack-wasm/statements`)

Focus: Visualizing the `evaluator.rs` DAG, `registry.rs` assumptions, capital structure integration, templates, adjustments, and all analysis tooling under `analysis/`.

1. **`StatementViewer`**
    * **Purpose:** Interactive Financial Statement (Income Statement, Balance Sheet, Cashflow).
    * **Tech:** TanStack Table (Matrix View) backed by the evaluator `Results`.
    * **Interaction:** "Corkscrew tracing" powered by the `CorkscrewExtension` – clicking a cell highlights its precedent cells and roll-forward partners (e.g., *Ending Cash* highlights *Opening Cash* + *Net Income*).
    * **Accessibility:** Keyboard navigation between cells, ARIA labels for values.

2. **`ForecastEditor`**
    * **Purpose:** Node-level editor for deterministic/statistical/seasonal/time-series forecasts (`ForecastMethod`) and explicit values.
    * **Logic:** Edits the `StatementModel` JSON (value vs forecast vs formula), triggers WASM re-calc, and refreshes `StatementViewer` plus any open analysis views.

3. **`FormulaBar`**
    * **Purpose:** Excel-like formula editing experience.
    * **Features:** Syntax highlighting, autocomplete for node references, error indicators.

4. **Templates & Roll-Forwards**
    * **Components:** `RollForwardBuilder`, `VintageAnalysisView`, `VintageWaterfallView` mirror `templates::TemplatesExtension` and `VintageExtension`.
    * **Behavior:** Wizard-like builders that add connected nodes (inventory roll-forwards, vintage waterfalls) to a model, with inline validation hooks to `CorkscrewExtension`.

5. **Capital Structure Panel**
    * **Components:**
        * `CapitalStructurePanel` - Main overview with debt schedule
        * `DebtScheduleGrid` - TanStack Table for debt instruments
        * `WaterfallViewer` - Payment priority waterfall
        * `BondInstrumentForm`, `LoanInstrumentForm`, `SwapInstrumentForm` - Instrument editors
    * **Features:** Debt schedule viewer, cashflow breakdown (interest/principal/fees), and read-only views of capital-structure-driven metrics used inside statements (the `cs.*` DSL namespace).

6. **Scenario Management & Comparison**
    * **Components:** `ScenarioSetManager` and `ScenarioComparisonView`, mirroring `analysis::ScenarioSet`, `ScenarioResults`, and `ScenarioDiff`.
    * **Features:** CRUD for named scenarios with parent/override chains, evaluation controls, and wide comparison tables/variance bridges across scenarios (including baseline vs downside/stress).

7. **Sensitivity, Goal Seek & Monte Carlo**
    * **Components:**
        * `SensitivityAnalyzer` - Parameter selection and sweep config
        * `TornadoChart` - Sensitivity tornado visualization
        * `GoalSeekPanel` - Target metric, driver selection, solve button
        * `MonteCarloConfigEditor` - Distribution config per parameter
        * `MonteCarloResultsView` - Histograms, fan charts, percentile tables
    * **Backed by:** `SensitivityAnalyzer`, `goal_seek`, `MonteCarloConfig`, `MonteCarloResults`, `PercentileSeries`.
    * **Features:** Parameter grids & tornado charts, "solve-for-X" panels (targeting a metric by varying drivers), and distribution views for Monte Carlo runs.

8. **Forecast Backtesting & Variance Analysis**
    * **Components:**
        * `BacktestDashboard` - Forecast vs actual comparison
        * `BacktestAccuracyView` - `ForecastMetrics` visualization (MAE, MAPE, etc.)
        * `VarianceBridgeView` - Two-scenario variance analysis
        * `VarianceBridgeChart` - Renders `BridgeStep[]` as waterfall
    * **Backed by:** `backtest_forecast`, `ForecastMetrics`, `VarianceAnalyzer`, `VarianceReport`, `BridgeChart`.
    * **Features:** Forecast vs actual accuracy dashboards, variance/two-scenario bridge charts, and drill-down into period/metric-level deltas.

9. **Covenants & Credit Scorecards**
    * **Components:** `CovenantMonitor` and `CreditScorecardViewer`.
    * **Features:** Visualization of covenant tests and breaches (from `analysis::covenants`), credit scorecard outputs from `CreditScorecardExtension`.

10. **EBITDA Adjustments**
    * **Components:** `EbitdaAdjustmentsPanel` backed by `adjustments` module.
    * **Features:** Normalized EBITDA calculations, adjustments bridge waterfall, add-back/deduction categorization.

11. **Formula Explain & Dependency Tracing**
    * **Components:**
        * `DependencyTreeViewer` - Tree/table view of dependencies
        * `DependencyGraphViewer` - Interactive DAG (using vis-network or reactflow)
        * `FormulaExplainPanel` - Step-by-step formula breakdown
    * **Wired to:** `DependencyTracer`, `DependencyTree`, `FormulaExplainer`, `Explanation`, `ExplanationStep`.
    * **Features:** Interactive dependency graphs with per-period values, and step-by-step formula breakdowns usable for debugging and LLM explanations.

12. **Metric Registry & Results Export**
    * **Components:** `RegistryBrowser` and export actions on `StatementViewer` / `ScenarioComparisonView`.
    * **Features:** Browse/search metric namespaces (from the `registry` module, `fin.*` and custom), inspect definitions and dependencies, and export Polars-backed tables (DataFrame/CSV) from `results::export` and `ScenarioResults::to_comparison_df()`.

13. **Extensions Console**
    * **Components:** `ExtensionsConsole` surface that lists registered extensions from `extensions::ExtensionRegistry` (including `CorkscrewExtension` and `CreditScorecardExtension`).
    * **Features:** Run extensions on current models/results, inspect status/metadata, and show diagnostics panels alongside `StatementViewer`.

#### D. Domain: Market (`finstack-wasm/core`)

1. **`CurveEditor`**
    * **Purpose:** visualize and shock yield curves.
    * **Tech:** Recharts (Line) + Drag-and-Drop handles.
    * **Interaction:** Dragging a point updates the `MarketContext` and triggers a global re-price.

2. **`VolSurfaceViewer`**
    * **Purpose:** Display volatility surface as 2D heatmap.
    * **Tech:** ECharts heatmap.

3. **`VolSurface3D`**
    * **Purpose:** Interactive 3D volatility surface.
    * **Tech:** ECharts WebGL surface with Canvas 2D fallback.
    * **Fallback:** Detect WebGL support; fall back to 2D heatmap on unsupported devices.

4. **`FxMatrixViewer`**
    * **Purpose:** Display FX rate matrix.
    * **Tech:** TanStack Table with editable cells.

5. **`InflationIndexViewer`**
    * **Purpose:** Display CPI index series.
    * **Tech:** Recharts time series.

#### E. Domain: Scenarios (`finstack-wasm/scenarios`)

Focus: Providing a cross-domain scenario engine UI that mirrors `ScenarioSpec`, `OperationSpec`, `ScenarioEngine`, and `ExecutionContext` from the `finstack-scenarios` crate.

1. **`ScenarioBuilder`**
    * **Purpose:** JSON-backed editor for individual `ScenarioSpec` objects and their ordered list of `OperationSpec` variants (FX, curves, vols, statements, instruments, time roll-forward).
    * **Features:** Operation-type pickers, parameter forms (e.g., curve IDs, bp/pct shocks, tenors), and live validation against the active `MarketContext`/statement model.

2. **`ScenarioLibrary`**
    * **Purpose:** Manage a library of named scenarios with priorities, ready for composition via `ScenarioEngine::compose`.
    * **Features:** Tagging and grouping (e.g., "Q1 Stress", "Horizon 1M"), priority sliders, and JSON import/export that matches the serde-stable wire format.

3. **`ScenarioExecutionPanel`**
    * **Purpose:** Apply a single scenario or composed scenario set to the current `ExecutionContext` (Market + Statements + Valuations) and surface the resulting `ApplicationReport`.
    * **Features:** Phase-by-phase execution view (time roll → market → rate bindings → statements → re-eval), operation counts, and a structured warnings/error console aligned with `ApplicationReport::warnings`.

4. **`HorizonScenarioGrid`**
    * **Purpose:** Specialized viewer for horizon scenarios (e.g., 1W/1M/3M `TimeRollForward` plus shocks) across key metrics.
    * **Features:** Matrix layout with columns for horizons and rows for selected metrics (PV, DV01, statement KPIs); integrates with `ScenarioExecutionPanel` to re-use underlying `ScenarioEngine` runs.

5. **Operation Editors**
    * **Components:** Specialized editors for each operation type:
        * `CurveShockEditor` - Parallel/node shifts, curve selection
        * `EquityShockEditor` - Ticker selection, percentage shocks
        * `VolShockEditor` - Surface selection, parallel/bucket shocks
        * `FxShockEditor` - Currency pair selection, rate shocks
        * `StatementShockEditor` - Node selection, forecast adjustments
        * `TimeRollEditor` - Period selection, carry calculation options

### 3.6 Performance Architecture

Financial computations can block the main thread. The UI Kit employs several strategies to maintain responsiveness.

#### Web Worker Strategy

Heavy computations are offloaded to Web Workers using Comlink for type-safe communication:

```typescript
// workers/valuationWorker.ts
import { expose } from 'comlink';
import init, {
  Bond,
  MarketContext,
  FinstackConfig
} from 'finstack-wasm';

let initialized = false;

const api = {
  async initialize() {
    if (!initialized) {
      await init();
      initialized = true;
    }
  },

  async priceInstruments(
    instrumentsJson: string[],
    marketJson: string,
    configJson: string
  ): Promise<ValuationResult[]> {
    await this.initialize();

    const market = MarketContext.fromJSON(marketJson);
    const config = FinstackConfig.fromJSON(configJson);

    return instrumentsJson.map(json => {
      const instrument = deserializeInstrument(json);
      return {
        pv: instrument.price(market, config),
        metrics: instrument.computeAllMetrics(market, config),
        cashflows: instrument.cashflows(),
      };
    });
  },

  async runMonteCarlo(
    modelJson: string,
    configJson: string
  ): Promise<MonteCarloResults> {
    await this.initialize();
    // Heavy MC simulation runs in worker
    const model = JsStatementModel.fromJSON(modelJson);
    const config = MonteCarloConfig.fromJSON(configJson);
    return runMonteCarloSimulation(model, config);
  },

  async valuePortfolio(
    portfolioJson: string,
    marketJson: string
  ): Promise<PortfolioValuationResult> {
    await this.initialize();
    // Portfolio valuation with many positions
    const portfolio = JsPortfolio.fromJSON(portfolioJson);
    const market = MarketContext.fromJSON(marketJson);
    return valuePortfolio(portfolio, market, new FinstackConfig());
  },
};

expose(api);
export type ValuationWorkerAPI = typeof api;
```

#### Virtualization Strategy

Large data tables use TanStack Virtual for efficient rendering:

```typescript
// components/tables/VirtualDataTable.tsx
import { useVirtualizer } from '@tanstack/react-virtual';

export function VirtualDataTable<T>({
  data,
  columns,
  rowHeight = 35,
}: VirtualDataTableProps<T>) {
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: data.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => rowHeight,
    overscan: 10, // Render 10 extra rows for smooth scrolling
  });

  // Only render visible rows
  const virtualRows = virtualizer.getVirtualItems();

  return (
    <div ref={parentRef} className="h-[600px] overflow-auto">
      <div style={{ height: virtualizer.getTotalSize() }}>
        {virtualRows.map(virtualRow => (
          <TableRow
            key={virtualRow.key}
            data={data[virtualRow.index]}
            style={{
              position: 'absolute',
              top: virtualRow.start,
              height: rowHeight,
            }}
          />
        ))}
      </div>
    </div>
  );
}
```

#### WebGL Fallback Strategy

3D visualizations detect WebGL support and provide fallbacks:

```typescript
// components/charts/SurfaceViewer.tsx
import { useEffect, useState } from 'react';

function detectWebGL(): boolean {
  try {
    const canvas = document.createElement('canvas');
    return !!(
      window.WebGLRenderingContext &&
      (canvas.getContext('webgl') || canvas.getContext('experimental-webgl'))
    );
  } catch {
    return false;
  }
}

export function SurfaceViewer({ data, config }: SurfaceViewerProps) {
  const [hasWebGL] = useState(detectWebGL);

  if (hasWebGL) {
    return <Surface3DWebGL data={data} config={config} />;
  }

  // Fallback to 2D heatmap
  return <SurfaceHeatmap2D data={data} config={config} />;
}
```

#### Computation Thresholds

Guidelines for when to use workers vs main thread:

| Computation | Threshold | Strategy |
|------------|-----------|----------|
| Single instrument pricing | Always | Main thread |
| Portfolio valuation | > 10 positions | Web Worker |
| Monte Carlo | > 100 paths | Web Worker |
| Statement evaluation | > 50 nodes × 20 periods | Web Worker |
| Cashflow table | > 100 rows | Virtual scrolling |
| Risk heatmap | > 50 × 50 cells | Canvas rendering |

### 3.7 Development Guidelines

1. **WASM Separation:** UI components never import `finstack-wasm` directly. They use `hooks/` which manage Suspense, Error Boundaries, and async loading.
2. **Strict Typing:** Props must use `finstack` types (e.g., `Currency` enum), not strings.
3. **Metric Integrity:** Never format numbers manually. Use the `RoundingContext` from the WASM engine and `AmountDisplay` components.
4. **Error Handling:** Wrap all "Organisms" in Error Boundaries to gracefully handle WASM panics (e.g., "Curve missing for GBP").
5. **State Serialization:** All component state must be JSON-serializable for LLM snapshots.

#### Accessibility Requirements

* **Keyboard Navigation:** All interactive elements must be keyboard accessible.
* **ARIA Labels:** Financial data tables must have proper ARIA labels for screen readers.
* **Focus Management:** Modal dialogs and drawers must trap focus appropriately.
* **High Contrast:** Support high-contrast mode for trading floor environments.
* **Announcements:** Real-time value changes should be announced to screen readers.

```typescript
// utils/accessibility.ts
export function announceToScreenReader(message: string) {
  const announcement = document.createElement('div');
  announcement.setAttribute('aria-live', 'polite');
  announcement.setAttribute('aria-atomic', 'true');
  announcement.className = 'sr-only';
  announcement.textContent = message;
  document.body.appendChild(announcement);
  setTimeout(() => announcement.remove(), 1000);
}

// Usage in components
function PriceDisplay({ value, currency }: PriceDisplayProps) {
  const prevValue = usePrevious(value);

  useEffect(() => {
    if (prevValue !== undefined && value !== prevValue) {
      const change = value > prevValue ? 'increased' : 'decreased';
      announceToScreenReader(
        `${currency} price ${change} to ${formatAmount(value)}`
      );
    }
  }, [value, prevValue, currency]);

  return <AmountDisplay value={value} currency={currency} />;
}
```

#### Internationalization

* **Number Formatting:** Use `Intl.NumberFormat` with `RoundingContext` settings.
* **Currency Display:** ISO-4217 codes with localized symbols.
* **Date Formatting:** Business calendar + locale-aware display.

```typescript
// utils/formatting.ts
export function formatAmount(
  value: number,
  currency: Currency,
  locale: string = navigator.language
): string {
  return new Intl.NumberFormat(locale, {
    style: 'currency',
    currency: currency.code,
    minimumFractionDigits: currency.decimals,
    maximumFractionDigits: currency.decimals,
  }).format(value);
}

export function formatRate(
  value: number,
  asBasisPoints: boolean = false,
  locale: string = navigator.language
): string {
  if (asBasisPoints) {
    return `${(value * 10000).toFixed(2)} bps`;
  }
  return new Intl.NumberFormat(locale, {
    style: 'percent',
    minimumFractionDigits: 2,
    maximumFractionDigits: 4,
  }).format(value);
}
```

#### Theming

Support both light and dark themes with CSS variables:

```css
/* themes/financial.css */
:root {
  /* Light theme */
  --color-positive: #16a34a;
  --color-negative: #dc2626;
  --color-neutral: #6b7280;

  /* Risk heatmap colors */
  --heatmap-low: #10b981;
  --heatmap-mid: #f59e0b;
  --heatmap-high: #ef4444;

  /* Chart colors */
  --chart-1: hsl(221, 83%, 53%);
  --chart-2: hsl(142, 76%, 36%);
  --chart-3: hsl(38, 92%, 50%);
}

[data-theme="dark"] {
  --color-positive: #22c55e;
  --color-negative: #f87171;
  --color-neutral: #9ca3af;
}
```

### 3.8 Testing Strategy

#### Unit Tests (Vitest + React Testing Library)

```typescript
// __tests__/components/AmountDisplay.test.tsx
import { render, screen } from '@testing-library/react';
import { AmountDisplay } from '../components/primitives/AmountDisplay';
import { Currency } from 'finstack-wasm';

// Mock WASM module
vi.mock('finstack-wasm', () => ({
  Currency: vi.fn().mockImplementation((code) => ({ code, decimals: 2 })),
}));

describe('AmountDisplay', () => {
  it('formats USD amount correctly', () => {
    render(<AmountDisplay value={1234.56} currency="USD" />);
    expect(screen.getByText('$1,234.56')).toBeInTheDocument();
  });

  it('respects decimal places from currency', () => {
    render(<AmountDisplay value={100} currency="JPY" />);
    expect(screen.getByText('¥100')).toBeInTheDocument();
  });

  it('applies positive/negative styling', () => {
    const { rerender } = render(<AmountDisplay value={100} currency="USD" />);
    expect(screen.getByText('$100.00')).toHaveClass('text-positive');

    rerender(<AmountDisplay value={-100} currency="USD" />);
    expect(screen.getByText('-$100.00')).toHaveClass('text-negative');
  });
});
```

#### Integration Tests (Playwright)

```typescript
// e2e/calibration.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Discount Curve Calibration', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/examples/calibration');
    // Wait for WASM to initialize
    await page.waitForSelector('[data-testid="wasm-ready"]');
  });

  test('calibrates curve from deposit quotes', async ({ page }) => {
    // Enter quotes
    await page.fill('[data-testid="quote-rate-0"]', '0.05');
    await page.fill('[data-testid="quote-tenor-0"]', '1Y');

    // Run calibration
    await page.click('[data-testid="calibrate-button"]');

    // Verify success
    await expect(page.locator('[data-testid="calibration-status"]'))
      .toHaveText('Success');

    // Verify curve is displayed
    await expect(page.locator('[data-testid="curve-chart"]'))
      .toBeVisible();
  });
});
```

#### Visual Regression (Storybook + Chromatic)

```typescript
// stories/CurveChart.stories.tsx
import type { Meta, StoryObj } from '@storybook/react';
import { CurveChart } from '../components/charts/CurveChart';

const meta: Meta<typeof CurveChart> = {
  component: CurveChart,
  title: 'Charts/CurveChart',
  parameters: {
    chromatic: { viewports: [320, 768, 1200] },
  },
};

export default meta;
type Story = StoryObj<typeof CurveChart>;

export const DiscountCurve: Story = {
  args: {
    data: [
      { time: 0.25, value: 0.995 },
      { time: 0.5, value: 0.99 },
      { time: 1, value: 0.98 },
      { time: 2, value: 0.95 },
      { time: 5, value: 0.88 },
      { time: 10, value: 0.75 },
    ],
    config: {
      title: 'USD SOFR Discount Curve',
      yLabel: 'Discount Factor',
      xLabel: 'Maturity (Years)',
    },
  },
};

export const ForwardCurve: Story = {
  args: {
    data: [
      { time: 0.25, value: 0.05 },
      { time: 0.5, value: 0.052 },
      { time: 1, value: 0.055 },
      { time: 2, value: 0.058 },
      { time: 5, value: 0.06 },
    ],
    config: {
      title: 'USD SOFR Forward Curve',
      yLabel: 'Rate',
      xLabel: 'Maturity (Years)',
      yFormatter: (v) => `${(v * 100).toFixed(2)}%`,
    },
  },
};
```

#### Golden Tests (Rust Parity)

```typescript
// __tests__/parity/bondPricing.test.ts
import { describe, it, expect } from 'vitest';
import init, { Bond, MarketContext, FinstackConfig } from 'finstack-wasm';
import goldenValues from '../../fixtures/golden/bond_pricing.json';

describe('Bond Pricing Parity', () => {
  beforeAll(async () => {
    await init();
  });

  goldenValues.testCases.forEach((testCase) => {
    it(`matches Rust output for ${testCase.name}`, () => {
      const bond = Bond.fromJSON(testCase.instrument);
      const market = MarketContext.fromJSON(testCase.market);
      const config = new FinstackConfig();

      const pv = bond.price(market, config);

      // Match to 6 decimal places (Decimal precision)
      expect(pv.amount).toBeCloseTo(testCase.expected.pv, 6);
    });
  });
});
```

### 3.9 Critical Architecture Patterns

The intersection of heavy WASM computation, virtualized UI rendering, and LLM orchestration creates specific implementation challenges. This section documents patterns to avoid common "trap doors."

#### 3.9.1 The Serialization Bottleneck

**Problem:** JSON serialization (`toJSON()` / `fromJSON()`) to bridge the Rust/JS boundary is expensive. A `MarketContext` with yield curves, vol surfaces, and fixings can easily exceed 5-10MB. If a user drags a curve point in `CurveEditor` and the system serializes the whole market for every pixel of drag, frames will drop. Serialization cost often exceeds actual calculation cost.

**Solution: The Handle Pattern**

Do not move heavy data between the Main Thread and Worker unless necessary. The UI should hold **Handles** (opaque references) to objects that live in WASM linear memory.

```typescript
// BAD: Full serialization on every call
async price(instrumentJson: string, marketJson: string): Promise<Result>

// GOOD: Handle-based - market lives in worker memory
async price(instrumentJson: string, marketHandleId: string): Promise<Result>
```

**Implementation:**

```typescript
// workers/finstackEngine.ts - Unified stateful engine
import { expose } from 'comlink';
import init, { MarketContext, FinstackConfig } from 'finstack-wasm';

// State lives in the worker
let marketContext: MarketContext | null = null;
let config: FinstackConfig | null = null;
const instruments = new Map<string, any>();

const engine = {
  async initialize() {
    await init();
    config = new FinstackConfig();
    marketContext = new MarketContext();
  },

  // Load market ONCE, return a handle
  async loadMarket(marketJson: string): Promise<string> {
    marketContext = MarketContext.fromJSON(marketJson);
    return 'market-main'; // Handle ID
  },

  // Send DELTA updates, not full objects
  async applyShock(curveId: string, shockBps: number): Promise<void> {
    if (!marketContext) throw new Error('Market not loaded');
    // Mutate in-place, no serialization
    marketContext.shiftCurve(curveId, shockBps);
  },

  // Price using handle reference
  async priceInstrument(instrumentJson: string): Promise<ValuationResult> {
    if (!marketContext || !config) throw new Error('Not initialized');
    const instrument = deserializeInstrument(instrumentJson);
    return {
      pv: instrument.price(marketContext, config),
      metrics: instrument.computeAllMetrics(marketContext, config),
    };
  },
};

expose(engine);
export type FinstackEngineAPI = typeof engine;
```

**Delta Updates for Interactive Editing:**

```typescript
// hooks/useCurveEditor.ts
export function useCurveEditor(curveId: string) {
  const engine = useFinstackEngine();

  // Debounced shock application during drag
  const applyShock = useDeferredValue(
    useCallback(async (shockBps: number) => {
      // Send only the delta, not the whole market
      await engine.applyShock(curveId, shockBps);
      // Trigger re-price of affected instruments
      invalidateQueries(['valuations', curveId]);
    }, [curveId, engine])
  );

  return { applyShock };
}
```

#### 3.9.2 Numeric Precision: The BigInt Problem

**Problem:** JavaScript uses IEEE 754 floats. Rust uses `rust_decimal` or fixed-point math.

```javascript
// In Rust: 0.1 + 0.2 = 0.3
// In JS:   0.1 + 0.2 = 0.30000000000000004
```

If you pass a calculated PV from WASM to JS as a `number`, you have already lost precision.

**Solution: String Transport**

All monetary values crossing the WASM bridge must be **Strings** or a custom struct:

```typescript
// types/money.ts
interface MoneyTransport {
  /** String representation preserving all decimal places */
  value: string;
  /** ISO-4217 currency code */
  currency: string;
  /** Number of decimal places in the value */
  scale: number;
}

// Example: { value: "1234567.89012345", currency: "USD", scale: 8 }
```

**Critical Rule: No Math in JavaScript**

The `AmountDisplay` component takes a string and formats it. **Never** perform arithmetic in the UI layer:

```typescript
// components/primitives/AmountDisplay.tsx
interface AmountDisplayProps {
  /** String value from WASM - DO NOT convert to number */
  value: string;
  currency: string;
  /** Optional: override scale for display */
  displayScale?: number;
}

export function AmountDisplay({ value, currency, displayScale }: AmountDisplayProps) {
  // Format WITHOUT parsing to float
  const formatted = formatDecimalString(value, currency, displayScale);
  return <span className="font-mono tabular-nums">{formatted}</span>;
}

// BAD: Never do this
function BadSumFooter({ values }: { values: number[] }) {
  const sum = values.reduce((a, b) => a + b, 0); // PRECISION LOSS
  return <span>{sum}</span>;
}

// GOOD: Ask WASM to compute aggregates
function GoodSumFooter({ columnId }: { columnId: string }) {
  const { data } = useQuery(['column-sum', columnId], () =>
    engine.computeColumnSum(columnId) // Returns string
  );
  return <AmountDisplay value={data} currency="USD" />;
}
```

#### 3.9.3 Unified Worker Strategy

**Problem:** Separate workers (`valuationWorker`, `statementWorker`, `portfolioWorker`) force loading the Market Context into memory multiple times. In a browser environment, this is wasteful and can cause OOM on large market data sets.

**Solution: Single Finstack Engine Worker**

Use one **"Finstack Engine Worker"** that holds the shared Market Context and handles requests from all domains:

```typescript
// workers/finstackEngine.ts
const engine = {
  // Shared state
  market: null as MarketContext | null,
  config: null as FinstackConfig | null,
  models: new Map<string, StatementModel>(),
  portfolios: new Map<string, Portfolio>(),

  // Initialization
  async initialize(): Promise<void>,
  async loadMarket(json: string): Promise<string>,

  // Valuations domain
  async priceInstrument(json: string): Promise<ValuationResult>,
  async computeMetrics(instrumentId: string, metrics: string[]): Promise<Record<string, string>>,

  // Statements domain
  async loadModel(id: string, json: string): Promise<void>,
  async evaluateModel(id: string): Promise<EvaluationResult>,
  async runMonteCarlo(modelId: string, config: MonteCarloConfig): Promise<MonteCarloResult>,

  // Portfolio domain
  async loadPortfolio(id: string, json: string): Promise<void>,
  async valuePortfolio(id: string): Promise<PortfolioValuationResult>,
  async runOptimization(portfolioId: string, problem: OptimizationProblem): Promise<OptimizationResult>,

  // Scenarios domain (operates on loaded data)
  async applyScenario(spec: ScenarioSpec): Promise<ApplicationReport>,
};

expose(engine);
```

**Directory structure update:**

```text
├── workers/
│   ├── finstackEngine.ts    # Single unified worker
│   └── types.ts             # Shared worker types
```

#### 3.9.4 Panic Hooks & Error Recovery

**Problem:** If Rust panics (e.g., `unwrap()` on `None`), the WASM module aborts, killing the entire browser tab.

**Solution:** Use `std::panic::set_hook` in Rust to catch panics and throw them as JavaScript exceptions:

```rust
// finstack-wasm/src/lib.rs
use wasm_bindgen::prelude::*;
use std::panic;

#[wasm_bindgen(start)]
pub fn init_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        // Convert panic to a JS-readable error
        let msg = panic_info.to_string();
        web_sys::console::error_1(&msg.into());
    }));
}
```

**React Error Boundary Integration:**

```typescript
// components/common/WasmErrorBoundary.tsx
interface WasmError extends Error {
  wasmStackTrace?: string;
  recoverable: boolean;
}

export function WasmErrorBoundary({ children, fallback }: Props) {
  return (
    <ErrorBoundary
      fallbackRender={({ error, resetErrorBoundary }) => (
        <WasmErrorFallback
          error={error as WasmError}
          onRetry={resetErrorBoundary}
        />
      )}
      onError={(error) => {
        // Log to telemetry
        captureWasmError(error);
      }}
    >
      {children}
    </ErrorBoundary>
  );
}

function WasmErrorFallback({ error, onRetry }: FallbackProps) {
  const isRecoverable = error.recoverable !== false;

  return (
    <Alert variant="destructive">
      <AlertTitle>Calculation Error</AlertTitle>
      <AlertDescription>
        {error.message}
        {error.wasmStackTrace && (
          <details className="mt-2">
            <summary>Technical Details</summary>
            <pre className="text-xs">{error.wasmStackTrace}</pre>
          </details>
        )}
      </AlertDescription>
      {isRecoverable && (
        <Button onClick={onRetry} className="mt-2">
          Retry Calculation
        </Button>
      )}
    </Alert>
  );
}
```

#### 3.9.5 Transactional UX: Draft Mode

**Problem:** Financial models are often "invalid" while being edited (e.g., incomplete curves, missing references). Triggering WASM recalculation on every keystroke causes errors and poor UX.

**Solution:** Implement a "Draft" state with explicit commit:

```typescript
// store/draft.ts
import { create } from 'zustand';

interface DraftState {
  // Pending changes not yet sent to WASM
  pendingChanges: Map<string, any>;
  isDirty: boolean;
  validationErrors: ValidationError[];

  // Actions
  addChange: (key: string, value: any) => void;
  validate: () => Promise<ValidationError[]>;
  commit: () => Promise<void>;  // Send to WASM
  discard: () => void;
}

export const useDraftStore = create<DraftState>((set, get) => ({
  pendingChanges: new Map(),
  isDirty: false,
  validationErrors: [],

  addChange: (key, value) => {
    set(state => {
      const changes = new Map(state.pendingChanges);
      changes.set(key, value);
      return { pendingChanges: changes, isDirty: true };
    });
  },

  validate: async () => {
    const { pendingChanges } = get();
    // Lightweight validation without full recalc
    const errors = await engine.validateChanges(
      Object.fromEntries(pendingChanges)
    );
    set({ validationErrors: errors });
    return errors;
  },

  commit: async () => {
    const { pendingChanges, validate } = get();
    const errors = await validate();
    if (errors.length > 0) {
      throw new ValidationError('Cannot commit with errors');
    }
    // Now trigger full WASM recalculation
    await engine.applyChanges(Object.fromEntries(pendingChanges));
    set({ pendingChanges: new Map(), isDirty: false });
  },

  discard: () => {
    set({ pendingChanges: new Map(), isDirty: false, validationErrors: [] });
  },
}));
```

**UI Integration with `useDeferredValue`:**

```typescript
// hooks/useDeferredCalculation.ts
import { useDeferredValue, useTransition } from 'react';

export function useDeferredCalculation<T>(
  calculate: () => Promise<T>,
  deps: any[]
) {
  const [isPending, startTransition] = useTransition();
  const [result, setResult] = useState<T | null>(null);

  // Defer non-urgent updates
  const deferredDeps = useDeferredValue(deps);

  useEffect(() => {
    startTransition(async () => {
      const value = await calculate();
      setResult(value);
    });
  }, [deferredDeps]);

  return { result, isPending };
}
```

### 3.10 GenUI Architecture & LLM Safety

The GenUI system is the core of "LLM can build UIs." This section covers the complete architecture.

#### 3.10.1 Component Registry (Typed)

The `ComponentRegistry` maps string types to typed React components with schema validation:

```typescript
// engine/ComponentRegistry.ts
import { z } from 'zod';
import { ComponentType } from 'react';

interface RegisteredComponent<TProps = any> {
  Component: ComponentType<TProps>;
  propsSchema: z.ZodType<TProps>;
  // Metadata for LLM
  description: string;
  exampleProps: TProps;
  // Safety
  allowedModes: ('viewer' | 'editor' | 'llm-assisted')[];
}

export const ComponentRegistry: Record<string, RegisteredComponent> = {
  RiskHeatmap: {
    Component: RiskHeatmap,
    propsSchema: RiskHeatmapPropsSchema,
    description: 'Displays portfolio risk metrics as a color-coded heatmap',
    exampleProps: { metric: 'DV01', grouping: 'Sector' },
    allowedModes: ['viewer', 'llm-assisted'],
  },
  CurveEditor: {
    Component: CurveEditor,
    propsSchema: CurveEditorPropsSchema,
    description: 'Interactive yield curve visualization and editing',
    exampleProps: { curveId: 'USD-SOFR', showHandles: true },
    allowedModes: ['viewer', 'editor'],
  },
  PositionGrid: {
    Component: PositionGrid,
    propsSchema: PositionGridPropsSchema,
    description: 'Tabular view of portfolio positions with metrics',
    exampleProps: { portfolioId: 'main', columns: ['pv', 'dv01'] },
    allowedModes: ['viewer', 'editor', 'llm-assisted'],
  },
  // ... all registered components
};

// Generate LLM documentation from registry
export function generateComponentDocs(): string {
  return Object.entries(ComponentRegistry)
    .map(([type, def]) =>
      `- **${type}**: ${def.description}\n  Example: ${JSON.stringify(def.exampleProps)}`
    )
    .join('\n');
}
```

#### 3.10.2 Dynamic Renderer with Validation

```typescript
// engine/DynamicRenderer.tsx
import { ComponentRegistry } from './ComponentRegistry';
import { DashboardDefinition } from '../schemas/dashboard';

interface DynamicRendererProps {
  dashboard: DashboardDefinition;
  onError?: (componentId: string, error: Error) => void;
}

export function DynamicRenderer({ dashboard, onError }: DynamicRendererProps) {
  const renderComponent = (instance: ComponentInstance) => {
    const registered = ComponentRegistry[instance.type];

    if (!registered) {
      return (
        <ErrorCard
          title="Unknown Component"
          message={`Component type "${instance.type}" is not registered`}
        />
      );
    }

    // Validate props against schema
    const parseResult = registered.propsSchema.safeParse(instance.props);
    if (!parseResult.success) {
      onError?.(instance.id, new Error(parseResult.error.message));
      return (
        <ErrorCard
          title="Invalid Props"
          message={`Props for ${instance.type} failed validation`}
          details={parseResult.error.format()}
        />
      );
    }

    // Check mode is allowed
    if (!registered.allowedModes.includes(instance.mode)) {
      return (
        <ErrorCard
          title="Mode Not Allowed"
          message={`${instance.type} does not support mode "${instance.mode}"`}
        />
      );
    }

    const { Component } = registered;
    return (
      <WasmErrorBoundary key={instance.id}>
        <Component {...parseResult.data} mode={instance.mode} />
      </WasmErrorBoundary>
    );
  };

  return (
    <LayoutRenderer layout={dashboard.layout}>
      {dashboard.components.map(renderComponent)}
    </LayoutRenderer>
  );
}
```

#### 3.10.3 Mutation Actions (Not Full Redefines)

Instead of LLMs sending entire `DashboardDefinition` objects, we provide granular mutation actions:

```typescript
// engine/dashboardActions.ts
import { z } from 'zod';

// Mutation action schemas for LLM function calling
export const DashboardActionSchema = z.discriminatedUnion('action', [
  // Add a new component
  z.object({
    action: z.literal('add_component'),
    component: ComponentInstanceSchema,
    position: z.object({
      layoutSlot: z.string(),  // e.g., "left", "tab:Risk"
      index: z.number().optional(),
    }),
  }),

  // Remove a component
  z.object({
    action: z.literal('remove_component'),
    componentId: z.string().uuid(),
  }),

  // Update component props (partial)
  z.object({
    action: z.literal('update_component'),
    componentId: z.string().uuid(),
    propsPatch: z.record(z.unknown()),  // Partial update
  }),

  // Change component mode
  z.object({
    action: z.literal('set_component_mode'),
    componentId: z.string().uuid(),
    mode: z.enum(['viewer', 'editor', 'llm-assisted']),
  }),

  // Change layout
  z.object({
    action: z.literal('set_layout'),
    layout: LayoutTemplateSchema,
  }),

  // Batch multiple actions
  z.object({
    action: z.literal('batch'),
    actions: z.array(z.lazy(() => DashboardActionSchema)),
  }),
]);

export type DashboardAction = z.infer<typeof DashboardActionSchema>;

// Reducer for applying actions
export function applyDashboardAction(
  state: DashboardDefinition,
  action: DashboardAction
): DashboardDefinition {
  switch (action.action) {
    case 'add_component':
      return {
        ...state,
        components: [...state.components, action.component],
        updatedAt: new Date().toISOString(),
      };
    case 'remove_component':
      return {
        ...state,
        components: state.components.filter(c => c.id !== action.componentId),
        updatedAt: new Date().toISOString(),
      };
    case 'update_component':
      return {
        ...state,
        components: state.components.map(c =>
          c.id === action.componentId
            ? { ...c, props: { ...c.props, ...action.propsPatch } }
            : c
        ),
        updatedAt: new Date().toISOString(),
      };
    case 'batch':
      return action.actions.reduce(applyDashboardAction, state);
    // ... other cases
  }
}
```

**Benefits of mutations over full redefines:**
* Smaller payloads
* Natural undo/redo (each action is a history entry)
* Easier to reason about diffs
* LLMs make fewer mistakes with simple operations

#### 3.10.4 LLM-Safe Modes

Components that can change scenarios, portfolios, or trades expose a `mode` prop:

```typescript
// types/modes.ts
export type ComponentMode = 'viewer' | 'editor' | 'llm-assisted';

// In llm-assisted mode:
// - Show extra review panels
// - Require user confirmation for mutations
// - Apply safe defaults (e.g., shock limits)

// Example: TradeEntryForm
interface TradeEntryFormProps {
  mode: ComponentMode;
  onSubmit: (trade: TradeSpec) => void;
}

function TradeEntryForm({ mode, onSubmit }: TradeEntryFormProps) {
  const handleSubmit = (trade: TradeSpec) => {
    if (mode === 'llm-assisted') {
      // Show confirmation dialog
      showConfirmation({
        title: 'LLM-Suggested Trade',
        message: 'Review this trade before submitting',
        trade,
        onConfirm: () => onSubmit(trade),
      });
    } else {
      onSubmit(trade);
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      {/* Form fields */}
      {mode === 'llm-assisted' && (
        <Alert variant="info">
          This trade was suggested by an AI assistant. Please review carefully.
        </Alert>
      )}
    </form>
  );
}
```

**Safe defaults for LLM-assisted mode:**

| Component | Safe Default |
|-----------|--------------|
| ScenarioBuilder | Shock limits: ±500 bps rates, ±50% equity |
| PortfolioOptimizer | Max trade size limits, require human approval |
| TradeEntryForm | Preview-only until user confirms |
| ForecastEditor | Show diff against current values |

#### 3.10.5 Anti-Hallucination Patterns

The GenUI concept requires careful design to prevent LLM hallucination and context overflow.

#### 3.10.1 Context Window Management

**Problem:** A full `FinancialState` with thousands of positions will blow up an LLM's context window (typically 8K-128K tokens).

**Solution: Semantic Summaries**

Create a `toLLMContext()` method in Rust that returns a schema-compliant **summary**, not raw data:

```rust
// finstack-wasm/src/llm_context.rs
#[wasm_bindgen]
pub struct LLMContext {
    portfolio_summary: PortfolioSummary,
    available_curves: Vec<String>,
    available_surfaces: Vec<String>,
    model_nodes: Vec<String>,
    risk_factors: Vec<String>,
}

#[wasm_bindgen]
impl MarketContext {
    pub fn to_llm_context(&self) -> LLMContext {
        LLMContext {
            available_curves: self.discount_curve_ids(),
            available_surfaces: self.vol_surface_ids(),
            // ... summarized data
        }
    }
}
```

**TypeScript Integration:**

```typescript
// engine/llmAdapter.ts
interface LLMContext {
  portfolio: {
    positionCount: number;
    totalPV: string;  // Formatted, not raw number
    currencies: string[];
    riskFactors: string[];
  };
  market: {
    availableCurves: string[];
    availableSurfaces: string[];
    asOfDate: string;
  };
  statements: {
    modelIds: string[];
    nodeCount: number;
    periodRange: string;
  };
}

// Generate context for LLM prompt
export async function generateLLMContext(): Promise<LLMContext> {
  return engine.toLLMContext();
}
```

**Instead of sending:**

```json
{ "positions": [ { "id": 1, "pv": 100 }, /* ...10,000 items */ ] }
```

**Send:**

```json
{
  "portfolio": {
    "positionCount": 10000,
    "totalPV": "$1.5M",
    "riskFactors": ["USD-Rates", "EUR-Rates", "SPX-Vol"]
  },
  "availableCurves": ["USD-SOFR", "EUR-ESTR"]
}
```

#### 3.10.2 Dynamic Schema Injection

**Problem:** An LLM might generate a valid Zod object requesting `curve_id: "USD-SOFR"`, but if your loaded market only has `USD-LIBOR`, the engine will panic.

**Solution:** Inject currently available identifiers into Zod schemas at runtime:

```typescript
// schemas/dynamic.ts
import { z } from 'zod';

export function createDynamicSchemas(context: LLMContext) {
  // Curve ID must be one of the actually loaded curves
  const CurveIdSchema = z.enum(
    context.market.availableCurves as [string, ...string[]]
  );

  // Surface ID must be one of the actually loaded surfaces
  const SurfaceIdSchema = z.enum(
    context.market.availableSurfaces as [string, ...string[]]
  );

  // Statement node must exist in the model
  const NodeIdSchema = z.enum(
    context.statements.nodeIds as [string, ...string[]]
  );

  // Compose into operation schemas
  const CurveShockSchema = z.object({
    type: z.literal('curve_shock'),
    curveId: CurveIdSchema,  // Constrained to available curves
    shockBps: z.number(),
  });

  return { CurveIdSchema, SurfaceIdSchema, NodeIdSchema, CurveShockSchema };
}

// Generate OpenAI function schema with current constraints
export function generateFunctionSchemas(context: LLMContext) {
  const schemas = createDynamicSchemas(context);
  return {
    render_risk_heatmap: zodToJsonSchema(schemas.RiskHeatmapSchema),
    apply_curve_shock: zodToJsonSchema(schemas.CurveShockSchema),
    // ...
  };
}
```

#### 3.10.3 Intent vs Content Separation

**Critical Rule:** The LLM generates **Configuration/Intent**, never **Values/Data**.

```typescript
// GOOD: LLM generates intent
interface LLMRenderCommand {
  type: 'RiskHeatmap';
  props: {
    metric: 'DV01' | 'CS01' | 'Gamma';  // What to show
    grouping: 'Sector' | 'Currency' | 'Rating';  // How to group
    filters?: { currency?: string };  // Optional filters
  };
}

// The React component fetches actual numbers from WASM
function RiskHeatmap({ metric, grouping, filters }: RiskHeatmapProps) {
  // Ask WASM for the data based on LLM's intent
  const { data } = useQuery(['risk-heatmap', metric, grouping, filters], () =>
    engine.computeRiskHeatmap(metric, grouping, filters)
  );

  return <HeatmapGrid data={data} />;
}

// BAD: Never let LLM generate the numbers
interface BadLLMResponse {
  type: 'RiskHeatmap';
  data: [  // LLM HALLUCINATED THESE NUMBERS
    { sector: 'Tech', dv01: 12345.67 },  // Fake!
    { sector: 'Finance', dv01: 98765.43 },  // Fake!
  ];
}
```

### 3.11 Virtualization & Overlay Patterns

#### 3.11.1 Corkscrew Tracing with Virtualization

**Problem:** `StatementViewer` uses TanStack Virtual for the grid and implements "Corkscrew tracing" (arrows connecting precedent cells). Virtualization physically removes DOM nodes that are off-screen. If Cell A (visible) depends on Cell B (scrolled off-screen), you cannot draw an SVG line between them because Cell B's DOM element *does not exist*.

**Solution: Canvas Overlay with Math-Based Coordinates**

```typescript
// components/statements/CorkscrewOverlay.tsx
import { useRef, useEffect } from 'react';

interface DependencyEdge {
  from: { nodeId: string; period: string };
  to: { nodeId: string; period: string };
}

interface CorkscrewOverlayProps {
  dependencies: DependencyEdge[];
  selectedCell: { nodeId: string; period: string } | null;
  virtualizer: Virtualizer<HTMLDivElement, Element>;
  rowHeight: number;
  columnWidth: number;
  nodeOrder: string[];  // For calculating row index
  periodOrder: string[];  // For calculating column index
}

export function CorkscrewOverlay({
  dependencies,
  selectedCell,
  virtualizer,
  rowHeight,
  columnWidth,
  nodeOrder,
  periodOrder,
}: CorkscrewOverlayProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = virtualizer.scrollElement;

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !selectedCell) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Clear previous drawings
    ctx.clearRect(0, 0, canvas.width, canvas.height);

    // Get visible range
    const scrollTop = containerRef?.scrollTop ?? 0;
    const scrollLeft = containerRef?.scrollLeft ?? 0;
    const viewportHeight = containerRef?.clientHeight ?? 0;
    const viewportWidth = containerRef?.clientWidth ?? 0;

    // Filter to edges involving selected cell
    const relevantEdges = dependencies.filter(
      edge =>
        (edge.from.nodeId === selectedCell.nodeId && edge.from.period === selectedCell.period) ||
        (edge.to.nodeId === selectedCell.nodeId && edge.to.period === selectedCell.period)
    );

    for (const edge of relevantEdges) {
      // Calculate VIRTUAL coordinates (not DOM-based)
      const fromRow = nodeOrder.indexOf(edge.from.nodeId);
      const fromCol = periodOrder.indexOf(edge.from.period);
      const toRow = nodeOrder.indexOf(edge.to.nodeId);
      const toCol = periodOrder.indexOf(edge.to.period);

      const fromX = (fromCol + 0.5) * columnWidth - scrollLeft;
      const fromY = (fromRow + 0.5) * rowHeight - scrollTop;
      const toX = (toCol + 0.5) * columnWidth - scrollLeft;
      const toY = (toRow + 0.5) * rowHeight - scrollTop;

      // Check if either endpoint is visible
      const fromVisible = fromX >= 0 && fromX <= viewportWidth && fromY >= 0 && fromY <= viewportHeight;
      const toVisible = toX >= 0 && toX <= viewportWidth && toY >= 0 && toY <= viewportHeight;

      if (!fromVisible && !toVisible) continue;

      // Draw arrow (clipped at viewport edges)
      ctx.beginPath();
      ctx.strokeStyle = 'var(--color-primary)';
      ctx.lineWidth = 2;

      // Clip coordinates to viewport
      const clippedFrom = clipToViewport(fromX, fromY, viewportWidth, viewportHeight);
      const clippedTo = clipToViewport(toX, toY, viewportWidth, viewportHeight);

      ctx.moveTo(clippedFrom.x, clippedFrom.y);
      ctx.lineTo(clippedTo.x, clippedTo.y);
      ctx.stroke();

      // Draw arrowhead at target
      if (toVisible) {
        drawArrowhead(ctx, clippedFrom.x, clippedFrom.y, clippedTo.x, clippedTo.y);
      }

      // Draw "off-screen indicator" if target is not visible
      if (!toVisible) {
        drawOffscreenIndicator(ctx, clippedTo.x, clippedTo.y, edge.to);
      }
    }
  }, [dependencies, selectedCell, virtualizer.scrollOffset]);

  return (
    <canvas
      ref={canvasRef}
      className="absolute inset-0 pointer-events-none"
      width={containerRef?.clientWidth ?? 0}
      height={containerRef?.clientHeight ?? 0}
    />
  );
}
```

**Key Insight:** The dependency graph comes from WASM/Rust, not from DOM inspection:

```rust
// finstack/statements/src/evaluator/dag.rs
impl StatementModel {
    pub fn get_dependencies(&self, node_id: &str, period: &str) -> Vec<DependencyEdge> {
        // Return the DAG edges from Rust
        self.dag.predecessors(node_id, period)
    }
}
```

### 3.12 Schema Generation Pipeline

#### 3.12.1 Auto-Generation with `ts-rs` or `specta`

**Problem:** Manually keeping `src/schemas/*.ts` (Zod) in sync with `finstack-wasm/src/*.rs` (Rust) is a maintenance nightmare. A drift here means the LLM generates invalid JSON that crashes the WASM engine.

**Solution:** Use `ts-rs` or `specta` crates to auto-generate TypeScript types from Rust:

```rust
// finstack-wasm/src/valuations/instruments/bond.rs
use ts_rs::TS;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../packages/finstack-ui/src/schemas/generated/")]
pub struct BondSpec {
    pub id: String,
    pub notional: MoneySpec,
    pub coupon_rate: String,  // Decimal as string
    pub issue_date: DateSpec,
    pub maturity_date: DateSpec,
    pub frequency: FrequencySpec,
    pub day_count: DayCountSpec,
    pub discount_curve_id: String,
}
```

**Build script generates TypeScript:**

```bash
# build.rs or Makefile
cargo test --features ts-export  # Generates .ts files

# Output: packages/finstack-ui/src/schemas/generated/BondSpec.ts
export interface BondSpec {
  id: string;
  notional: MoneySpec;
  coupon_rate: string;
  issue_date: DateSpec;
  maturity_date: DateSpec;
  frequency: FrequencySpec;
  day_count: DayCountSpec;
  discount_curve_id: string;
}
```

**Zod schema derived from generated types:**

```typescript
// schemas/instruments/bond.ts
import { z } from 'zod';
import type { BondSpec } from '../generated/BondSpec';
import { MoneySpecSchema, DateSpecSchema } from '../generated';

// Zod schema mirrors generated type
export const BondSpecSchema: z.ZodType<BondSpec> = z.object({
  id: z.string(),
  notional: MoneySpecSchema,
  coupon_rate: z.string(),  // Decimal as string
  issue_date: DateSpecSchema,
  maturity_date: DateSpecSchema,
  frequency: FrequencySpecSchema,
  day_count: DayCountSpecSchema,
  discount_curve_id: z.string(),
});
```

#### 3.12.2 Auto-Forms from Schemas

Build a **Generic Instrument Form** that takes a generated schema and renders inputs automatically. Only build custom panels for complex instruments:

```typescript
// components/forms/SchemaForm.tsx
import { z } from 'zod';
import { useForm } from 'react-hook-form';
import { zodResolver } from '@hookform/resolvers/zod';

interface SchemaFormProps<T extends z.ZodObject<any>> {
  schema: T;
  defaultValues?: Partial<z.infer<T>>;
  onSubmit: (data: z.infer<T>) => void;
  fieldOverrides?: Partial<Record<keyof z.infer<T>, React.ComponentType<any>>>;
}

export function SchemaForm<T extends z.ZodObject<any>>({
  schema,
  defaultValues,
  onSubmit,
  fieldOverrides = {},
}: SchemaFormProps<T>) {
  const form = useForm({
    resolver: zodResolver(schema),
    defaultValues,
  });

  // Introspect schema shape
  const shape = schema.shape;
  const fields = Object.entries(shape);

  return (
    <form onSubmit={form.handleSubmit(onSubmit)}>
      {fields.map(([key, fieldSchema]) => {
        // Check for custom override
        const Override = fieldOverrides[key as keyof z.infer<T>];
        if (Override) {
          return <Override key={key} form={form} name={key} />;
        }

        // Auto-generate based on Zod type
        return (
          <FormField key={key} form={form} name={key} schema={fieldSchema} />
        );
      })}
      <Button type="submit">Submit</Button>
    </form>
  );
}

// Auto-detect field type and render appropriate input
function FormField({ form, name, schema }: FormFieldProps) {
  const typeName = schema._def.typeName;

  switch (typeName) {
    case 'ZodString':
      if (name.includes('date')) {
        return <DatePicker {...form.register(name)} />;
      }
      if (name.includes('currency')) {
        return <CurrencySelect {...form.register(name)} />;
      }
      return <Input {...form.register(name)} />;

    case 'ZodNumber':
      if (name.includes('rate')) {
        return <RateInput {...form.register(name)} />;
      }
      return <Input type="number" {...form.register(name)} />;

    case 'ZodEnum':
      const options = schema._def.values;
      return <Select options={options} {...form.register(name)} />;

    default:
      return <Input {...form.register(name)} />;
  }
}
```

**Usage:**

```typescript
// Auto-generated form for simple instruments
<SchemaForm
  schema={DepositSpecSchema}
  onSubmit={handleCreateDeposit}
/>

// Custom panel only needed for complex instruments
<SwaptionPanel {...props} />  // Has vol surface picker, exercise schedule, etc.
```

### 3.13 Generic Instrument Form System

The `domains/valuations/instruments` directory contains dozens of panels. To avoid maintenance burden, we implement a **descriptor-based form system**.

#### 3.13.1 Instrument Descriptor DSL

```typescript
// schemas/instrumentDescriptor.ts
import { z } from 'zod';

// Field kinds map to specialized input components
const FieldKindSchema = z.enum([
  'money',      // AmountInput + CurrencySelect
  'rate',       // RateInput (bps/% toggle)
  'tenor',      // TenorInput (1M, 2Y, etc.)
  'date',       // DatePicker (business day aware)
  'enum',       // Select dropdown
  'boolean',    // Checkbox/Switch
  'string',     // Text input
  'curveId',    // CurveSelect (from available curves)
  'surfaceId',  // SurfaceSelect (from available surfaces)
]);

const InstrumentFieldSchema = z.object({
  name: z.string(),
  label: z.string(),
  kind: FieldKindSchema,
  required: z.boolean().default(true),
  // For enum fields
  enumValues: z.array(z.string()).optional(),
  // For conditional display
  showWhen: z.object({
    field: z.string(),
    equals: z.unknown(),
  }).optional(),
  // Help text for users and LLMs
  description: z.string().optional(),
  // Default value
  defaultValue: z.unknown().optional(),
});

export const InstrumentDescriptorSchema = z.object({
  type: z.string(),  // e.g., "Bond", "InterestRateSwap"
  category: z.enum(['rates', 'credit', 'fx', 'equity', 'inflation', 'exotic']),
  description: z.string(),
  fields: z.array(InstrumentFieldSchema),
  // Which standard sections to include
  sections: z.object({
    cashflows: z.boolean().default(true),
    metrics: z.boolean().default(true),
    marketData: z.boolean().default(true),
  }),
});

export type InstrumentDescriptor = z.infer<typeof InstrumentDescriptorSchema>;
```

#### 3.13.2 Example Descriptors (Generated from Rust)

```typescript
// schemas/instruments/descriptors/bond.ts
export const BondDescriptor: InstrumentDescriptor = {
  type: 'Bond',
  category: 'rates',
  description: 'Fixed or floating rate bond instrument',
  fields: [
    { name: 'id', label: 'Bond ID', kind: 'string', required: true },
    { name: 'notional', label: 'Notional', kind: 'money', required: true },
    { name: 'couponRate', label: 'Coupon Rate', kind: 'rate', required: true },
    { name: 'issueDate', label: 'Issue Date', kind: 'date', required: true },
    { name: 'maturityDate', label: 'Maturity Date', kind: 'date', required: true },
    {
      name: 'frequency',
      label: 'Payment Frequency',
      kind: 'enum',
      enumValues: ['Annual', 'SemiAnnual', 'Quarterly', 'Monthly'],
      defaultValue: 'SemiAnnual',
    },
    {
      name: 'dayCount',
      label: 'Day Count',
      kind: 'enum',
      enumValues: ['Act360', 'Act365F', 'Thirty360'],
      defaultValue: 'Thirty360',
    },
    { name: 'discountCurveId', label: 'Discount Curve', kind: 'curveId', required: true },
  ],
  sections: { cashflows: true, metrics: true, marketData: true },
};

// Exotic instruments have simpler descriptors - complex logic is in Rust
export const BarrierOptionDescriptor: InstrumentDescriptor = {
  type: 'BarrierOption',
  category: 'exotic',
  description: 'Option with knock-in/knock-out barrier',
  fields: [
    { name: 'id', label: 'Option ID', kind: 'string', required: true },
    { name: 'underlying', label: 'Underlying', kind: 'string', required: true },
    { name: 'notional', label: 'Notional', kind: 'money', required: true },
    { name: 'strike', label: 'Strike', kind: 'rate', required: true },
    { name: 'barrier', label: 'Barrier Level', kind: 'rate', required: true },
    {
      name: 'barrierType',
      label: 'Barrier Type',
      kind: 'enum',
      enumValues: ['UpAndIn', 'UpAndOut', 'DownAndIn', 'DownAndOut'],
    },
    { name: 'expiry', label: 'Expiry', kind: 'date', required: true },
    { name: 'volSurfaceId', label: 'Vol Surface', kind: 'surfaceId', required: true },
  ],
  sections: { cashflows: false, metrics: true, marketData: true },
};
```

#### 3.13.3 Generic Instrument Panel

```typescript
// components/domains/valuations/instruments/GenericInstrumentPanel.tsx
import { InstrumentDescriptor } from '../../../schemas/instrumentDescriptor';
import { SchemaForm } from '../../forms/SchemaForm';
import { CashflowWaterfall } from '../CashflowWaterfall';
import { InstrumentRiskTable } from '../metrics/InstrumentRiskTable';
import { useValuation } from '../../../hooks/useValuation';

interface GenericInstrumentPanelProps {
  descriptor: InstrumentDescriptor;
  initialValues?: Record<string, unknown>;
  mode: ComponentMode;
  onSubmit?: (instrument: unknown) => void;
}

export function GenericInstrumentPanel({
  descriptor,
  initialValues,
  mode,
  onSubmit,
}: GenericInstrumentPanelProps) {
  const [instrumentData, setInstrumentData] = useState(initialValues);

  // Convert descriptor to Zod schema dynamically
  const schema = useMemo(
    () => descriptorToZodSchema(descriptor),
    [descriptor]
  );

  // Hook into valuation engine
  const { pv, metrics, cashflows, isLoading, error } = useValuation({
    instrumentType: descriptor.type,
    instrumentData,
    enabled: !!instrumentData && mode !== 'editor',
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>{descriptor.type}</CardTitle>
        <CardDescription>{descriptor.description}</CardDescription>
      </CardHeader>

      <CardContent className="space-y-6">
        {/* Input Form */}
        <section>
          <h3 className="text-sm font-medium mb-2">Instrument Details</h3>
          <SchemaForm
            schema={schema}
            defaultValues={initialValues}
            onSubmit={(data) => {
              setInstrumentData(data);
              onSubmit?.(data);
            }}
            fieldOverrides={getFieldOverrides(descriptor)}
            disabled={mode === 'viewer'}
          />
        </section>

        {/* Cashflows Section */}
        {descriptor.sections.cashflows && cashflows.length > 0 && (
          <section>
            <h3 className="text-sm font-medium mb-2">Projected Cashflows</h3>
            <CashflowWaterfall cashflows={cashflows} />
          </section>
        )}

        {/* Metrics Section */}
        {descriptor.sections.metrics && metrics && (
          <section>
            <h3 className="text-sm font-medium mb-2">Risk Metrics</h3>
            <InstrumentRiskTable
              pv={pv}
              metrics={metrics}
              isLoading={isLoading}
            />
          </section>
        )}

        {/* Market Data Section */}
        {descriptor.sections.marketData && (
          <section>
            <h3 className="text-sm font-medium mb-2">Market Data</h3>
            <MarketDataSection instrumentData={instrumentData} />
          </section>
        )}

        {error && (
          <Alert variant="destructive">
            <AlertTitle>Valuation Error</AlertTitle>
            <AlertDescription>{error.message}</AlertDescription>
          </Alert>
        )}
      </CardContent>
    </Card>
  );
}

// Helper: Convert descriptor to Zod schema
function descriptorToZodSchema(descriptor: InstrumentDescriptor): z.ZodObject<any> {
  const shape: Record<string, z.ZodTypeAny> = {};

  for (const field of descriptor.fields) {
    let fieldSchema: z.ZodTypeAny;

    switch (field.kind) {
      case 'money':
        fieldSchema = MoneySpecSchema;
        break;
      case 'rate':
        fieldSchema = z.string(); // Decimal as string
        break;
      case 'date':
        fieldSchema = DateSpecSchema;
        break;
      case 'enum':
        fieldSchema = z.enum(field.enumValues as [string, ...string[]]);
        break;
      case 'curveId':
      case 'surfaceId':
      case 'string':
        fieldSchema = z.string();
        break;
      case 'boolean':
        fieldSchema = z.boolean();
        break;
      default:
        fieldSchema = z.unknown();
    }

    shape[field.name] = field.required ? fieldSchema : fieldSchema.optional();
  }

  return z.object(shape);
}
```

#### 3.13.4 When to Use Generic vs Custom Panels

| Instrument Complexity | Approach |
|----------------------|----------|
| Simple (Bond, Deposit, FRA) | Generic panel from descriptor |
| Medium (IRS, CDS, Cap/Floor) | Generic panel + custom sections |
| Complex (Swaption, Convertible, Autocallable) | Custom panel with specialized UI |

**Custom panels are needed when:**
* Complex interdependent fields (e.g., swaption exercise schedule)
* Specialized visualizations (e.g., autocallable payoff diagram)
* Multi-step workflows (e.g., structured credit waterfall builder)

The generic system handles the "long tail" of instruments (Barrier, Cliquet, Quanto, Asian, Lookback, etc.) without hand-coding 30+ forms.

### 3.14 Worker Pool & Configurable Thresholds

#### 3.14.1 Centralized Worker Management

Instead of instantiating workers per component, use a shared pool:

```typescript
// workers/workerPool.ts
import { wrap, Remote } from 'comlink';
import type { FinstackEngineAPI } from './finstackEngine';

class WorkerPool {
  private worker: Worker | null = null;
  private api: Remote<FinstackEngineAPI> | null = null;
  private initPromise: Promise<void> | null = null;

  async getEngine(): Promise<Remote<FinstackEngineAPI>> {
    if (!this.initPromise) {
      this.initPromise = this.initialize();
    }
    await this.initPromise;
    return this.api!;
  }

  private async initialize(): Promise<void> {
    this.worker = new Worker(
      new URL('./finstackEngine.ts', import.meta.url),
      { type: 'module' }
    );
    this.api = wrap<FinstackEngineAPI>(this.worker);
    await this.api.initialize();
  }

  terminate(): void {
    this.worker?.terminate();
    this.worker = null;
    this.api = null;
    this.initPromise = null;
  }
}

// Singleton pool
export const workerPool = new WorkerPool();

// Hook for components
export function useFinstackEngine() {
  const [engine, setEngine] = useState<Remote<FinstackEngineAPI> | null>(null);

  useEffect(() => {
    workerPool.getEngine().then(setEngine);
  }, []);

  return engine;
}
```

#### 3.14.2 Configurable Computation Thresholds

Make thresholds config-driven, not hardcoded:

```typescript
// config/computeThresholds.ts
export interface ComputeThresholds {
  // Portfolio
  portfolioPositionsForWorker: number;

  // Monte Carlo
  monteCarloPathsForWorker: number;

  // Statements
  statementNodesForWorker: number;
  statementPeriodsForWorker: number;

  // Tables
  virtualScrollRowThreshold: number;

  // Risk
  riskHeatmapCellsForCanvas: number;
}

// Default thresholds (can be overridden by environment)
export const defaultThresholds: ComputeThresholds = {
  portfolioPositionsForWorker: 10,
  monteCarloPathsForWorker: 100,
  statementNodesForWorker: 50,
  statementPeriodsForWorker: 20,
  virtualScrollRowThreshold: 100,
  riskHeatmapCellsForCanvas: 2500, // 50x50
};

// Load from environment/config
export function loadThresholds(): ComputeThresholds {
  return {
    ...defaultThresholds,
    // Override from env vars or runtime config
    ...(typeof window !== 'undefined' && window.__FINSTACK_THRESHOLDS__),
  };
}

// Usage in hooks
export function useComputeStrategy(computationType: string, size: number) {
  const thresholds = useMemo(loadThresholds, []);

  const useWorker = useMemo(() => {
    switch (computationType) {
      case 'portfolio':
        return size > thresholds.portfolioPositionsForWorker;
      case 'monteCarlo':
        return size > thresholds.monteCarloPathsForWorker;
      // ...
    }
  }, [computationType, size, thresholds]);

  return { useWorker };
}
```

### 3.15 Bundle Optimization & Lazy Loading

#### 3.15.1 Lazy Loading Heavy Dependencies

ECharts and 3D charting are expensive. Lazy-load them:

```typescript
// components/charts/lazyCharts.ts
import { lazy } from 'react';

// Only load ECharts when needed
export const VolSurface3D = lazy(() =>
  import('./VolSurface3D').then(m => ({ default: m.VolSurface3D }))
);

export const HeatmapGrid = lazy(() =>
  import('./HeatmapGrid').then(m => ({ default: m.HeatmapGrid }))
);

// Usage with Suspense
function ChartContainer({ chartType, ...props }) {
  return (
    <Suspense fallback={<ChartSkeleton />}>
      {chartType === '3d' && <VolSurface3D {...props} />}
      {chartType === 'heatmap' && <HeatmapGrid {...props} />}
    </Suspense>
  );
}
```

#### 3.15.2 Optional Pro Package

For hitting `< 500KB gzipped`, consider splitting advanced features:

```text
packages/
├── finstack-ui/           # Core package (~300KB)
│   └── src/
│       ├── components/
│       ├── domains/
│       │   ├── market/
│       │   ├── portfolio/
│       │   ├── statements/
│       │   └── valuations/
│       │       └── instruments/  # Basic instruments only
│       └── ...
│
└── finstack-ui-pro/       # Pro package (~200KB, optional)
    └── src/
        ├── charts/
        │   ├── VolSurface3D.tsx     # WebGL 3D surfaces
        │   └── AdvancedHeatmap.tsx  # Large-scale heatmaps
        ├── instruments/
        │   └── exotics/             # Complex exotic panels
        └── analysis/
            └── MonteCarloViewer.tsx # Heavy MC visualization
```

---

## 4. Implementation Roadmap

> **Key Principles:**
>
> 1. **GenUI first** - The LLM integration dictates data structures across all domains
> 2. **Vertical slices** - Ship narrow but complete features before expanding breadth
> 3. **Prove patterns early** - Validate Handle Pattern, String Transport, and Schema Generation before building 30+ panels

### Phase 1: Core Infrastructure (Weeks 1-2)

* Initialize `packages/finstack-ui` with Vite/React/TS.
* Setup Tailwind + Shadcn.
* **Implement Unified Finstack Engine Worker** with Handle-based architecture.
* Implement Rust panic hooks for graceful error recovery.
* Create `FinstackProvider` context with WASM initialization (singleton pattern).
* Implement core hooks: `useFinstack`, `useFinstackEngine`.
* Build primitives with **String Transport** (no JS math):
  * `AmountDisplay` (string-based, no float conversion)
  * `AmountInput`, `CurrencySelect`, `TenorInput`, `DatePicker`
* Configure Vitest + React Testing Library.
* Setup worker pool singleton.

### Phase 2: Schema Pipeline & GenUI Foundation (Weeks 3-4)

* **Setup `ts-rs` or `specta`** in Rust crates for auto-generated TypeScript types.
* Build Zod schema derivation from generated types with **schema versioning**.
* Define `DashboardDefinitionSchema v1` with layout templates.
* Implement `ComponentRegistry` with typed components.
* Build `DynamicRenderer` that reads hard-coded JSON dashboards.
* Create mutation action reducers (add/remove/update components).
* Build `toLLMContext()` semantic summary generation in Rust.
* Implement `schemaGenerator.ts` for OpenAI function schemas.
* **Test:** Validate end-to-end "JSON → UI" rendering with 3 sample dashboards.

### Phase 3: Vertical Slice #1 - Basic Rates (Weeks 5-7)

**Goal:** Prove all patterns with a narrow but complete feature set.

* **Instruments (2 only):**
  * `Bond` - via `GenericInstrumentPanel` + descriptor
  * `InterestRateSwap` - custom panel (complexity test)
* **Calibration (2 only):**
  * `DiscountCurveCalibration`
  * `ForwardCurveCalibration`
* **Charts:**
  * `CurveChart` (Recharts)
  * `VirtualDataTable` (TanStack)
  * `CashflowWaterfall` (virtualized)
* **GenUI integration:**
  * Register `CurveChart`, `BondPanel`, `SwapPanel` in `ComponentRegistry`
  * Test LLM dashboard creation with these components
* **Validation:**
  * Schema parity tests (Rust ↔ TS ↔ Zod)
  * Numeric parity golden tests
  * LLM dashboard snapshot tests

### Phase 4: Vertical Slice #2 - Portfolio (Weeks 8-10)

* **Portfolio core:**
  * `PositionGrid` with virtual scrolling
  * `EntityTreeView` for entity hierarchy
  * `PortfolioSummaryPanel` with aggregated metrics
* **Risk:**
  * `InstrumentRiskTable` (DV01/CS01/Greeks)
  * Basic `RiskHeatmap` (table-based, not canvas yet)
* **GenUI integration:**
  * Register portfolio components
  * Mutation actions for position selection/filtering
  * Data binding DSL for portfolio paths
* **LLM safety:**
  * Implement `mode` prop on portfolio editors
  * Test `llm-assisted` mode with confirmation dialogs

### Phase 5: Vertical Slice #3 - Statements (Weeks 11-13)

* **Statements core:**
  * `StatementViewer` (Matrix rendering)
  * `ForecastEditor` with method selection
  * `FormulaBar` with autocomplete
* **Corkscrew tracing:**
  * Build **Canvas Overlay** (virtualization-compatible)
  * Dependency graph from Rust DAG
* **Analysis (2 only):**
  * `GoalSeekPanel`
  * `SensitivityAnalyzer` + `TornadoChart`
* **GenUI integration:**
  * Register statement components
  * Data binding DSL for statement paths (node.period)

### Phase 6: Editors & Draft Mode (Weeks 14-15)

* Implement Draft Mode state management with `useDraftStore`.
* Build `EditableGrid` for quote entry with validation.
* Build `TradeEntryForm` for position entry.
* Implement `ConstraintEditor` for optimization.
* Add undo/redo middleware for editor state.
* Implement `useDeferredValue` for responsive editing.

### Phase 7: Breadth Expansion - All Instruments (Weeks 16-18)

* **Generic Instrument Form system:**
  * Generate `InstrumentDescriptor` for all instruments from Rust metadata
  * Build remaining instrument panels via `GenericInstrumentPanel`
* **Calibration breadth:**
  * `HazardCurveCalibration`, `InflationCurveCalibration`
  * `VolSurfaceCalibration` (2D heatmap)
  * Lazy-load `VolSurface3D` (WebGL)
* **Custom panels (complex instruments only):**
  * `SwaptionPanel`, `ConvertibleBondPanel`, `AutocallablePanel`

### Phase 8: Breadth Expansion - Analysis & Scenarios (Weeks 19-21)

* **Statements analysis:**
  * `MonteCarloConfigEditor` + `MonteCarloResultsView`
  * `VarianceBridgeView` + `VarianceBridgeChart`
  * `DependencyTreeViewer`, `FormulaExplainPanel`
  * `CapitalStructurePanel`
* **Portfolio analysis:**
  * `PortfolioAttributionView` with factor breakdown
  * `PortfolioOptimizerView`, `EfficientFrontierChart`, `TradeProposalGrid`
  * Canvas-based `RiskHeatmap` for large grids
* **Scenarios domain:**
  * `ScenarioBuilder` with operation editors
  * `ScenarioLibrary` with tagging
  * `ScenarioExecutionPanel` with phase view
  * `HorizonScenarioGrid`

### Phase 9: Testing & Documentation (Weeks 22-23)

* **Unit tests:** >80% coverage
* **Schema parity tests:** For each Rust type, assert JSON validates against Zod and vice versa
* **LLM dashboard snapshots:** Library of canonical LLM outputs under test
* **Performance budgets:** CI checks for rendering time and bundle size
* **Integration tests:** Playwright for critical flows
* **Storybook:** All components with examples
* **API documentation**

### Phase 10: Accessibility & Final Polish (Weeks 24-25)

* Accessibility audit with axe-core.
* Keyboard navigation testing.
* Screen reader testing (NVDA, VoiceOver).
* High contrast theme validation.
* Performance profiling and optimization.
* Bundle size optimization (target: <500KB core, lazy-load pro features).
* Memory leak testing for long-running sessions.

---

## 5. Success Criteria

### Functional

1. **Numeric Parity:** All displayed values match Rust engine output exactly (string transport, no JS float math).
2. **Schema Sync:** 100% of Rust types have auto-generated TypeScript counterparts via `ts-rs`/`specta`.
3. **Error Recovery:** WASM panics are caught and surfaced via Error Boundaries without crashing the tab.
4. **Schema Versioning:** All LLM-facing schemas include `schemaVersion` field with migration support.

### Performance

5. **60fps Rendering:** Smooth scrolling on 10,000-row virtualized tables.
6. **Handle Pattern:** Market context serialization occurs ≤1 time per session (delta updates thereafter).
7. **Interactive Editing:** Curve drag operations complete in <16ms (single frame budget).
8. **Lazy Loading:** ECharts/3D components load on-demand, not in initial bundle.

### LLM Integration

9. **Zero Hallucination:** LLM never generates numeric values; all data comes from WASM engine.
10. **Context Efficiency:** LLM context payloads < 4KB (semantic summaries, not raw data).
11. **Schema Validation:** LLM-generated JSON validates against dynamic Zod schemas with < 5% rejection rate.
12. **Mutation Actions:** LLMs use granular actions (add/remove/update) not full dashboard redefines.
13. **Safe Modes:** All sensitive components support `viewer | editor | llm-assisted` modes.

### Quality

14. **Accessibility:** WCAG 2.1 AA compliance.
15. **Bundle Size:** < 300KB core gzipped, < 500KB with pro features (excluding WASM).
16. **Test Coverage:** > 80% line coverage, all golden tests passing.
17. **Schema Parity Tests:** Every Rust ↔ TS ↔ Zod type validated in CI.
18. **LLM Dashboard Snapshots:** Canonical dashboard outputs maintained as regression tests.

---

## 6. Architecture Decision Records (ADRs)

### ADR-001: Unified Worker over Domain-Specific Workers

**Decision:** Use a single `finstackEngine` worker instead of separate `valuationWorker`, `statementWorker`, etc.

**Rationale:**
* Market Context (5-10MB) would be duplicated across workers
* Shared state enables cross-domain operations (e.g., scenarios affecting both market and statements)
* Single initialization point simplifies error handling

**Trade-offs:**
* Single point of failure (mitigated by panic hooks)
* Cannot parallelize across domains (acceptable given computation profiles)

### ADR-002: String Transport for Monetary Values

**Decision:** All monetary values cross the WASM bridge as strings, never as JavaScript numbers.

**Rationale:**
* JavaScript floats lose precision (0.1 + 0.2 ≠ 0.3)
* Rust uses `rust_decimal` with arbitrary precision
* Golden test parity requires exact decimal representation

**Trade-offs:**
* Slightly higher serialization overhead
* Cannot use JS number formatting directly (mitigated by `AmountDisplay` component)

### ADR-003: Canvas Overlay for Dependency Visualization

**Decision:** Use a transparent `<canvas>` overlay for corkscrew arrows instead of SVG lines between DOM elements.

**Rationale:**
* TanStack Virtual removes off-screen DOM nodes
* Cannot draw lines between non-existent elements
* Canvas allows math-based coordinate calculation independent of DOM

**Trade-offs:**
* Requires manual coordinate math
* Canvas redraws on every scroll (mitigated by requestAnimationFrame)

### ADR-004: Schema Generation Pipeline

**Decision:** Auto-generate TypeScript types from Rust using `ts-rs` or `specta`, then derive Zod schemas.

**Rationale:**
* Manual sync between Rust and TypeScript is error-prone
* LLM integration requires accurate schemas
* Single source of truth (Rust) reduces drift

**Trade-offs:**
* Build-time dependency on schema generation
* Generated code must not be manually edited

### ADR-005: Schema Versioning for LLM Persistence

**Decision:** All LLM-facing schemas include a `schemaVersion` field with migration functions between versions.

**Rationale:**
* LLM-generated dashboards may be persisted and reloaded months later
* Schema evolution is inevitable
* Without versioning, old JSON becomes invalid after updates

**Trade-offs:**
* Migration code must be maintained
* Complexity increases with each version

### ADR-006: Engine/UI State Separation

**Decision:** Hard-separate `EngineState` (protocol-like, versioned) from `UIState` (transient, unversioned) in the Zustand store.

**Rationale:**
* Serializing everything for LLMs or history is heavy
* Only `EngineState + DashboardDefinition` typically needed for snapshots
* Easier to version protocol-like engine state than transient UI bits

**Trade-offs:**
* Two state trees to manage
* Must ensure UI state doesn't accidentally depend on stale engine state

### ADR-007: Layout Templates over Component Soup

**Decision:** LLMs choose from predefined layout templates (TwoColumn, Grid, TabSet, Report) rather than positioning arbitrary component arrays.

**Rationale:**
* Unconstrained LLM output produces noisy, inconsistent layouts
* Templates ensure professional-looking dashboards
* Smaller schema surface area for LLMs

**Trade-offs:**
* Less flexibility for advanced users
* May need to add new templates over time

### ADR-008: Mutation Actions for LLM Interactions

**Decision:** LLMs send granular mutation actions (add_component, update_component, etc.) rather than full dashboard definitions.

**Rationale:**
* Smaller payloads
* Natural undo/redo (each action = history entry)
* Easier to reason about diffs
* LLMs make fewer mistakes with simple operations

**Trade-offs:**
* More action types to implement and document
* Must handle action validation and rollback

### ADR-009: Generic Instrument Form System

**Decision:** Use descriptor-based `GenericInstrumentPanel` for most instruments; hand-craft custom panels only for complex cases.

**Rationale:**
* 30+ instrument panels is a maintenance nightmare
* Most instruments have similar structure: inputs → valuation → cashflows → metrics
* Complex instruments (Swaption, Convertible) genuinely need custom UI

**Trade-offs:**
* Descriptors must stay in sync with Rust types
* Generic panels may feel less polished than custom ones

### ADR-010: Singleton Worker Pool

**Decision:** Use a single shared worker pool rather than spawning workers per component.

**Rationale:**
* Multiple components mounting workers = resource exhaustion
* Worker spawn/teardown overhead for short tasks
* Shared market context avoids memory duplication

**Trade-offs:**
* No parallelism across domains (acceptable given computation profiles)
* Pool management complexity

### ADR-011: LLM-Safe Component Modes

**Decision:** Components that modify state expose a `mode` prop: `viewer | editor | llm-assisted`. In `llm-assisted` mode, mutations require user confirmation.

**Rationale:**
* LLMs can suggest trades, scenarios, optimizations - powerful and dangerous
* Guard rails at component level, not just API level
* Clear UX distinction between human actions and AI suggestions

**Trade-offs:**
* Extra UI for confirmation dialogs
* Mode prop propagation through component trees
