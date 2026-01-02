# Finstack UI Kit: GenUI, Patterns & Schema Pipeline

## 3.6 Performance Architecture (Overview)

Financial computations can block the main thread. The UI Kit employs several strategies to maintain responsiveness (see also **UI_KIT_PERFORMANCE.md** for details):

- Heavy computations offloaded to Web Workers (Comlink).
- Virtualized rendering for large tables.
- WebGL with graceful fallbacks.
- Configurable thresholds for when to use workers vs main thread.

---

## 3.9 Critical Architecture Patterns

### 3.9.1 The Serialization Bottleneck (Handle Pattern)

**Problem:** JSON serialization (`toJSON()` / `fromJSON()`) to bridge the Rust/JS boundary is expensive. A `MarketContext` with yield curves, vol surfaces, and fixings can easily exceed 5–10MB. If a user drags a curve point in `CurveEditor` and the system serializes the whole market for every pixel of drag, frames will drop. Serialization cost often exceeds actual calculation cost.

**Solution: The Handle Pattern**

Do not move heavy data between the Main Thread and Worker unless necessary. The UI should hold **Handles** (opaque references) to objects that live in WASM linear memory.

```typescript
// BAD: Full serialization on every call
async price(instrumentJson: string, marketJson: string): Promise<Result>

// GOOD: Handle-based - market lives in worker memory
async price(instrumentJson: string, marketHandleId: string): Promise<Result>
```

Implementation sketch:

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

---

### 3.9.2 Numeric Precision: The BigInt Problem (String Transport)

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
```

---

### 3.9.3 Unified Worker Strategy

**Problem:** Separate workers (`valuationWorker`, `statementWorker`, `portfolioWorker`) force loading the Market Context into memory multiple times. In a browser environment, this is wasteful and can cause OOM on large market data sets.

**Solution:** Single Finstack Engine Worker holding the shared Market Context and handling requests from all domains.

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
```

---

## 3.10 GenUI Architecture & LLM Safety

### 3.10.1 Component Registry (Typed)

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
```

The registry entries drive:

- Runtime props validation.
- LLM documentation generation.
- Safe mode enforcement (`viewer`, `editor`, `llm-assisted`).

### 3.10.2 Dynamic Renderer with Validation

```typescript
// engine/DynamicRenderer.tsx
import { ComponentRegistry } from './ComponentRegistry';
import { DashboardDefinition } from '../schemas/dashboard';

interface DynamicRendererProps {
  dashboard: DashboardDefinition;
  onError?: (componentId: string, error: Error) => void;
}
```

The renderer:

- Looks up components in `ComponentRegistry`.
- Validates props via Zod.
- Enforces allowed modes.
- Wraps components with a WASM-aware error boundary.

### 3.10.3 Mutation Actions (Not Full Redefines)

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
  // ... other actions
]);
```

Benefits:

- Smaller payloads.
- Natural undo/redo (each action is a history entry).
- Easier to reason about diffs.

### 3.10.4 LLM-Safe Modes

Components that can change scenarios, portfolios, or trades expose a `mode` prop:

```typescript
// types/modes.ts
export type ComponentMode = 'viewer' | 'editor' | 'llm-assisted';
```

In `llm-assisted` mode:

- Mutations require user confirmation.
- Safe defaults (e.g., shock limits) are applied.
- Extra review UI is shown (e.g., "This trade was suggested by an AI assistant").

### 3.10.5 Anti-Hallucination Patterns

**Intent vs Content Separation**

- LLMs generate configuration/intent (what to show, how to group, which metrics).
- The React components fetch **actual numbers** from WASM.
- LLMs never provide numeric PVs, Greeks, or cashflows.

---

## 3.12 Schema Generation Pipeline

### 3.12.1 Auto-Generation with `ts-rs` or `specta`

**Problem:** Manually keeping `src/schemas/*.ts` (Zod) in sync with `finstack-wasm/src/*.rs` (Rust) is a maintenance nightmare.

**Solution:** Use `ts-rs` or `specta` crates to auto-generate TypeScript types from Rust:

```rust
// finstack-wasm/src/valuations/instruments/bond.rs
use ts_rs::TS;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../finstack-ui/src/schemas/generated/")]
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

The build process generates TypeScript interfaces, which are then wrapped with Zod schemas for runtime validation.

---

## 3.13 Generic Instrument Form System

The `domains/valuations/instruments` directory contains dozens of panels. To avoid maintenance burden, we implement a **descriptor-based form system**.

### 3.13.1 Instrument Descriptor DSL

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
```

Descriptors drive:

- Auto-generated instrument entry forms.
- Generic panels for the long tail of instruments.
- Consistent sectioning (inputs, cashflows, metrics, market data).
