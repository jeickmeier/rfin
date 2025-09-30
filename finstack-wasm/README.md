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
- `DayCount`/`DayCountContext`/`Frequency` – compute year fractions using
  finstack's day-count conventions with optional calendar/frequency hints.
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
    ctx.setFrequency(Frequency.SemiAnnual);
    const yf = dayCount.yearFraction(tradeDate, adjusted, ctx);
    console.log(yf); // year fraction respecting DayCountContext

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
ctx.setFrequency(Frequency.Quarterly);
console.log(DayCount.act365f().yearFraction(date, adjusted, ctx));
```

## Testing

```bash
wasm-pack test --chrome --firefox --headless
```
