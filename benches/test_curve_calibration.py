"""Benchmark: Calibrate discount curve with 50 pillars.

This benchmark measures the computational efficiency of curve bootstrapping,
which involves solving for discount factors from instrument quotes.
"""

from datetime import date

from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.calibration import (
    CalibrationStep,
    RatesQuote,
    execute_calibration_v2,
)
import pytest
from pytest_benchmark.fixture import BenchmarkFixture


def create_50_pillar_quotes() -> list[RatesQuote]:
    """Create realistic quote set for 50-pillar calibration."""
    quotes = []
    date(2024, 1, 1)

    # 1. Short end: 6 deposits (ON to 1Y)
    deposit_tenors = ["1D", "1W", "1M", "3M", "6M", "12M"]
    deposit_rates = [4.50, 4.55, 4.60, 4.70, 4.80, 4.90]
    for tenor, rate in zip(deposit_tenors, deposit_rates, strict=False):
        quotes.append(RatesQuote.deposit(tenor, rate / 100.0, "Act360", "Unadjusted"))

    # 2. Short FRAs: 6 FRAs (3x6, 6x9, 9x12, 12x15, 15x18, 18x21)
    fra_starts = ["3M", "6M", "9M", "12M", "15M", "18M"]
    fra_ends = ["6M", "9M", "12M", "15M", "18M", "21M"]
    fra_rates = [4.95, 5.00, 5.05, 5.10, 5.15, 5.20]
    for start, end, rate in zip(fra_starts, fra_ends, fra_rates, strict=False):
        quotes.append(RatesQuote.fra(start, end, rate / 100.0, "Act360"))

    # 3. Futures: 8 contracts (quarterly strip)
    future_dates = [
        date(2024, 3, 20),
        date(2024, 6, 19),
        date(2024, 9, 18),
        date(2024, 12, 18),
        date(2025, 3, 19),
        date(2025, 6, 18),
        date(2025, 9, 17),
        date(2025, 12, 17),
    ]
    future_prices = [95.00, 94.90, 94.80, 94.70, 94.60, 94.50, 94.40, 94.30]
    for settle_date, price in zip(future_dates, future_prices, strict=False):
        quotes.append(RatesQuote.future(settle_date, price, "3M", "Act360"))

    # 4. Swaps: 30 swaps (2Y to 50Y)
    swap_tenors = [
        "2Y",
        "3Y",
        "4Y",
        "5Y",
        "6Y",
        "7Y",
        "8Y",
        "9Y",
        "10Y",
        "11Y",
        "12Y",
        "13Y",
        "14Y",
        "15Y",
        "16Y",
        "17Y",
        "18Y",
        "19Y",
        "20Y",
        "21Y",
        "22Y",
        "23Y",
        "24Y",
        "25Y",
        "27Y",
        "30Y",
        "35Y",
        "40Y",
        "45Y",
        "50Y",
    ]
    # Upward-sloping swap curve
    swap_rates = [
        5.25,
        5.30,
        5.35,
        5.40,
        5.45,
        5.48,
        5.50,
        5.52,
        5.54,
        5.56,
        5.58,
        5.60,
        5.61,
        5.62,
        5.63,
        5.64,
        5.65,
        5.66,
        5.67,
        5.68,
        5.69,
        5.70,
        5.71,
        5.72,
        5.74,
        5.76,
        5.80,
        5.82,
        5.84,
        5.85,
    ]
    for tenor, rate in zip(swap_tenors, swap_rates, strict=False):
        quotes.append(
            RatesQuote.swap(
                tenor,
                rate / 100.0,
                "SemiAnnual",  # Fixed leg
                "Quarterly",  # Float leg
                "Act360",  # Fixed day count
                "Act360",  # Float day count
            )
        )

    return quotes


class TestCurveCalibrationBenchmarks:
    """Benchmarks for curve calibration operations."""

    def test_bench_calibrate_50_pillar_curve(self, benchmark: BenchmarkFixture) -> None:
        """Benchmark: Calibrate discount curve with 50 instruments."""
        base_date = date(2024, 1, 1)
        quotes = create_50_pillar_quotes()

        # Create calibration step
        step = CalibrationStep(
            kind="discount",
            curve_id="USD.OIS",
            base_date=base_date,
            quotes=quotes,
        )

        def calibrate_curve() -> DiscountCurve:
            # Execute calibration
            result = execute_calibration_v2([step])

            # Extract the calibrated curve
            curve = result.market.getDiscountCurve("USD.OIS")
            return curve

        # Run benchmark
        curve = benchmark(calibrate_curve)

        # Verify curve has reasonable size
        # Note: We can't directly access knots, but we can verify it exists
        assert curve is not None
        assert curve.curveId == "USD.OIS"

    def test_bench_calibrate_10_pillar_curve(self, benchmark: BenchmarkFixture) -> None:
        """Benchmark: Calibrate smaller 10-pillar curve (baseline)."""
        base_date = date(2024, 1, 1)

        # Create 10-instrument quote set
        quotes = []

        # 3 deposits
        quotes.append(RatesQuote.deposit("1M", 0.045, "Act360", "Unadjusted"))
        quotes.append(RatesQuote.deposit("3M", 0.047, "Act360", "Unadjusted"))
        quotes.append(RatesQuote.deposit("6M", 0.048, "Act360", "Unadjusted"))

        # 7 swaps
        swap_tenors = ["1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "30Y"]
        swap_rates = [0.049, 0.0525, 0.053, 0.054, 0.0548, 0.0554, 0.0576]
        for tenor, rate in zip(swap_tenors, swap_rates, strict=False):
            quotes.append(RatesQuote.swap(tenor, rate, "SemiAnnual", "Quarterly", "Act360", "Act360"))

        step = CalibrationStep(
            kind="discount",
            curve_id="USD.OIS",
            base_date=base_date,
            quotes=quotes,
        )

        def calibrate_curve() -> DiscountCurve:
            result = execute_calibration_v2([step])
            return result.market.getDiscountCurve("USD.OIS")

        # Run benchmark
        curve = benchmark(calibrate_curve)
        assert curve is not None

    def test_bench_multi_curve_calibration(self, benchmark: BenchmarkFixture) -> None:
        """Benchmark: Calibrate 3 curves simultaneously (discount + 2 forwards)."""
        base_date = date(2024, 1, 1)

        # Step 1: Discount curve (OIS)
        ois_quotes = []
        ois_quotes.append(RatesQuote.deposit("1M", 0.045, "Act360", "Unadjusted"))
        ois_quotes.append(RatesQuote.deposit("3M", 0.047, "Act360", "Unadjusted"))
        for tenor, rate in [("1Y", 0.049), ("2Y", 0.0525), ("5Y", 0.054), ("10Y", 0.0554)]:
            ois_quotes.append(RatesQuote.swap(tenor, rate, "SemiAnnual", "Quarterly", "Act360", "Act360"))

        # Step 2: Forward curve 1 (SOFR)
        sofr_quotes = []
        for tenor, spread in [("1Y", 0.10), ("2Y", 0.12), ("5Y", 0.15), ("10Y", 0.18)]:
            # Forward curve calibration uses basis swaps or FRAs
            sofr_quotes.append(RatesQuote.fra("0M", tenor, 0.05 + spread / 100.0, "Act360"))

        # Step 3: Forward curve 2 (LIBOR)
        libor_quotes = []
        for tenor, spread in [("1Y", 0.20), ("2Y", 0.22), ("5Y", 0.25), ("10Y", 0.28)]:
            libor_quotes.append(RatesQuote.fra("0M", tenor, 0.05 + spread / 100.0, "Act360"))

        steps = [
            CalibrationStep(kind="discount", curve_id="USD.OIS", base_date=base_date, quotes=ois_quotes),
            CalibrationStep(kind="forward", curve_id="USD.SOFR", base_date=base_date, quotes=sofr_quotes),
            CalibrationStep(kind="forward", curve_id="USD.LIBOR", base_date=base_date, quotes=libor_quotes),
        ]

        def calibrate_all() -> MarketContext:
            result = execute_calibration_v2(steps)
            return result.market

        # Run benchmark
        market = benchmark(calibrate_all)

        # Verify all 3 curves exist
        assert market.getDiscountCurve("USD.OIS") is not None
        assert market.getForwardCurve("USD.SOFR") is not None
        assert market.getForwardCurve("USD.LIBOR") is not None


if __name__ == "__main__":
    pytest.main([__file__, "-v", "--benchmark-only"])
