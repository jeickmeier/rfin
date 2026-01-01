# React + WASM Integration Patterns for finstack-wasm

## Critical WASM Memory Management Rules

### 1. WASM Initialization (CRITICAL)

**NEVER** call `init()` more than once. Multiple initializations corrupt WASM memory and cause "memory access out of bounds" errors.

```typescript
// ❌ WRONG: Don't call init() in individual components
const MyComponent = () => {
  useEffect(() => {
    init(); // This causes memory corruption on hot reload!
  }, []);
};

// ✅ CORRECT: Call init() once at app level with guard against hot reloads
let wasmInitialized = false;

const App: React.FC = () => {
  const [wasmReady, setWasmReady] = useState(wasmInitialized);

  useEffect(() => {
    // Guard against React hot module replacement
    if (wasmInitialized) {
      setWasmReady(true);
      return;
    }

    init()
      .then(() => {
        wasmInitialized = true;
        setWasmReady(true);
      })
      .catch((err) => setError(err.message));
  }, []);

  if (!wasmReady) return <div>Loading WASM...</div>;
  return <YourApp />;
};
```

### 2. Disable React.StrictMode in Development

React.StrictMode intentionally runs effects twice, which will double-initialize WASM and cause memory corruption.

```typescript
// ❌ WRONG
ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);

// ✅ CORRECT
ReactDOM.createRoot(document.getElementById('root')!).render(
  <BrowserRouter>
    <App />
  </BrowserRouter>
);
```

### 3. Extract Primitives Immediately from WASM Objects

**NEVER** store WASM objects in React state. They will be garbage collected and cause "null pointer passed to rust" errors.

```typescript
// ❌ WRONG: Storing WASM objects in state
const [result, setResult] = useState<ValuationResult | null>(null);
const [bond, setBond] = useState<Bond | null>(null);
const [dates, setDates] = useState<FsDate[]>([]);

useEffect(() => {
  const result = registry.priceBond(bond, 'discounting', market);
  setResult(result); // WASM object will get garbage collected!
}, []);

// Later in render
return <div>{result.presentValue.amount}</div>; // ERROR: memory access out of bounds

// ✅ CORRECT: Extract all primitives immediately
type BondMetrics = {
  presentValue: number;
  cleanPrice: number;
  accrued: number;
  // ... only primitives
};

const [metrics, setMetrics] = useState<BondMetrics | null>(null);

useEffect(() => {
  const result = registry.priceBondWithMetrics(bond, 'discounting', market, metricKeys);

  // Extract ALL primitives before storing in state
  const primitives = {
    presentValue: result.presentValue.amount,  // Extract number
    cleanPrice: result.metric('clean_price') ?? 0,  // Extract number
    accrued: result.metric('accrued') ?? 0,  // Extract number
  };

  setMetrics(primitives);  // Only primitives in state!
}, []);
```

### 4. Convert WASM Dates to Strings Immediately

WASM Date objects are particularly prone to GC issues. Convert to strings immediately:

```typescript
// ❌ WRONG: Storing Date objects
type Cashflow = {
  date: FsDate;  // WASM object - will get corrupted
  amount: number;
};

const flows: Cashflow[] = [];
flows.push({ date: someDate, amount: 100 });  // someDate will be GC'd

// Later in JSX
{flows.map(flow => <td>{flow.date.toString()}</td>)}  // ERROR!

// ✅ CORRECT: Convert to string immediately
type Cashflow = {
  date: string;  // Primitive string
  amount: number;
};

const flows: Cashflow[] = [];
flows.push({ date: someDate.toString(), amount: 100 });  // Convert immediately

// Later in JSX
{flows.map(flow => <td>{flow.date}</td>)}  // Works!
```

### 5. Use Async Wrapper Pattern

All working examples use an async IIFE wrapper:

```typescript
// ✅ CORRECT pattern from working examples
useEffect(() => {
  let cancelled = false;
  (async () => {
    try {
      // Your WASM calls here
      const bond = Bond.fixedSemiannual(...);
      const result = registry.priceBond(bond, 'discounting', market);

      // Extract primitives
      const pv = result.presentValue.amount;

      if (!cancelled) {
        setState({ pv });
      }
    } catch (err) {
      if (!cancelled) {
        setError((err as Error).message);
      }
    }
  })();

  return () => {
    cancelled = true;
  };
}, []);
```

### 6. Rust Pricing Engine Usage

**ALWAYS** use the Rust pricing engine through the registry. Never reimplement financial calculations in JavaScript.

```typescript
// ❌ WRONG: Manual cashflow generation in JavaScript
const buildCashflows = (bond, periods, dayCount) => {
  const flows = [];
  periods.forEach(period => {
    const accrual = dayCount.yearFraction(period.start, period.end);
    const coupon = notional * rate * accrual;
    flows.push({ date: period.end, amount: coupon });
  });
  return flows;
};

// ✅ CORRECT: Use Rust pricing engine
const registry = createStandardRegistry();
const result = registry.priceBondWithMetrics(
  bond,
  'discounting',  // Model key
  market,         // MarketContext with curves
  ['clean_price', 'accrued', 'duration_mod', 'dv01']  // Metrics to compute
);

// Extract what you need
const pv = result.presentValue.amount;
const cleanPrice = result.metric('clean_price') ?? 0;
const accrued = result.metric('accrued') ?? 0;
```

### 7. Component Structure Pattern

```typescript
export const MyValuationExample: React.FC = () => {
  // State: only primitives (numbers, strings, booleans)
  const [metrics, setMetrics] = useState<MyMetrics | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        // 1. Create instruments
        const notional = Money.fromCode(1_000_000, 'USD');
        const issue = new FsDate(2024, 1, 15);
        const maturity = new FsDate(2029, 1, 15);

        // 2. Create market data
        const discountCurve = new DiscountCurve(...);
        const market = new MarketContext();
        market.insertDiscount(discountCurve);

        // 3. Create instruments
        const bond = Bond.fixedSemiannual('id', notional, 0.05, issue, maturity, 'USD-OIS');

        // 4. Price through Rust
        const registry = createStandardRegistry();
        const result = registry.priceBondWithMetrics(bond, 'discounting', market, metricKeys);

        // 5. Extract primitives IMMEDIATELY
        const primitives = {
          presentValue: result.presentValue.amount,
          cleanPrice: result.metric('clean_price') ?? 0,
          // ... all primitives
        };

        // 6. Store only primitives
        if (!cancelled) {
          setMetrics(primitives);
        }
      } catch (err) {
        if (!cancelled) {
          setError((err as Error).message);
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, []);

  if (error) return <p className="error">{error}</p>;
  if (!metrics) return <p>Loading...</p>;

  return <div>{/* Render primitives */}</div>;
};
```

## Common Errors and Solutions

### Error: "memory access out of bounds"

**Cause**: WASM init() called multiple times or WASM objects stored in React state and garbage collected.
**Fix**: Use global init guard and extract primitives immediately.

### Error: "null pointer passed to rust"

**Cause**: WASM object was garbage collected before use.
**Fix**: Extract primitives immediately, don't store WASM objects.

### Error: "Invalid input data"

**Cause**: Wrong parameters to Rust functions (e.g., date ranges, curve tenors).
**Fix**: Check Rust validation rules (dates must be ordered, curves need minimum points, etc.).

## API-Specific Patterns

### Date Handling

```typescript
// Properties, not methods
const year = date.year;      // NOT date.year()
const month = date.month;    // NOT date.month()
const day = date.day;        // NOT date.day()

// Convert to string immediately if storing
const dateStr = date.toString();
```

### Tenor Handling

```typescript
// Property, not method
const months = tenor.months;  // NOT tenor.months()
```

### Curve Handling

```typescript
// Methods, not properties
const dayCount = curve.dayCount();  // Method call
const baseDate = curve.baseDate;    // Property
```

### Pricing Registry

```typescript
// Type-specific methods (not generic)
const bondResult = registry.priceBondWithMetrics(bond, model, market, metrics);
const depositResult = registry.priceDepositWithMetrics(deposit, model, market, metrics);
```

## Testing WASM Examples

1. **Always test with a full page reload** (not just HMR) after WASM changes
2. **Kill and restart dev server** to clear corrupted WASM memory: `pkill -f vite && npm run examples:dev`
3. **Check browser console** for WASM errors - they're often more descriptive than React errors
4. **Watch for Finalization errors** in console - indicates GC corruption from stored WASM objects

## References

- Working examples: `DatesAndMarketData.tsx`, `CashflowBasics.tsx`, `DepositsValuation.tsx`, `BondsValuation.tsx`
- WASM Memory FAQ: `finstack-wasm/examples/README.md`
- Rust Pricing Registry: `finstack/valuations/src/pricer/`
