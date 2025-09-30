# Finstack WASM Bindings

WebAssembly bindings for the Finstack financial computation library.

## Currently exposed APIs

The initial release focuses on the `finstack-core` primitives that underpin the
Python bindings.

- `Date` – construct calendar dates, inspect components, and adjust by weekdays.
- `Currency` – create ISO-4217 currencies by code or numeric identifier and
  enumerate the compiled set via `Currency.all()`.
- `Money` – construct currency-tagged amounts, format them, or hydrate from
  `[amount, currencyCode]` tuples through `Money.fromTuple`. Use
  `Money.fromCode(amount, "USD")` for ergonomic construction without a
  `Currency` instance.
- `FinstackConfig`/`RoundingMode` – manage rounding strategies and per-currency
  decimal scales for ingest/output, mirroring the Python bindings.
- `Calendar`/`BusinessDayConvention` – retrieve registry calendars, inspect
  holidays, and perform business-day adjustments.
- `ScheduleBuilder`/`Schedule` – generate business-day aware cashflow schedules
  with stub handling, end-of-month alignment, or CDS IMM rolls.
- `DayCount`/`DayCountContext`/`Frequency` – compute year fractions using
  finstack's day-count conventions with optional calendar/frequency hints.
- `PeriodId`/`PeriodPlan`/`FiscalConfig` – build calendar or fiscal period plans
  with actual/forecast segmentation via `buildPeriods` and `buildFiscalPeriods`.
- `DiscountCurve` – construct discount-factor term structures with selectable
  interpolation and extrapolation policies.
- `ForwardCurve`, `HazardCurve`, `InflationCurve`, `BaseCorrelationCurve` – additional
  market data term structures for rates, credit, inflation, and tranche pricing.
- IMM and utility helpers – IMM rolls, option expiries, month arithmetic, and
  epoch conversions via `daysSinceEpochToDate` and friends.

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
} from './pkg/finstack_wasm.js';

async function run() {
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
}

run();
```

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

## Testing

```bash
wasm-pack test --chrome --firefox --headless
```
