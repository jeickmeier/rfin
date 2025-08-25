### WASM Bindings — JavaScript Usage Examples

Version: 1.0 (examples)  
Audience: JavaScript/TypeScript developers using `rfin-wasm` in browsers or Node.js

---

## Overview

This guide shows how to use the WASM bindings to access core Finstack functionality from JavaScript and TypeScript. It focuses on:

- Core types: `Currency`, `Money`, `Date`, `Calendar`, `DayCount`
- Business‑day adjustments and day count fractions
- Period planning for statements and evaluations
- JSON serialization and deserialization
- Error handling and performance tips
- Optional feature‑gated modules (statements, valuations, portfolio) when enabled

The examples use modern ESM syntax. CommonJS examples are included where relevant.

---

## Installation

```bash
npm install rfin-wasm
# or
yarn add rfin-wasm
```

---

## Import and Initialization

### Browser (ESM)

When targeting the web, initialize the WASM module before use.

```javascript
import init, { Currency, Money, Date as FinDate, Calendar, BusDayConvention, DayCount } from 'rfin-wasm';

// Ensure WASM is loaded (e.g., at app startup)
await init();

// Now it is safe to use the bindings
const usd = Currency.USD; // enum/constructor variant
const amount = new Money(100, usd);
```

### Node.js

Node.js builds are usually ready to use without explicit initialization, but if your toolchain requires it you can still call `init()`.

```javascript
// ESM
import { Currency, Money } from 'rfin-wasm';

const value = new Money(250, Currency.USD);

// CommonJS
// const { Currency, Money } = require('rfin-wasm');
// const value = new Money(250, Currency.USD);
```

---

## Core Types

### Currency and Money

```javascript
import init, { Currency, Money } from 'rfin-wasm';
await init();

const usd = Currency.USD;             // Shorthand constant
const eur = Currency.fromCode('EUR'); // Or construct by code if provided

const usd100 = new Money(100, usd);
const usd50  = new Money(50, usd);
const eur75  = new Money(75, eur);

// Currency‑safe arithmetic
const totalUsd = usd100.add(usd50);   // -> Money(150, USD)

// Mismatched currencies throw a readable error
try {
  usd100.add(eur75);
} catch (e) {
  console.error(String(e)); // "Currency mismatch: expected USD, got EUR"
}

// Optional: explicit FX conversion when an FX provider is available
// (exact provider shape depends on enabled features)
// const fx = FxProvider.fromJSON({ 'USD/EUR': 0.92, 'EUR/USD': 1.087 });
// const eurInUsd = eur75.convertTo(Currency.USD, fx);
// const total = usd100.add(eurInUsd);
```

### Dates, Calendars, and Business‑Day Adjustment

```javascript
import init, { Date as FinDate, Calendar, BusDayConvention } from 'rfin-wasm';
await init();

const d = new FinDate(2025, 2, 15);            // YYYY, MM, DD
const tgt = Calendar.Target();                  // TARGET calendar
const nextBiz = tgt.adjust(d, BusDayConvention.Following);

console.log(`Adjusted date: ${nextBiz.toString()}`);
```

### Day Count Fractions

```javascript
import init, { Date as FinDate, DayCount } from 'rfin-wasm';
await init();

const start = new FinDate(2025, 1, 1);
const end   = new FinDate(2025, 7, 1);

const act360 = DayCount.ACT_360();
const yf = act360.yearFraction(start, end); // -> number

console.log(`ACT/360 year fraction: ${yf}`);
```

---

## Period Planning (Statements/Evaluations)

Use `PeriodPlan` to construct ranges of periods (e.g., quarters) for modeling and aggregation.

```javascript
import init, { PeriodPlan } from 'rfin-wasm';
await init();

// Plan from 2025 Q1 to 2026 Q4, marking 2025 Q1..Q2 as actuals
const plan = PeriodPlan.build('2025Q1..2026Q4', '2025Q1..Q2');

for (const period of plan.periods) {
  console.log(`${period.id}: ${period.start} to ${period.end} (actual=${period.isActual})`);
}
```

---

## JSON Serialization (Round‑Trip)

Most complex types support JSON round‑trip via `serde-wasm-bindgen`.

```javascript
import init, { Money, Currency } from 'rfin-wasm';
await init();

const m1 = new Money(123.45, Currency.USD);
const json = m1.toJSON();              // -> JsValue/JSON‑serializable object
const m2 = Money.fromJSON(json);       // -> Money

console.log(m2.equals(m1));            // true
```

---

## Error Handling

Errors thrown by the bindings are JavaScript‑friendly and include helpful messages that mirror Rust errors.

```javascript
import init, { Money, Currency } from 'rfin-wasm';
await init();

try {
  const a = new Money(10, Currency.USD);
  const b = new Money(10, Currency.EUR);
  a.add(b);
} catch (e) {
  const message = e instanceof Error ? e.message : String(e);
  console.warn('Operation failed:', message);
}
```

---

## Performance Tips (WASM Boundaries)

- Batch operations where possible to minimize JS↔WASM crossings.
- Prefer vectorized/array methods for large datasets.

```javascript
// Hypothetical batch API — actual names may differ
import init, { Calculator } from 'rfin-wasm';
await init();

const inputs = Float64Array.from([1, 2, 3, 4, 5]);
const outputs = Calculator.calculateMultiple(inputs); // Single boundary crossing
```

---

## Optional Feature‑Gated Modules

When the package is built with additional features, extra modules become available. The patterns mirror the core examples above.

### Statements (if enabled)

```javascript
import init, { Expression, ExpressionContext } from 'rfin-wasm';
await init();

const expr = Expression.parse('revenue * (1 + growth_rate)');
const ctx = new ExpressionContext({ revenue: 1_000_000, growth_rate: 0.05 });
const result = expr.evaluate(ctx); // 1_050_000
```

#### Build and Evaluate a Financial Model

```javascript
import init, { FinancialModel, PeriodPlan, Currency } from 'rfin-wasm';
await init();

// 1) Build periods
const plan = PeriodPlan.build('2025Q1..2026Q4', '2025Q1..Q2');

// 2) Define nodes (values + formulas) using serde-compatible JSON
const nodes = {
  revenue: { nodeId: 'revenue', nodeType: 'Value', values: { '2025Q1': 1000000 } },
  gross_profit: { nodeId: 'gross_profit', nodeType: 'Value', values: { '2025Q1': 420000 } },
  gross_margin: { nodeId: 'gross_margin', nodeType: 'Calculated', formula: 'gross_profit / revenue' },
  growth_rate: { nodeId: 'growth_rate', nodeType: 'Value', values: { default: 0.05 } },
  revenue_forecast: { nodeId: 'revenue_forecast', nodeType: 'Calculated', formula: 'revenue * (1 + growth_rate)' }
};

// 3) Construct model (constructor shape may vary by bindings version)
const model = new FinancialModel('Acme', plan.periods, nodes);

// 4) Evaluate
const results = model.evaluate({ parallel: false, baseCurrency: Currency.USD });

console.log(results.numericMode); // e.g., "decimal"
console.log(results.values['gross_margin']);
```

### Valuations (if enabled)

```javascript
import init, { MarketData, Bond } from 'rfin-wasm';
await init();

const market = new MarketData({ asOf: '2025-01-01', curves: { /* ... */ }, fx: { /* ... */ } });
const bond = Bond.builder('AAPL-5Y').coupon(0.04).maturity('2030-01-25').build();
const pv = bond.price(market);
```

### Portfolio (if enabled)

```javascript
import init, { Portfolio, Position, PortfolioRunner, Currency } from 'rfin-wasm';
await init();

const portfolio = Portfolio.builder('Fund A')
  .plan(Currency.USD, '2025-01-01', /* Periods or PeriodPlan */)
  .position(new Position({ id: 'B1', instrument: 'AAPL-5Y', quantity: 1_000_000 }))
  .build();

const runner = new PortfolioRunner({ parallel: false });
const out = await runner.run(portfolio, /* market data */);
console.log(out.valuation.portfolio_total_base);
```

### Scenarios

#### Parse and Preview a Scenario (DSL)

```javascript
import init, { Scenario } from 'rfin-wasm';
await init();

const dsl = `
market.fx.USD/EUR:+%2             # FX shock
statements.nodes.revenue:+%5      # Increase revenue by 5%
portfolio.positions."B1".quantity:+%10
@during 2025-01-01..2025-06-30    # Effective window
`;

const scenario = Scenario.parse(dsl, { strict: true });

// Optional preview to see expansions, ordering, and conflicts
const preview = scenario.preview({ asOf: '2025-03-31', market, portfolio, model });
console.table(preview.operations);
```

#### Compose Scenarios and Execute with PortfolioRunner

```javascript
import init, { Scenario, PortfolioRunner } from 'rfin-wasm';
await init();

const base = Scenario.parse(`market.curves.USD.Shift:+bp25`);
const overlay = Scenario.parse(`statements.nodes.growth_rate:+%2`);

const composed = Scenario.compose([base, overlay], {
  priorities: [0, 10],
  conflict: 'last_wins'
});

const runner = new PortfolioRunner({ parallel: false });
const out = await runner.run(portfolio, market, { scenario: composed, asOf: '2025-04-01' });
console.log(out.meta.appliedScenarioId);
```

#### JSON Form (Programmatic)

```javascript
import init, { Scenario } from 'rfin-wasm';
await init();

const spec = {
  strict: true,
  lines: [
    { path: 'market.fx.USD/EUR', op: 'percent', value: 2 },
    { path: 'statements.nodes.revenue', op: 'percent', value: 5, during: ['2025-01-01', '2025-06-30'] }
  ]
};

const scenario = Scenario.fromJSON(spec);
```

### Analysis

#### Discover and Run Built-in Analyzers

```javascript
import init, { Analysis, FinancialModel } from 'rfin-wasm';
await init();

// List available analyzers
const analyzers = Analysis.listAnalyzers();
console.table(analyzers.map(a => ({ id: a.id, version: a.version, category: a.category })));

// 1) Validation Report
const validation = await Analysis.create('validation_report').run({ model, strict: true });
console.log(validation.passed, validation.errors.length, validation.warnings.length);

// 2) Node Explainer
const explainer = await Analysis.create('node_explainer').run({ model, nodeId: 'gross_margin', periodId: '2025Q1' });
console.log(explainer.formula, explainer.dependencies);

// 3) Sensitivity (single variable)
const sens = await Analysis.create('sensitivity').run({
  model,
  variables: [{ path: 'statements.nodes.growth_rate', grid: { type: 'percent', points: [-5, 0, 5, 10] } }],
  outputs: ['gross_profit', 'revenue_forecast'],
  parallel: true
});
console.table(sens.table);
```

#### Pipelines

```javascript
import init, { Analysis } from 'rfin-wasm';
await init();

const pipeline = Analysis.pipeline()
  .step('validate', 'validation_report', { model, strict: true })
  .step('explain_margin', 'node_explainer', { model, nodeId: 'gross_margin', periodId: '2025Q1' }, { if: '${validate.passed}' })
  .step('grid_growth', 'grid', {
    model,
    variables: [
      { path: 'statements.nodes.growth_rate', grid: { type: 'percent', start: -5, stop: 10, step: 5 } }
    ],
    outputs: ['revenue_forecast']
  })
  .build();

const result = await pipeline.run({ parallel: true });
console.log(Object.keys(result)); // { validate, explain_margin, grid_growth }
```

### Structured Credit

#### Build a Structured Product and Project Cashflows

```javascript
import init, {
  Currency,
  StructuredProduct,
  CollateralPool,
  Tranche,
  Waterfall,
  WaterfallStep,
  PaymentRecipient,
  WaterfallAmountType,
  Trigger,
  TriggerType,
  StructuredProductAssumptions,
  DefaultCurve,
  DefaultCurveType,
  PrepaymentCurve,
  PrepaymentCurveType,
  MarketData
} from 'rfin-wasm';

await init();

// Minimal sketch (real models will be more detailed)
const pool = new CollateralPool({ assets: [] });

const senior = new Tranche({
  id: 'A', class: 'Senior', seniority: 1,
  originalBalance: { amount: 100_000_000, currency: Currency.USD },
  currentBalance: { amount: 100_000_000, currency: Currency.USD },
  coupon: { type: 'Fixed', rate: 0.06 },
  creditEnhancement: { subordination: 0.30 }
});

const equity = new Tranche({
  id: 'E', class: 'Equity', seniority: 4,
  originalBalance: { amount: 20_000_000, currency: Currency.USD },
  currentBalance: { amount: 20_000_000, currency: Currency.USD },
  coupon: { type: 'Fixed', rate: 0.00 },
  creditEnhancement: { subordination: 0.00 }
});

const waterfall = new Waterfall({
  paymentDates: [ '2025-03-31', '2025-06-30' ],
  interest: [ new WaterfallStep({ priority: 1, description: 'Senior interest', recipient: { type: PaymentRecipient.Tranche, id: 'A' }, amount: { type: WaterfallAmountType.CurrentInterest } }) ],
  principal: [ new WaterfallStep({ priority: 1, description: 'Senior principal', recipient: { type: PaymentRecipient.Tranche, id: 'A' }, amount: { type: WaterfallAmountType.Principal } }) ]
});

const triggers = [ new Trigger({ id: 'OC_A', kind: { type: TriggerType.OC, tranche: 'A' }, threshold: 1.30 }) ];

const deal = new StructuredProduct({
  id: 'CLO-1',
  collateralPool: pool,
  tranches: [senior, equity],
  waterfall,
  triggers,
  reserveAccounts: [],
  fees: { managementFee: { bps: 50 }, servicingFee: { bps: 25 }, trusteeFee: { bps: 10 }, otherFees: [] }
});

const assumptions = new StructuredProductAssumptions({
  defaultCurve: new DefaultCurve({ curveType: { type: DefaultCurveType.Constant }, parameters: [0.02] }),
  prepaymentCurve: new PrepaymentCurve({ curveType: { type: PrepaymentCurveType.PSA }, parameters: [100] }),
  recoveryRates: { 'BB': 0.40, 'B': 0.35 },
  recoveryLag: 6
});

const market = new MarketData({ asOf: '2025-01-01', curves: { /* ... */ }, fx: { /* ... */ } });

const proj = deal.projectCashflows(market, assumptions);
console.log(Object.keys(proj.trancheCashflows));

// Run a single waterfall period with collections
const wf = deal.runWaterfall({ amount: 5_000_000, currency: Currency.USD }, '2025-03-31');
console.table(wf.distributions);
```

#### Scenario Integration (Structured Credit)

```javascript
import init, { Scenario } from 'rfin-wasm';
await init();

// Example: tighten OC trigger and increase fees under a stress
const sc = Scenario.parse(`
structured.credit.triggers."OC_A".threshold:=1.35
structured.credit.fees.management_fee:+bp10
`);

const preview = sc.preview({ structuredProduct: deal });
console.table(preview.operations);
```

---

## TypeScript Hints

`rfin-wasm` ships `.d.ts` declarations for strong typing and editor IntelliSense.

```typescript
import init, { Money, Currency } from 'rfin-wasm';

await init();

const usd: Currency = Currency.USD;
const m: Money = new Money(42, usd);

// Narrowed types for JSON round‑trip
const json: unknown = m.toJSON();
const restored: Money = Money.fromJSON(json as object);
```

---

## Bundlers and Tree Shaking

- The package is side‑effect‑free to enable tree shaking in modern bundlers.
- Import only what you need to keep bundles small.

```javascript
// Good: selective imports
import init, { Money } from 'rfin-wasm';

// Avoid: star imports if bundle size is a concern
// import * as Rfin from 'rfin-wasm';
```

---

## Troubleshooting

- Ensure `await init()` is called in browsers before using any exports.
- If you see MIME type or CORS errors in development, configure your dev server to serve `.wasm` with the correct `application/wasm` content type.
- For Node.js, ensure your runtime supports ESM if using `import` syntax, or switch to CommonJS `require`.

---

## See Also

- Technical design: `docs/new/08_bindings/08_bindings_tdd.md`
- Product requirements: `docs/new/08_bindings/08_bindings_prd.md`


