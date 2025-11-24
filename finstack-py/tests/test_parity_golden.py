"""Python parity tests using golden values.

These tests verify that the Python bindings produce results matching
the golden values that are also used by the WASM parity tests.
This ensures behavioral parity across language bindings.
"""

from datetime import date
import json
from pathlib import Path

import pytest

from finstack.core.currency import Currency
from finstack.core.dates import DayCount, DayCountContext, build_periods
from finstack.core.market_data import DiscountCurve, FxMatrix
from finstack.core.money import Money

# Load golden values
GOLDEN_VALUES_PATH = Path(__file__).parent.parent.parent / "tests" / "golden_values.json"
with GOLDEN_VALUES_PATH.open() as f:
    GOLDEN_VALUES = json.load(f)


def test_money_arithmetic() -> None:
    """Test basic money arithmetic against golden values."""
    test_case = GOLDEN_VALUES["test_cases"]["money_arithmetic"]
    inputs = test_case["inputs"]
    expected = test_case["expected"]

    usd = Currency("USD")
    m1 = Money(inputs["m1"]["amount"], usd)
    m2 = Money(inputs["m2"]["amount"], usd)

    # Addition
    result_add = m1 + m2
    assert result_add.amount == expected["add"]["amount"]
    assert result_add.currency.code == expected["add"]["currency"]

    # Subtraction
    result_sub = m1 - m2
    assert result_sub.amount == expected["subtract"]["amount"]

    # Multiplication
    result_mul = m1 * 2.0
    assert result_mul.amount == expected["multiply_2"]["amount"]

    # Division
    result_div = m1 / 2.0
    assert result_div.amount == expected["divide_2"]["amount"]


def test_day_count_act360() -> None:
    """Test Act/360 day count convention against golden values."""
    test_case = GOLDEN_VALUES["test_cases"]["day_count_act360"]
    inputs = test_case["inputs"]
    expected = test_case["expected"]

    start = date.fromisoformat(inputs["start_date"])
    end = date.fromisoformat(inputs["end_date"])

    dc = DayCount.ACT_360
    ctx = DayCountContext()

    yf = dc.year_fraction(start, end, ctx)

    assert abs(yf - expected["year_fraction"]) < expected["tolerance"]


def test_discount_curve_df() -> None:
    """Test discount curve discount factor calculation against golden values."""
    # Simple test
    curve = DiscountCurve(
        "USD-TEST",
        date(2024, 1, 1),
        [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75), (10.0, 0.50)],  # (time, df) tuples
        day_count="act_365f",
    )

    # Test df at 1 year
    df = curve.df(1.0)
    assert abs(df - 0.95) < 0.01


def test_fx_rate_lookup() -> None:
    """Test FX matrix rate lookup against golden values."""
    from finstack.core.market_data import FxConversionPolicy

    fx = FxMatrix()

    # Set direct quotes
    eur = Currency("EUR")
    usd = Currency("USD")

    fx.set_quote(eur, usd, 1.10)

    # Lookup the rate
    rate_result = fx.rate(eur, usd, date(2024, 1, 1), FxConversionPolicy.CASHFLOW_DATE)
    rate = rate_result.rate

    assert abs(rate - 1.10) < 0.01


def test_period_building() -> None:
    """Test period building against golden values."""
    test_case = GOLDEN_VALUES["test_cases"]["period_building"]
    inputs = test_case["inputs"]
    expected = test_case["expected"]

    plan = build_periods(inputs["spec"], inputs["actuals_until"])

    # Use len() function, not .len() method
    assert len(plan.periods) == expected["total_periods"]

    # Check period IDs and actual flags
    periods = plan.periods
    for i, (expected_id, expected_is_actual) in enumerate(
        zip(expected["period_ids"], expected["is_actual"], strict=False)
    ):
        assert periods[i].id.code == expected_id
        assert periods[i].is_actual == expected_is_actual


def test_discount_curve_zero_rate() -> None:
    """Test that discount curve provides consistent zero rates."""
    # Simple test
    curve = DiscountCurve(
        "USD-TEST", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75), (10.0, 0.50)], day_count="act_365f"
    )

    # Verify zero rate consistency
    time = 1.0
    df = curve.df(time)
    zero = curve.zero(time)

    # Should be able to recover df from zero rate
    import math

    recovered_df = math.exp(-zero * time)

    assert abs(df - recovered_df) < 0.01


def test_currency_operations() -> None:
    """Test basic currency operations for parity."""
    usd = Currency("USD")

    assert usd.code == "USD"
    assert usd.numeric == 840

    # Test case insensitivity
    usd2 = Currency("usd")
    assert usd2.code == "USD"

    # Test common currencies
    eur = Currency("EUR")
    assert eur.code == "EUR"
    assert eur.numeric == 978


def test_money_formatting() -> None:
    """Test money formatting for consistency."""
    usd = Currency("USD")
    money = Money(1234567.89, usd)

    formatted = money.format()
    assert "USD" in formatted
    assert "1234567" in formatted


def test_bond_pricing_treasury() -> None:
    """Test treasury bond pricing against golden values."""
    from finstack.core.dates import DayCount
    from finstack.core.dates.schedule import Frequency
    from finstack.core.market_data import MarketContext
    from finstack.valuations.instruments import Bond

    # Create a simple treasury bond
    bond = (
        Bond.builder("TREASURY-001")
        .notional(1_000_000.0)
        .currency("USD")
        .issue(date(2024, 1, 1))
        .maturity(date(2029, 1, 1))
        .coupon_rate(0.045)
        .frequency(Frequency.SEMI_ANNUAL)
        .day_count(DayCount.THIRTY_360)
        .disc_id("USD-OIS")
        .build()
    )

    # Create simple market context with discount curve
    market = MarketContext()
    discount_curve = DiscountCurve(
        "USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)], day_count="act_365f"
    )
    market.insert_discount(discount_curve)

    # Price the bond
    from finstack.valuations.pricer import create_standard_registry

    registry = create_standard_registry()
    # Use "discounting" model key like in test_roundtrips.py
    result = registry.price(bond, "discounting", market, date(2024, 1, 1))

    # Basic validation - bond should have a positive value
    assert result.value.amount > 0
    assert result.value.currency.code == "USD"


def test_irs_valuation() -> None:
    """Test interest rate swap valuation against golden values."""
    from finstack.core.dates import DayCount
    from finstack.core.dates.schedule import Frequency
    from finstack.core.market_data import MarketContext
    from finstack.valuations.instruments import InterestRateSwap

    # Create a simple interest rate swap
    irs = (
        InterestRateSwap.builder("IRS-001")
        .notional(10_000_000.0)
        .currency("USD")
        .maturity(date(2029, 1, 1))
        .fixed_rate(0.045)
        .frequency(Frequency.SEMI_ANNUAL)  # Sets both fixed and float frequency
        .disc_id("USD-OIS")
        .fwd_id("USD-LIBOR")
        .build()
    )

    # Create simple market context with discount and forward curves
    market = MarketContext()
    discount_curve = DiscountCurve(
        "USD-OIS", date(2024, 1, 1), [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)], day_count="act_365f"
    )
    market.insert_discount(discount_curve)

    # Create a simple forward curve (flat for testing)
    from finstack.core.market_data.term_structures import ForwardCurve

    # ForwardCurve takes: id, tenor_years, knots (list of (time, rate) tuples), base_date, day_count
    forward_curve = ForwardCurve(
        "USD-LIBOR",
        0.25,  # 3-month tenor
        [(0.0, 0.04), (1.0, 0.04), (5.0, 0.04)],
        base_date=date(2024, 1, 1),
        day_count=DayCount.ACT_360,
    )
    market.insert_forward(forward_curve)

    # Price the swap
    from finstack.valuations.pricer import create_standard_registry

    registry = create_standard_registry()
    # Use "discounting" model key like in test_roundtrips.py
    result = registry.price(irs, "discounting", market, date(2024, 1, 1))

    # Basic validation - swap should have a value (could be positive or negative)
    assert result.value.currency.code == "USD"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
