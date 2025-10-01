# WASM Calibration API Guide

**Complete Feature Parity with Python Bindings - All 5 Calibrators**

## Quick Reference

| Calibrator | Input Quotes | Output | Use Case |
|------------|--------------|--------|----------|
| **DiscountCurveCalibrator** | Deposits, Swaps | DiscountCurve | Bootstrap OIS/Treasury curves |
| **ForwardCurveCalibrator** | FRAs, Swaps | ForwardCurve | Calibrate LIBOR/SOFR forward curves |
| **HazardCurveCalibrator** | CDS spreads | HazardCurve | Credit default probability curves |
| **InflationCurveCalibrator** | ZC Inflation Swaps | InflationCurve | CPI projection curves |
| **VolSurfaceCalibrator** | Option/Swaption vols | VolSurface | Implied volatility surfaces |

## 1. Discount Curve Calibration

Bootstrap a discount curve from deposit and swap quotes.

```typescript
import {
  CalibrationConfig,
  DiscountCurveCalibrator,
  RatesQuote,
  SolverKind,
  Date,
  Frequency,
} from 'finstack-wasm';

const baseDate = new Date(2024, 1, 2);

// Configure
const config = CalibrationConfig.multiCurve()
  .withSolverKind(SolverKind.Hybrid())
  .withMaxIterations(40)
  .withTolerance(1e-10);

// Create quotes
const quotes = [
  RatesQuote.deposit(new Date(2024, 2, 1), 0.0450, 'act_360'),
  RatesQuote.deposit(new Date(2024, 4, 2), 0.0465, 'act_360'),
  RatesQuote.swap(
    new Date(2025, 1, 2),
    0.0475,
    Frequency.annual(),
    Frequency.quarterly(),
    '30_360',
    'act_360',
    'USD-SOFR'
  ),
  RatesQuote.swap(
    new Date(2027, 1, 2),
    0.0485,
    Frequency.annual(),
    Frequency.quarterly(),
    '30_360',
    'act_360',
    'USD-SOFR'
  ),
];

// Calibrate
const calibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, 'USD')
  .withConfig(config);

const [curve, report] = calibrator.calibrate(quotes, null);

if (report.success) {
  console.log('Discount factors:');
  console.log('DF(1Y):', curve.df(1.0));
  console.log('DF(2Y):', curve.df(2.0));
  console.log('DF(3Y):', curve.df(3.0));
}
```

## 2. Forward Curve Calibration

Calibrate a forward curve from FRA and swap quotes (requires existing discount curve).

```typescript
import {
  ForwardCurveCalibrator,
  RatesQuote,
  Date,
  MarketContext,
} from 'finstack-wasm';

// Create market context with discount curve
const market = new MarketContext();
market.insertDiscount(discountCurve); // from previous calibration

// Create forward quotes
const quotes = [
  RatesQuote.fra(
    new Date(2024, 2, 1),
    new Date(2024, 5, 1),
    0.0470,
    'act_360'
  ),
  RatesQuote.fra(
    new Date(2024, 4, 1),
    new Date(2024, 7, 1),
    0.0480,
    'act_360'
  ),
];

// Calibrate
const calibrator = new ForwardCurveCalibrator(
  'USD-SOFR-3M',
  0.25,        // 3-month tenor
  baseDate,
  'USD',
  'USD-OIS'    // discount curve dependency
).withConfig(config);

const [fwdCurve, report] = calibrator.calibrate(quotes, market);

if (report.success) {
  console.log('Forward rates:');
  console.log('Rate(1Y):', fwdCurve.rate(1.0));
  console.log('Rate(2Y):', fwdCurve.rate(2.0));
}
```

## 3. Hazard Curve Calibration

Calibrate a credit hazard curve from CDS spreads.

```typescript
import {
  HazardCurveCalibrator,
  CreditQuote,
  Date,
  MarketContext,
} from 'finstack-wasm';

// Create market context with discount curve
const market = new MarketContext();
market.insertDiscount(discountCurve);

// Create CDS quotes
const quotes = [
  CreditQuote.cds('ACME', new Date(2026, 1, 2), 120.0, 0.40, 'USD'),
  CreditQuote.cds('ACME', new Date(2027, 1, 2), 135.0, 0.40, 'USD'),
  CreditQuote.cds('ACME', new Date(2029, 1, 2), 150.0, 0.40, 'USD'),
];

// Calibrate
const calibrator = new HazardCurveCalibrator(
  'ACME',           // entity name
  'senior',         // seniority: 'senior', 'subordinated', 'junior'
  0.40,             // recovery rate
  baseDate,
  'USD',
  'USD-OIS'         // discount curve (pass null for default)
).withConfig(config);

const [hazardCurve, report] = calibrator.calibrate(quotes, market);

if (report.success) {
  console.log('Survival probabilities:');
  console.log('SP(1Y):', hazardCurve.survival(1.0));
  console.log('SP(3Y):', hazardCurve.survival(3.0));
  console.log('SP(5Y):', hazardCurve.survival(5.0));
}
```

## 4. Inflation Curve Calibration

Calibrate an inflation curve from zero-coupon inflation swap quotes.

```typescript
import {
  InflationCurveCalibrator,
  InflationQuote,
  Date,
  MarketContext,
} from 'finstack-wasm';

// Create market context with discount curve
const market = new MarketContext();
market.insertDiscount(discountCurve);

// Create inflation swap quotes
const quotes = [
  InflationQuote.inflationSwap(new Date(2026, 1, 2), 0.021, 'US-CPI-U'),
  InflationQuote.inflationSwap(new Date(2029, 1, 2), 0.023, 'US-CPI-U'),
];

// Calibrate
const calibrator = new InflationCurveCalibrator(
  'US-CPI-U',       // curve ID
  baseDate,
  'USD',
  300.0,            // base CPI level
  'USD-OIS'         // discount curve
).withConfig(config);

const [inflCurve, report] = calibrator.calibrate(quotes, market);

if (report.success) {
  console.log('CPI levels:');
  console.log('CPI(1Y):', inflCurve.cpi(1.0));
  console.log('CPI(3Y):', inflCurve.cpi(3.0));
  console.log('CPI(5Y):', inflCurve.cpi(5.0));
}
```

## 5. Volatility Surface Calibration

Calibrate an implied volatility surface from option and swaption quotes.

```typescript
import {
  VolSurfaceCalibrator,
  VolQuote,
  Date,
  MarketContext,
  MarketScalar,
  Money,
} from 'finstack-wasm';

// Create market context with discount curve and spot prices
const market = new MarketContext();
market.insertDiscount(discountCurve);
market.insertPrice('AAPL', MarketScalar.price(Money.fromCode(150.0, 'USD')));
market.insertPrice('AAPL-DIVYIELD', MarketScalar.unitless(0.015));

// Create volatility quotes
const quotes = [
  VolQuote.optionVol('AAPL', new Date(2024, 7, 1), 90.0, 0.24, 'Call'),
  VolQuote.optionVol('AAPL', new Date(2024, 7, 1), 100.0, 0.22, 'Call'),
  VolQuote.optionVol('AAPL', new Date(2024, 7, 1), 110.0, 0.23, 'Call'),
  VolQuote.optionVol('AAPL', new Date(2025, 1, 2), 90.0, 0.26, 'Call'),
  VolQuote.optionVol('AAPL', new Date(2025, 1, 2), 100.0, 0.24, 'Call'),
  VolQuote.optionVol('AAPL', new Date(2025, 1, 2), 110.0, 0.25, 'Call'),
];

// Calibrate
const calibrator = new VolSurfaceCalibrator(
  'AAPL-VOL',
  1.0,                               // beta parameter
  [0.5, 1.0],                       // target expiries (years)
  [90.0, 100.0, 110.0]              // target strikes
)
  .withBaseDate(baseDate)
  .withConfig(config)
  .withDiscountId('USD-OIS');

const [surface, report] = calibrator.calibrate(quotes, market);

if (report.success) {
  console.log('Volatilities:');
  console.log('Vol(0.5Y, ATM):', surface.value(0.5, 100.0));
  console.log('Vol(1.0Y, ATM):', surface.value(1.0, 100.0));
}
```

## 6. Simple Multi-Curve Calibration

One-shot calibration workflow for building a complete market context.

```typescript
import {
  SimpleCalibration,
  CalibrationConfig,
  RatesQuote,
  CreditQuote,
  SolverKind,
  Date,
  Frequency,
} from 'finstack-wasm';

const baseDate = new Date(2024, 1, 2);

const config = CalibrationConfig.multiCurve()
  .withSolverKind(SolverKind.Hybrid())
  .withMaxIterations(40);

const calibration = new SimpleCalibration(baseDate, 'USD', config);

// Mixed quote types
const quotes = [
  RatesQuote.deposit(new Date(2024, 2, 1), 0.045, 'act_360').toMarketQuote(),
  RatesQuote.swap(
    new Date(2025, 1, 2),
    0.047,
    Frequency.annual(),
    Frequency.quarterly(),
    '30_360',
    'act_360',
    'USD-SOFR'
  ).toMarketQuote(),
  CreditQuote.cds('ACME', new Date(2029, 1, 2), 120.0, 0.40, 'USD').toMarketQuote(),
];

const [market, report] = calibration.calibrate(quotes);

if (report.success) {
  const stats = market.stats();
  console.log('Calibrated curves:', stats.total_curves);
  console.log('Iterations:', report.iterations);
  console.log('Max residual:', report.maxResidual);
  
  // Use calibrated curves
  const discCurve = market.discount('USD-OIS');
  console.log('DF(1Y):', discCurve.df(1.0));
}
```

## Configuration Options

### CalibrationConfig Builder

```typescript
const config = new CalibrationConfig()
  .withTolerance(1e-10)              // convergence tolerance
  .withMaxIterations(100)            // iteration limit
  .withSolverKind(SolverKind.Hybrid()) // solver strategy
  .withParallel(false)               // parallel execution (WASM: use false)
  .withVerbose(true);                // detailed logging

// Or use preset
const multiCurveConfig = CalibrationConfig.multiCurve();
```

### Solver Strategies

```typescript
// Newton-Raphson (fast, requires good initial guess)
SolverKind.Newton()

// Brent's method (robust bracketing)
SolverKind.Brent()

// Hybrid (Newton then Brent fallback) - RECOMMENDED
SolverKind.Hybrid()

// Levenberg-Marquardt (for nonlinear least squares)
SolverKind.LevenbergMarquardt()

// Differential Evolution (global optimization, slower)
SolverKind.DifferentialEvolution()
```

## Calibration Report

Every calibration returns a detailed report:

```typescript
const [curve, report] = calibrator.calibrate(quotes, market);

// Check success
if (report.success) {
  console.log('✓ Calibration converged');
  console.log('Iterations:', report.iterations);
  console.log('RMSE:', report.rmse);
  console.log('Max residual:', report.maxResidual);
  console.log('Reason:', report.convergenceReason);
  
  // Get full report as JSON
  const fullReport = report.toJson();
  console.log('Residuals:', fullReport.residuals);
  console.log('Metadata:', fullReport.metadata);
  
  // Check specific instrument residual
  const residual = report.getResidual('swap_2y');
  console.log('2Y swap residual:', residual);
} else {
  console.error('✗ Calibration failed');
  console.error('Reason:', report.convergenceReason);
}
```

## Error Handling

```typescript
try {
  const [curve, report] = calibrator.calibrate(quotes, market);
  
  if (!report.success) {
    throw new Error(`Calibration failed: ${report.convergenceReason}`);
  }
  
  // Use calibrated curve
  market.insertDiscount(curve);
  
} catch (error) {
  console.error('Calibration error:', error);
  
  // Fallback strategies:
  // 1. Use direct curve construction
  const fallbackCurve = new DiscountCurve(/*...*/);
  
  // 2. Retry with different solver
  const retryConfig = config.withSolverKind(SolverKind.Brent());
  // ...
}
```

## Advanced Usage

### Chain Calibrations

```typescript
// Step 1: Calibrate discount curve
const [discCurve, discReport] = discCalibrator.calibrate(discQuotes, null);

// Step 2: Insert into market and calibrate forward curve
const market = new MarketContext();
market.insertDiscount(discCurve);

const [fwdCurve, fwdReport] = fwdCalibrator.calibrate(fwdQuotes, market);
market.insertForward(fwdCurve);

// Step 3: Calibrate hazard curve
const [hazardCurve, hazReport] = hazCalibrator.calibrate(cdsQuotes, market);
market.insertHazard(hazardCurve);

// Now use complete market for pricing
const registry = createStandardRegistry();
const result = registry.priceCreditDefaultSwap(cds, 'discounting', market);
```

### Custom Quote Sets

```typescript
// Build dynamic quote set from market data API
async function buildQuotesFromApi(apiData) {
  const quotes = [];
  
  for (const deposit of apiData.deposits) {
    quotes.push(RatesQuote.deposit(
      new Date(deposit.maturity.year, deposit.maturity.month, deposit.maturity.day),
      deposit.rate,
      deposit.dayCount
    ));
  }
  
  for (const swap of apiData.swaps) {
    quotes.push(RatesQuote.swap(
      new Date(swap.maturity.year, swap.maturity.month, swap.maturity.day),
      swap.rate,
      Frequency.fromMonths(swap.fixedFreqMonths),
      Frequency.fromMonths(swap.floatFreqMonths),
      swap.fixedDayCount,
      swap.floatDayCount,
      swap.indexName
    ));
  }
  
  return quotes;
}

const marketQuotes = await buildQuotesFromApi(apiResponse);
const [curve, report] = calibrator.calibrate(marketQuotes, null);
```

## Performance Optimization

### Reuse Calibrators

```typescript
// Create calibrator once
const calibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, 'USD')
  .withConfig(config);

// Recalibrate with different quote sets
const [curve1, report1] = calibrator.calibrate(morningQuotes, null);
const [curve2, report2] = calibrator.calibrate(afternoonQuotes, null);
const [curve3, report3] = calibrator.calibrate(closingQuotes, null);
```

### Parallel Calibration (Web Workers)

```typescript
// main.ts
const worker = new Worker('calibration-worker.js');

worker.postMessage({
  type: 'calibrate_discount',
  quotes: serializableQuotes,
  config: {
    tolerance: 1e-10,
    maxIterations: 40,
    solverKind: 'hybrid',
  }
});

worker.onmessage = (event) => {
  const { curve, report } = event.data;
  console.log('Calibration complete:', report);
};

// calibration-worker.js
import init, { DiscountCurveCalibrator, RatesQuote, /*...*/ } from 'finstack-wasm';

await init();

self.onmessage = async (event) => {
  const { type, quotes, config } = event.data;
  
  if (type === 'calibrate_discount') {
    const calibrator = new DiscountCurveCalibrator(/*...*/);
    const [curve, report] = calibrator.calibrate(quotes, null);
    
    self.postMessage({ curve: serializeCurve(curve), report: report.toJson() });
  }
};
```

## TypeScript Types

All calibration types have full TypeScript definitions:

```typescript
import type {
  CalibrationConfig,
  CalibrationReport,
  DiscountCurveCalibrator,
  ForwardCurveCalibrator,
  HazardCurveCalibrator,
  InflationCurveCalibrator,
  VolSurfaceCalibrator,
  SimpleCalibration,
  SolverKind,
  RatesQuote,
  CreditQuote,
  VolQuote,
  InflationQuote,
  MarketQuote,
  DiscountCurve,
  ForwardCurve,
  HazardCurve,
  InflationCurve,
  VolSurface,
  MarketContext,
  Date,
  Frequency,
} from 'finstack-wasm';

// Type-safe calibration function
function calibrateDiscount(
  quotes: RatesQuote[],
  baseDate: Date,
  currency: string,
  config?: CalibrationConfig
): [DiscountCurve, CalibrationReport] {
  const calibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, currency);
  
  if (config) {
    return calibrator.withConfig(config).calibrate(quotes, null) as [DiscountCurve, CalibrationReport];
  }
  
  return calibrator.calibrate(quotes, null) as [DiscountCurve, CalibrationReport];
}
```

## Bundle Size Considerations

**Full Calibration Suite** (all 5 calibrators):
- WASM: +300 KB
- JavaScript: +12 KB
- Total impact: ~13.6%

**Optimization Options**:

1. **Tree-shaking** - Unused calibrators are automatically removed if you only import what you need
2. **Feature flag** - Can be added later if needed to exclude calibration entirely
3. **Lazy loading** - Load calibration module only when needed

```typescript
// Lazy load calibration
const loadCalibration = async () => {
  const { DiscountCurveCalibrator, RatesQuote } = await import('finstack-wasm');
  return { DiscountCurveCalibrator, RatesQuote };
};

// Use when needed
const handleCalibrate = async () => {
  const { DiscountCurveCalibrator, RatesQuote } = await loadCalibration();
  // ... calibrate
};
```

## Production Checklist

- [ ] Sufficient quotes (5-10+ instruments per curve)
- [ ] Market context with required dependencies
- [ ] Error handling for calibration failures
- [ ] Convergence validation (`report.success`)
- [ ] Residual inspection (`report.maxResidual < threshold`)
- [ ] Fallback curves for failed calibration
- [ ] Memory cleanup (WASM objects auto-GC'd)
- [ ] Performance testing with realistic quote sets

## Complete Example

```typescript
import {
  CalibrationConfig,
  DiscountCurveCalibrator,
  ForwardCurveCalibrator,
  HazardCurveCalibrator,
  RatesQuote,
  CreditQuote,
  SolverKind,
  Date,
  Frequency,
  MarketContext,
} from 'finstack-wasm';

async function buildFullMarket() {
  const baseDate = new Date(2024, 1, 2);
  const config = CalibrationConfig.multiCurve()
    .withSolverKind(SolverKind.Hybrid())
    .withMaxIterations(40);

  // Step 1: Discount curve
  const discQuotes = [
    RatesQuote.deposit(new Date(2024, 2, 1), 0.045, 'act_360'),
    RatesQuote.swap(new Date(2025, 1, 2), 0.047, Frequency.annual(), Frequency.quarterly(), '30_360', 'act_360', 'USD-SOFR'),
    RatesQuote.swap(new Date(2027, 1, 2), 0.049, Frequency.annual(), Frequency.quarterly(), '30_360', 'act_360', 'USD-SOFR'),
  ];
  
  const discCalibrator = new DiscountCurveCalibrator('USD-OIS', baseDate, 'USD').withConfig(config);
  const [discCurve, discReport] = discCalibrator.calibrate(discQuotes, null);
  
  if (!discReport.success) {
    throw new Error('Discount curve calibration failed');
  }
  
  // Step 2: Build market and calibrate forward curve
  const market = new MarketContext();
  market.insertDiscount(discCurve);
  
  const fwdQuotes = [
    RatesQuote.fra(new Date(2024, 4, 1), new Date(2024, 7, 1), 0.048, 'act_360'),
    RatesQuote.fra(new Date(2024, 7, 1), new Date(2024, 10, 1), 0.049, 'act_360'),
  ];
  
  const fwdCalibrator = new ForwardCurveCalibrator('USD-SOFR-3M', 0.25, baseDate, 'USD', 'USD-OIS').withConfig(config);
  const [fwdCurve, fwdReport] = fwdCalibrator.calibrate(fwdQuotes, market);
  
  if (fwdReport.success) {
    market.insertForward(fwdCurve);
  }
  
  // Step 3: Calibrate hazard curve
  const cdsQuotes = [
    CreditQuote.cds('ACME', new Date(2027, 1, 2), 120.0, 0.40, 'USD'),
    CreditQuote.cds('ACME', new Date(2029, 1, 2), 135.0, 0.40, 'USD'),
  ];
  
  const hazCalibrator = new HazardCurveCalibrator('ACME', 'senior', 0.40, baseDate, 'USD', 'USD-OIS').withConfig(config);
  const [hazCurve, hazReport] = hazCalibrator.calibrate(cdsQuotes, market);
  
  if (hazReport.success) {
    market.insertHazard(hazCurve);
  }
  
  return market;
}

// Use calibrated market
const market = await buildFullMarket();
const registry = createStandardRegistry();
const swapResult = registry.priceInterestRateSwap(swap, 'discounting', market);
```

## See Also

- [CALIBRATION_SUMMARY.md](./CALIBRATION_SUMMARY.md) - Implementation details
- [WASM_CALIBRATION_COMPLETE.md](../WASM_CALIBRATION_COMPLETE.md) - Completion report
- [CalibrationExample.tsx](./examples/src/components/CalibrationExample.tsx) - Interactive example
- Python calibration examples in `finstack-py/examples/scripts/valuations/calibration_capabilities.py`

