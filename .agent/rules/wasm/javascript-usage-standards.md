---
trigger: model_decision
globs: *.tsx,*.ts,*.js
---

# JavaScript/TypeScript Usage Standards for rfin-wasm

## Overview

This document covers standards for JavaScript and TypeScript code that uses the rfin-wasm module, including:
- Web applications
- Node.js applications
- Example code
- Test files
- Documentation examples

## Setup and Initialization

### Browser Setup
```javascript
// ES Modules
import init, { Currency, Money, Date } from './pkg/rfin_wasm.js';

async function initialize() {
    // Always await initialization before using any types
    await init();

    // Now you can use the library
    const usd = new Currency("USD");
    const amount = new Money(100.0, usd);
}

// Call initialization
initialize().catch(console.error);
```

### Node.js Setup
```javascript
// CommonJS
const { Currency, Money, Date } = require('./pkg-node/rfin_wasm.js');

// No initialization needed for Node.js builds
const usd = new Currency("USD");
```

### TypeScript Setup
```typescript
// types.d.ts or inline
import type { Currency, Money, Date, DayCount } from './pkg/rfin_wasm';

// With proper typing
async function createMoney(amount: number, currencyCode: string): Promise<Money> {
    await init();
    const currency = new Currency(currencyCode);
    return new Money(amount, currency);
}
```

## Import Patterns

### Recommended Import Structure
```javascript
// Import init function and types
import init, {
    // Core types
    Currency,
    Money,
    Date,

    // Enums
    Frequency,
    DayCount,
    BusDayConvention,

    // Complex types
    Calendar,
    FixedRateLeg,

    // Functions (with camelCase names)
    generateSchedule,
    thirdWednesday,
    nextImm,
    nextCdsDate
} from './pkg/rfin_wasm.js';
```

### Avoid Global Scope Pollution
```javascript
// Good: Import only what you need
import { Currency, Money } from './pkg/rfin_wasm.js';

// Avoid: Importing everything into global scope
import * as rfin from './pkg/rfin_wasm.js';
window.rfin = rfin; // Don't do this
```

## Type Construction

### Currency Creation
```javascript
// Good: Direct construction
const usd = new Currency("USD");
const eur = new Currency("EUR");

// Handle errors appropriately
try {
    const invalid = new Currency("XXX");
} catch (error) {
    console.error("Invalid currency code:", error);
}

// Access properties
console.log(usd.code);         // "USD"
console.log(usd.numericCode);  // 840
console.log(usd.decimals);     // 2
```

### Money Creation
```javascript
// Create money instances
const amount = new Money(1000.50, usd);

// Access properties
console.log(amount.amount);    // 1000.5
console.log(amount.currency);  // Currency instance

// Arithmetic operations
try {
    const sum = amount1.add(amount2);
    const difference = amount1.subtract(amount2);
    const scaled = amount.multiply(1.1);
    const divided = amount.divide(2);
} catch (error) {
    // Handle currency mismatch or other errors
    console.error("Operation failed:", error);
}
```

### Date Creation
```javascript
// Create dates (month is 1-based)
const date = new Date(2023, 12, 25); // December 25, 2023

// Access properties
console.log(date.year);        // 2023
console.log(date.month);       // 12
console.log(date.day);         // 25

// Date operations
const isWeekend = date.isWeekend();
const quarter = date.quarter();
const nextBusinessDay = date.addBusinessDays(1);
```

## Error Handling

### Try-Catch Pattern
```javascript
async function safeCurrencyOperation() {
    try {
        await init();
        const currency = new Currency("USD");
        return currency;
    } catch (error) {
        console.error("Failed to create currency:", error);
        // Provide fallback or re-throw
        throw new Error(`Currency initialization failed: ${error.message}`);
    }
}
```

### Validation Before Operations
```javascript
function addMoney(amount1, amount2) {
    // Validate inputs
    if (!(amount1 instanceof Money) || !(amount2 instanceof Money)) {
        throw new TypeError("Both arguments must be Money instances");
    }

    // Check currency compatibility
    if (!amount1.currency.equals(amount2.currency)) {
        throw new Error(
            `Cannot add ${amount1.currency.code} and ${amount2.currency.code}`
        );
    }

    return amount1.add(amount2);
}
```

## Enum Usage

### Frequency Enum
```javascript
import { Frequency } from './pkg/rfin_wasm.js';

// Use enum values
const schedule = generateSchedule(
    startDate,
    endDate,
    Frequency.SemiAnnual  // Enum value
);

// In configurations
const bondConfig = {
    frequency: Frequency.Quarterly,
    dayCount: DayCount.Act360(),
    convention: BusDayConvention.ModifiedFollowing
};
```

### DayCount Static Methods
```javascript
import { DayCount } from './pkg/rfin_wasm.js';

// Create day count conventions
const act360 = DayCount.Act360();
const act365f = DayCount.Act365F();
const thirty360 = DayCount.Thirty360();

// Use in calculations
const yearFraction = act360.yearFraction(startDate, endDate);
const days = act360.days(startDate, endDate);
```

## Complex Operations

### Schedule Generation
```javascript
async function generatePaymentSchedule(config) {
    await init();

    const {
        startDate,
        endDate,
        frequency,
        calendar,
        convention
    } = config;

    // Generate raw schedule
    const dates = generateSchedule(startDate, endDate, frequency);

    // Adjust for business days if needed
    if (calendar && convention) {
        return dates.map(date =>
            calendar.adjust(date, convention)
        );
    }

    return dates;
}
```

### Fixed Rate Leg Creation
```javascript
async function createBond(params) {
    await init();

    const {
        notional,
        currencyCode,
        couponRate,
        issueDate,
        maturityDate,
        frequency = Frequency.SemiAnnual,
        dayCount = DayCount.Thirty360()
    } = params;

    const currency = new Currency(currencyCode);
    const leg = new FixedRateLeg(
        notional,
        currency,
        couponRate,
        issueDate,
        maturityDate,
        frequency,
        dayCount
    );

    return {
        leg,
        numFlows: leg.numFlows,
        npv: leg.npv(),
        flows: leg.getCashFlows()
    };
}
```

## Memory Management

### Proper Cleanup
```javascript
// WASM objects are automatically garbage collected
// But avoid holding unnecessary references

class PortfolioManager {
    constructor() {
        this.positions = new Map();
    }

    addPosition(id, money) {
        this.positions.set(id, money);
    }

    removePosition(id) {
        // Clear reference to allow GC
        this.positions.delete(id);
    }

    clear() {
        // Clear all references
        this.positions.clear();
    }
}
```

### Avoid Memory Leaks
```javascript
// Bad: Creating objects in loops without need
function calculateTotal(amounts) {
    let total = new Money(0, new Currency("USD"));
    for (const amount of amounts) {
        total = total.add(new Money(amount, new Currency("USD"))); // Creates many temporary objects
    }
    return total;
}

// Good: Reuse objects where possible
function calculateTotalEfficient(amounts, currency) {
    let total = new Money(0, currency);
    for (const amount of amounts) {
        const temp = new Money(amount, currency);
        total = total.add(temp);
    }
    return total;
}
```

## TypeScript Best Practices

### Type Definitions
```typescript
// Define interfaces for your domain objects
interface BondParameters {
    notional: number;
    currency: Currency;
    couponRate: number;
    issueDate: Date;
    maturityDate: Date;
    frequency?: Frequency;
    dayCount?: DayCount;
}

interface CashFlow {
    date: Date;
    amount: number;
    currency: string;
    type: string;
}
```

### Generic Functions
```typescript
// Type-safe wrappers
function createMoney<T extends number>(
    amount: T,
    currency: Currency
): Money {
    return new Money(amount, currency);
}

// Result types
type Result<T, E = Error> =
    | { success: true; value: T }
    | { success: false; error: E };

async function safeCurrencyCreation(code: string): Promise<Result<Currency>> {
    try {
        const currency = new Currency(code);
        return { success: true, value: currency };
    } catch (error) {
        return { success: false, error: error as Error };
    }
}
```

## Testing

### Jest/Mocha Test Structure
```javascript
import { beforeAll, describe, it, expect } from '@jest/globals';
import init, { Currency, Money, Date } from '../pkg/rfin_wasm.js';

describe('Money Operations', () => {
    beforeAll(async () => {
        await init();
    });

    describe('Creation', () => {
        it('should create money with valid currency', () => {
            const usd = new Currency('USD');
            const money = new Money(100, usd);

            expect(money.amount).toBe(100);
            expect(money.currency.code).toBe('USD');
        });

        it('should throw on invalid currency', () => {
            expect(() => new Currency('INVALID')).toThrow();
        });
    });

    describe('Arithmetic', () => {
        it('should add money with same currency', () => {
            const usd = new Currency('USD');
            const m1 = new Money(100, usd);
            const m2 = new Money(50, usd);

            const result = m1.add(m2);
            expect(result.amount).toBe(150);
        });

        it('should throw on currency mismatch', () => {
            const usd = new Currency('USD');
            const eur = new Currency('EUR');
            const m1 = new Money(100, usd);
            const m2 = new Money(50, eur);

            expect(() => m1.add(m2)).toThrow(/Currency mismatch/);
        });
    });
});
```

### Browser Testing
```html
<!DOCTYPE html>
<html>
<head>
    <title>rfin-wasm Tests</title>
    <script type="module">
        import init, { Currency, Money, Date } from './pkg/rfin_wasm.js';

        async function runTests() {
            console.log('Initializing WASM...');
            await init();

            console.log('Running tests...');

            // Test 1: Currency creation
            try {
                const usd = new Currency('USD');
                console.assert(usd.code === 'USD', 'Currency code should be USD');
                console.log('✓ Currency creation test passed');
            } catch (e) {
                console.error('✗ Currency creation test failed:', e);
            }

            // Test 2: Money arithmetic
            try {
                const usd = new Currency('USD');
                const m1 = new Money(100, usd);
                const m2 = new Money(50, usd);
                const sum = m1.add(m2);
                console.assert(sum.amount === 150, 'Sum should be 150');
                console.log('✓ Money arithmetic test passed');
            } catch (e) {
                console.error('✗ Money arithmetic test failed:', e);
            }
        }

        runTests().catch(console.error);
    </script>
</head>
<body>
    <h1>rfin-wasm Browser Tests</h1>
    <p>Check the console for test results.</p>
</body>
</html>
```

## Performance Optimization

### Batch Operations
```javascript
// Good: Process multiple items efficiently
function calculatePortfolioValue(positions) {
    const byurrency = new Map();

    // Group by currency first
    for (const position of positions) {
        const key = position.currency.code;
        if (!byCurrency.has(key)) {
            byCurrency.set(key, []);
        }
        byCurrency.get(key).push(position);
    }

    // Sum within each currency
    const totals = new Map();
    for (const [currencyCode, amounts] of byCurrency) {
        let total = amounts[0];
        for (let i = 1; i < amounts.length; i++) {
            total = total.add(amounts[i]);
        }
        totals.set(currencyCode, total);
    }

    return totals;
}
```

### Caching Instances
```javascript
// Cache frequently used objects
class CurrencyCache {
    constructor() {
        this.cache = new Map();
    }

    get(code) {
        if (!this.cache.has(code)) {
            this.cache.set(code, new Currency(code));
        }
        return this.cache.get(code);
    }

    clear() {
        this.cache.clear();
    }
}

const currencyCache = new CurrencyCache();

// Use cached currencies
const usd = currencyCache.get('USD');
const eur = currencyCache.get('EUR');
```

## Framework Integration

### React Example
```jsx
import React, { useState, useEffect } from 'react';
import init, { Currency, Money } from './pkg/rfin_wasm.js';

function MoneyCalculator() {
    const [initialized, setInitialized] = useState(false);
    const [amount, setAmount] = useState('');
    const [currency, setCurrency] = useState('USD');
    const [result, setResult] = useState(null);

    useEffect(() => {
        init().then(() => setInitialized(true));
    }, []);

    const handleCalculate = () => {
        if (!initialized) return;

        try {
            const curr = new Currency(currency);
            const money = new Money(parseFloat(amount), curr);
            setResult(money);
        } catch (error) {
            console.error('Calculation failed:', error);
        }
    };

    if (!initialized) {
        return <div>Loading WASM module...</div>;
    }

    return (
        <div>
            <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="Amount"
            />
            <select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                <option value="USD">USD</option>
                <option value="EUR">EUR</option>
                <option value="GBP">GBP</option>
            </select>
            <button onClick={handleCalculate}>Create Money</button>
            {result && (
                <div>
                    Result: {result.amount} {result.currency.code}
                </div>
            )}
        </div>
    );
}
```

### Vue.js Example
```vue
<template>
  <div>
    <h2>Currency Converter</h2>
    <input v-model.number="amount" type="number" placeholder="Amount">
    <select v-model="fromCurrency">
      <option v-for="curr in currencies" :key="curr" :value="curr">
        {{ curr }}
      </option>
    </select>
    <button @click="convert" :disabled="!initialized">
      Convert
    </button>
    <div v-if="result">
      Result: {{ result.amount }} {{ result.currency.code }}
    </div>
  </div>
</template>

<script>
import init, { Currency, Money } from './pkg/rfin_wasm.js';

export default {
  data() {
    return {
      initialized: false,
      amount: 100,
      fromCurrency: 'USD',
      currencies: ['USD', 'EUR', 'GBP', 'JPY'],
      result: null
    };
  },
  async mounted() {
    await init();
    this.initialized = true;
  },
  methods: {
    convert() {
      try {
        const currency = new Currency(this.fromCurrency);
        const money = new Money(this.amount, currency);
        this.result = money;
      } catch (error) {
        console.error('Conversion failed:', error);
      }
    }
  }
};
</script>
```

## Documentation

### JSDoc Comments
```javascript
/**
 * Calculate the present value of a bond.
 *
 * @param {Object} params - Bond parameters
 * @param {number} params.faceValue - Face value of the bond
 * @param {number} params.couponRate - Annual coupon rate (e.g., 0.05 for 5%)
 * @param {Date} params.issueDate - Issue date of the bond
 * @param {Date} params.maturityDate - Maturity date of the bond
 * @param {Currency} params.currency - Currency of the bond
 * @param {Frequency} [params.frequency=Frequency.SemiAnnual] - Payment frequency
 * @param {DayCount} [params.dayCount=DayCount.Thirty360()] - Day count convention
 * @returns {Promise<{npv: number, flows: Array}>} Present value and cash flows
 * @throws {Error} If initialization fails or parameters are invalid
 *
 * @example
 * const bond = await calculateBondPV({
 *   faceValue: 1000000,
 *   couponRate: 0.05,
 *   issueDate: new Date(2023, 1, 1),
 *   maturityDate: new Date(2028, 1, 1),
 *   currency: new Currency('USD')
 * });
 * console.log(`NPV: ${bond.npv}`);
 */
async function calculateBondPV(params) {
    // Implementation
}
```

## Common Patterns

### Builder Pattern
```javascript
class BondBuilder {
    constructor() {
        this.params = {
            frequency: Frequency.SemiAnnual,
            dayCount: DayCount.Thirty360()
        };
    }

    withNotional(amount) {
        this.params.notional = amount;
        return this;
    }

    withCurrency(currencyCode) {
        this.params.currency = new Currency(currencyCode);
        return this;
    }

    withCouponRate(rate) {
        this.params.couponRate = rate;
        return this;
    }

    withDates(issueDate, maturityDate) {
        this.params.issueDate = issueDate;
        this.params.maturityDate = maturityDate;
        return this;
    }

    withFrequency(frequency) {
        this.params.frequency = frequency;
        return this;
    }

    withDayCount(dayCount) {
        this.params.dayCount = dayCount;
        return this;
    }

    build() {
        const { notional, currency, couponRate, issueDate, maturityDate, frequency, dayCount } = this.params;

        if (!notional || !currency || !couponRate || !issueDate || !maturityDate) {
            throw new Error('Missing required parameters');
        }

        return new FixedRateLeg(
            notional,
            currency,
            couponRate,
            issueDate,
            maturityDate,
            frequency,
            dayCount
        );
    }
}

// Usage
const bond = new BondBuilder()
    .withNotional(1000000)
    .withCurrency('USD')
    .withCouponRate(0.05)
    .withDates(new Date(2023, 1, 1), new Date(2028, 1, 1))
    .build();
```

### Factory Functions
```javascript
// Currency factory with validation
function createCurrency(code) {
    const validCodes = ['USD', 'EUR', 'GBP', 'JPY', 'CHF'];

    if (!validCodes.includes(code)) {
        throw new Error(`Unsupported currency code: ${code}`);
    }

    return new Currency(code);
}

// Money factory with formatting
function createMoney(amount, currencyCode, options = {}) {
    const { roundTo = 2 } = options;

    const currency = createCurrency(currencyCode);
    const roundedAmount = Math.round(amount * Math.pow(10, roundTo)) / Math.pow(10, roundTo);

    return new Money(roundedAmount, currency);
}
``` # JavaScript/TypeScript Usage Standards for rfin-wasm

## Overview

This document covers standards for JavaScript and TypeScript code that uses the rfin-wasm module, including:
- Web applications
- Node.js applications
- Example code
- Test files
- Documentation examples

## Setup and Initialization

### Browser Setup
```javascript
// ES Modules
import init, { Currency, Money, Date } from './pkg/rfin_wasm.js';

async function initialize() {
    // Always await initialization before using any types
    await init();

    // Now you can use the library
    const usd = new Currency("USD");
    const amount = new Money(100.0, usd);
}

// Call initialization
initialize().catch(console.error);
```

### Node.js Setup
```javascript
// CommonJS
const { Currency, Money, Date } = require('./pkg-node/rfin_wasm.js');

// No initialization needed for Node.js builds
const usd = new Currency("USD");
```

### TypeScript Setup
```typescript
// types.d.ts or inline
import type { Currency, Money, Date, DayCount } from './pkg/rfin_wasm';

// With proper typing
async function createMoney(amount: number, currencyCode: string): Promise<Money> {
    await init();
    const currency = new Currency(currencyCode);
    return new Money(amount, currency);
}
```

## Import Patterns

### Recommended Import Structure
```javascript
// Import init function and types
import init, {
    // Core types
    Currency,
    Money,
    Date,

    // Enums
    Frequency,
    DayCount,
    BusDayConvention,

    // Complex types
    Calendar,
    FixedRateLeg,

    // Functions (with camelCase names)
    generateSchedule,
    thirdWednesday,
    nextImm,
    nextCdsDate
} from './pkg/rfin_wasm.js';
```

### Avoid Global Scope Pollution
```javascript
// Good: Import only what you need
import { Currency, Money } from './pkg/rfin_wasm.js';

// Avoid: Importing everything into global scope
import * as rfin from './pkg/rfin_wasm.js';
window.rfin = rfin; // Don't do this
```

## Type Construction

### Currency Creation
```javascript
// Good: Direct construction
const usd = new Currency("USD");
const eur = new Currency("EUR");

// Handle errors appropriately
try {
    const invalid = new Currency("XXX");
} catch (error) {
    console.error("Invalid currency code:", error);
}

// Access properties
console.log(usd.code);         // "USD"
console.log(usd.numericCode);  // 840
console.log(usd.decimals);     // 2
```

### Money Creation
```javascript
// Create money instances
const amount = new Money(1000.50, usd);

// Access properties
console.log(amount.amount);    // 1000.5
console.log(amount.currency);  // Currency instance

// Arithmetic operations
try {
    const sum = amount1.add(amount2);
    const difference = amount1.subtract(amount2);
    const scaled = amount.multiply(1.1);
    const divided = amount.divide(2);
} catch (error) {
    // Handle currency mismatch or other errors
    console.error("Operation failed:", error);
}
```

### Date Creation
```javascript
// Create dates (month is 1-based)
const date = new Date(2023, 12, 25); // December 25, 2023

// Access properties
console.log(date.year);        // 2023
console.log(date.month);       // 12
console.log(date.day);         // 25

// Date operations
const isWeekend = date.isWeekend();
const quarter = date.quarter();
const nextBusinessDay = date.addBusinessDays(1);
```

## Error Handling

### Try-Catch Pattern
```javascript
async function safeCurrencyOperation() {
    try {
        await init();
        const currency = new Currency("USD");
        return currency;
    } catch (error) {
        console.error("Failed to create currency:", error);
        // Provide fallback or re-throw
        throw new Error(`Currency initialization failed: ${error.message}`);
    }
}
```

### Validation Before Operations
```javascript
function addMoney(amount1, amount2) {
    // Validate inputs
    if (!(amount1 instanceof Money) || !(amount2 instanceof Money)) {
        throw new TypeError("Both arguments must be Money instances");
    }

    // Check currency compatibility
    if (!amount1.currency.equals(amount2.currency)) {
        throw new Error(
            `Cannot add ${amount1.currency.code} and ${amount2.currency.code}`
        );
    }

    return amount1.add(amount2);
}
```

## Enum Usage

### Frequency Enum
```javascript
import { Frequency } from './pkg/rfin_wasm.js';

// Use enum values
const schedule = generateSchedule(
    startDate,
    endDate,
    Frequency.SemiAnnual  // Enum value
);

// In configurations
const bondConfig = {
    frequency: Frequency.Quarterly,
    dayCount: DayCount.Act360(),
    convention: BusDayConvention.ModifiedFollowing
};
```

### DayCount Static Methods
```javascript
import { DayCount } from './pkg/rfin_wasm.js';

// Create day count conventions
const act360 = DayCount.Act360();
const act365f = DayCount.Act365F();
const thirty360 = DayCount.Thirty360();

// Use in calculations
const yearFraction = act360.yearFraction(startDate, endDate);
const days = act360.days(startDate, endDate);
```

## Complex Operations

### Schedule Generation
```javascript
async function generatePaymentSchedule(config) {
    await init();

    const {
        startDate,
        endDate,
        frequency,
        calendar,
        convention
    } = config;

    // Generate raw schedule
    const dates = generateSchedule(startDate, endDate, frequency);

    // Adjust for business days if needed
    if (calendar && convention) {
        return dates.map(date =>
            calendar.adjust(date, convention)
        );
    }

    return dates;
}
```

### Fixed Rate Leg Creation
```javascript
async function createBond(params) {
    await init();

    const {
        notional,
        currencyCode,
        couponRate,
        issueDate,
        maturityDate,
        frequency = Frequency.SemiAnnual,
        dayCount = DayCount.Thirty360()
    } = params;

    const currency = new Currency(currencyCode);
    const leg = new FixedRateLeg(
        notional,
        currency,
        couponRate,
        issueDate,
        maturityDate,
        frequency,
        dayCount
    );

    return {
        leg,
        numFlows: leg.numFlows,
        npv: leg.npv(),
        flows: leg.getCashFlows()
    };
}
```

## Memory Management

### Proper Cleanup
```javascript
// WASM objects are automatically garbage collected
// But avoid holding unnecessary references

class PortfolioManager {
    constructor() {
        this.positions = new Map();
    }

    addPosition(id, money) {
        this.positions.set(id, money);
    }

    removePosition(id) {
        // Clear reference to allow GC
        this.positions.delete(id);
    }

    clear() {
        // Clear all references
        this.positions.clear();
    }
}
```

### Avoid Memory Leaks
```javascript
// Bad: Creating objects in loops without need
function calculateTotal(amounts) {
    let total = new Money(0, new Currency("USD"));
    for (const amount of amounts) {
        total = total.add(new Money(amount, new Currency("USD"))); // Creates many temporary objects
    }
    return total;
}

// Good: Reuse objects where possible
function calculateTotalEfficient(amounts, currency) {
    let total = new Money(0, currency);
    for (const amount of amounts) {
        const temp = new Money(amount, currency);
        total = total.add(temp);
    }
    return total;
}
```

## TypeScript Best Practices

### Type Definitions
```typescript
// Define interfaces for your domain objects
interface BondParameters {
    notional: number;
    currency: Currency;
    couponRate: number;
    issueDate: Date;
    maturityDate: Date;
    frequency?: Frequency;
    dayCount?: DayCount;
}

interface CashFlow {
    date: Date;
    amount: number;
    currency: string;
    type: string;
}
```

### Generic Functions
```typescript
// Type-safe wrappers
function createMoney<T extends number>(
    amount: T,
    currency: Currency
): Money {
    return new Money(amount, currency);
}

// Result types
type Result<T, E = Error> =
    | { success: true; value: T }
    | { success: false; error: E };

async function safeCurrencyCreation(code: string): Promise<Result<Currency>> {
    try {
        const currency = new Currency(code);
        return { success: true, value: currency };
    } catch (error) {
        return { success: false, error: error as Error };
    }
}
```

## Testing

### Jest/Mocha Test Structure
```javascript
import { beforeAll, describe, it, expect } from '@jest/globals';
import init, { Currency, Money, Date } from '../pkg/rfin_wasm.js';

describe('Money Operations', () => {
    beforeAll(async () => {
        await init();
    });

    describe('Creation', () => {
        it('should create money with valid currency', () => {
            const usd = new Currency('USD');
            const money = new Money(100, usd);

            expect(money.amount).toBe(100);
            expect(money.currency.code).toBe('USD');
        });

        it('should throw on invalid currency', () => {
            expect(() => new Currency('INVALID')).toThrow();
        });
    });

    describe('Arithmetic', () => {
        it('should add money with same currency', () => {
            const usd = new Currency('USD');
            const m1 = new Money(100, usd);
            const m2 = new Money(50, usd);

            const result = m1.add(m2);
            expect(result.amount).toBe(150);
        });

        it('should throw on currency mismatch', () => {
            const usd = new Currency('USD');
            const eur = new Currency('EUR');
            const m1 = new Money(100, usd);
            const m2 = new Money(50, eur);

            expect(() => m1.add(m2)).toThrow(/Currency mismatch/);
        });
    });
});
```

### Browser Testing
```html
<!DOCTYPE html>
<html>
<head>
    <title>rfin-wasm Tests</title>
    <script type="module">
        import init, { Currency, Money, Date } from './pkg/rfin_wasm.js';

        async function runTests() {
            console.log('Initializing WASM...');
            await init();

            console.log('Running tests...');

            // Test 1: Currency creation
            try {
                const usd = new Currency('USD');
                console.assert(usd.code === 'USD', 'Currency code should be USD');
                console.log('✓ Currency creation test passed');
            } catch (e) {
                console.error('✗ Currency creation test failed:', e);
            }

            // Test 2: Money arithmetic
            try {
                const usd = new Currency('USD');
                const m1 = new Money(100, usd);
                const m2 = new Money(50, usd);
                const sum = m1.add(m2);
                console.assert(sum.amount === 150, 'Sum should be 150');
                console.log('✓ Money arithmetic test passed');
            } catch (e) {
                console.error('✗ Money arithmetic test failed:', e);
            }
        }

        runTests().catch(console.error);
    </script>
</head>
<body>
    <h1>rfin-wasm Browser Tests</h1>
    <p>Check the console for test results.</p>
</body>
</html>
```

## Performance Optimization

### Batch Operations
```javascript
// Good: Process multiple items efficiently
function calculatePortfolioValue(positions) {
    const byurrency = new Map();

    // Group by currency first
    for (const position of positions) {
        const key = position.currency.code;
        if (!byCurrency.has(key)) {
            byCurrency.set(key, []);
        }
        byCurrency.get(key).push(position);
    }

    // Sum within each currency
    const totals = new Map();
    for (const [currencyCode, amounts] of byCurrency) {
        let total = amounts[0];
        for (let i = 1; i < amounts.length; i++) {
            total = total.add(amounts[i]);
        }
        totals.set(currencyCode, total);
    }

    return totals;
}
```

### Caching Instances
```javascript
// Cache frequently used objects
class CurrencyCache {
    constructor() {
        this.cache = new Map();
    }

    get(code) {
        if (!this.cache.has(code)) {
            this.cache.set(code, new Currency(code));
        }
        return this.cache.get(code);
    }

    clear() {
        this.cache.clear();
    }
}

const currencyCache = new CurrencyCache();

// Use cached currencies
const usd = currencyCache.get('USD');
const eur = currencyCache.get('EUR');
```

## Framework Integration

### React Example
```jsx
import React, { useState, useEffect } from 'react';
import init, { Currency, Money } from './pkg/rfin_wasm.js';

function MoneyCalculator() {
    const [initialized, setInitialized] = useState(false);
    const [amount, setAmount] = useState('');
    const [currency, setCurrency] = useState('USD');
    const [result, setResult] = useState(null);

    useEffect(() => {
        init().then(() => setInitialized(true));
    }, []);

    const handleCalculate = () => {
        if (!initialized) return;

        try {
            const curr = new Currency(currency);
            const money = new Money(parseFloat(amount), curr);
            setResult(money);
        } catch (error) {
            console.error('Calculation failed:', error);
        }
    };

    if (!initialized) {
        return <div>Loading WASM module...</div>;
    }

    return (
        <div>
            <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="Amount"
            />
            <select value={currency} onChange={(e) => setCurrency(e.target.value)}>
                <option value="USD">USD</option>
                <option value="EUR">EUR</option>
                <option value="GBP">GBP</option>
            </select>
            <button onClick={handleCalculate}>Create Money</button>
            {result && (
                <div>
                    Result: {result.amount} {result.currency.code}
                </div>
            )}
        </div>
    );
}
```

### Vue.js Example
```vue
<template>
  <div>
    <h2>Currency Converter</h2>
    <input v-model.number="amount" type="number" placeholder="Amount">
    <select v-model="fromCurrency">
      <option v-for="curr in currencies" :key="curr" :value="curr">
        {{ curr }}
      </option>
    </select>
    <button @click="convert" :disabled="!initialized">
      Convert
    </button>
    <div v-if="result">
      Result: {{ result.amount }} {{ result.currency.code }}
    </div>
  </div>
</template>

<script>
import init, { Currency, Money } from './pkg/rfin_wasm.js';

export default {
  data() {
    return {
      initialized: false,
      amount: 100,
      fromCurrency: 'USD',
      currencies: ['USD', 'EUR', 'GBP', 'JPY'],
      result: null
    };
  },
  async mounted() {
    await init();
    this.initialized = true;
  },
  methods: {
    convert() {
      try {
        const currency = new Currency(this.fromCurrency);
        const money = new Money(this.amount, currency);
        this.result = money;
      } catch (error) {
        console.error('Conversion failed:', error);
      }
    }
  }
};
</script>
```

## Documentation

### JSDoc Comments
```javascript
/**
 * Calculate the present value of a bond.
 *
 * @param {Object} params - Bond parameters
 * @param {number} params.faceValue - Face value of the bond
 * @param {number} params.couponRate - Annual coupon rate (e.g., 0.05 for 5%)
 * @param {Date} params.issueDate - Issue date of the bond
 * @param {Date} params.maturityDate - Maturity date of the bond
 * @param {Currency} params.currency - Currency of the bond
 * @param {Frequency} [params.frequency=Frequency.SemiAnnual] - Payment frequency
 * @param {DayCount} [params.dayCount=DayCount.Thirty360()] - Day count convention
 * @returns {Promise<{npv: number, flows: Array}>} Present value and cash flows
 * @throws {Error} If initialization fails or parameters are invalid
 *
 * @example
 * const bond = await calculateBondPV({
 *   faceValue: 1000000,
 *   couponRate: 0.05,
 *   issueDate: new Date(2023, 1, 1),
 *   maturityDate: new Date(2028, 1, 1),
 *   currency: new Currency('USD')
 * });
 * console.log(`NPV: ${bond.npv}`);
 */
async function calculateBondPV(params) {
    // Implementation
}
```

## Common Patterns

### Builder Pattern
```javascript
class BondBuilder {
    constructor() {
        this.params = {
            frequency: Frequency.SemiAnnual,
            dayCount: DayCount.Thirty360()
        };
    }

    withNotional(amount) {
        this.params.notional = amount;
        return this;
    }

    withCurrency(currencyCode) {
        this.params.currency = new Currency(currencyCode);
        return this;
    }

    withCouponRate(rate) {
        this.params.couponRate = rate;
        return this;
    }

    withDates(issueDate, maturityDate) {
        this.params.issueDate = issueDate;
        this.params.maturityDate = maturityDate;
        return this;
    }

    withFrequency(frequency) {
        this.params.frequency = frequency;
        return this;
    }

    withDayCount(dayCount) {
        this.params.dayCount = dayCount;
        return this;
    }

    build() {
        const { notional, currency, couponRate, issueDate, maturityDate, frequency, dayCount } = this.params;

        if (!notional || !currency || !couponRate || !issueDate || !maturityDate) {
            throw new Error('Missing required parameters');
        }

        return new FixedRateLeg(
            notional,
            currency,
            couponRate,
            issueDate,
            maturityDate,
            frequency,
            dayCount
        );
    }
}

// Usage
const bond = new BondBuilder()
    .withNotional(1000000)
    .withCurrency('USD')
    .withCouponRate(0.05)
    .withDates(new Date(2023, 1, 1), new Date(2028, 1, 1))
    .build();
```

### Factory Functions
```javascript
// Currency factory with validation
function createCurrency(code) {
    const validCodes = ['USD', 'EUR', 'GBP', 'JPY', 'CHF'];

    if (!validCodes.includes(code)) {
        throw new Error(`Unsupported currency code: ${code}`);
    }

    return new Currency(code);
}

// Money factory with formatting
function createMoney(amount, currencyCode, options = {}) {
    const { roundTo = 2 } = options;

    const currency = createCurrency(currencyCode);
    const roundedAmount = Math.round(amount * Math.pow(10, roundTo)) / Math.pow(10, roundTo);

    return new Money(roundedAmount, currency);
}
```
