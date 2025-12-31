# Finstack UI Kit: Performance, Virtualization & Overlays

## 3.6 Performance Architecture

Financial computations can block the main thread. The UI Kit employs several strategies to maintain responsiveness:

### 3.6.1 Web Worker Offload

- Heavy tasks (portfolio valuation, Monte Carlo, large statements) run in Web Workers.
- Comlink provides a type-safe RPC layer.
- The unified engine worker holds shared state (market, models, portfolios) to avoid repeated serialization.

### 3.6.2 Virtualization Strategy

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
}
```

Guidelines:

- Use virtualization for tables over ~100 rows.
- Keep row height stable for predictable scroll math.
- Combine with memoized row rendering to minimize React work.

### 3.6.3 WebGL Fallback Strategy

3D visualizations detect WebGL support and provide fallbacks:

```typescript
// components/charts/SurfaceViewer.tsx
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
```

If WebGL is unavailable:

- Fallback to 2D heatmap (ECharts).
- Maintain consistent props API across 2D/3D variants.

### 3.6.4 Computation Thresholds

Guidelines for when to use workers vs main thread:

| Computation | Threshold | Strategy |
|------------|-----------|----------|
| Single instrument pricing | Always | Main thread |
| Portfolio valuation | > 10 positions | Web Worker |
| Monte Carlo | > 100 paths | Web Worker |
| Statement evaluation | > 50 nodes × 20 periods | Web Worker |
| Cashflow table | > 100 rows | Virtual scrolling |
| Risk heatmap | > 50 × 50 cells | Canvas rendering |

These thresholds are implemented via `ComputeThresholds` and hooks like `useComputeStrategy`.

---

## 3.11 Virtualization & Overlay Patterns

### 3.11.1 Corkscrew Tracing with Virtualization

**Problem:** `StatementViewer` uses TanStack Virtual for the grid and implements "Corkscrew tracing" (arrows connecting precedent cells). Virtualization physically removes DOM nodes that are off-screen. If Cell A (visible) depends on Cell B (scrolled off-screen), you cannot draw an SVG line between them because Cell B's DOM element *does not exist*.

**Solution: Canvas Overlay with Math-Based Coordinates**

```typescript
// components/statements/CorkscrewOverlay.tsx
import { useRef, useEffect } from 'react';

interface DependencyEdge {
  from: { nodeId: string; period: string };
  to: { nodeId: string; period: string };
}
```

Key ideas:

- Compute cell coordinates from **row/column indices**, not DOM elements.
- Use scroll offsets and viewport size to clip drawing.
- Draw arrows and off-screen indicators on a transparent `<canvas>` overlay.
- Retrieve dependency graph from Rust (DAG) rather than DOM inspection.

---

## 3.15 Bundle Optimization & Lazy Loading

### 3.15.1 Lazy Loading Heavy Dependencies

ECharts and 3D charting are expensive. Lazy-load them:

```typescript
// components/charts/lazyCharts.ts
import { lazy } from 'react';

// Only load ECharts when needed
export const VolSurface3D = lazy(() =>
  import('./VolSurface3D').then(m => ({ default: m.VolSurface3D }))
);
```

Use `React.Suspense` with lightweight skeletons to keep UX responsive.

### 3.15.2 Optional Pro Package

To hit `< 500KB gzipped` for the core, split advanced features:

- `finstack-ui` core: common components, basic instruments, standard charts.
- `finstack-ui-pro`: 3D surfaces, advanced analysis views, exotic instruments.

This allows:

- Smaller default bundle for most users.
- Opt-in heavy features for power users.
