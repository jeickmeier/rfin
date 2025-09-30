# finstack-wasm Examples

This directory contains TypeScript/React examples demonstrating the usage of finstack-wasm in a browser environment using Vite.

## Prerequisites

Before running the examples, you need to build the WASM package:

```bash
# From the finstack-wasm directory
npm run build
# or
wasm-pack build --target web --out-dir pkg
```

**Note**: The examples use `vite-plugin-wasm` and `vite-plugin-top-level-await` to properly handle WASM modules in Vite. These are already included in the dev dependencies.

## Getting Started

1. Install dependencies:

```bash
cd examples
npm install
```

2. Run the development server:

```bash
npm run dev
```

This will start a local development server (usually at http://localhost:3000) with hot module replacement.

## Available Scripts

- `npm run dev` - Start the development server
- `npm run build` - Build for production
- `npm run preview` - Preview the production build locally
- `npm run check` - Type-check the TypeScript code without emitting files

## Examples Included

### Comprehensive Date Functionality (`DatesShowcase.tsx`)

This example suite demonstrates all date and calendar functionality with feature parity to the Python bindings:

1. **Date Construction & Properties**
   - Creating dates, accessing components
   - Weekend checks, quarter/fiscal year calculations
   - Adding weekdays

2. **Date Utilities**
   - Month arithmetic (`addMonths`)
   - Month-end handling (`lastDayOfMonth`, `daysInMonth`)
   - Leap year checks
   - Epoch conversions

3. **Calendars & Business Day Adjustments**
   - Available holiday calendars
   - Business day and holiday checks
   - Multiple adjustment conventions (Following, Modified Following, Preceding, etc.)

4. **Day Count Conventions**
   - Act/360, Act/365F, 30/360, Act/Act (ISDA), Act/Act (ISMA), BUS/252
   - Year fraction calculations with calendar and frequency context

5. **Schedule Builder**
   - Monthly, quarterly, semi-annual schedules
   - Stub rules (short/long front/back)
   - Business day adjustment with calendars
   - End-of-month handling
   - CDS IMM schedules

6. **Period Plans**
   - Calendar periods with actual/forecast segmentation
   - Fiscal periods (US Federal, UK, etc.)

7. **IMM Dates & Option Expiries**
   - Next IMM dates
   - CDS roll dates
   - Equity option expiries
   - Third Friday/Wednesday calculations

8. **Frequency Conventions**
   - Standard frequencies (annual, semi-annual, quarterly, monthly, etc.)
   - Custom frequencies by months or days

### Market Data (`DatesAndMarketData.tsx`)

This example demonstrates:

1. **Market Data Example**
   - Creating and using discount curves
   - Working with time series data (CPI example)
   - FX matrix configuration and rate lookups
   - Market context for storing and retrieving market data
   - Proper cleanup of WASM objects

### Cashflow Primitives (`CashflowBasics.tsx`)

Mirrors the Python `cashflow_basics.py` walkthrough:

1. **Cashflow Construction**
   - Fixed and floating coupons (with accrual factors and reset dates)
   - Up-front fees and principal exchanges
2. **Inspection Helpers**
   - Reading kind, dates, and formatted amounts
   - Converting instances to tuple views for serialization
3. **Schedule Utilities**
   - Sorting flows chronologically for schedule previews

### Math Utilities (`MathShowcase.tsx`)

Feature parity with `math_core_showcase.py`:

1. **Integration**
   - Gauss-Hermite expectations for standard normal moments
   - Fixed-order Gauss-Legendre quadrature
   - Adaptive Simpson integration on oscillatory integrands
2. **Probability Helpers**
   - Binomial probabilities and logarithmic combinatorics
3. **Root Finding**
   - Newton, Brent, and hybrid solvers applied to classic equations

## Key Patterns

### Initialization

**IMPORTANT**: Initialize the WASM module **once** at the application level, not in individual components:

```typescript
// ✅ CORRECT: Initialize once in App.tsx or main entry point
import React, { useEffect, useState } from 'react';
import init from 'finstack-wasm';

const App: React.FC = () => {
  const [wasmReady, setWasmReady] = useState(false);

  useEffect(() => {
    init().then(() => setWasmReady(true));
  }, []);

  if (!wasmReady) return <p>Loading WASM...</p>;
  return <YourComponents />;
};

// ❌ INCORRECT: Don't call init() in every component
// This causes memory corruption!
const Component = () => {
  useEffect(() => {
    await init(); // ❌ DON'T DO THIS
    // ...
  }, []);
};
```

Calling `init()` multiple times will reinitialize the WASM memory, causing "memory access out of bounds" errors during garbage collection.

### Memory Management

WASM objects need to be explicitly freed to avoid memory leaks:

```typescript
const date = new FsDate(2024, 1, 1);
// ... use date
date.free(); // Clean up when done
```

### Error Handling

Wrap WASM calls in try-catch blocks:

```typescript
try {
  const currency = new Currency("USD");
  // ... use currency
} catch (error) {
  console.error("Failed:", error);
}
```

## Adding New Examples

To add a new example:

1. Create a new file in `src/examples/`
2. Export your React components
3. Import and use them in `src/App.tsx`

## TypeScript Configuration

The project uses strict TypeScript settings. See `tsconfig.json` for details.

## Troubleshooting

### WASM module not found

Make sure you've built the WASM package first:

```bash
cd .. # back to finstack-wasm root
npm run build
```

### Type errors

The package should include TypeScript definitions. If you encounter type errors, ensure the WASM package was built successfully and the `pkg` directory exists.

### Memory issues

If you experience memory leaks or crashes, ensure all WASM objects are properly freed with `.free()` when no longer needed.
