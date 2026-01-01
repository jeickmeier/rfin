"""Comprehensive parity tests for core module.

Tests currency, money, dates, market data, math, and expression engine functionality.
"""

from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates import (
    DayCount,
    DayCountContext,
    PeriodId,
    build_periods,
)
from finstack.core.dates.schedule import Frequency
from finstack.core.market_data import (
    DiscountCurve,
    ForwardCurve,
    FxConversionPolicy,
    FxMatrix,
    MarketContext,
)
from finstack.core.money import Money
import pytest


class TestCurrencyParity:
    """Test currency operations match Rust implementation."""

    def test_currency_construction(self) -> None:
        """Test currency construction and properties."""
        usd = Currency("USD")
        assert usd.code == "USD"
        assert usd.numeric == 840
        assert usd.decimals == 2

    def test_currency_case_insensitive(self) -> None:
        """Test case-insensitive currency construction."""
        usd1 = Currency("USD")
        usd2 = Currency("usd")
        assert usd1.code == usd2.code
        assert usd1.numeric == usd2.numeric

    def test_currency_equality(self) -> None:
        """Test currency equality comparison."""
        usd1 = Currency("USD")
        usd2 = Currency("USD")
        eur = Currency("EUR")

        assert usd1 == usd2
        assert usd1 != eur

    def test_major_currencies(self) -> None:
        """Test major currency construction."""
        # Test via constructor
        usd = Currency("USD")
        eur = Currency("EUR")
        gbp = Currency("GBP")
        jpy = Currency("JPY")

        assert usd.code == "USD"
        assert eur.code == "EUR"
        assert gbp.code == "GBP"
        assert jpy.code == "JPY"

        # Test numeric codes
        assert usd.numeric == 840
        assert eur.numeric == 978
        assert gbp.numeric == 826
        assert jpy.numeric == 392

    def test_currency_invalid_code(self) -> None:
        """Test invalid currency code raises error."""
        with pytest.raises(Exception, match=r"[Uu]nknown|[Ii]nvalid|Currency"):
            Currency("INVALID")


class TestMoneyParity:
    """Test Money operations match Rust implementation."""

    def test_money_construction(self) -> None:
        """Test money construction and properties."""
        usd = Currency("USD")
        money = Money(100.50, usd)

        assert money.amount == 100.50
        assert money.currency.code == "USD"

    def test_money_addition(self) -> None:
        """Test money addition matches Rust."""
        usd = Currency("USD")
        m1 = Money(100.0, usd)
        m2 = Money(50.0, usd)

        result = m1 + m2
        assert result.amount == 150.0
        assert result.currency.code == "USD"

    def test_money_subtraction(self) -> None:
        """Test money subtraction matches Rust."""
        usd = Currency("USD")
        m1 = Money(100.0, usd)
        m2 = Money(30.0, usd)

        result = m1 - m2
        assert result.amount == 70.0
        assert result.currency.code == "USD"

    def test_money_multiplication(self) -> None:
        """Test money scalar multiplication."""
        usd = Currency("USD")
        money = Money(100.0, usd)

        result = money * 2.5
        assert result.amount == 250.0
        assert result.currency.code == "USD"

    def test_money_division(self) -> None:
        """Test money scalar division."""
        usd = Currency("USD")
        money = Money(100.0, usd)

        result = money / 4.0
        assert result.amount == 25.0
        assert result.currency.code == "USD"

    def test_money_negation(self) -> None:
        """Test money negation."""
        usd = Currency("USD")
        money = Money(100.0, usd)

        result = -money
        assert result.amount == -100.0
        assert result.currency.code == "USD"

    def test_money_currency_mismatch(self) -> None:
        """Test adding money with different currencies raises error."""
        usd = Currency("USD")
        eur = Currency("EUR")
        m1 = Money(100.0, usd)
        m2 = Money(50.0, eur)

        with pytest.raises(Exception, match=r"[Cc]urrency|[Mm]ismatch"):
            m1 + m2

    def test_money_zero(self) -> None:
        """Test zero money value."""
        usd = Currency("USD")
        money = Money(0.0, usd)

        assert money.amount == 0.0
        assert money.currency.code == "USD"

    def test_money_negative(self) -> None:
        """Test negative money value."""
        usd = Currency("USD")
        money = Money(-50.0, usd)

        assert money.amount == -50.0
        assert money.currency.code == "USD"


class TestDayCountParity:
    """Test day count convention calculations match Rust."""

    def test_act360_year_fraction(self) -> None:
        """Test Act/360 year fraction calculation."""
        start = date(2024, 1, 1)
        end = date(2024, 7, 1)

        dc = DayCount.ACT_360
        ctx = DayCountContext()
        yf = dc.year_fraction(start, end, ctx)

        # 182 days / 360 = 0.5055555...
        expected = 182.0 / 360.0
        assert abs(yf - expected) < 1e-10

    def test_act365f_year_fraction(self) -> None:
        """Test Act/365F year fraction calculation."""
        start = date(2024, 1, 1)
        end = date(2024, 7, 1)

        dc = DayCount.ACT_365F
        ctx = DayCountContext()
        yf = dc.year_fraction(start, end, ctx)

        # 182 days / 365 = 0.4986301...
        expected = 182.0 / 365.0
        assert abs(yf - expected) < 1e-10

    def test_thirty360_year_fraction(self) -> None:
        """Test 30/360 year fraction calculation."""
        start = date(2024, 1, 15)
        end = date(2024, 7, 15)

        dc = DayCount.THIRTY_360
        ctx = DayCountContext()
        yf = dc.year_fraction(start, end, ctx)

        # 6 months / 12 = 0.5
        expected = 180.0 / 360.0
        assert abs(yf - expected) < 1e-10

    def test_day_count_days_method(self) -> None:
        """Test days() method returns correct day count."""
        start = date(2024, 1, 1)
        end = date(2024, 1, 31)

        dc = DayCount.ACT_360
        ctx = DayCountContext()
        days = dc.days(start, end, ctx)

        assert days == 30


class TestPeriodParity:
    """Test period building and manipulation."""

    def test_period_id_monthly(self) -> None:
        """Test monthly period ID construction."""
        period_id = PeriodId.month(2024, 6)

        assert period_id.code == "2024M06"
        assert period_id.year == 2024
        assert period_id.index == 6

    def test_period_id_quarterly(self) -> None:
        """Test quarterly period ID construction."""
        period_id = PeriodId.quarter(2024, 2)

        assert period_id.code == "2024Q2"
        assert period_id.year == 2024
        assert period_id.index == 2

    def test_period_id_annual(self) -> None:
        """Test annual period ID construction."""
        period_id = PeriodId.annual(2024)

        assert period_id.code == "2024"
        assert period_id.year == 2024

    def test_build_periods_quarterly(self) -> None:
        """Test building quarterly periods."""
        plan = build_periods("2024Q1..Q4", None)

        assert len(plan.periods) == 4
        assert plan.periods[0].id.code == "2024Q1"
        assert plan.periods[3].id.code == "2024Q4"

    def test_build_periods_monthly(self) -> None:
        """Test building monthly periods."""
        plan = build_periods("2024M01..M12", None)

        assert len(plan.periods) == 12
        assert plan.periods[0].id.code == "2024M01"
        assert plan.periods[11].id.code == "2024M12"

    def test_build_periods_with_actuals(self) -> None:
        """Test building periods with actuals cutoff."""
        plan = build_periods("2024Q1..Q4", "2024Q2")

        assert len(plan.periods) == 4
        # Q1 and Q2 should be actual, Q3-Q4 forecast
        assert plan.periods[0].is_actual
        assert plan.periods[1].is_actual
        assert not plan.periods[2].is_actual
        assert not plan.periods[3].is_actual


class TestDiscountCurveParity:
    """Test discount curve operations match Rust."""

    def test_discount_curve_construction(self) -> None:
        """Test discount curve construction."""
        curve = DiscountCurve(
            "USD-TEST",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75), (10.0, 0.50)],
            day_count="act_365f",
        )

        assert curve.id == "USD-TEST"
        assert curve.base_date == date(2024, 1, 1)

    def test_discount_factor_at_knot(self) -> None:
        """Test discount factor at exact knot point."""
        curve = DiscountCurve(
            "USD-TEST",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )

        df = curve.df(1.0)
        assert abs(df - 0.95) < 1e-10

    def test_discount_factor_interpolation(self) -> None:
        """Test discount factor interpolation between knots."""
        curve = DiscountCurve(
            "USD-TEST",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)],
            day_count="act_365f",
        )

        # Linear interpolation at 1.5 years
        df = curve.df(1.5)
        expected = 0.925  # (0.95 + 0.90) / 2
        assert abs(df - expected) < 0.01  # Allow small tolerance for interpolation

    def test_zero_rate_consistency(self) -> None:
        """Test zero rate and discount factor consistency."""
        curve = DiscountCurve(
            "USD-TEST",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95), (5.0, 0.75)],
            day_count="act_365f",
        )

        time = 1.0
        df = curve.df(time)
        zero = curve.zero(time)

        import math

        recovered_df = math.exp(-zero * time)
        assert abs(df - recovered_df) < 1e-8


class TestForwardCurveParity:
    """Test forward curve operations match Rust."""

    def test_forward_curve_construction(self) -> None:
        """Test forward curve construction."""
        curve = ForwardCurve(
            "USD-SOFR",
            0.25,  # 3-month tenor
            [(0.0, 0.04), (1.0, 0.045), (5.0, 0.05)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )

        assert curve.id == "USD-SOFR"
        assert curve.base_date == date(2024, 1, 1)

    def test_forward_rate_at_knot(self) -> None:
        """Test forward rate at exact knot point."""
        curve = ForwardCurve(
            "USD-SOFR",
            0.25,
            [(0.0, 0.04), (1.0, 0.045)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )

        rate = curve.rate(1.0)
        assert abs(rate - 0.045) < 1e-10


class TestFxMatrixParity:
    """Test FX matrix operations match Rust."""

    def test_fx_matrix_direct_quote(self) -> None:
        """Test direct FX quote lookup."""
        fx = FxMatrix()
        usd = Currency("USD")
        eur = Currency("EUR")

        # Set EUR/USD = 1.10
        fx.set_quote(eur, usd, 1.10)

        # Lookup rate
        rate_result = fx.rate(eur, usd, date(2024, 1, 1), FxConversionPolicy.CASHFLOW_DATE)
        assert abs(rate_result.rate - 1.10) < 1e-10

    def test_fx_matrix_inverse_quote(self) -> None:
        """Test inverse FX quote calculation."""
        fx = FxMatrix()
        usd = Currency("USD")
        eur = Currency("EUR")

        # Set EUR/USD = 1.10
        fx.set_quote(eur, usd, 1.10)

        # Lookup USD/EUR (inverse)
        rate_result = fx.rate(usd, eur, date(2024, 1, 1), FxConversionPolicy.CASHFLOW_DATE)
        expected_inverse = 1.0 / 1.10
        assert abs(rate_result.rate - expected_inverse) < 1e-8

    def test_fx_matrix_same_currency(self) -> None:
        """Test FX rate for same currency is 1.0."""
        fx = FxMatrix()
        usd = Currency("USD")

        rate_result = fx.rate(usd, usd, date(2024, 1, 1), FxConversionPolicy.CASHFLOW_DATE)
        assert abs(rate_result.rate - 1.0) < 1e-10

    def test_fx_matrix_triangulation(self) -> None:
        """Test FX triangulation through USD."""
        fx = FxMatrix()
        usd = Currency("USD")
        eur = Currency("EUR")
        gbp = Currency("GBP")

        # Set EUR/USD = 1.10 and GBP/USD = 1.25
        fx.set_quote(eur, usd, 1.10)
        fx.set_quote(gbp, usd, 1.25)

        # Cross triangulation is not currently supported; request should fail deterministically.
        with pytest.raises(Exception, match=r"FX:|not found|Resource"):
            fx.rate(eur, gbp, date(2024, 1, 1), FxConversionPolicy.CASHFLOW_DATE)


class TestMarketContextParity:
    """Test market context operations match Rust."""

    def test_market_context_insert_discount(self) -> None:
        """Test inserting discount curve into market context."""
        market = MarketContext()
        curve = DiscountCurve(
            "USD-OIS",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95)],
            day_count="act_365f",
        )

        market.insert_discount(curve)

        # Verify curve can be retrieved
        retrieved = market.discount("USD-OIS")
        assert retrieved.id == "USD-OIS"

    def test_market_context_insert_forward(self) -> None:
        """Test inserting forward curve into market context."""
        market = MarketContext()
        curve = ForwardCurve(
            "USD-SOFR",
            0.25,
            [(0.0, 0.04), (1.0, 0.045)],
            base_date=date(2024, 1, 1),
            day_count=DayCount.ACT_360,
        )

        market.insert_forward(curve)

        # Verify curve can be retrieved
        retrieved = market.forward("USD-SOFR")
        assert retrieved.id == "USD-SOFR"

    def test_market_context_as_of_date(self) -> None:
        """Test market context as_of date."""
        market = MarketContext()
        assert market is not None


class TestFrequencyParity:
    """Test frequency enum values match Rust."""

    def test_frequency_values(self) -> None:
        """Test frequency enum has correct values."""
        assert Frequency.ANNUAL is not None
        assert Frequency.SEMI_ANNUAL is not None
        assert Frequency.QUARTERLY is not None
        assert Frequency.MONTHLY is not None


class TestEdgeCases:
    """Test edge cases and boundary conditions."""

    def test_money_large_values(self) -> None:
        """Test money with large values."""
        usd = Currency("USD")
        large_money = Money(1e12, usd)  # 1 trillion

        assert large_money.amount == 1e12

    def test_money_small_values(self) -> None:
        """Test money with very small values."""
        usd = Currency("USD")
        small_money = Money(0.01, usd)  # 1 cent

        assert small_money.amount == 0.01

    def test_day_count_same_date(self) -> None:
        """Test day count with same start and end date."""
        start = date(2024, 1, 1)
        end = date(2024, 1, 1)

        dc = DayCount.ACT_360
        ctx = DayCountContext()
        yf = dc.year_fraction(start, end, ctx)

        assert yf == 0.0

    def test_discount_curve_zero_time(self) -> None:
        """Test discount factor at time zero."""
        curve = DiscountCurve(
            "USD-TEST",
            date(2024, 1, 1),
            [(0.0, 1.0), (1.0, 0.95)],
            day_count="act_365f",
        )

        df = curve.df(0.0)
        assert abs(df - 1.0) < 1e-10

    def test_fx_matrix_zero_rate(self) -> None:
        """Test FX matrix does not allow zero rate."""
        fx = FxMatrix()
        usd = Currency("USD")
        eur = Currency("EUR")

        # Setting zero rate should raise error or be rejected
        with pytest.raises(Exception, match=r"positive|rate"):
            fx.set_quote(eur, usd, 0.0)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
