"""Test suite for Python-Rust-Python roundtrip conversions.

This module tests that data can successfully roundtrip between Python and Rust
without loss of information or corruption.
"""

import datetime as dt

from finstack.core.currency import Currency
from finstack.core.dates import (
    BusinessDayConvention,
    DayCount,
    DayCountContext,
    DayCountContextState,
    Frequency,
    PeriodId,
    ScheduleBuilder,
    ScheduleSpec,
    StubKind,
    adjust,
)
from finstack.core.market_data import DiscountCurve, MarketContext
from finstack.core.money import Money
from finstack.statements.types import AmountOrScalar
from finstack.valuations.instruments import Bond, InterestRateSwap
from finstack.valuations.pricer import create_standard_registry
import pytest

from finstack.core import get_calendar
from finstack.statements import Evaluator, ModelBuilder
from finstack.valuations import calibration as cal


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

        curve = DiscountCurve(curve_id, base_date, points, day_count=day_count)

        # Store in market context and retrieve
        market = MarketContext()
        market.insert_discount(curve)

        retrieved = market.discount(curve_id)

        # Verify roundtrip
        assert retrieved.id == curve_id
        assert retrieved.base_date == base_date

    def test_market_context_multiple_curves(self) -> None:
        """Market context should handle multiple curves."""
        curves = {
            "USD-OIS": DiscountCurve(
                "USD-OIS", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.97)], day_count=DayCount.ACT_365F
            ),
            "EUR-OIS": DiscountCurve(
                "EUR-OIS", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.98)], day_count=DayCount.ACT_365F
            ),
            "GBP-OIS": DiscountCurve(
                "GBP-OIS", dt.date(2024, 1, 2), [(0.0, 1.0), (1.0, 0.96)], day_count=DayCount.ACT_365F
            ),
        }

        market = MarketContext()
        for curve in curves.values():
            market.insert_discount(curve)

        # Retrieve and verify
        for curve_id in curves:
            retrieved = market.discount(curve_id)
            assert retrieved.id == curve_id


class TestInstrumentRoundtrips:
    """Test instrument serialization roundtrips."""

    def test_bond_builder_roundtrip(self) -> None:
        """Bond built with builder should preserve properties."""
        bond = (
            Bond.builder("BOND_001")
            .notional(1_000_000.0)
            .currency("USD")
            .coupon_rate(0.05)
            .frequency("semiannual")
            .maturity(dt.date(2029, 6, 15))
            .disc_id("USD-OIS")
            .build()
        )

        # Verify properties are accessible
        assert bond.instrument_id == "BOND_001"
        assert bond.notional.amount == pytest.approx(1_000_000.0)
        assert bond.notional.currency.code == "USD"

    def test_swap_builder_roundtrip(self) -> None:
        """IRS built with builder should preserve properties."""
        irs = (
            InterestRateSwap.builder("SWAP_001")
            .notional(10_000_000.0)
            .currency("USD")
            .fixed_rate(0.03)
            .float_spread_bp(25.0)
            .frequency(Frequency.QUARTERLY)  # Sets both fixed and float frequency
            .maturity(dt.date(2029, 1, 15))
            .disc_id("USD-OIS")
            .fwd_id("USD-LIBOR-3M")
            .build()
        )

        assert irs.instrument_id == "SWAP_001"
        assert irs.notional.amount == pytest.approx(10_000_000.0)


class TestScheduleAndDayCountDtos:
    """Test JSON DTO helpers for schedules and day-count contexts."""

    def test_schedule_spec_json_roundtrip(self) -> None:
        """ScheduleSpec should provide a stable JSON representation."""
        spec = ScheduleSpec(
            dt.date(2025, 1, 15),
            dt.date(2025, 4, 15),
            Frequency.MONTHLY,
            stub=StubKind.NONE,
            business_day_convention=BusinessDayConvention.FOLLOWING,
            calendar_id="target2",
        )
        json_payload = spec.to_json()
        restored = ScheduleSpec.from_json(json_payload)
        schedule = restored.build()
        assert len(schedule.dates) == 4

    def test_daycount_context_state_roundtrip(self) -> None:
        """DayCountContextState should rebuild runtime contexts."""
        ctx = DayCountContext(calendar=get_calendar("target2"), frequency=Frequency.MONTHLY)
        state = ctx.to_state()
        payload = state.to_json()
        restored = DayCountContextState.from_json(payload)
        restored_ctx = restored.to_context()

        start = dt.date(2025, 1, 2)
        end = dt.date(2025, 1, 12)
        fraction = DayCount.BUS_252.year_fraction(start, end, restored_ctx)
        assert fraction > 0.0


class TestStatementModelRoundtrips:
    """Test statement model roundtrips."""

    def test_simple_model_build_and_evaluate(self) -> None:
        """Statement model should evaluate and return accessible results."""
        builder = ModelBuilder.new("Test Model")
        builder.periods("2025Q1..Q2", "2025Q1")

        # Add a simple value for all periods
        builder.value(
            "revenue",
            [
                (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1_000_000.0)),
                (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(1_000_000.0)),
            ],
        )

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
        quotes = [
            cal.RatesQuote.deposit("DEPO-1", "USD-DEPOSIT", dt.date(2024, 4, 2), 0.0500),
            cal.RatesQuote.deposit("DEPO-2", "USD-DEPOSIT", dt.date(2024, 7, 2), 0.0505),
            cal.RatesQuote.deposit("DEPO-3", "USD-DEPOSIT", dt.date(2025, 1, 2), 0.0510),
            cal.RatesQuote.deposit("DEPO-4", "USD-DEPOSIT", dt.date(2026, 1, 2), 0.0520),
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
                },
            }
        ]

        market, report, _step_reports = cal.execute_calibration_v2(
            "plan_roundtrip_discount",
            quote_sets,
            steps,
        )

        assert report.success
        retrieved = market.discount("USD-OIS")
        assert retrieved.id == "USD-OIS"


class TestPricingRoundtrips:
    """Test pricing input/output roundtrips."""

    def test_bond_pricing_roundtrip(self) -> None:
        """Bond should price and return accessible results."""
        # Setup market
        market = MarketContext()
        market.insert_discount(
            DiscountCurve(
                "USD-OIS",
                dt.date(2024, 1, 2),
                [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85), (10.0, 0.70)],
                day_count=DayCount.ACT_365F,
            )
        )

        # Create bond
        bond = (
            Bond.builder("TEST_BOND")
            .notional(1_000_000.0)
            .currency("USD")
            .coupon_rate(0.05)
            .frequency("annual")
            .maturity(dt.date(2029, 1, 2))
            .disc_id("USD-OIS")
            .build()
        )

        # Price
        registry = create_standard_registry()
        result = registry.price(bond, "discounting", market, as_of=dt.date(2024, 1, 2))

        # Verify result is accessible
        assert result.value is not None
        assert result.value.amount > 0
        assert result.value.currency.code == "USD"


class TestDateRoundtrips:
    """Test date handling roundtrips."""

    def test_date_adjustment_roundtrip(self) -> None:
        """Date adjustment should return valid dates."""
        calendar = get_calendar("usny")
        original_date = dt.date(2024, 7, 4)  # US Independence Day (holiday)

        adjusted = adjust(original_date, BusinessDayConvention.FOLLOWING, calendar)

        # Adjusted date should be a valid date object
        assert isinstance(adjusted, dt.date)
        assert adjusted >= original_date  # FOLLOWING convention

    def test_schedule_generation_roundtrip(self) -> None:
        """Schedule generation should produce valid dates."""
        calendar = get_calendar("usny")

        schedule = (
            ScheduleBuilder.new(dt.date(2024, 1, 15), dt.date(2024, 12, 15))
            .frequency(Frequency.QUARTERLY)
            .adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, calendar)
            .build()
        )

        dates = list(schedule.dates)

        # Should have start, quarterly dates, and end
        assert len(dates) >= 5  # At least start + 4 quarters
        assert all(isinstance(d, dt.date) for d in dates)
        # Dates may be adjusted due to business day convention
        assert dates[0] >= dt.date(2024, 1, 15)
        assert dates[-1] >= dt.date(2024, 12, 15)  # May be adjusted forward


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
            interp="linear",
        )

        # Multiple queries at the same point should give same result
        queries = [curve.df(1.5) for _ in range(10)]
        assert all(abs(q - queries[0]) < 1e-10 for q in queries)


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
