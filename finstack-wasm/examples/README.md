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

This will start a local development server (usually at <http://localhost:3000>) with hot module replacement.

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

8. **Tenor Conventions**
   - Standard tenors (annual, semi-annual, quarterly, monthly, etc.)
   - Custom tenors by months, days, weeks, or years

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

### Cashflow Builder (`CashflowBuilderExample.tsx`)

**NEW**: Composable builder for complex coupon structures with full Python parity:

1. **Fixed and Floating Coupons**
   - Simple fixed-rate bonds (quarterly, semi-annual, annual)
   - Floating-rate notes with index + margin (e.g., SOFR + 150 bps)
2. **Payment Types**
   - Cash coupons (100% cash payment)
   - PIK coupons (100% capitalized into principal)
   - Split coupons (e.g., 70% cash / 30% PIK)
3. **Amortization**
   - Linear amortization to final balance
   - Step amortization schedules
4. **Advanced Programs**
   - Step-up coupons (4% → 5% → 6% over time)
   - Payment split programs (cash → 50/50 → PIK transitions)
5. **Builder Pattern**
   - Fluent chainable API
   - Matches Python `CashflowBuilder` capabilities

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

### Interest Rate Derivatives (`RatesInstruments.tsx`)

Comprehensive rates derivatives suite:

1. **Interest Rate Swaps** - Plain-vanilla fixed-for-floating swaps
2. **FRAs** - Forward rate agreements with fixing and settlement
3. **Swaptions** - Options on interest rate swaps (payer/receiver)
4. **Basis Swaps** - Floating-for-floating basis swaps
5. **Caps & Floors** - Interest rate options with Black pricing
6. **IR Futures** - Interest rate futures contracts

### FX Instruments (`FxInstruments.tsx`)

Foreign exchange instruments:

1. **FX Spot** - Spot foreign exchange transactions
2. **FX Options** - European calls and puts on FX rates
3. **FX Swaps** - Near and far leg FX swaps

### Credit Derivatives (`CreditInstruments.tsx`)

Credit instruments with survival probability modeling:

1. **CDS** - Single-name credit default swaps
2. **CDS Index** - Standardized credit indices
3. **CDS Tranches** - Synthetic CDO tranches with base correlation
4. **CDS Options** - Options on credit spreads
5. **Revolving Credit** - Credit facilities with both deterministic draw/repayment schedules (discounting model) and stochastic utilization (Monte Carlo model with mean-reverting process)

### Equity Instruments (`EquityInstruments.tsx`)

Equity spot and derivatives:

1. **Equity Spot** - Stock positions with pricing
2. **Equity Options** - European calls and puts with Greeks (for exotic options, see `ExoticEquityOptions.tsx`)

### Inflation Instruments (`InflationInstruments.tsx`)

Inflation-linked products:

1. **Inflation-Linked Bonds** - TIPS-style bonds with CPI indexation
2. **Inflation Swaps** - Zero-coupon inflation swaps

### Structured Products (`StructuredProducts.tsx`)

Complex structured instruments using JSON definitions:

1. **Baskets** - Multi-asset baskets with constituent weighting
2. **Private Markets Funds** - PE/credit funds with waterfall structures
3. **Autocallables** - Autocallable notes with barrier observations and early redemption
4. **ABS, CLO, CMBS, RMBS** - Asset-backed and mortgage securities (available via JSON)

### Exotic Equity Options (`ExoticEquityOptions.tsx`)

Advanced equity options priced using Monte Carlo simulation:

1. **Barrier Options** - Up-and-out, down-and-in options with continuous monitoring
2. **Asian Options** - Options on arithmetic or geometric averages
3. **Lookback Options** - Fixed and floating strike lookback options
4. **Cliquet Options** - Options with local/global floors and caps

All exotic options use the `monte_carlo_gbm` pricing model.

### Exotic FX Derivatives (`ExoticFxDerivatives.tsx`)

Exotic foreign exchange derivatives:

1. **FX Barrier Options** - Barrier options on FX rates with multi-currency discounting
2. **Quanto Options** - Cross-currency equity options (equity in one currency, payment in another)

These instruments require multi-currency market setup with FX matrices and are priced using Monte Carlo simulation.

### Exotic Rates Derivatives (`ExoticRatesDerivatives.tsx`)

Advanced interest rate derivatives:

1. **CMS Options** - Options on constant maturity swap rates (priced using Hull-White model)
2. **Range Accrual Notes** - Notes with coupons that accrue when reference rate stays within a range

CMS options use `monte_carlo_hull_white_1f` model, while range accruals use `monte_carlo_gbm`.

### Monte Carlo Path Generation (`MonteCarloPathExample.tsx`)

Standalone Monte Carlo path generation and analysis:

1. **Path Generation** - Generate GBM paths with configurable parameters (spot, drift, volatility)
2. **Sampling Strategies** - Capture all paths or sample a subset for memory efficiency
3. **Statistical Analysis** - Compute mean, standard deviation, and distribution statistics
4. **Data Export** - Export paths to CSV or JSON for external analysis
5. **Process Parameters** - Inspect and display underlying stochastic process parameters

Key features:

- Flexible capture modes (`all` vs `sample`)
- Deterministic and reproducible via seed parameter
- Path-by-path access for detailed analysis
- Terminal value distribution statistics

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
  const currency = new Currency('USD');
  // ... use currency
} catch (error) {
  console.error('Failed:', error);
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

### Date class naming

The finstack date class is exported as `FsDate` to avoid conflicts with JavaScript's built-in `Date` class:

```typescript
import { FsDate } from 'finstack-wasm';

const date = new FsDate(2024, 9, 30); // September 30, 2024
```

### Type errors

The package should include TypeScript definitions. If you encounter type errors, ensure the WASM package was built successfully and the `pkg` directory exists.

### Memory issues

If you experience memory leaks or crashes, ensure all WASM objects are properly freed with `.free()` when no longer needed.
