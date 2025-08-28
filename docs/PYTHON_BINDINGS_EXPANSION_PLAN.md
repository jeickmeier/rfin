# Python Bindings Expansion Plan for Finstack

## Executive Summary

This document outlines a structured plan to expand the Python bindings (`finstack-py`) to provide comprehensive coverage of the core and valuations crates, focusing on functionality required by credit analysts. The plan emphasizes creating a pythonic API that balances functionality with usability, without exposing unnecessary low-level implementation details.

## Current State Analysis

### Already Implemented
- **Core Module:**
  - ✅ Currency handling
  - ✅ Money type with currency-safe arithmetic  
  - ✅ Date handling with calendar support
  - ✅ Day count conventions
  - ✅ Basic schedule generation
  - ✅ Period generation
  - ✅ Basic FX support

- **Market Data Module:**
  - ✅ Discount, Forward, Hazard, and Inflation curves
  - ✅ Volatility surfaces
  - ✅ Interpolation methods
  - ✅ FX matrix and providers
  - ✅ Market context container
  - ✅ Inflation indices
  - ✅ Market scalars and time series

- **Cashflow Module:**
  - ✅ Fixed rate leg (basic)
  - ✅ Individual cashflow representation

### Major Gaps for Credit Analysts
1. **Instruments:** No bonds, loans, swaps, or other instruments
2. **Pricing:** No valuation engine or metrics
3. **Risk:** No sensitivity measures (DV01, duration, convexity, etc.)
4. **Advanced Cashflows:** No floating legs, amortization, or fees
5. **Credit-Specific:** No covenant modeling, PIK/toggle structures, or workout scenarios
6. **Analytics:** No yield calculations, spread measures, or performance metrics

## Prioritized Implementation Plan

### Phase 1: Core Instruments & Basic Pricing (High Priority)
**Timeline:** 2-3 weeks
**Goal:** Enable basic bond and loan pricing with standard metrics

#### 1.1 Bond Instrument
```python
# Target API
from finstack.instruments import Bond
from finstack.valuations import price_bond, calculate_metrics

bond = Bond(
    notional=Money(1_000_000, Currency.USD),
    coupon=0.05,
    frequency=Frequency.SemiAnnual,
    day_count=DayCount.Thirty360,
    issue_date=Date(2023, 1, 1),
    maturity=Date(2028, 1, 1),
    discount_curve="USD-OIS"
)

# Price with market context
result = price_bond(bond, market_context, as_of=Date(2024, 1, 1))
print(f"Clean Price: {result.clean_price}")
print(f"Dirty Price: {result.dirty_price}")
print(f"Accrued: {result.accrued}")
print(f"YTM: {result.ytm:.2%}")
print(f"Duration: {result.duration}")
print(f"Convexity: {result.convexity}")
```

**Implementation Tasks:**
- [ ] Create `PyBond` wrapper class
- [ ] Implement basic pricing function
- [ ] Add metric calculators (YTM, duration, convexity)
- [ ] Support clean/dirty price calculations
- [ ] Add accrued interest calculation

#### 1.2 Fixed-Rate Loan
```python
from finstack.instruments import Loan, AmortizationSchedule

loan = Loan(
    notional=Money(10_000_000, Currency.USD),
    rate=0.08,
    issue_date=Date(2024, 1, 1),
    maturity=Date(2029, 1, 1),
    frequency=Frequency.Quarterly,
    day_count=DayCount.Act360,
    amortization=AmortizationSchedule.LinearTo(Money(0, Currency.USD))
)

cashflows = loan.generate_cashflows(market_context)
npv = loan.npv(market_context, as_of=Date(2024, 1, 1))
```

**Implementation Tasks:**
- [ ] Create `PyLoan` wrapper class
- [ ] Support various amortization types
- [ ] Generate cashflow schedules
- [ ] Calculate NPV and yields

### Phase 2: Interest Rate Derivatives (High Priority)
**Timeline:** 2 weeks
**Goal:** Enable swap pricing and risk analytics

#### 2.1 Interest Rate Swap
```python
from finstack.instruments import InterestRateSwap, SwapLeg

irs = InterestRateSwap(
    notional=Money(100_000_000, Currency.USD),
    fixed_leg=SwapLeg.fixed(rate=0.03, frequency=Frequency.SemiAnnual),
    float_leg=SwapLeg.floating(index="USD-SOFR", spread_bps=10, frequency=Frequency.Quarterly),
    start_date=Date(2024, 1, 1),
    end_date=Date(2034, 1, 1),
    pay_receive="PAY_FIXED"
)

# Price and compute sensitivities
result = price_swap(irs, market_context)
print(f"NPV: {result.npv}")
print(f"Par Rate: {result.par_rate:.3%}")
print(f"DV01: {result.dv01}")
print(f"Annuity: {result.annuity}")
```

**Implementation Tasks:**
- [ ] Create `PyInterestRateSwap` wrapper
- [ ] Implement fixed and floating leg builders
- [ ] Add par rate calculation
- [ ] Compute DV01 and annuity factors

### Phase 3: Advanced Cashflow Features (Medium Priority)
**Timeline:** 2-3 weeks
**Goal:** Support complex private credit structures

#### 3.1 Enhanced Cashflow Builder
```python
from finstack.cashflow import CashflowBuilder, CouponType, FeeSpec

builder = CashflowBuilder()
    .principal(Money(5_000_000, Currency.EUR), Date(2024, 1, 1), Date(2027, 1, 1))
    .fixed_coupon(rate=0.06, frequency=Frequency.Quarterly)
    .add_pik_period(Date(2024, 1, 1), Date(2025, 1, 1))  # PIK for first year
    .add_cash_period(Date(2025, 1, 1), Date(2027, 1, 1))  # Cash thereafter
    .add_fee(FeeSpec.commitment(bps=50, on_undrawn=True))
    .add_fee(FeeSpec.upfront(Money(100_000, Currency.EUR)))
    .with_amortization("LINEAR_TO_ZERO")

schedule = builder.build()
flows = schedule.get_flows()

# Analyze by type
interest_flows = flows.filter(kind="INTEREST")
fee_flows = flows.filter(kind="FEE")
principal_flows = flows.filter(kind="PRINCIPAL")
```

**Implementation Tasks:**
- [ ] Expose full cashflow builder API
- [ ] Support PIK/Cash/Toggle structures
- [ ] Add comprehensive fee types
- [ ] Enable cashflow filtering and analysis
- [ ] Support custom amortization schedules

#### 3.2 Floating Rate Support
```python
from finstack.cashflow import FloatingLeg

floating_leg = FloatingLeg(
    notional=Money(50_000_000, Currency.USD),
    index="USD-SOFR",
    spread_bps=150,
    frequency=Frequency.Monthly,
    lookback_days=5,
    reset_frequency=Frequency.Daily
)

# Project cashflows using forward curve
projected_flows = floating_leg.project(market_context)
```

### Phase 4: Risk Analytics (High Priority)
**Timeline:** 2 weeks  
**Goal:** Provide comprehensive risk measures for portfolios

#### 4.1 Risk Metrics Engine
```python
from finstack.risk import RiskEngine, BucketSpec

risk_engine = RiskEngine(market_context)

# Single instrument risk
bond_risk = risk_engine.calculate(bond, metrics=["DV01", "CS01", "CONVEXITY", "DURATION"])

# Bucketed DV01
buckets = BucketSpec.standard()  # 3M, 6M, 1Y, 2Y, 3Y, 5Y, 7Y, 10Y, 15Y, 20Y, 30Y
bucketed_dv01 = risk_engine.bucketed_dv01(bond, buckets)

# Greeks for options
option_greeks = risk_engine.greeks(option, ["DELTA", "GAMMA", "VEGA", "THETA", "RHO"])
```

**Implementation Tasks:**
- [ ] Create unified risk calculation interface
- [ ] Implement DV01/CS01 calculations
- [ ] Add bucketed risk measures
- [ ] Support option Greeks
- [ ] Enable batch risk calculation

### Phase 5: Performance & Analytics (Medium Priority)
**Timeline:** 1-2 weeks
**Goal:** Provide return and performance calculations

#### 5.1 Performance Metrics
```python
from finstack.analytics import PerformanceAnalyzer

analyzer = PerformanceAnalyzer()

# XIRR calculation
cashflows = [
    (Date(2023, 1, 1), Money(-1_000_000, Currency.USD)),
    (Date(2023, 7, 1), Money(50_000, Currency.USD)),
    (Date(2024, 1, 1), Money(50_000, Currency.USD)),
    (Date(2024, 7, 1), Money(1_050_000, Currency.USD))
]
xirr = analyzer.xirr(cashflows)

# Time-weighted return
twr = analyzer.twr(values, dates)

# Money-weighted return  
mwr = analyzer.mwr(cashflows, ending_value, end_date)
```

### Phase 6: Credit-Specific Features (Medium Priority)
**Timeline:** 2-3 weeks
**Goal:** Support private credit and distressed debt workflows

#### 6.1 Covenant Modeling
```python
from finstack.credit import Covenant, CovenantTest

leverage_covenant = Covenant(
    name="Maximum Leverage",
    test=CovenantTest.max_ratio(numerator="DEBT", denominator="EBITDA", threshold=5.0),
    frequency=Frequency.Quarterly,
    cure_rights=2,
    consequences=["RATE_STEP_UP", "DISTRIBUTION_BLOCK"]
)

# Test covenant
result = leverage_covenant.test(financials, as_of=Date(2024, 3, 31))
print(f"Passed: {result.passed}")
print(f"Ratio: {result.actual_value:.2f}")
print(f"Headroom: {result.headroom:.2%}")
```

#### 6.2 Workout/Recovery Modeling
```python
from finstack.credit import WorkoutScenario

workout = WorkoutScenario(
    recovery_rate=0.65,
    default_date=Date(2025, 6, 30),
    resolution_lag_months=18,
    recovery_costs=Money(500_000, Currency.USD)
)

recovery_pv = workout.calculate_recovery_pv(loan, market_context)
```

### Phase 7: Convenience Functions & Utilities (Low Priority)
**Timeline:** 1 week
**Goal:** Add helper functions for common workflows

```python
from finstack.utils import (
    calculate_spread,
    bootstrap_curve,
    interpolate_curve,
    build_amortization_schedule
)

# Spread calculation
spread = calculate_spread(bond, market_context, reference_curve="USD-TREASURY")

# Schedule generation helpers
schedule = build_amortization_schedule(
    principal=Money(10_000_000, Currency.USD),
    method="MORTGAGE_STYLE",
    rate=0.05,
    periods=60
)
```

## Implementation Guidelines

### 1. API Design Principles
- **Pythonic:** Follow Python naming conventions (snake_case for functions/methods)
- **Type Hints:** Provide comprehensive type hints for IDE support
- **Defaults:** Sensible defaults for optional parameters
- **Chaining:** Support method chaining where appropriate
- **DataFrames:** Return pandas DataFrames for tabular data

### 2. Error Handling
```python
from finstack.exceptions import (
    PricingError,
    MarketDataError,
    InvalidInstrumentError,
    CurrencyMismatchError
)

try:
    result = price_bond(bond, market_context)
except MarketDataError as e:
    print(f"Missing market data: {e.missing_curves}")
except PricingError as e:
    print(f"Pricing failed: {e.reason}")
```

### 3. Documentation Standards
- Comprehensive docstrings with Google-style formatting
- Working examples in docstrings
- Type stubs (.pyi files) for better IDE support
- Jupyter notebook examples for complex workflows

### 4. Testing Strategy
- Unit tests for each instrument type
- Integration tests with market context
- Property-based testing for cashflow generation
- Performance benchmarks for large portfolios
- Cross-validation with existing systems

## Performance Considerations

### 1. Batch Operations
```python
# Efficient batch pricing
instruments = [bond1, bond2, loan1, swap1]
results = price_batch(instruments, market_context, parallel=True)
```

### 2. Caching
```python
# Cache intermediate calculations
market_context.enable_caching(max_size_mb=100)
```

### 3. Lazy Evaluation
```python
# Defer calculation until needed
bond.cashflows  # Property calculates on first access and caches
```

## Migration Path for Existing Users

### 1. Compatibility Layer
```python
# Support legacy API during transition
from finstack.compat import v1_to_v2_adapter

old_style_bond = create_bond_v1(...)
new_style_bond = v1_to_v2_adapter(old_style_bond)
```

### 2. Deprecation Warnings
```python
import warnings

def old_function():
    warnings.warn(
        "old_function is deprecated, use new_function instead",
        DeprecationWarning,
        stacklevel=2
    )
    return new_function()
```

## Success Metrics

1. **Coverage:** 80% of valuations functionality exposed to Python
2. **Performance:** Python overhead < 10% for batch operations
3. **Adoption:** Used by 5+ credit analysts in production within 3 months
4. **Quality:** >90% test coverage, <0.1% error rate in production
5. **Documentation:** All public APIs documented with examples

## Next Steps

1. **Week 1-2:** Implement Phase 1 (Bonds and basic loans)
2. **Week 3-4:** Implement Phase 2 (Swaps) and Phase 4 (Risk metrics)
3. **Week 5-6:** Implement Phase 3 (Advanced cashflows)
4. **Week 7-8:** Implement Phase 5-6 (Performance and credit features)
5. **Week 9:** Polish, documentation, and examples
6. **Week 10:** Beta testing with credit analysts

## Appendix: Function Mapping

### Core Functions NOT to Expose
- Low-level interpolation internals
- Raw curve construction (use builders instead)
- Internal caching mechanisms
- Decimal/F64 conversion utilities
- DAG execution internals
- Expression AST manipulation

### Essential Functions to Expose
- All instrument constructors
- Pricing functions with standard metrics
- Cashflow generation and analysis
- Risk calculation (DV01, duration, convexity, Greeks)
- Performance metrics (XIRR, TWR, MWR)
- Market data access patterns
- FX conversion with policy control
- Schedule generation with business day adjustment
