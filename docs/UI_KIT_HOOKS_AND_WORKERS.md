# Finstack UI Kit: Hooks, WASM Integration & Workers

## 3.4 Hooks Layer Specification

The hooks layer provides a clean abstraction between React components and WASM bindings, handling async initialization, error boundaries, and memoization.

### 3.4.1 Singleton WASM Initialization

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

### 3.4.2 Core Context Provider

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
```

The provider:

- Guards against SSR environments (`canInitWasm`).
- Uses a shared `ensureWasmInit` promise.
- Exposes readiness, loading, error, market, and rounding context.

### 3.4.3 Valuation Hook

Key considerations:

- `options.instrument` object identity may change often → use stable ID for deps.
- Avoid recompute storms with proper memoization.
- Implement automatic caching for identical inputs.

```typescript
// hooks/useValuation.ts
import { useMemo, useCallback, useState, useEffect } from 'react';
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
```

The hook:

- Memoizes `instrumentJson` via `JSON.stringify`.
- Checks a small LRU cache before calling into WASM.
- Delegates heavy work to the unified engine worker when appropriate.

### 3.4.4 Statement Evaluation Hook

```typescript
// hooks/useStatement.ts
import { useMemo, useCallback, useState, useEffect } from 'react';
import { useFinstack } from './useFinstack';

interface UseStatementOptions {
  model: StatementModel;
  scenarios?: ScenarioSpec[];
}
```

Responsibilities:

- Evaluate a statement model via WASM or JS helper.
- Expose helpers like `getValue(nodeId, period)` and `getNodeSeries(nodeId)`.
- Manage loading/error state and `refetch`.

### 3.4.5 Web Worker Hook

```typescript
// hooks/useWasmWorker.ts
import { useEffect, useRef, useCallback } from 'react';
import { wrap, Remote } from 'comlink';

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

---

## 3.6 Web Worker Strategy

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
};

expose(api);
export type ValuationWorkerAPI = typeof api;
```

---

## 3.14 Worker Pool & Configurable Thresholds (Summary)

A centralized worker pool and configuration-driven thresholds decide when to:

- Use the main thread vs workers (e.g., by portfolio size or Monte Carlo paths).
- Enable virtual scrolling vs plain tables.
- Switch to canvas-based risk heatmaps.

```typescript
// config/computeThresholds.ts
export interface ComputeThresholds {
  portfolioPositionsForWorker: number;
  monteCarloPathsForWorker: number;
  statementNodesForWorker: number;
  statementPeriodsForWorker: number;
  virtualScrollRowThreshold: number;
  riskHeatmapCellsForCanvas: number;
}
```

These thresholds are:

- Centralized.
- Overrideable via environment/runtime config.
- Used by hooks like `useComputeStrategy` to pick execution strategy.



