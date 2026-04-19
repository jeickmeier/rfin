# Fixed Income Instruments

## Bond

The primary fixed income instrument. Supports fixed-rate, floating-rate,
zero-coupon, amortizing, callable, puttable, and make-whole bonds.

### Basic Bond Construction

```python
from finstack.valuations.instruments import Bond
from finstack.core.money import Money
from datetime import date

# Fixed-rate bond
bond = Bond.builder("ACME-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.045) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .build()

# With credit risk
bond_credit = Bond.builder("ACME-5Y-CREDIT") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.055) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .credit_curve("ACME-HZD") \
    .build()
```

**Key metrics:** `dirty_price`, `clean_price`, `ytm`, `ytw`, `duration_mac`,
`duration_mod`, `convexity`, `dv01`, `yield_dv01`, `z_spread`, `oas`,
`accrued`, `embedded_option_value`

---

## Coupon Specifications

Finstack supports rich coupon structures through `CouponType` and the
`CashFlowBuilder`.

### CouponType

Controls how interest is settled:

| Variant | Description |
|---------|-------------|
| `CouponType.CASH` | Standard cash-pay coupon (default) |
| `CouponType.PIK` | Payment-in-kind — interest capitalizes into principal |
| `CouponType.split(cash_pct, pik_pct)` | Split between cash and PIK |

### PIK Bond

Interest is added to the outstanding principal instead of being paid in cash:

```python
pik_bond = Bond.builder("PIK-7Y") \
    .money(Money(25_000_000, "USD")) \
    .coupon_rate(0.09) \
    .coupon_type("pik") \
    .frequency("semiannual") \
    .issue(date(2024, 6, 15)) \
    .maturity(date(2031, 6, 15)) \
    .disc_id("USD-OIS") \
    .build()
```

### Cash/PIK Split

A portion of the coupon is paid in cash, the remainder capitalizes:

```python
from finstack.valuations.cashflow.builder import (
    CashFlowBuilder, CouponType, FixedCouponSpec,
    ScheduleParams,
)
from finstack.core.currency import Currency

# 50% cash / 50% PIK
split = CouponType.split(cash_pct=0.50, pik_pct=0.50)

schedule = CashFlowBuilder.new() \
    .principal(25_000_000, Currency("USD"),
               date(2024, 6, 15), date(2031, 6, 15)) \
    .fixed_cf(FixedCouponSpec.new(
        rate=0.09,
        schedule=ScheduleParams.semiannual_30360(),
        coupon_type=split,
    )) \
    .build_with_curves()

bond = Bond.builder("SPLIT-7Y") \
    .money(Money(25_000_000, "USD")) \
    .cashflows(schedule) \
    .issue(date(2024, 6, 15)) \
    .maturity(date(2031, 6, 15)) \
    .disc_id("USD-OIS") \
    .build()
```

### Floating Rate Bond

```python
from finstack.valuations.cashflow.builder import (
    FloatingRateSpec, FloatingCouponSpec,
)

float_spec = FloatingRateSpec.new(
    index_id="USD-SOFR-3M",
    spread_bp=150.0,              # SOFR + 150bp
    schedule=ScheduleParams.quarterly_act360(),
    index_floor_bp=0.0,                 # SOFR floor at 0%
    all_in_cap_bp=500.0,                 # cap at 5%
    reset_lag_days=2,             # T-2 reset
)

schedule = CashFlowBuilder.new() \
    .principal(10_000_000, Currency("USD"),
               date(2024, 1, 15), date(2029, 1, 15)) \
    .floating_cf(FloatingCouponSpec.from_rate_spec(
        rate_spec=float_spec,
        schedule=ScheduleParams.quarterly_act360(),
    )) \
    .build_with_curves(market)

frn = Bond.builder("FRN-5Y") \
    .money(Money(10_000_000, "USD")) \
    .cashflows(schedule) \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .forward_curve("USD-SOFR-3M") \
    .build()
```

Alternatively, use the shorthand builder methods for simple floaters:

```python
frn_simple = Bond.builder("FRN-5Y-SIMPLE") \
    .money(Money(10_000_000, "USD")) \
    .frequency("quarterly") \
    .forward_curve("USD-SOFR-3M") \
    .float_margin_bp(150.0) \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .build()
```

### Overnight Compounding Methods

For SOFR/SONIA-linked bonds, control how overnight rates compound:

```python
from finstack.valuations.cashflow.builder import OvernightCompoundingMethod

# Standard compounded in arrears (SOFR default)
sofr_arrears = OvernightCompoundingMethod.COMPOUNDED_IN_ARREARS

# With 2-day lookback (SONIA convention)
sonia_lookback = OvernightCompoundingMethod.compounded_with_lookback(
    lookback_days=5,
)

# With lockout (last N days use same rate)
lockout = OvernightCompoundingMethod.compounded_with_lockout(
    lockout_days=2,
)

# With observation shift
obs_shift = OvernightCompoundingMethod.compounded_with_observation_shift(
    shift_days=2,
)

float_spec = FloatingRateSpec.new(
    index_id="USD-SOFR-ON",
    spread_bp=100.0,
    schedule=ScheduleParams.quarterly_act360(),
    overnight_compounding=sofr_arrears,
)
```

### Fixed-to-Float Switch

A bond that pays fixed for an initial period, then switches to floating:

```python
from finstack.valuations.cashflow.builder import (
    FixedWindow, FloatingCouponSpec, FloatingRateSpec,
)

schedule = CashFlowBuilder.new() \
    .principal(10_000_000, Currency("USD"),
               date(2024, 1, 15), date(2034, 1, 15)) \
    .fixed_to_float(
        switch=date(2029, 1, 15),  # fixed for 5Y, float for 5Y
        fixed_win=FixedWindow(
            rate=0.05,
            schedule=ScheduleParams.semiannual_30360(),
        ),
        float_spec=FloatingCouponSpec(
            coupon_type=CouponType.CASH,
            rate_spec=FloatingRateSpec(
                index_id="USD-SOFR-3M",
                spread_bp=200.0,
                reset_freq="3M",
                dc="Act360",
                bdc="modified_following",
                calendar_id="weekends_only",
            ),
            freq="3M",
            stub="short_front",
        ),
        fixed_split=CouponType.CASH,
    ) \
    .build_with_curves(market)
```

### Step-Up Coupon

Fixed rate that increases on scheduled dates:

```python
schedule = CashFlowBuilder.new() \
    .principal(10_000_000, Currency("USD"),
               date(2024, 1, 15), date(2031, 1, 15)) \
    .fixed_stepup(
        steps=[
            (date(2026, 1, 15), 0.045),   # 4.5% until Jan 2026
            (date(2028, 1, 15), 0.050),   # 5.0% until Jan 2028
            (date(2031, 1, 15), 0.060),   # 6.0% until maturity
        ],
        schedule=ScheduleParams.semiannual_30360(),
        default_split=CouponType.CASH,
    ) \
    .build_with_curves()
```

### Floating Margin Step-Up

Floating spread that increases over time (common in leveraged loans):

```python
schedule = CashFlowBuilder.new() \
    .principal(50_000_000, Currency("USD"),
               date(2024, 3, 1), date(2029, 3, 1)) \
    .float_margin_stepup(
        steps=[
            (date(2026, 3, 1), 300.0),   # SOFR + 300bp first 2 years
            (date(2028, 3, 1), 350.0),   # SOFR + 350bp years 3–4
            (date(2029, 3, 1), 400.0),   # SOFR + 400bp year 5
        ],
        base_spec=FloatingCouponSpec(
            coupon_type=CouponType.CASH,
            rate_spec=FloatingRateSpec(
                index_id="USD-SOFR-3M",
                spread_bp=300.0,
                reset_freq="3M",
                dc="Act360",
                bdc="modified_following",
                calendar_id="weekends_only",
            ),
            freq="3M",
            stub="short_front",
        ),
    ) \
    .build_with_curves(market)
```

### PIK Toggle Program

Switch between cash-pay and PIK on scheduled dates:

```python
schedule = CashFlowBuilder.new() \
    .principal(25_000_000, Currency("USD"),
               date(2024, 6, 15), date(2031, 6, 15)) \
    .fixed_cf(FixedCouponSpec.new(
        rate=0.09,
        schedule=ScheduleParams.semiannual_30360(),
    )) \
    .payment_split_program([
        (date(2026, 6, 15), CouponType.CASH),                      # cash first 2Y
        (date(2028, 6, 15), CouponType.split(0.50, 0.50)),         # 50/50 Y3–4
        (date(2031, 6, 15), CouponType.PIK),                       # full PIK Y5–7
    ]) \
    .build_with_curves()
```

---

## Amortization Specifications

Control how principal is repaid over the life of the instrument.

### AmortizationSpec Variants

| Factory Method | Description |
|---------------|-------------|
| `AmortizationSpec.none()` | Bullet — full repayment at maturity |
| `AmortizationSpec.linear_to(final)` | Linear amortization to a target balance |
| `AmortizationSpec.percent_per_period(pct)` | Constant % of current outstanding per period |
| `AmortizationSpec.step_remaining(schedule)` | Step schedule of remaining balances |
| `AmortizationSpec.custom_principal(items)` | Explicit principal payment amounts |

### Linear Amortization

```python
from finstack.valuations.cashflow.builder import AmortizationSpec

# Amortize to 20% of original by maturity
bond = Bond.builder("AMORT-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.05) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .amortization(AmortizationSpec.linear_to(Money(2_000_000, "USD"))) \
    .build()
```

### Step-Down Schedule

Specify remaining notional at each date:

```python
bond = Bond.builder("STEP-AMORT") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.05) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .amortization(AmortizationSpec.step_remaining([
        (date(2025, 7, 15), Money(8_000_000, "USD")),
        (date(2026, 7, 15), Money(6_000_000, "USD")),
        (date(2027, 7, 15), Money(4_000_000, "USD")),
        (date(2028, 7, 15), Money(2_000_000, "USD")),
    ])) \
    .build()
```

### Custom Principal Payments

Specify exact principal amounts paid on each date:

```python
bond = Bond.builder("CUSTOM-AMORT") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.05) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .amortization(AmortizationSpec.custom_principal([
        (date(2025, 1, 15), Money(1_000_000, "USD")),
        (date(2026, 1, 15), Money(2_000_000, "USD")),
        (date(2027, 1, 15), Money(3_000_000, "USD")),
        # remaining 4M at maturity
    ])) \
    .build()
```

### Amortization with CashFlowBuilder

For more control, use the `CashFlowBuilder` directly:

```python
schedule = CashFlowBuilder.new() \
    .principal(10_000_000, Currency("USD"),
               date(2024, 1, 15), date(2029, 1, 15)) \
    .amortization(AmortizationSpec.percent_per_period(0.05)) \
    .fixed_cf(FixedCouponSpec.new(
        rate=0.05,
        schedule=ScheduleParams.semiannual_30360(),
    )) \
    .build_with_curves()

# Inspect the outstanding balance path
for dt, balance in schedule.outstanding_by_date():
    print(f"  {dt}: {balance}")
```

---

## Call and Put Schedules

### Discrete Call Schedule

Specify dates at which the issuer can call the bond at given prices:

```python
from finstack.valuations.instruments.fixed_income.bond import (
    CallPut, CallPutSchedule,
)

callable_bond = Bond.builder("CALL-5Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.06) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2029, 1, 15)) \
    .disc_id("USD-OIS") \
    .call_schedule([
        {"date": date(2026, 1, 15), "price_pct": 103.0},   # NC-2, call at 103
        {"date": date(2027, 1, 15), "price_pct": 102.0},   # call at 102
        {"date": date(2028, 1, 15), "price_pct": 101.0},   # call at 101
        {"date": date(2028, 7, 15), "price_pct": 100.0},   # par call
    ]) \
    .build()
```

### Make-Whole Call

Treasury spread + make-whole premium. Requires tree pricing:

```python
from finstack.valuations.instruments.fixed_income.bond import MakeWholeSpec

mw_bond = Bond.builder("MW-10Y") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.055) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2034, 1, 15)) \
    .disc_id("USD-OIS") \
    .call_schedule([
        {"date": date(2025, 1, 15), "price_pct": 100.0},
    ]) \
    .tree_steps(100) \
    .tree_volatility(0.01) \
    .build()
```

### CallPut Object API

For finer control, use `CallPut` and `CallPutSchedule` directly:

```python
calls = CallPutSchedule(
    calls=[
        CallPut(date(2027, 1, 15), price_pct_of_par=103.0),
        CallPut(date(2028, 1, 15), price_pct_of_par=101.5),
        CallPut(
            date(2029, 1, 15),
            price_pct_of_par=100.0,
            end_date=date(2034, 1, 15),  # continuously callable
        ),
    ],
    puts=[
        CallPut(date(2029, 1, 15), price_pct_of_par=100.0),
    ],
)
```

### Tree Pricing for Callable/Puttable Bonds

Callable and puttable bonds require a tree model for `oas` and
`embedded_option_value`:

```python
callable_bond = Bond.builder("OAS-BOND") \
    .money(Money(10_000_000, "USD")) \
    .coupon_rate(0.055) \
    .frequency("semiannual") \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2034, 1, 15)) \
    .disc_id("USD-OIS") \
    .quoted_clean_price(98.5) \
    .call_schedule([
        {"date": date(2029, 1, 15), "price_pct": 100.0},
    ]) \
    .tree_steps(200) \
    .tree_volatility(0.012) \
    .mean_reversion(0.03) \
    .call_friction_cents(0.25) \
    .build()
```

---

## Advanced CashFlowBuilder Patterns

The `CashFlowBuilder` enables full composition of complex cashflow structures.

### Combined Amortization + PIK + Step-Up

```python
schedule = CashFlowBuilder.new() \
    .principal(25_000_000, Currency("USD"),
               date(2024, 6, 15), date(2031, 6, 15)) \
    .amortization(AmortizationSpec.percent_per_period(0.025)) \
    .fixed_stepup(
        steps=[
            (date(2026, 6, 15), 0.07),
            (date(2029, 6, 15), 0.08),
            (date(2031, 6, 15), 0.09),
        ],
        schedule=ScheduleParams.semiannual_30360(),
        default_split=CouponType.split(0.75, 0.25),  # 75% cash / 25% PIK
    ) \
    .build_with_curves()
```

### Adding Fees to a Schedule

```python
from finstack.valuations.cashflow.builder import FeeSpec, FeeBase

schedule = CashFlowBuilder.new() \
    .principal(10_000_000, Currency("USD"),
               date(2024, 1, 15), date(2029, 1, 15)) \
    .fixed_cf(FixedCouponSpec.new(
        rate=0.05,
        schedule=ScheduleParams.semiannual_30360(),
    )) \
    .fee(FeeSpec.fixed(date(2024, 1, 15), Money(100_000, "USD"))) \
    .fee(FeeSpec.periodic_bps(
        base=FeeBase.drawn(),
        bps=25.0,
        schedule=ScheduleParams.quarterly_act360(),
    )) \
    .build_with_curves()
```

### Inspecting Cashflows

```python
schedule = bond.cashflows()

# Iterate all flows
for cf in schedule.flows():
    print(f"  {cf.date}  {cf.kind:>12}  {cf.amount}")

# Outstanding balance path
for dt, bal in schedule.outstanding_by_date():
    print(f"  {dt}: {bal}")

# As a Polars DataFrame
df = schedule.to_dataframe(
    market=market,
    discount_curve_id="USD-OIS",
    as_of=date(2025, 1, 15),
    include_floating_decomposition=True,
)
```

---

## Inflation-Linked Bond

Bonds whose principal and/or coupons are indexed to CPI:

```python
from finstack.valuations.instruments import InflationLinkedBond

ilb = InflationLinkedBond.builder("TIPS-10Y") \
    .money(Money(5_000_000, "USD")) \
    .real_coupon_rate(0.015) \
    .issue(date(2024, 1, 15)) \
    .maturity(date(2034, 1, 15)) \
    .disc_id("USD-OIS") \
    .inflation_id("USD-CPI") \
    .base_cpi(310.0) \
    .build()
```

---

## Term Loan

Amortizing loans with rich customization for leveraged finance.

### Basic Floating-Rate Term Loan

```python
from finstack.valuations.instruments import TermLoan
from finstack.valuations.instruments.fixed_income.term_loan import (
    RateSpec, TermLoanAmortizationSpec, CouponType as LoanCouponType,
    LoanCallSchedule, LoanCall, LoanCallType,
    DdtlSpec, CovenantSpec, MarginStepUp, PikToggle,
    CashSweepEvent, OidEirSpec, OidPolicy,
)

loan = TermLoan.builder("TL-ACME") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating(
        index_id="USD-SOFR-3M",
        spread_bp=325.0,
        reset_freq="3M",
        reset_lag_days=2,
        index_floor_bp=0.0,
    )) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Term Loan Amortization

```python
# 1% per quarter of current balance (geometric decay)
loan = TermLoan.builder("TL-AMORT") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=325.0)) \
    .amortization(TermLoanAmortizationSpec.percent_per_period(bp=100)) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()

# 5% per quarter of original notional (flat dollar)
loan_flat = TermLoan.builder("TL-FLAT") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=325.0)) \
    .amortization(TermLoanAmortizationSpec.percent_of_original_notional(bp=500)) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()

# Custom schedule
loan_custom = TermLoan.builder("TL-CUSTOM") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=325.0)) \
    .amortization(TermLoanAmortizationSpec.custom([
        (date(2025, 3, 1), Money(5_000_000, "USD")),
        (date(2026, 3, 1), Money(10_000_000, "USD")),
        (date(2027, 3, 1), Money(10_000_000, "USD")),
    ])) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Term Loan PIK / Split Coupon

```python
# Full PIK
loan_pik = TermLoan.builder("TL-PIK") \
    .notional(Money(25_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=500.0)) \
    .coupon_type(LoanCouponType.PIK) \
    .issue(date(2024, 6, 1)) \
    .maturity(date(2029, 6, 1)) \
    .disc_id("USD-OIS") \
    .build()

# 60/40 cash/PIK split
loan_split = TermLoan.builder("TL-SPLIT") \
    .notional(Money(25_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=500.0)) \
    .coupon_type(LoanCouponType.split(cash_pct=0.60, pik_pct=0.40)) \
    .issue(date(2024, 6, 1)) \
    .maturity(date(2029, 6, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Term Loan Call Schedule

```python
loan = TermLoan.builder("TL-CALLABLE") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=350.0)) \
    .call_schedule(LoanCallSchedule(calls=[
        LoanCall(date(2025, 3, 1), price_pct_of_par=102.0,
                 call_type=LoanCallType.Hard),
        LoanCall(date(2026, 3, 1), price_pct_of_par=101.0,
                 call_type=LoanCallType.Hard),
        LoanCall(date(2027, 3, 1), price_pct_of_par=100.0,
                 call_type=LoanCallType.Hard),
    ])) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Delayed Draw Term Loan (DDTL)

```python
from finstack.valuations.instruments.fixed_income.term_loan import (
    DrawEvent, CommitmentStepDown, CommitmentFeeBase,
)

loan = TermLoan.builder("DDTL-ACME") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=350.0)) \
    .ddtl(DdtlSpec(
        commitment_limit=Money(100_000_000, "USD"),
        availability_start=date(2024, 3, 1),
        availability_end=date(2025, 9, 1),
        draws=[
            DrawEvent(date(2024, 6, 1), Money(25_000_000, "USD")),
            DrawEvent(date(2025, 1, 1), Money(25_000_000, "USD")),
        ],
        commitment_step_downs=[
            CommitmentStepDown(date(2025, 3, 1), Money(75_000_000, "USD")),
        ],
        commitment_fee_bp=50,
        fee_base=CommitmentFeeBase.UNDRAWN,
    )) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Covenant-Driven Events

```python
loan = TermLoan.builder("TL-COVENANTS") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=300.0)) \
    .covenants(CovenantSpec(
        margin_stepups=[
            MarginStepUp(date(2025, 6, 1), delta_bp=25),
            MarginStepUp(date(2026, 6, 1), delta_bp=50),
        ],
        pik_toggles=[
            PikToggle(date(2026, 6, 1), enable_pik=True),
        ],
        cash_sweeps=[
            CashSweepEvent(date(2025, 12, 1), Money(5_000_000, "USD")),
        ],
    )) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

### Original Issue Discount (OID)

```python
loan = TermLoan.builder("TL-OID") \
    .notional(Money(50_000_000, "USD")) \
    .rate(RateSpec.floating("USD-SOFR-3M", spread_bp=350.0)) \
    .upfront_fee(Money(500_000, "USD")) \
    .oid_eir(OidEirSpec(
        oid_policy=OidPolicy.withheld_pct(pct_bp=100),  # 1% OID
    )) \
    .issue(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

---

## Revolving Credit Facility

Drawn/undrawn revolving facility with tiered fees:

```python
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.instruments.fixed_income.revolving_credit import (
    BaseRateSpec, RevolvingCreditFees, FeeTier,
    DrawRepaySpec, DrawRepayEvent,
)

revolver = RevolvingCredit.builder("RC-ACME") \
    .commitment_amount(75_000_000) \
    .drawn_amount(30_000_000) \
    .currency("USD") \
    .base_rate(BaseRateSpec.floating(
        index_id="USD-SOFR-3M",
        spread_bp=250.0,
    )) \
    .fees(RevolvingCreditFees(
        facility_fee_bp=10.0,
        commitment_fee_tiers=[
            FeeTier.from_bps(0.0, 25.0),    # 0-50% util: 25bp
            FeeTier.from_bps(0.5, 37.5),    # 50-100% util: 37.5bp
        ],
    )) \
    .commitment_date(date(2024, 3, 1)) \
    .maturity(date(2029, 3, 1)) \
    .disc_id("USD-OIS") \
    .build()
```

---

## Prepayment and Default Models

For mortgage-backed and asset-backed securities:

```python
from finstack.valuations.cashflow.builder import (
    PrepaymentModelSpec, DefaultModelSpec, RecoveryModelSpec,
)

# PSA 150% prepayment model (ramps to 9% CPR over 30 months)
prepay = PrepaymentModelSpec.psa(speed_multiplier=1.5)

# 2% constant CDR
default = DefaultModelSpec.constant_cdr(cdr=0.02)

# 40% recovery with 6 month lag
recovery = RecoveryModelSpec.with_lag(rate=0.40, recovery_lag=6)

# CMBS with lockout
cmbs_prepay = PrepaymentModelSpec.cmbs_with_lockout(
    lockout_months=36,
    post_lockout_cpr=0.10,
)
```

---

## Other Fixed Income Types

| Type | Description |
|------|-------------|
| `RevolvingCredit` | Drawn/undrawn revolving facility |
| `ConvertibleBond` | Bond with equity conversion option |
| `StructuredCredit` | CLO/CDO tranches with waterfall logic |
| `AgencyMbsPassthrough` | Agency mortgage pass-through securities |
| `AgencyCmo` | Collateralized mortgage obligations |
| `AgencyTba` | To-be-announced MBS trades |
| `DollarRoll` | MBS dollar roll financing |
