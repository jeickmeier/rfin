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

### Dates and Market Data (`DatesAndMarketData.tsx`)

This example demonstrates:

1. **Period Plan Example**
   - Building fiscal quarter periods using the period DSL
   - Working with `Date` objects and `Period` objects
   - Proper memory management with `.free()` calls

2. **Market Data Example**
   - Creating and using discount curves
   - Working with time series data (CPI example)
   - FX matrix configuration and rate lookups
   - Market context for storing and retrieving market data
   - Proper cleanup of WASM objects

## Key Patterns

### Initialization

Always initialize the WASM module before using any finstack-wasm types:

```typescript
import init from 'finstack-wasm';

await init();
// Now you can use finstack-wasm types
```

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
