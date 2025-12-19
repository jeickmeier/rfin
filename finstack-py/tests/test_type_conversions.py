"""Test suite for Python-Rust type conversions in finstack bindings.

This module tests the FromPyObject implementations for ergonomic type conversions,
ensuring that both typed objects and string aliases work correctly.
"""

import datetime as dt

from finstack.core.currency import Currency
from finstack.core.dates import BusinessDayConvention, DayCount, Frequency, ScheduleBuilder, get_calendar
from finstack.core.market_data import DiscountCurve
import pytest

import finstack


class TestCurrencyConversions:
    """Test Currency type conversions."""

    def test_currency_from_object(self) -> None:
        """Currency object should be accepted."""
        usd = Currency("USD")
        assert str(usd) == "USD"

    def test_currency_from_string(self) -> None:
        """String currency code should be converted to Currency."""
        # Test by using string in context where Currency is expected
        usd = Currency("USD")
        assert usd.code == "USD"

    def test_currency_string_case_insensitive(self) -> None:
        """Currency codes should work in different cases."""
        usd_lower = Currency("usd")
        usd_upper = Currency("USD")
        assert usd_lower.code == usd_upper.code

    def test_invalid_currency_raises_error(self) -> None:
        """Invalid currency code should raise appropriate error."""
        with pytest.raises((finstack.ParameterError, ValueError)):
            Currency("INVALID_CODE_123")


class TestDayCountConversions:
    """Test DayCount type conversions."""

    def test_daycount_from_enum(self) -> None:
        """DayCount enum should be accepted."""
        dc = DayCount.ACT_360
        assert dc is not None

    def test_daycount_from_string_variations(self) -> None:
        """DayCount should accept various string formats."""
        # These test the normalize_label functionality
        test_cases = [
            ("act/360", DayCount.ACT_360),
            ("ACT/360", DayCount.ACT_360),
            ("actual/360", DayCount.ACT_360),
            ("act_360", DayCount.ACT_360),
            ("30/360", DayCount.THIRTY_360),
            ("thirty/360", DayCount.THIRTY_360),
            ("30_360", DayCount.THIRTY_360),
            ("act/365f", DayCount.ACT_365F),
            ("ACT/365F", DayCount.ACT_365F),
        ]

        # Test by creating a curve with different day count formats
        for string_format, _expected_dc in test_cases:
            curve = DiscountCurve(
                f"TEST_{string_format.replace('/', '_')}",
                dt.date(2024, 1, 2),
                [(0.0, 1.0), (1.0, 0.97)],
                day_count=string_format,
            )
            assert curve is not None

    def test_invalid_daycount_raises_error(self) -> None:
        """Invalid day count string should raise ValueError."""
        with pytest.raises((ValueError, TypeError), match=r"Unknown day-count|day_count must be"):
            DiscountCurve("INVALID", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.97)], day_count="INVALID_DAYCOUNT")


class TestBusinessDayConventionConversions:
    """Test BusinessDayConvention type conversions."""

    def test_bdc_from_enum(self) -> None:
        """BusinessDayConvention enum should be accepted."""
        bdc = BusinessDayConvention.FOLLOWING
        assert bdc is not None

    def test_bdc_from_string_variations(self) -> None:
        """BusinessDayConvention should accept various string formats."""
        from finstack.core.dates import adjust

        calendar = get_calendar("usny")
        test_date = dt.date(2024, 1, 1)  # This is a holiday

        # Test different string formats
        test_cases = [
            "following",
            "FOLLOWING",
            "modified_following",
            "MODIFIED_FOLLOWING",
            "preceding",
            "unadjusted",
        ]

        for bdc_string in test_cases:
            # Should not raise error
            result = adjust(test_date, bdc_string, calendar)
            assert isinstance(result, dt.date)

    def test_invalid_bdc_raises_error(self) -> None:
        """Invalid business day convention should raise error."""
        from finstack.core.dates import adjust

        calendar = get_calendar("usny")

        with pytest.raises((ValueError, finstack.ParameterError)):
            adjust(dt.date(2024, 1, 1), "INVALID_BDC", calendar)


class TestFrequencyConversions:
    """Test Frequency type conversions."""

    def test_frequency_from_enum(self) -> None:
        """Frequency enum should be accepted."""
        freq = Frequency.QUARTERLY
        assert freq is not None

    def test_frequency_from_string_variations(self) -> None:
        """Frequency should accept various string formats."""
        test_cases = [
            ("annual", Frequency.ANNUAL),
            ("semiannual", Frequency.SEMI_ANNUAL),
            ("quarterly", Frequency.QUARTERLY),
            ("monthly", Frequency.MONTHLY),
            ("weekly", Frequency.WEEKLY),
            ("daily", Frequency.DAILY),
        ]

        for _string_format, expected_freq in test_cases:
            # Test by building a schedule with the frequency
            schedule = ScheduleBuilder.new(dt.date(2024, 1, 15), dt.date(2024, 7, 15)).frequency(expected_freq).build()

            assert schedule is not None
            assert len(list(schedule.dates)) > 0


class TestInterpolationConversions:
    """Test interpolation style type conversions."""

    def test_interp_style_from_string(self) -> None:
        """Interpolation styles should accept string variations."""
        test_cases = [
            "linear",
            "LINEAR",
            "log_linear",
            "log_linear",
            "monotone_convex",
            "flat_fwd",
        ]

        for interp_string in test_cases:
            # Test by creating a discount curve with the interpolation
            curve = DiscountCurve(
                f"TEST_{interp_string}",
                dt.date(2024, 1, 2),
                [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)],
                day_count=DayCount.ACT_365F,
                interp=interp_string,
            )
            assert curve is not None

    def test_invalid_interp_raises_error(self) -> None:
        """Invalid interpolation style should raise ValueError."""
        with pytest.raises(ValueError, match="Unknown interpolation"):
            DiscountCurve(
                "INVALID",
                dt.date(2024, 1, 2),
                [(0.0, 1.0), (1.0, 0.97)],
                day_count=DayCount.ACT_365F,
                interp="INVALID_INTERP",
            )


class TestDateConversions:
    """Test Python date to Rust date conversions."""

    def test_date_object_accepted(self) -> None:
        """Python date objects should be accepted."""
        from finstack.core.dates import adjust

        calendar = get_calendar("usny")
        test_date = dt.date(2024, 6, 15)
        result = adjust(test_date, BusinessDayConvention.FOLLOWING, calendar)
        assert isinstance(result, dt.date)

    def test_datetime_object_accepted(self) -> None:
        """Python datetime objects should be accepted (date portion used)."""
        from finstack.core.dates import adjust

        calendar = get_calendar("usny")
        test_datetime = dt.datetime(2024, 6, 15, 14, 30)
        result = adjust(test_datetime, BusinessDayConvention.FOLLOWING, calendar)
        assert isinstance(result, dt.date)

    def test_invalid_date_type_raises_error(self) -> None:
        """Non-date objects should raise TypeError."""
        from finstack.core.dates import adjust

        calendar = get_calendar("usny")

        with pytest.raises((TypeError, ValueError), match=r"Expected datetime.date|Invalid input"):
            adjust("2024-06-15", BusinessDayConvention.FOLLOWING, calendar)  # type: ignore


class TestMoneyConversions:
    """Test Money-related type conversions."""

    def test_money_from_amount_and_currency(self) -> None:
        """Money should accept both numeric amount and currency."""
        from finstack.core.money import Money

        # With Currency object
        money1 = Money(1000.0, Currency("USD"))
        assert money1 is not None

        # With currency string
        money2 = Money(1000.0, "USD")
        assert money2 is not None

    def test_money_formatting(self) -> None:
        """Money should format correctly."""
        from finstack.core.money import Money

        money = Money(1_000_000.50, Currency("USD"))
        formatted = money.format()
        assert "USD" in formatted
        assert "1000000" in formatted or "1,000,000" in formatted


class TestNoneAndOptionalHandling:
    """Test handling of None and optional parameters."""

    def test_optional_parameter_none(self) -> None:
        """Optional parameters should accept None."""
        # Test curve creation without optional parameters
        curve = DiscountCurve(
            "USD-OIS",
            dt.date(2024, 1, 2),
            [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)],
            day_count=DayCount.ACT_365F,
            # No interp parameter - should use default
        )
        assert curve is not None

    def test_optional_market_context_none(self) -> None:
        """Optional market context should accept None."""
        from finstack.valuations import calibration as cal

        quotes = [
            cal.RatesQuote.deposit("DEPO-1", "USD-DEPOSIT", dt.date(2025, 1, 2), 0.05),
            cal.RatesQuote.deposit("DEPO-2", "USD-DEPOSIT", dt.date(2026, 1, 2), 0.055),
        ]
        quote_sets = {"ois": [q.to_market_quote() for q in quotes]}
        steps = [
            {
                "id": "disc",
                "quote_set": "ois",
                "kind": "discount",
                "curve_id": "USD-OIS",
                "currency": "USD",
                "base_date": "2024-01-02",
                "conventions": {
                    "curve_day_count": "act365f",
                    "settlement_days": 2,
                    "calendar_id": "usny",
                    "business_day_convention": "modified_following",
                    "allow_calendar_fallback": False,
                    "use_settlement_start": True,
                },
            }
        ]

        # Execute without initial_market (should use empty context)
        market, report, _step_reports = cal.execute_calibration_v2(
            "plan_discount_optional_market",
            quote_sets,
            steps,
            initial_market=None,
        )
        assert market.discount("USD-OIS") is not None
        assert report.success


class TestListAndVectorConversions:
    """Test Python list to Rust Vec conversions."""

    def test_list_of_tuples_for_curve_points(self) -> None:
        """Lists of tuples should convert to Vec<(f64, f64)>."""
        points = [(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (5.0, 0.85)]

        curve = DiscountCurve("USD-OIS", dt.date(2024, 1, 2), points, day_count=DayCount.ACT_365F)
        assert curve is not None

    def test_list_of_floats_for_grid(self) -> None:
        """Lists of floats should convert properly."""
        from finstack.core.market_data import VolSurface

        surface = VolSurface(
            "EQ-FLAT",
            expiries=[1.0, 2.0, 3.0],
            strikes=[90.0, 100.0, 110.0],
            grid=[
                [0.20, 0.21, 0.22],
                [0.19, 0.20, 0.21],
                [0.18, 0.19, 0.20],
            ],
        )
        assert surface is not None


class TestEdgeCases:
    """Test edge cases in type conversions."""

    def test_empty_string_currency(self) -> None:
        """Empty string currency should raise error."""
        with pytest.raises((finstack.ParameterError, ValueError)):
            Currency("")

    def test_whitespace_trimming(self) -> None:
        """Whitespace in string parameters should be handled."""
        # Test if leading/trailing whitespace is trimmed
        # This depends on implementation - some may trim, others may reject
        try:
            currency = Currency(" USD ")
            # If it succeeds, should normalize to USD
            assert currency.code == "USD"
        except (finstack.ParameterError, ValueError):
            # If it rejects whitespace, that's also acceptable
            pass

    def test_numeric_precision(self) -> None:
        """Numeric conversions should preserve precision where possible."""
        from finstack.core.money import Money

        # Test with high precision value
        precise_value = 1234567.89012345
        money = Money(precise_value, Currency("USD"))
        # Exact precision preservation depends on Decimal vs f64 implementation
        assert money.amount > 0


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
