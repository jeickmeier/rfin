# Finstack Examples Index

This document provides an index to examples across both Python and TypeScript/WASM bindings.

## Quick Links

- **Python Examples:** [`finstack-py/examples/scripts/`](finstack-py/examples/scripts/)
- **TypeScript Examples:** [`finstack-wasm/examples/src/`](finstack-wasm/examples/src/)
- **Side-by-Side Comparison:** [`book/src/bindings/examples.md`](book/src/bindings/examples.md)

## Example Parity Matrix

| Topic | Python Example | TypeScript Example | Side-by-Side Doc |
|-------|----------------|-------------------|------------------|
| **Core Basics** | [`core_basics.py`](finstack-py/examples/scripts/core/core_basics.py) | [`CoreBasics.tsx`](finstack-wasm/examples/src/demos/CoreBasics.tsx) | [✓](book/src/bindings/examples.md#example-1-basic-bond-pricing) |
| **Bond Pricing** | [`bond_capabilities.py`](finstack-py/examples/scripts/valuations/bond_capabilities.py) | [`BondDemo.tsx`](finstack-wasm/examples/src/demos/BondDemo.tsx) | [✓](book/src/bindings/examples.md#example-1-basic-bond-pricing) |
| **Curve Calibration** | [`calibration_capabilities.py`](finstack-py/examples/scripts/valuations/calibration_capabilities.py) | [`CalibrationDemo.tsx`](finstack-wasm/examples/src/demos/CalibrationDemo.tsx) | [✓](book/src/bindings/examples.md#example-2-curve-calibration) |
| **IRS Valuation** | [`irs_capabilities.py`](finstack-py/examples/scripts/valuations/irs_capabilities.py) | [`SwapDemo.tsx`](finstack-wasm/examples/src/demos/SwapDemo.tsx) | ✓ |
| **Credit Derivatives** | [`credit_capabilities.py`](finstack-py/examples/scripts/valuations/credit_capabilities.py) | [`CreditDemo.tsx`](finstack-wasm/examples/src/demos/CreditDemo.tsx) | ✓ |
| **Equity Options** | [`equity_capabilities.py`](finstack-py/examples/scripts/valuations/equity_capabilities.py) | [`EquityOptionsDemo.tsx`](finstack-wasm/examples/src/demos/EquityOptionsDemo.tsx) | ✓ |
| **FX Options** | [`fx_capabilities.py`](finstack-py/examples/scripts/valuations/fx_capabilities.py) | [`FxDemo.tsx`](finstack-wasm/examples/src/demos/FxDemo.tsx) | ✓ |
| **Statements** | [`statements_example.py`](finstack-py/examples/scripts/statements/statements_example.py) | [`StatementsDemo.tsx`](finstack-wasm/examples/src/demos/StatementsDemo.tsx) | [✓](book/src/bindings/examples.md#example-3-statement-modeling) |
| **Scenarios** | [`scenarios_example.py`](finstack-py/examples/scripts/scenarios/scenarios_example.py) | [`ScenariosDemo.tsx`](finstack-wasm/examples/src/demos/ScenariosDemo.tsx) | [✓](book/src/bindings/examples.md#example-4-scenario-analysis) |
| **Portfolio** | [`portfolio_example.py`](finstack-py/examples/scripts/portfolio/portfolio_example.py) | [`PortfolioDemo.tsx`](finstack-wasm/examples/src/demos/PortfolioDemo.tsx) | [✓](book/src/bindings/examples.md#example-5-portfolio-aggregation) |
| **Math/Integration** | [`math_core_showcase.py`](finstack-py/examples/scripts/core/math_core_showcase.py) | [`MathDemo.tsx`](finstack-wasm/examples/src/demos/MathDemo.tsx) | ✓ |
| **Cashflow Builder** | [`cashflow_builder_capabilities.py`](finstack-py/examples/scripts/valuations/cashflow_builder_capabilities.py) | [`CashflowBuilderDemo.tsx`](finstack-wasm/examples/src/demos/CashflowBuilderDemo.tsx) | ✓ |

## Python Examples

### Core Examples
- **`core_basics.py`**: Currency, money, dates, calendars, day count conventions
- **`cashflow_basics.py`**: Cashflow modeling and scheduling
- **`math_core_showcase.py`**: Mathematical utilities, solvers, integration

### Valuations Examples
- **`bond_capabilities.py`**: Treasury, corporate, zero-coupon bonds with pricing
- **`equity_capabilities.py`**: Equity options, variance swaps, exotic options
- **`credit_capabilities.py`**: CDS, CDS index, tranches, credit options
- **`fx_capabilities.py`**: FX spots, options, swaps, barriers
- **`irs_capabilities.py`**: Interest rate swaps, caps/floors, swaptions
- **`inflation_capabilities.py`**: Inflation-linked bonds and swaps
- **`structured_credit_capabilities.py`**: ABS, CLO, CMBS, RMBS
- **`private_markets_capabilities.py`**: Private equity and credit funds
- **`calibration_capabilities.py`**: Multi-curve calibration workflows
- **`cashflow_builder_capabilities.py`**: Advanced cashflow construction

### Statements Examples
- **`statements_example.py`**: Financial statement modeling with forecasting

### Scenarios Examples
- **`scenarios_example.py`**: Scenario analysis and stress testing

### Portfolio Examples
- **`portfolio_example.py`**: Portfolio construction and aggregation

### Running Python Examples
```bash
# Run all examples
uv run python finstack-py/examples/scripts/run_all_examples.py

# Run specific example
uv run python finstack-py/examples/scripts/core/core_basics.py
```

## TypeScript/WASM Examples

### Available Demos
The TypeScript examples are in a React + Vite application with interactive demos for:

- **CoreBasics**: Dates, currencies, money, market data
- **BondDemo**: Bond construction and pricing
- **CalibrationDemo**: Curve and surface calibration
- **SwapDemo**: Interest rate swaps and swaptions
- **CreditDemo**: CDS and credit derivatives
- **EquityOptionsDemo**: Equity options and Greeks
- **FxDemo**: FX options and barriers
- **StatementsDemo**: Financial statement modeling
- **ScenariosDemo**: Scenario analysis
- **PortfolioDemo**: Portfolio management
- **MathDemo**: Mathematical utilities
- **CashflowBuilderDemo**: Advanced cashflow building

### Running TypeScript Examples
```bash
# Build WASM package
cd finstack-wasm
npm run build

# Install example dependencies
npm run examples:install

# Run development server
npm run examples:dev

# Open browser to http://localhost:5173
```

## Side-by-Side Comparison

The [`book/src/bindings/examples.md`](book/src/bindings/examples.md) document provides side-by-side code examples showing the same workflow in both Python and TypeScript.

**Topics covered:**
1. Basic bond pricing
2. Curve calibration
3. Statement modeling with forecasting
4. Scenario analysis
5. Portfolio aggregation

## Example Workflow Equivalence

### Bond Pricing Workflow
```python
# Python
from finstack.valuations.instruments import Bond
bond = Bond.treasury("US-10Y", 1_000_000, "USD", 0.0375, maturity, issue)
result = pricer.price(bond, market)
```

```typescript
// TypeScript
import { Bond } from 'finstack-wasm';
const bond = Bond.treasury("US-10Y", 1_000_000, "USD", 0.0375, maturity, issue);
const result = pricer.price(bond, market);
```

### Calibration Workflow
```python
# Python
calibrator = DiscountCurveCalibrator("USD-OIS", date, "USD")
curve, report = calibrator.calibrate(quotes, market)
```

```typescript
// TypeScript
const calibrator = new DiscountCurveCalibrator("USD-OIS", date, "USD");
const [curve, report] = calibrator.calibrate(quotes, market);
```

### Statement Modeling Workflow
```python
# Python
builder = ModelBuilder.new("Model")
builder.periods("2024Q1..Q4", "2024Q2")
builder.value("revenue", values)
builder.compute("ebitda", "revenue - opex")
model = builder.build()
```

```typescript
// TypeScript
const builder = ModelBuilder.new("Model");
builder.periods("2024Q1..Q4", "2024Q2");
builder.value("revenue", values);
builder.compute("ebitda", "revenue - opex");
const model = builder.build();
```

## Documentation References

- **API Reference:** [`book/src/bindings/api-reference.md`](book/src/bindings/api-reference.md)
- **Migration Guide:** [`book/src/bindings/migration-guide.md`](book/src/bindings/migration-guide.md)
- **Naming Conventions:** [`NAMING_CONVENTIONS.md`](NAMING_CONVENTIONS.md)
- **Parity Audit:** [`PARITY_AUDIT.md`](PARITY_AUDIT.md)

## Contributing Examples

To add a new example:

1. **Python:** Create in `finstack-py/examples/scripts/` with descriptive name
2. **TypeScript:** Create in `finstack-wasm/examples/src/demos/` as React component
3. **Update this index** with both examples
4. **Add to side-by-side doc** in `book/src/bindings/examples.md` if appropriate
5. **Run parity check:** Verify both examples work with same inputs

## Testing Examples

Both Python and TypeScript examples are tested in CI:

- Python: Ran via `run_all_examples.py` in CI
- TypeScript: Built and verified in CI

See `.github/workflows/bindings-parity.yml` for automated checks.

---

**Last Updated:** Auto-generated during parity implementation  
**Maintainers:** Finstack bindings team

