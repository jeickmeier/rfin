# Finstack WASM Bindings

WebAssembly bindings for the Finstack financial computation library.

## Currently exposed APIs

The bindings provide comprehensive coverage of finstack-core and finstack-valuations,
achieving feature parity with the Python bindings.

### Core Primitives

- `Date` – construct calendar dates, inspect components, and adjust by weekdays.
- `Currency` – create ISO-4217 currencies by code or numeric identifier and
  enumerate the compiled set via `Currency.all()`.
- `Money` – construct currency-tagged amounts, format them, or hydrate from
  `[amount, currencyCode]` tuples through `Money.fromTuple`. Use
  `Money.fromCode(amount, "USD")` for ergonomic construction without a
  `Currency` instance.
- `FinstackConfig`/`RoundingMode` – manage rounding strategies and per-currency
  decimal scales for ingest/output, mirroring the Python bindings.

### Dates & Calendars

- `Calendar`/`BusinessDayConvention` – retrieve registry calendars, inspect
  holidays, and perform business-day adjustments.
- `ScheduleBuilder`/`Schedule` – generate business-day aware cashflow schedules
  with stub handling, end-of-month alignment, or CDS IMM rolls.
- `DayCount`/`DayCountContext`/`Frequency` – compute year fractions using
  finstack's day-count conventions with optional calendar/frequency hints.
- `PeriodId`/`PeriodPlan`/`FiscalConfig` – build calendar or fiscal period plans
  with actual/forecast segmentation via `buildPeriods` and `buildFiscalPeriods`.
- IMM and utility helpers – IMM rolls, option expiries, month arithmetic, and
  epoch conversions via `daysSinceEpochToDate` and friends.

### Market Data

- `DiscountCurve` – construct discount-factor term structures with selectable
  interpolation and extrapolation policies.
- `ForwardCurve`, `HazardCurve`, `InflationCurve`, `BaseCorrelationCurve` – additional
  market data term structures for rates, credit, inflation, and tranche pricing.
- `MarketContext` – aggregate curves, surfaces, FX matrices, and scalars for pricing.
- `FxMatrix`/`FxConversionPolicy` – multi-currency conversion with triangulation support.
- `VolSurface` – two-dimensional implied volatility surfaces.
- `ScalarTimeSeries` – time series data with interpolation.

### Cashflows & Instruments

- `CashFlow`/`CFKind`/`AmortizationSpec` – primitive cashflow types with fixed, floating,
  PIK, fee, and amortization support.
- **`CashflowBuilder`** – composable builder for complex coupon structures with:
  - Fixed and floating coupons
  - Cash/PIK/split payment types
  - Amortization schedules (linear, step)
  - Step-up coupon programs
  - Payment split programs (cash-to-PIK transitions)
- `CouponType`, `ScheduleParams`, `FixedCouponSpec`, `FloatingCouponSpec` – supporting
  types for the builder pattern.

#### Rates Instruments

- `Bond` – fixed-income instruments with helpers for fixed, floating, zero-coupon, and callable bonds.
- `Deposit` – money-market deposits with simple interest.
- `InterestRateSwap` – plain-vanilla fixed-for-floating interest rate swaps.
- `ForwardRateAgreement` – FRA instruments for forward rate exposure.
- `Swaption` – options on interest rate swaps (payer/receiver).
- `BasisSwap` – floating-for-floating basis swaps.
- `InterestRateOption` – interest rate caps and floors.
- `InterestRateFuture` – interest rate futures contracts.

#### FX Instruments

- `FxSpot` – foreign exchange spot transactions.
- `FxOption` – FX options (European call/put).
- `FxSwap` – FX swap contracts with near and far legs.

#### Credit Instruments

- `CreditDefaultSwap` – single-name CDS (buy/sell protection).
- `CDSIndex` – standardized CDS index positions.
- `CdsTranche` – synthetic CDO tranches.
- `CdsOption` – options on CDS spreads.

#### Equity Instruments

- `Equity` – equity spot positions.
- `EquityOption` – equity options (European call/put).
- `EquityTotalReturnSwap` – equity total return swaps.
- `FiIndexTotalReturnSwap` – fixed-income index total return swaps.

#### Inflation Instruments

- `InflationLinkedBond` – inflation-linked bonds (TIPS-style).
- `InflationSwap` – zero-coupon inflation swaps.

#### Structured Products

- `Basket` – multi-asset baskets (JSON-based).
- `StructuredCredit` – unified instrument for ABS, CLO, CMBS, and RMBS (JSON-based).
- `PrivateMarketsFund` – private equity/credit funds with waterfall structures (JSON-based).

#### Other Instruments

- `Repo` – repurchase agreements with collateral.
- `VarianceSwap` – variance swap contracts.
- `ConvertibleBond` – convertible bond instruments.

### Pricing & Metrics

- `PricerRegistry` – instrument pricing with model selection.
- `ValuationResult` – pricing results with present value and risk metrics.
- Instrument-specific pricing methods (e.g., `priceBond`, `priceCreditDefaultSwap`, etc.).

### Scenarios & Stress Testing

- **`JsScenarioEngine`** – reproducible scenario execution engine with stable ordering.
- **`JsScenarioSpec`** – scenario specifications with operations and metadata.
- **`JsOperationSpec`** – individual shock operations (market, statement, time).
- **`JsExecutionContext`** – execution context wrapping market, model, and date.
- **`JsApplicationReport`** – results from scenario application with warnings.
- **`JsRollForwardReport`** – P&L breakdown from time roll-forward operations.
- Supported operations:
  - Market shocks: FX, equity, curves, volatility surfaces, base correlation
  - Statement adjustments: forecast percent changes and value assignments
  - Instrument shocks: price/spread by type or attributes
  - Time operations: roll forward with carry/theta calculations
- Features: scenario composition, priority ordering, JSON serialization

### Calibration

- `DiscountCurveCalibrator` – calibrate discount curves from deposits and swaps.
- `ForwardCurveCalibrator` – calibrate forward curves from FRAs and swaps.
- `HazardCurveCalibrator` – calibrate credit hazard curves from CDS quotes.
- `InflationCurveCalibrator` – calibrate inflation curves from inflation swap quotes.
- `VolSurfaceCalibrator` – calibrate implied volatility surfaces from option quotes.
- `BaseCorrelationCalibrator` – calibrate base correlation curves from CDO tranche quotes.
- `CalibrationConfig` – configure solver strategy, tolerance, and iterations.
- `SolverKind` – choose optimization method (Newton, Brent, Hybrid, LM, DE).
- `RatesQuote`, `CreditQuote`, `VolQuote`, `InflationQuote` – market quote types for calibration.
- `CalibrationReport` – detailed convergence diagnostics and residuals.

**Feature Parity**: The WASM bindings have **100% feature parity** with `finstack-py`, including all calibration APIs.

## Building

```bash
# Install wasm-pack
curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build for web targets
wasm-pack build --target web

# Build for Node.js
wasm-pack build --target nodejs
```

## Usage

### Web Browser

```javascript
import init, {
  // NOTE: For complete import list, see below

  // Core essentials shown here for brevity
  // Dates & Calendars
  Date as FinstackDate,
  Calendar,
  BusinessDayConvention,
  DayCount,
  DayCountContext,
  Frequency,
  ScheduleBuilder,
  StubKind,
  buildPeriods,
  availableCalendars,
  adjust,

  // Core Primitives
  Currency,
  Money,
  FinstackConfig,
  RoundingMode,

  // Market Data
  DiscountCurve,
  ForwardCurve,
  HazardCurve,
  InflationCurve,
  MarketContext,
  FxMatrix,
  VolSurface,

  // Cashflow Builder
  CashflowBuilder,
  CashFlowSchedule,
  CouponType,
  ScheduleParams,
  FixedCouponSpec,
  FloatingCouponSpec,
  FloatCouponParams,

  // Instruments
  Bond,
  Deposit,
  InterestRateSwap,
  ForwardRateAgreement,
  Swaption,
  BasisSwap,
  InterestRateOption,
  FxSpot,
  FxOption,
  FxSwap,
  CreditDefaultSwap,
  CDSIndex,
  CdsTranche,
  Equity,
  EquityOption,
  Repo,
  InflationLinkedBond,
  InflationSwap,
  VarianceSwap,
  ConvertibleBond,
  Basket,
  StructuredCredit,
  PrivateMarketsFund,

  // Pricing
  createStandardRegistry,
  PricerRegistry,
  ValuationResult,

  // Calibration
  CalibrationConfig,
  CalibrationReport,
  DiscountCurveCalibrator,
  ForwardCurveCalibrator,
  HazardCurveCalibrator,
  InflationCurveCalibrator,
  VolSurfaceCalibrator,
  SolverKind,
  RatesQuote,
  CreditQuote,
  VolQuote,
  InflationQuote,
  MarketQuote,
} from './pkg/finstack_wasm.js';

async function run() {
  // Initialize WASM module once at application startup
  await init();

  const tradeDate = new FinstackDate(2024, 9, 30);

  const usd = new Currency('USD');
  const amount = new Money(100.0, usd);

  console.log(amount.amount); // 100.0
  console.log(amount.currency.code); // "USD"

  const viaCode = Money.fromCode(42.5, 'EUR');
  console.log(viaCode.format()); // "EUR 42.50"

  const calendars = availableCalendars();
  const nyc = calendars.find((cal) => cal.code === 'usny') ?? new Calendar('usny');
  const adjusted = adjust(tradeDate, BusinessDayConvention.Following, nyc);
  console.log(adjusted.toString()); // business-day adjusted date

  const dayCount = DayCount.act365f();
  const ctx = new DayCountContext();
  ctx.setFrequency(Frequency.semiAnnual());
  const yf = dayCount.yearFraction(tradeDate, adjusted, ctx);
  console.log(yf); // year fraction respecting DayCountContext

  const schedule = new ScheduleBuilder(tradeDate, new FinstackDate(2025, 9, 30))
    .frequency(Frequency.quarterly())
    .stubRule(StubKind.none())
    .adjustWith(BusinessDayConvention.ModifiedFollowing, nyc)
    .endOfMonth(true)
    .build();
  console.log(schedule.toArray().map((d) => d.toString()));

  const discountCurve = new DiscountCurve(
    'USD-OIS',
    tradeDate,
    [0.0, 0.5, 1.0, 5.0],
    [1.0, 0.99, 0.96, 0.85],
    'act_365f',
    InterpStyle.MonotoneConvex,
    ExtrapolationPolicy.FlatForward,
    true
  );
  console.log(discountCurve.df(2.5));

  const plan = buildPeriods('2024Q1..Q4', '2024Q2');
  console.log(plan.toArray().map((period) => [period.id.code, period.isActual]));

  const cfg = new FinstackConfig();
  cfg.setRoundingMode(RoundingMode.AwayFromZero);
  cfg.setIngestScale(usd, 4);
  const highPrecision = Money.fromConfig(1.23456, usd, cfg);
  console.log(highPrecision.toTuple()); // [1.2346, Currency('USD')]

  // Cashflow builder example
  const notional = Money.fromCode(1_000_000, 'USD');
  const issue = new FinstackDate(2025, 1, 15);
  const maturity = new FinstackDate(2030, 1, 15);

  const schedule = ScheduleParams.quarterlyAct360();
  const fixedSpec = new FixedCouponSpec(0.05, schedule, CouponType.Cash());

  const cashflowSchedule = new CashflowBuilder()
    .principal(notional, issue, maturity)
    .fixedCf(fixedSpec)
    .build();

  console.log('Total flows:', cashflowSchedule.length);
  console.log('Notional:', cashflowSchedule.notional.format());
  console.log('Day count:', cashflowSchedule.dayCount.name);

  // Price instruments using the registry
  const registry = createStandardRegistry();

  // Example: Price an interest rate swap (explicit inputs, no defaults)
  const swap = new InterestRateSwap(
    'swap_1',
    Money.fromCode(10_000_000, 'USD'),
    0.0325,
    tradeDate,
    new FinstackDate(2029, 9, 30),
    'USD-OIS', // discount curve id
    'USD-SOFR-3M', // forward curve id
    'pay_fixed', // side: 'pay_fixed' | 'receive_fixed'
    null, // fixed frequency (optional)
    DayCount.thirty360(), // fixed day count (optional)
    null, // float frequency (optional)
    DayCount.act360(), // float day count (optional)
    null, // business day convention (optional)
    null, // calendar id (optional)
    null, // stub kind (optional)
    2 // reset lag days (optional)
  );
  const swapResult = registry.priceInterestRateSwapWithMetrics(swap, 'discounting', market, [
    'dv01',
    'annuity',
    'par_rate',
  ]);
  console.log('Swap PV:', swapResult.presentValue.format());
  console.log('Swap DV01:', swapResult.metric('dv01'));

  // Example: Price a credit default swap
  const cds = CreditDefaultSwap.buyProtection(
    'cds_1',
    Money.fromCode(5_000_000, 'USD'),
    120.0, // spread in bps
    tradeDate,
    new FinstackDate(2029, 9, 30),
    'USD-OIS',
    'ACME-HAZARD'
  );
  const cdsResult = registry.priceCreditDefaultSwap(cds, 'discounting', market);
  console.log('CDS PV:', cdsResult.presentValue.format());

  // Example: Calibrate a discount curve from market quotes
  const calibrationConfig = CalibrationConfig.multiCurve()
    .withSolverKind(SolverKind.Hybrid())
    .withMaxIterations(40);

  const discountCalibrator = new DiscountCurveCalibrator('USD-OIS', tradeDate, 'USD').withConfig(
    calibrationConfig
  );

  const quotes = [
    RatesQuote.deposit(new FinstackDate(2024, 11, 1), 0.045, 'act_360'),
    RatesQuote.swap(
      new FinstackDate(2025, 9, 30),
      0.047,
      Frequency.annual(),
      Frequency.quarterly(),
      '30_360',
      'act_360',
      'USD-SOFR'
    ),
  ];

  try {
    const [curve, report] = discountCalibrator.calibrate(quotes, null);
    console.log('Calibration success:', report.success);
    console.log('Iterations:', report.iterations);
    console.log('Max residual:', report.maxResidual);
    console.log('Discount factor at 1Y:', curve.df(1.0));
  } catch (err) {
    console.log('Calibration failed (expected with minimal quotes):', err);
  }
}

run();
```

**Important**: Call `init()` only once at application startup. Calling it multiple times will reinitialize WASM memory and cause memory corruption errors.

### Node.js

```javascript
const {
  Date: FinstackDate,
  Calendar,
  BusinessDayConvention,
  DayCount,
  DayCountContext,
  Frequency,
  ScheduleBuilder,
  StubKind,
  DiscountCurve,
  InterpStyle,
  ExtrapolationPolicy,
  buildPeriods,
  adjust,
  Currency,
  Money,
  FinstackConfig,
} = require('./pkg-node/finstack_wasm.js');

const usd = new Currency('USD');
const amount = new Money(100.0, usd);
console.log(amount.toTuple());

const cfg = new FinstackConfig();
cfg.setOutputScale(usd, 3);
console.log(Money.fromConfig(12.34567, usd, cfg).format());

const date = new FinstackDate(2024, 3, 29);
const calendar = new Calendar('gblo');
const adjusted = adjust(date, BusinessDayConvention.ModifiedFollowing, calendar);
const ctx = new DayCountContext();
ctx.setFrequency(Frequency.quarterly());
console.log(DayCount.act365f().yearFraction(date, adjusted, ctx));

const schedule = new ScheduleBuilder(date, new FinstackDate(2024, 12, 20))
  .cdsImm()
  .adjustWith(BusinessDayConvention.Following, calendar)
  .build();
console.log(schedule.toArray().map((d) => d.toString()));

const plan = buildPeriods('2023Q3..2024Q2', null);
console.log(plan.length);

const discountCurve = new DiscountCurve(
  'USD-OIS',
  date,
  [0.0, 1.0, 3.0],
  [1.0, 0.95, 0.85],
  'act_365f',
  InterpStyle.Linear,
  ExtrapolationPolicy.FlatZero,
  false
);
console.log(discountCurve.zero(2.0));
```

## Examples

The `examples/` directory contains a full-featured React + TypeScript + Vite application demonstrating finstack-wasm usage in a realistic browser environment with **feature parity to the Python bindings**.

To run the examples:

```bash
# Build the WASM package first
npm run build

# Install example dependencies
npm run examples:install

# Run the development server
npm run examples:dev
```

The examples demonstrate comprehensive functionality with **feature parity to the Python bindings**, including calibration:

### Date & Calendar Features

- Date construction and manipulation (weekdays, quarters, fiscal years)
- Date utilities (month arithmetic, leap years, epoch conversions)
- Holiday calendars and business day adjustments
- Day count conventions (Act/360, Act/365F, 30/360, Act/Act, BUS/252)
- Schedule generation (monthly, quarterly, semi-annual, CDS IMM)
- Period plans (calendar and fiscal periods)
- IMM dates and option expiries

### Market Data Features

- Discount curves with interpolation
- Forward, hazard, inflation, and base correlation curves
- FX matrices and rate lookups
- Time series with interpolation
- Market context for data storage
- Volatility surfaces

### Cashflow & Valuation Features

- **Cashflow Builder** – composable builder for complex coupon structures:
  - Fixed and floating coupons
  - Cash/PIK/split payment types
  - Amortization schedules
  - Step-up coupon programs
  - Payment split programs
- Primitive cashflows (fixed, floating, PIK, fees, principal)
- Bond instruments with valuation metrics
- Deposit instruments
- Pricing registry with standard models

### Math Utilities

- Numerical integration (Gauss-Hermite, Gauss-Legendre, adaptive Simpson)
- Root finding solvers (Newton, Brent, Hybrid)
- Distribution helpers (binomial probabilities)

### Calibration Features (100% Python Parity)

- **Discount Curve Calibration** – fit curves to deposit and swap quotes
- **Forward Curve Calibration** – calibrate forward curves from FRAs and swaps
- **Hazard Curve Calibration** – calibrate credit curves from CDS spreads
- **Inflation Curve Calibration** – calibrate CPI curves from inflation swap quotes
- **Vol Surface Calibration** – calibrate implied vol surfaces from option/swaption quotes
- **Simple Multi-Curve Workflow** – one-shot calibration for full market context
- **Solver Configuration** – choose optimization strategy (Newton, Brent, Hybrid, LM, DE)
- **Convergence Diagnostics** – inspect iterations, residuals, and success metrics
- **Quote Types** – rates, credit, vol, and inflation market quotes

All features include:

- Proper WASM memory management patterns
- TypeScript type safety
- Complete documentation

See `examples/README.md` for detailed documentation.

## Testing

```bash
wasm-pack test --chrome --firefox --headless
```

## Bundle Size Optimization

For production deployments, use the optimized build with wasm-opt:

```bash
npm run build:optimized
```

This runs wasm-pack with release optimizations and applies wasm-opt post-processing, typically reducing bundle size by 20-30%.

## Versioning & Breaking Changes

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR version** (0.x → 1.0, 1.x → 2.0): Breaking changes to public API
- **MINOR version** (0.1.x → 0.2.0): New features, backward-compatible
- **PATCH version** (0.1.1 → 0.1.2): Bug fixes, no API changes

See [CHANGELOG.md](./CHANGELOG.md) for detailed release notes and migration guides.

### Deprecation Policy

When we need to deprecate a feature:

1. The old API will be marked with JSDoc `@deprecated` tags
2. Deprecation will be maintained for at least one MINOR version
3. Removal will only occur in MAJOR version bumps
4. Migration guides will be provided in CHANGELOG.md

### MSRV (Minimum Supported Rust Version)

MSRV: **1.90+** (defined at workspace level)

We will clearly communicate MSRV increases in CHANGELOG.md and consider them breaking changes.

## CI/CD

This project uses GitHub Actions for continuous integration:

- **Build verification**: Web and Node.js targets
- **Tests**: Headless browser tests (Chrome)
- **Linting**: Clippy and rustfmt
- **Bundle size tracking**: Fails if bundle exceeds 5MB
- **Security**: cargo-audit for vulnerability scanning
- **Examples**: Automated example builds

See [.github/workflows/wasm-ci.yml](../.github/workflows/wasm-ci.yml) for configuration.

## Using with Python

The finstack library also provides Python bindings via PyO3. The Python bindings have **100% feature parity** with the WASM bindings, enabling seamless code migration between languages.

### Quick Links

- **Python Bindings:** See [`finstack-py/README.md`](../finstack-py/README.md)
- **API Reference:** Complete TypeScript ↔ Python mapping in [`book/src/bindings/api-reference.md`](../book/src/bindings/api-reference.md)
- **Migration Guide:** Detailed migration patterns in [`book/src/bindings/migration-guide.md`](../book/src/bindings/migration-guide.md)
- **Naming Conventions:** Function name mappings in [`NAMING_CONVENTIONS.md`](../NAMING_CONVENTIONS.md)
- **Side-by-Side Examples:** Code comparisons in [`book/src/bindings/examples.md`](../book/src/bindings/examples.md)

### Example: Same Code, Different Language

**TypeScript:**

```typescript
import { Bond, DiscountCurveCalibrator } from 'finstack-wasm';

const bond = Bond.treasury('US-10Y', 1_000_000, 'USD', 0.0375, maturity, issue);
const calibrator = new DiscountCurveCalibrator('USD-OIS', date, 'USD');
const [curve, report] = calibrator.calibrate(quotes, market);
```

**Python:**

```python
from finstack.valuations.instruments import Bond
from finstack.valuations.calibration import DiscountCurveCalibrator

bond = Bond.treasury("US-10Y", 1_000_000, "USD", 0.0375, maturity, issue)
calibrator = DiscountCurveCalibrator("USD-OIS", date, "USD")
curve, report = calibrator.calibrate(quotes, market)
```

**Key Differences:** Method names use snake_case in Python vs camelCase in TypeScript. Module imports are nested in Python (`finstack.core.dates`) vs flat in WASM (`'finstack-wasm'`). See the [Naming Conventions](../NAMING_CONVENTIONS.md) guide for complete mappings.

## Contributing

Contributions are welcome! Please ensure:

1. All tests pass: `wasm-pack test --chrome --headless`
2. Code is formatted: `cargo fmt --all`
3. No clippy warnings: `cargo clippy --target wasm32-unknown-unknown --all-features -- -D warnings`
4. Bundle size impact is documented for significant changes
5. Breaking changes are documented in CHANGELOG.md

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE) or http://opensource.org/licenses/MIT)

at your option.
