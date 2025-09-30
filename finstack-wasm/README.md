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
- **`CashflowBuilder`** – **NEW**: composable builder for complex coupon structures with:
  - Fixed and floating coupons
  - Cash/PIK/split payment types
  - Amortization schedules (linear, step)
  - Step-up coupon programs
  - Payment split programs (cash-to-PIK transitions)
- `CouponType`, `ScheduleParams`, `FixedCouponSpec`, `FloatingCouponSpec` – supporting
  types for the builder pattern.
- `Bond` – fixed-income instruments with helper methods for common structures.
- `Deposit` – money-market deposits with simple interest.

### Pricing & Metrics
- `PricerRegistry` – instrument pricing with model selection.
- `ValuationResult` – pricing results with present value and risk metrics.

Additional modules (dates, calendars, market data, valuations) will be ported
incrementally until the WASM bindings reach parity with `finstack-py`.

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
    Date as FinstackDate,
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
    availableCalendars,
    adjust,
    Currency,
    Money,
    FinstackConfig,
    RoundingMode,
    CashflowBuilder,
    CashFlowSchedule,
    CouponType,
    ScheduleParams,
    FixedCouponSpec,
    FloatingCouponSpec,
    FloatCouponParams,
} from './pkg/finstack_wasm.js';

async function run() {
    // Initialize WASM module once at application startup
    await init();

    const tradeDate = new FinstackDate(2024, 9, 30);

    const usd = new Currency("USD");
    const amount = new Money(100.0, usd);

    console.log(amount.amount); // 100.0
    console.log(amount.currency.code); // "USD"

    const viaCode = Money.fromCode(42.5, "EUR");
    console.log(viaCode.format()); // "EUR 42.50"

    const calendars = availableCalendars();
    const nyc = calendars.find((cal) => cal.code === "usny") ?? new Calendar("usny");
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
        "USD-OIS",
        tradeDate,
        [0.0, 0.5, 1.0, 5.0],
        [1.0, 0.99, 0.96, 0.85],
        "act_365f",
        InterpStyle.MonotoneConvex,
        ExtrapolationPolicy.FlatForward,
        true,
    );
    console.log(discountCurve.df(2.5));

    const plan = buildPeriods("2024Q1..Q4", "2024Q2");
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

const usd = new Currency("USD");
const amount = new Money(100.0, usd);
console.log(amount.toTuple());

const cfg = new FinstackConfig();
cfg.setOutputScale(usd, 3);
console.log(Money.fromConfig(12.34567, usd, cfg).format());

const date = new FinstackDate(2024, 3, 29);
const calendar = new Calendar("gblo");
const adjusted = adjust(date, BusinessDayConvention.ModifiedFollowing, calendar);
const ctx = new DayCountContext();
ctx.setFrequency(Frequency.quarterly());
console.log(DayCount.act365f().yearFraction(date, adjusted, ctx));

const schedule = new ScheduleBuilder(date, new FinstackDate(2024, 12, 20))
    .cdsImm()
    .adjustWith(BusinessDayConvention.Following, calendar)
    .build();
console.log(schedule.toArray().map((d) => d.toString()));

const plan = buildPeriods("2023Q3..2024Q2", null);
console.log(plan.length);

const discountCurve = new DiscountCurve(
    "USD-OIS",
    date,
    [0.0, 1.0, 3.0],
    [1.0, 0.95, 0.85],
    "act_365f",
    InterpStyle.Linear,
    ExtrapolationPolicy.FlatZero,
    false,
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

The examples demonstrate comprehensive functionality with **feature parity to the Python bindings**:

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

All features include:
- Proper WASM memory management patterns
- TypeScript type safety
- Complete documentation

See `examples/README.md` for detailed documentation.

## Testing

```bash
wasm-pack test --chrome --firefox --headless
```
