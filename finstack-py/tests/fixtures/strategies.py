"""Shared hypothesis strategies and constants for cross-language parity testing.

This module centralizes test data generation strategies and tolerance constants
used across parity and property tests.
"""

from collections.abc import Callable
from datetime import date, timedelta
import math
from typing import Any

from finstack.core.currency import Currency
from finstack.core.dates import DayCount
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import DiscountCurve, ForwardCurve, MarketContext
from finstack.core.money import Money
from finstack.valuations.instruments import Bond, Deposit, InterestRateSwap
from hypothesis import strategies as st

# Tolerance levels from parity README
# Deterministic operations: arithmetic, date calculations
TOLERANCE_DETERMINISTIC = 1e-10
# Floating-point operations: pricing, discounting
TOLERANCE_FLOATING_POINT = 1e-8
# Monte Carlo simulations with fixed seed
TOLERANCE_MONTE_CARLO = 1e-6

# Basic strategies
major_currencies = st.sampled_from(["USD", "EUR", "GBP", "JPY", "CHF"])
positive_rates = st.floats(min_value=0.001, max_value=0.20, allow_nan=False, allow_infinity=False)
positive_amounts = st.floats(min_value=1000.0, max_value=1e9, allow_nan=False, allow_infinity=False)
positive_notionals = st.floats(min_value=1_000.0, max_value=1e8, allow_nan=False, allow_infinity=False)
tenors_in_years = st.integers(min_value=1, max_value=10)
seeds = st.integers(min_value=1, max_value=2**32 - 1)
bump_sizes_bp = st.floats(min_value=1.0, max_value=100.0, allow_nan=False, allow_infinity=False)
small_bumps_bp = st.floats(min_value=0.1, max_value=10.0, allow_nan=False, allow_infinity=False)


@st.composite
def simple_date_strategy(draw: Callable[[Any], Any]) -> date:
    """Generate valid dates for testing."""
    base_date = date(2024, 1, 1)
    days_offset = draw(st.integers(min_value=0, max_value=3650))  # Up to 10 years
    return base_date + timedelta(days=days_offset)


@st.composite
def discount_curve_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> DiscountCurve:
    """Generate valid discount curves for testing.

    Creates a curve with 5 pillar points using simple compounding from a base rate.
    """
    curve_id = f"{currency_code}-OIS"

    # Generate base rate and derive discount factors
    base_rate = draw(st.floats(min_value=0.01, max_value=0.10))
    dates = [base_date + timedelta(days=365 * i) for i in range(1, 6)]
    dfs = [1.0 / ((1.0 + base_rate) ** i) for i in range(1, 6)]

    day_count = DayCount.ACT_365F

    # Convert dates to time years using day_count
    knots = [(day_count.year_fraction(base_date, d, None), df) for d, df in zip(dates, dfs, strict=False)]

    return DiscountCurve(
        curve_id,
        base_date,
        knots,
        day_count=day_count,
    )


@st.composite
def forward_curve_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> ForwardCurve:
    """Generate valid forward curves for testing.

    Creates a forward curve with a flat-ish rate structure.
    """
    curve_id = f"{currency_code}-SOFR"
    tenor = 0.25  # 3-month tenor

    # Generate forward rates
    base_rate = draw(st.floats(min_value=0.01, max_value=0.10))
    # Slight upward slope
    knots = [
        (0.0, base_rate),
        (1.0, base_rate + 0.005),
        (3.0, base_rate + 0.01),
        (5.0, base_rate + 0.015),
    ]

    return ForwardCurve(
        curve_id,
        tenor,
        knots,
        base_date=base_date,
        day_count=DayCount.ACT_360,
    )


@st.composite
def flat_discount_curve(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
    rate: float | None = None,
) -> DiscountCurve:
    """Generate a flat discount curve using continuous compounding.

    If rate is None, draws a random rate.
    """
    curve_id = f"{currency_code}-OIS"

    if rate is None:
        rate = draw(st.floats(min_value=0.01, max_value=0.10))

    # df(t) = exp(-rate * t)
    knots = [(t, math.exp(-rate * t)) for t in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 10.0]]

    return DiscountCurve(
        curve_id,
        base_date,
        knots,
        day_count="act_365f",
    )


@st.composite
def flat_forward_curve(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
    rate: float | None = None,
) -> ForwardCurve:
    """Generate a flat forward curve.

    If rate is None, draws a random rate.
    """
    curve_id = f"{currency_code}-SOFR"
    tenor = 0.25

    if rate is None:
        rate = draw(st.floats(min_value=0.01, max_value=0.10))

    knots = [(t, rate) for t in [0.0, 1.0, 3.0, 5.0, 10.0]]

    return ForwardCurve(
        curve_id,
        tenor,
        knots,
        base_date=base_date,
        day_count=DayCount.ACT_360,
    )


@st.composite
def deposit_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> Deposit:
    """Generate valid deposits for testing."""
    tenor_days = draw(st.integers(min_value=1, max_value=365))
    maturity_date = base_date + timedelta(days=tenor_days)

    notional = draw(st.floats(min_value=1000.0, max_value=1e6))
    rate = draw(st.floats(min_value=0.001, max_value=0.15))

    currency = Currency(currency_code)

    return (
        Deposit.builder(f"DEP-{tenor_days}D")
        .money(Money(notional, currency))
        .start(base_date)
        .end(maturity_date)
        .day_count(DayCount.ACT_360)
        .disc_id(f"{currency_code}-OIS")
        .quote_rate(rate)
        .build()
    )


@st.composite
def bond_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> Bond:
    """Generate valid bonds for testing."""
    tenor_years = draw(st.integers(min_value=1, max_value=10))
    maturity_date = date(base_date.year + tenor_years, base_date.month, base_date.day)

    notional = draw(st.floats(min_value=1000.0, max_value=1e6))
    coupon = draw(st.floats(min_value=0.01, max_value=0.10))

    currency = Currency(currency_code)

    return (
        Bond.builder(f"BOND-{tenor_years}Y")
        .money(Money(notional, currency))
        .coupon_rate(coupon)
        .issue(base_date)
        .maturity(maturity_date)
        .disc_id(f"{currency_code}-OIS")
        .build()
    )


@st.composite
def swap_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> InterestRateSwap:
    """Generate valid interest rate swaps for testing."""
    tenor_years = draw(st.integers(min_value=1, max_value=10))
    maturity_date = date(base_date.year + tenor_years, base_date.month, base_date.day)

    notional = draw(st.floats(min_value=100_000.0, max_value=100_000_000.0))
    fixed_rate = draw(st.floats(min_value=0.01, max_value=0.10))

    return (
        InterestRateSwap.builder(f"IRS-{tenor_years}Y")
        .notional(notional)
        .currency(currency_code)
        .maturity(maturity_date)
        .fixed_rate(fixed_rate)
        .frequency(Frequency.SEMI_ANNUAL)
        .disc_id(f"{currency_code}-OIS")
        .fwd_id(f"{currency_code}-SOFR")
        .build()
    )


@st.composite
def market_context_strategy(
    draw: Callable[[Any], Any],
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> MarketContext:
    """Generate a complete market context with discount and forward curves."""
    discount = draw(discount_curve_strategy(currency_code=currency_code, base_date=base_date))
    forward = draw(forward_curve_strategy(currency_code=currency_code, base_date=base_date))

    market = MarketContext()
    market.insert_discount(discount)
    market.insert_forward(forward)

    return market


def create_flat_market_context(
    discount_rate: float = 0.05,
    forward_rate: float = 0.05,
    currency_code: str = "USD",
    base_date: date = date(2024, 1, 1),
) -> MarketContext:
    """Create a market context with flat curves at specified rates.

    This is a deterministic helper, not a hypothesis strategy.
    """
    # Discount curve using continuous compounding
    disc_knots = [(t, math.exp(-discount_rate * t)) for t in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 10.0]]
    discount_curve = DiscountCurve(
        f"{currency_code}-OIS",
        base_date,
        disc_knots,
        day_count="act_365f",
    )

    # Flat forward curve
    fwd_knots = [(t, forward_rate) for t in [0.0, 1.0, 3.0, 5.0, 10.0]]
    forward_curve = ForwardCurve(
        f"{currency_code}-SOFR",
        0.25,  # 3-month tenor
        fwd_knots,
        base_date=base_date,
        day_count=DayCount.ACT_360,
    )

    market = MarketContext()
    market.insert_discount(discount_curve)
    market.insert_forward(forward_curve)

    return market


def create_test_bond(
    bond_id: str = "TEST-BOND",
    notional: float = 1_000_000.0,
    currency_code: str = "USD",
    issue: date = date(2024, 1, 1),
    maturity: date = date(2029, 1, 1),
    coupon_rate: float = 0.05,
    frequency: Frequency = Frequency.SEMI_ANNUAL,
    day_count: DayCount = DayCount.THIRTY_360,
    disc_id: str = "USD-OIS",
) -> Bond:
    """Create a test bond with specified parameters.

    This is a deterministic helper, not a hypothesis strategy.
    """
    return (
        Bond.builder(bond_id)
        .notional(notional)
        .currency(currency_code)
        .issue(issue)
        .maturity(maturity)
        .coupon_rate(coupon_rate)
        .frequency(frequency)
        .day_count(day_count)
        .disc_id(disc_id)
        .build()
    )


def create_test_swap(
    swap_id: str = "TEST-SWAP",
    notional: float = 10_000_000.0,
    currency_code: str = "USD",
    maturity: date = date(2029, 1, 1),
    fixed_rate: float = 0.05,
    frequency: Frequency = Frequency.SEMI_ANNUAL,
    disc_id: str = "USD-OIS",
    fwd_id: str = "USD-SOFR",
) -> InterestRateSwap:
    """Create a test swap with specified parameters.

    This is a deterministic helper, not a hypothesis strategy.
    """
    return (
        InterestRateSwap.builder(swap_id)
        .notional(notional)
        .currency(currency_code)
        .maturity(maturity)
        .fixed_rate(fixed_rate)
        .frequency(frequency)
        .disc_id(disc_id)
        .fwd_id(fwd_id)
        .build()
    )


def create_test_deposit(
    deposit_id: str = "TEST-DEP",
    notional: float = 1_000_000.0,
    currency_code: str = "USD",
    start: date = date(2024, 1, 1),
    end: date = date(2024, 4, 1),
    quote_rate: float = 0.05,
    day_count: DayCount = DayCount.ACT_360,
    disc_id: str = "USD-OIS",
) -> Deposit:
    """Create a test deposit with specified parameters.

    This is a deterministic helper, not a hypothesis strategy.
    """
    currency = Currency(currency_code)
    return (
        Deposit.builder(deposit_id)
        .money(Money(notional, currency))
        .start(start)
        .end(end)
        .day_count(day_count)
        .disc_id(disc_id)
        .quote_rate(quote_rate)
        .build()
    )
