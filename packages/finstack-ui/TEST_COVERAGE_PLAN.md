# Test Coverage Improvement Plan

## Current Status
- **Current Coverage**: 32.12%
- **Target Coverage**: >75%
- **Gap**: ~43% increase needed

## Coverage Analysis by Module

### 1. Primitives (59.28% → Target: 100%)
**Priority: HIGH**

#### CurrencySelect.tsx (0% coverage)
- [ ] Test basic rendering with default currencies
- [ ] Test custom currencies prop
- [ ] Test value selection and onChange callback
- [ ] Test disabled state
- [ ] Test className prop application

#### DatePicker.tsx (0% coverage)
- [ ] Test basic rendering
- [ ] Test value and onChange handling
- [ ] Test min/max date constraints
- [ ] Test disabled state
- [ ] Test custom placeholder
- [ ] Test className prop

**Already Covered:**
- AmountDisplay.tsx: 100% ✓
- AmountInput.tsx: 97.22% ✓ (minor edge case on line 39)
- TenorInput.tsx: 100% ✓

### 2. UI Components (9.41% → Target: 100%)
**Priority: HIGH**

#### button.tsx (0% coverage)
- [ ] Test all variants (default, destructive, outline, secondary, ghost, link)
- [ ] Test all sizes (default, sm, lg, icon)
- [ ] Test asChild prop (Slot component)
- [ ] Test className merging
- [ ] Test disabled state
- [ ] Test click handlers
- [ ] Test ref forwarding

#### select.tsx (0% coverage)
- [ ] Test Select root component
- [ ] Test SelectTrigger rendering and ref forwarding
- [ ] Test SelectContent with different positions
- [ ] Test SelectItem selection
- [ ] Test SelectLabel rendering
- [ ] Test SelectSeparator rendering
- [ ] Test SelectScrollUpButton and SelectScrollDownButton
- [ ] Test SelectGroup functionality
- [ ] Test disabled states
- [ ] Test className merging

**Already Covered:**
- input.tsx: 100% ✓

### 3. Hooks (79.35% → Target: 90%+)
**Priority: MEDIUM**

#### useFinstack.tsx (76.74% coverage)
**Missing lines: 165-166, 193-194**
- [ ] Test Suspense mode (suspense prop)
- [ ] Test error when used outside provider (line 193-194)
- [ ] Test autoInit=false behavior
- [ ] Test setMarket function
- [ ] Test error handling during initialization
- [ ] Test cancellation during initialization
- [ ] Test config/market JSON parsing edge cases

#### useFinstackEngine.ts (92.3% coverage)
**Missing lines: 30-31**
- [ ] Test edge cases around lines 30-31

### 4. Workers (2.22% → Target: 80%+)
**Priority: CRITICAL** (largest impact on overall coverage)

#### finstackEngine.ts (0% coverage)
- [ ] Test initialize() with config and market JSON
- [ ] Test initialize() with null/undefined inputs
- [ ] Test loadMarket() function
- [ ] Test priceInstrument() with valid Bond JSON
- [ ] Test priceInstrument() error handling
- [ ] Test extractRounding() function with various config formats
- [ ] Test parseJsonSafe() function
- [ ] Test ensureWorkerWasmInit() function
- [ ] Test market context creation (fromJson, fromJSON, new MarketContext)
- [ ] Test instrument hydration (Bond fromJson)
- [ ] Test registry creation (createStandardRegistry vs PricerRegistry)
- [ ] Test rounding context extraction and application
- [ ] Test error normalization in priceInstrument

#### pool.ts (25% coverage)
**Missing lines: 7-12, 15-19, 22-25**
- [ ] Test createWorker() function
- [ ] Test getEngineWorker() caching behavior
- [ ] Test resetEngineWorker() function
- [ ] Test worker termination

### 5. Utils (80% → Target: 95%+)
**Priority: MEDIUM**

#### amount.ts (90.38% coverage)
**Missing lines: 16-17, 25-26, 50**
- [ ] Test roundAmountString() with invalid scale (line 16-17)
- [ ] Test roundAmountString() with empty input (line 25-26)
- [ ] Test normalizeAmountInput() edge cases (line 50)

#### errors.ts (11.11% coverage)
**Missing lines: 8-17**
- [ ] Test normalizeError() with Error instance
- [ ] Test normalizeError() with string error
- [ ] Test normalizeError() with unknown error type
- [ ] Test error stack and cause preservation

**Already Covered:**
- cn.ts: 100% ✓

### 6. Lib (11.76% → Target: 80%+)
**Priority: MEDIUM**

#### wasmSingleton.ts (11.76% coverage)
**Missing lines: 3-19, 23, 26-38, 41-42**
- [ ] Test canInitWasm() in browser environment
- [ ] Test canInitWasm() in non-browser environment
- [ ] Test ensureWasmInit() success path
- [ ] Test ensureWasmInit() caching behavior
- [ ] Test getInitFn() with default export
- [ ] Test getInitFn() with init export
- [ ] Test getInitFn() error when no init found
- [ ] Test __resetWasmInitForTests() helper

### 7. Store (0% coverage)
**Priority: LOW** (just type definitions)

#### index.ts (0% coverage)
- [ ] Test type exports (if runtime behavior exists)

### 8. Types (0% coverage)
**Priority: LOW** (just type definitions)

#### rounding.ts (0% coverage)
- [ ] Test type exports (if runtime behavior exists)

### 9. Main index.ts (0% coverage)
**Priority: LOW** (just re-exports)

## Implementation Strategy

### Phase 1: High-Impact Components (Target: +25% coverage)
1. **Workers** (finstackEngine.ts, pool.ts) - Estimated +15%
2. **Primitives** (CurrencySelect, DatePicker) - Estimated +5%
3. **UI Components** (button, select) - Estimated +5%

### Phase 2: Medium-Impact Improvements (Target: +15% coverage)
1. **Hooks** (useFinstack edge cases) - Estimated +3%
2. **Utils** (amount edge cases, errors.ts) - Estimated +5%
3. **Lib** (wasmSingleton.ts) - Estimated +7%

### Phase 3: Polish (Target: +3% coverage)
1. Remaining edge cases
2. Error boundary testing
3. Integration tests

## Test File Structure

```
tests/
├── primitives.test.tsx (extend existing)
├── provider-and-engine.test.tsx (extend existing)
├── ui-components.test.tsx (new)
├── workers.test.ts (new)
├── utils.test.ts (new)
└── lib.test.ts (new)
```

## Estimated Coverage After Implementation

- **Primitives**: 100% (from 59.28%)
- **UI Components**: 100% (from 9.41%)
- **Hooks**: 90% (from 79.35%)
- **Workers**: 80% (from 2.22%)
- **Utils**: 95% (from 80%)
- **Lib**: 80% (from 11.76%)

**Projected Overall Coverage**: ~78-82%

## Notes

- Worker tests will require mocking WASM modules and Comlink
- UI component tests should use React Testing Library
- Focus on testing behavior, not implementation details
- Ensure tests are maintainable and don't break with refactoring
- Mock external dependencies appropriately (finstack-wasm, comlink, etc.)
