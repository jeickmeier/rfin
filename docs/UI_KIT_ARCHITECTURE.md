# Finstack UI Kit: Architecture & Core Stack

## 3. Technical Design Document (TDD)

### 3.1 Technology Stack

#### Frontend

- **Framework:** React 19 (leveraging Hooks, Suspense, and `useDeferredValue`).
- **Language:** TypeScript (Strict Mode).
- **Build System:** Vite (Library Mode).
- **Styling:** Tailwind CSS + `clsx` + `tailwind-merge` + Shadcn/UI (Base primitives).
- **State Management:** **Zustand** (Chosen for its ability to work outside components and ease of JSON hydration/snapshotting).
- **Data Grids:** **TanStack Table** + **TanStack Virtual** (Headless, high-performance, critical for financial statements).
- **Charts (Complex):** **Apache ECharts** (WebGL support for Vol Surfaces and dense scatters).
- **Charts (Simple):** **Recharts** (SVG-based for standard time-series).
- **Forms:** **React Hook Form** + **Zod Resolver** (Schema-driven validation).

#### WASM Integration

- **Worker Communication:** **Comlink** (Type-safe Web Worker RPC).
- **Schema Validation:** **Zod** (Runtime validation + OpenAI function schema generation).
- **Schema Generation:** **`zod-to-json-schema`** (Generate OpenAI function schemas from Zod).
- **WASM Bridge:** `finstack-wasm` (Direct dependency).

#### Rust-Side (Schema Generation)

- **TypeScript Generation:** **`ts-rs`** or **`specta`** (Auto-generate TypeScript types from Rust structs).
- **Panic Handling:** Custom panic hook for graceful JS exception conversion.

---

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

- Engine types match Rust's serde representation exactly
- UI types are optimized for LLM consumption and validation

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

- **Serializing everything** for LLMs or history is heavy. Typically you want snapshots of `EngineState + DashboardDefinition`, not every transient UI bit.
- It's easier to version protocol-like `EngineState` than "was the 3rd accordion panel open".

**Critical:** The Zustand store must be **100% JSON-serializable** (no class instances, Maps, etc.). For WASM objects, store the **wire form** (`toJSON()` output) and re-hydrate at the boundary.

---

### 3.3 Data Binding DSL

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

---

### 3.4 Dashboard Definition

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

---

### 3.5 Directory Structure

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
│   │   ├── market/
│   │   ├── portfolio/
│   │   ├── valuations/
│   │   ├── statements/
│   │   └── scenarios/
│   │
│   ├── hooks/                  # WASM Integration Layer
│   ├── workers/                # Web Worker (UNIFIED)
│   ├── store/                  # Global State (Zustand)
│   ├── engine/                 # GenUI Renderer
│   ├── schemas/                # Zod Definitions (The "LLM Interface")
│   └── utils/
```



