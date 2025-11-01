"""Test suite for Python-Rust-Python roundtrip conversions.

This module tests that data can successfully roundtrip between Python and Rust
without loss of information or corruption.
"""

import datetime as dt

import pytest

from finstack.core.currency import Currency
from finstack.core.dates import DayCount, Frequency
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.core.money import Money


class TestCurrencyRoundtrips:
    """Test currency roundtrips."""

    def test_currency_code_roundtrip(self) -> None:
        """Currency code should roundtrip correctly."""
        original_code = "USD"
        currency = Currency(original_code)
        assert currency.code == original_code

    def test_multiple_currencies(self) -> None:
        """Multiple currency objects should maintain identity."""
        codes = ["USD", "EUR", "GBP", "JPY", "CHF"]
        currencies = [Currency(code) for code in codes]

        for currency, original_code in zip(currencies, codes, strict=False):
            assert currency.code == original_code


class TestMoneyRoundtrips:
    """Test Money roundtrips."""

    def test_money_amount_and_currency_roundtrip(self) -> None:
        """Money amount and currency should roundtrip."""
        original_amount = 1234567.89
        original_currency = "USD"

        money = Money(original_amount, Currency(original_currency))

        assert money.amount == pytest.approx(original_amount, rel=1e-9)
        assert money.currency.code == original_currency

    def test_money_formatting_and_parsing(self) -> None:
        """Money should format and maintain precision."""
        money = Money(999999.99, Currency("EUR"))
        formatted = money.format()

        # Should contain both amount and currency
        assert "EUR" in formatted
        assert "999999" in formatted


class TestMarketDataRoundtrips:
    """Test market data structure roundtrips."""

    def test_discount_curve_roundtrip(self) -> None:
        """Discount curve should preserve data through storage/retrieval."""
        curve_id = "USD-OIS"
        base_date = dt.date(2024, 1, 2)
        points = [(0.0, 1.0), (1.0, 0.97), (2.0, 0.94), (5.0, 0.85)]
        day_count = DayCount.ACT_365F

        curve = DiscountCurve(
            curve_id,
            base_date,
            points,
            day_count=day_count
        )

        # Store in market context and retrieve
        market = MarketContext()
        market.insert_discount(curve)

        retrieved = market.get_discount(curve_id)

        # Verify roundtrip
        assert retrieved.id == curve_id
        assert retrieved.base_date == base_date

    def test_market_context_multiple_curves(self) -> None:
        """Market context should handle multiple curves."""
        curves = {
            "USD-OIS": DiscountCurve("USD-OIS", dt.date(2024, 1, 2),
                                     [(0.0, 1.0), (1.0, 0.97)], day_count=DayCount.ACT_365F),
            "EUR-OIS": DiscountCurve("EUR-OIS", dt.date(2024, 1, 2),
                                     [(0.0, 1.0), (1.0, 0.98)], day_count=DayCount.ACT_365F),
            "GBP-OIS": DiscountCurve("GBP-OIS", dt.date(2024, 1, 2),
                                     [(0.0, 1.0), (1.0, 0.96)], day_count=DayCount.ACT_365F),
        }

        market = MarketContext()
        for curve in curves.values():
            market.insert_discount(curve)

        # Retrieve and verify
        for curve_id in curves:
            retrieved = market.get_discount(curve_id)
            assert retrieved.id == curve_id


class TestInstrumentRoundtrips:
    """Test instrument serialization roundtrips."""

    def test_bond_builder_roundtrip(self) -> None:
        """Bond built with builder should preserve properties."""
        from finstack.valuations.instruments import Bond

        bond = Bond.builder("BOND_001") \
            .notional(1_000_000.0) \
            .currency("USD") \
            .coupon_rate(0.05) \
            .frequency("semiannual") \
            .maturity(dt.date(2029, 6, 15)) \
            .disc_id("USD-OIS") \
            .build()

        # Verify properties are accessible
        assert bond.id == "BOND_001"
        assert bond.notional.amount == pytest.approx(1_000_000.0)
        assert bond.notional.currency.code == "USD"

    def test_swap_builder_roundtrip(self) -> None:
        """IRS built with builder should preserve properties."""
        from finstack.valuations.instruments import IRS

        irs = IRS.builder("SWAP_001") \
            .notional(10_000_000.0) \
            .currency("USD") \
            .fixed_rate(0.03) \
            .float_spread_bp(25.0) \
            .frequency("quarterly") \
            .maturity(dt.date(2029, 1, 15)) \
            .disc_id("USD-OIS") \
            .fwd_id("USD-LIBOR-3M") \
            .build()

        assert irs.id == "SWAP_001"
        assert irs.notional.amount == pytest.approx(10_000_000.0)


class TestStatementModelRoundtrips:
    """Test statement model roundtrips."""

    def test_simple_model_build_and_evaluate(self) -> None:
        """Statement model should evaluate and return accessible results."""
        from finstack.core.dates import PeriodId
        from finstack.statements import Evaluator, ModelBuilder
        from finstack.statements.types import AmountOrScalar

        builder = ModelBuilder.new("Test Model")
        builder.periods("2025Q1..Q2", "2025Q1")

        # Add a simple value
        builder.value("revenue", [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1_000_000.0)),
        ])

        # Add computed value
        builder.compute("double_revenue", "revenue * 2")

        model = builder.build()

        # Evaluate
        evaluator = Evaluator.new()
        results = evaluator.evaluate(model)

        # Retrieve and verify roundtrip
        q1 = PeriodId.quarter(2025, 1)
        revenue = results.get("revenue", q1)
        double_revenue = results.get("double_revenue", q1)

        assert revenue == pytest.approx(1_000_000.0)
        assert double_revenue == pytest.approx(2_000_000.0)


class TestCalibrationRoundtrips:
    """Test calibration input/output roundtrips."""

    def test_calibration_quotes_roundtrip(self) -> None:
        """Calibration should accept quotes and return usable curve."""
        from finstack.valuations.calibration import DiscountCurveCalibrator, RatesQuote

        calibrator = DiscountCurveCalibrator(
            "USD-OIS",
            dt.date(2024, 1, 2),
            Currency("USD")
        )

        quotes = [
            RatesQuote.from_deposit(0.25, 0.0500, DayCount.ACT_360),
            RatesQuote.from_deposit(0.50, 0.0505, DayCount.ACT_360),
            RatesQuote.from_deposit(1.00, 0.0510, DayCount.ACT_360),
            RatesQuote.from_deposit(2.00, 0.0520, DayCount.ACT_360),
        ]

        curve, report = calibrator.calibrate(quotes)

        # Verify calibration succeeded
        assert report.success
        assert curve.id == "USD-OIS"

        # Curve should be usable in market context
        market = MarketContext()
        market.insert_discount(curve)
        retrieved = market.get_discount("USD-OIS")
        assert retrieved.id == curve.id


class TestPricingRoundtrips:
    """Test pricing input/output roundtrips."""

    def test_bond_pricing_roundtrip(self) -> None:
        """Bond should price and return accessible results."""
        from finstack.valuations import create_standard_registry
        from finstack.valuations.instruments import Bond

        # Setup market
        market = MarketContext()
        market.insert_discount(DiscountCurve(
            "USD-OIS",
            dt.date(2024, 1, 2),
            [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.70)],
            day_count=DayCount.ACT_365F
        ))

        # Create bond
        bond = Bond.builder("TEST_BOND") \
            .notional(1_000_000.0) \
            .currency("USD") \
            .coupon_rate(0.05) \
            .frequency("annual") \
            .maturity(dt.date(2029, 1, 2)) \
            .disc_id("USD-OIS") \
            .build()

        # Price
        registry = create_standard_registry()
        result = registry.price(bond, "discounting", market, as_of=dt.date(2024, 1, 2))

        # Verify result is accessible
        assert result.present_value is not None
        assert result.present_value.amount > 0
        assert result.present_value.currency.code == "USD"


class TestDateRoundtrips:
    """Test date handling roundtrips."""

    def test_date_adjustment_roundtrip(self) -> None:
        """Date adjustment should return valid dates."""
        from finstack.core.dates import BusinessDayConvention, adjust, get_calendar

        calendar = get_calendar("usny")
        original_date = dt.date(2024, 7, 4)  # US Independence Day (holiday)

        adjusted = adjust(original_date, BusinessDayConvention.FOLLOWING, calendar)

        # Adjusted date should be a valid date object
        assert isinstance(adjusted, dt.date)
        assert adjusted >= original_date  # FOLLOWING convention

    def test_schedule_generation_roundtrip(self) -> None:
        """Schedule generation should produce valid dates."""
        from finstack.core.dates import ScheduleBuilder, get_calendar

        calendar = get_calendar("usny")

        schedule = ScheduleBuilder.new(
            dt.date(2024, 1, 15),
            dt.date(2024, 12, 15)
        ).frequency(Frequency.QUARTERLY) \
         .adjust_with("modified_following", calendar) \
         .build()

        dates = list(schedule.dates)

        # Should have start, quarterly dates, and end
        assert len(dates) >= 5  # At least start + 4 quarters
        assert all(isinstance(d, dt.date) for d in dates)
        assert dates[0] == dt.date(2024, 1, 15)
        assert dates[-1] == dt.date(2024, 12, 15)


class TestNumericalPrecision:
    """Test numerical precision in roundtrips."""

    def test_high_precision_money(self) -> None:
        """High precision amounts should be preserved."""
        precise_amount = 123456789.123456789
        money = Money(precise_amount, Currency("USD"))

        # Precision depends on whether Decimal or f64 is used
        # Test that we don't lose too much precision
        assert money.amount == pytest.approx(precise_amount, rel=1e-6)

    def test_curve_interpolation_consistency(self) -> None:
        """Curve interpolation should be consistent."""
        curve = DiscountCurve(
            "TEST",
            dt.date(2024, 1, 2),
            [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (5.0, 0.80)],
            day_count=DayCount.ACT_365F,
            interp="linear"
        )

        # Multiple queries at the same point should give same result
        queries = [curve.df(1.5) for _ in range(10)]
        assert all(abs(q - queries[0]) < 1e-10 for q in queries)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])

