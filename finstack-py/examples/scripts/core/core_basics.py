"""Minimal walkthrough of the finstack core Python bindings.

Run with:

    uv run python finstack-py/examples/core_basics.py

It assumes ``uv run maturin develop --release`` (or similar) has been executed so
that the compiled ``finstack`` module is available to Python.
"""

from __future__ import annotations

from datetime import date
import logging

from finstack.core.config import FinstackConfig
from finstack.core.currency import Currency
from finstack.core.dates import BusinessDayConvention, adjust, available_calendar_codes, get_calendar
from finstack.core.dates.daycount import DayCount, DayCountContext
from finstack.core.dates.imm import next_imm
from finstack.core.dates.periods import FiscalConfig, build_fiscal_periods, build_periods
from finstack.core.dates.schedule import Frequency, ScheduleBuilder
from finstack.core.dates.utils import add_months, date_to_days_since_epoch, days_in_month, last_day_of_month
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.dividends import DividendScheduleBuilder
from finstack.core.market_data.fx import FxConfig, FxConversionPolicy, FxMatrix
from finstack.core.market_data.scalars import MarketScalar, ScalarTimeSeries, SeriesInterpolation
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import BaseCorrelationCurve, DiscountCurve, HazardCurve
from finstack.core.money import Money

logger = logging.getLogger(__name__)


def show_currency() -> None:
    """Demonstrate basic ``Currency`` lookups and properties."""
    usd = Currency("USD")
    logger.info("Currency: %s %s %s", usd.code, usd.numeric, usd.decimals)

    gbp = Currency.from_numeric(826)
    logger.info("Lookup by numeric code: %s", gbp)

    logger.info("Total built-in currencies: %d", len(Currency.all()))


def show_money() -> None:
    """Show ``Money`` formatting, config scaling, and tuple round-trips."""
    usd = Currency("USD")
    amount = Money(1_234.567, usd)
    logger.info("Raw amount: %s", amount.format())

    config = FinstackConfig()
    config.set_output_scale(usd, 4)

    logger.info("Formatted with custom output scale: %s", amount.format_with_config(config))

    subtotal = amount + Money(10.0, usd)
    logger.info("Addition result: %s", subtotal.format())

    # Round-trip via tuple conversion (handy when working with pandas/polars).
    as_tuple = subtotal.to_tuple()
    logger.info("Tuple form: %s", as_tuple)

    reconstructed = Money.from_tuple(as_tuple)
    logger.info("Reconstructed equal to subtotal: %s", reconstructed == subtotal)


def show_calendars() -> None:
    """Show available calendars and basic business-day adjustments."""
    codes = available_calendar_codes()
    logger.info("Available calendar sample: %s ...", codes[:5])

    calendar = get_calendar("usny")
    logger.info("Calendar name: %s (ignores weekends: %s)", calendar.name, calendar.ignore_weekends)

    start = date(2025, 1, 4)  # Saturday
    adjusted = adjust(start, BusinessDayConvention.FOLLOWING, calendar)
    logger.info("Adjusted business day: %s", adjusted)

    dc_ctx = DayCountContext(calendar=calendar, frequency=Frequency.SEMI_ANNUAL)
    yf = DayCount.ACT_ACT_ISMA.year_fraction(date(2025, 1, 4), adjusted, dc_ctx)
    logger.info("Year fraction (Act/Act ISMA): %.6f", round(yf, 6))

    next_roll = next_imm(start)
    logger.info("Next IMM date after %s is %s", start, next_roll)


def show_schedule() -> None:
    """Generate a simple monthly schedule with business-day adjustment and EOM handling."""
    start = date(2025, 1, 15)
    end = date(2025, 7, 15)
    calendar = get_calendar("usny")

    schedule = (
        ScheduleBuilder.new(start, end)
        .frequency(Frequency.MONTHLY)
        .adjust_with(BusinessDayConvention.MODIFIED_FOLLOWING, calendar)
        .end_of_month(True)
        .build()
    )

    logger.info("Generated schedule (business-day adjusted, EOM):")
    for anchor in schedule.dates:
        logger.info("  %s", anchor)


def show_periods() -> None:
    """Build calendar and fiscal periods and log summaries."""
    plan = build_periods("2024Q1..Q3", actuals_until="2024Q2")
    logger.info("Calendar periods:")
    for period in plan.periods:
        logger.info("  %s: %s -> %s (actual=%s)", period.id.code, period.start, period.end, period.is_actual)

    fiscal_plan = build_fiscal_periods("2025Q1..Q4", FiscalConfig.US_FEDERAL, None)
    logger.info("US Federal fiscal periods:")
    for period in fiscal_plan.periods:
        logger.info("  %s: %s -> %s", period.id.code, period.start, period.end)


def show_utils() -> None:
    """Demonstrate date utilities such as add-months and month-end handling."""
    base = date(2025, 1, 31)
    logger.info("Add months: %s", add_months(base, 1))
    logger.info("Last day of month: %s", last_day_of_month(base))
    logger.info("Days in Feb 2024: %s", days_in_month(2024, 2))
    logger.info("Days since epoch: %s", date_to_days_since_epoch(base))


def show_market_data() -> None:
    """Construct market data primitives and aggregate them inside a context."""
    base = date(2024, 1, 2)
    usd = Currency("USD")

    discount = DiscountCurve(
        "USD-OIS",
        base,
        [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)],
        day_count=DayCount.ACT_365F,
        interp="monotone_convex",
    )

    hazard = HazardCurve(
        "CDX-IG",
        base,
        [(0.0, 0.01), (5.0, 0.015), (10.0, 0.02)],
        recovery_rate=0.4,
        currency=usd,
        day_count=DayCount.ACT_365F,
    )

    base_corr = BaseCorrelationCurve(
        "CDX-IG",
        [(3.0, 0.25), (7.0, 0.45), (10.0, 0.6)],
    )

    surface = VolSurface(
        "EQ-FLAT",
        expiries=[1.0, 2.0],
        strikes=[90.0, 100.0, 110.0],
        grid=[[0.2, 0.21, 0.22], [0.19, 0.2, 0.21]],
    )

    fx = FxMatrix(config=FxConfig(enable_triangulation=True))
    fx.set_quote(Currency("EUR"), usd, 1.1)
    fx_rate = fx.rate(Currency("EUR"), usd, base, FxConversionPolicy.CASHFLOW_DATE)

    scalar = MarketScalar.price(Money(188.25, usd))
    series = ScalarTimeSeries(
        "US-CPI",
        [(date(2023, 12, 31), 300.0), (date(2024, 1, 31), 301.5)],
        interpolation=SeriesInterpolation.LINEAR,
    )

    dividends_builder = DividendScheduleBuilder("AAPL-DIVS")
    dividends_builder.underlying("AAPL")
    dividends_builder.cash(date(2024, 2, 15), Money(0.24, usd))
    dividends_builder.cash(date(2024, 5, 15), Money(0.25, usd))
    dividends = dividends_builder.build()

    context = MarketContext()
    context.insert_discount(discount)
    context.insert_hazard(hazard)
    context.insert_base_correlation(base_corr)
    context.insert_surface(surface)
    context.insert_price("AAPL", scalar)
    context.insert_series(series)
    context.insert_dividends(dividends)
    context.insert_fx(fx)

    stats = context.stats()
    logger.info("Context stats: %s", stats)
    discount_curve = context.discount("USD-OIS")
    logger.info("Discount df(5y): %.4f", discount_curve.df(5.0))
    logger.info(
        "EUR/USD fx rate (triangulated=%s): %.4f",
        fx_rate.triangulated,
        fx_rate.rate,
    )


def main() -> None:
    """Run the full example sequence."""
    logging.basicConfig(level=logging.INFO, format="%(message)s")

    logger.info("=== Currency ===")
    show_currency()
    logger.info("")

    logger.info("=== Money ===")
    show_money()
    logger.info("")

    logger.info("=== Calendars ===")
    show_calendars()
    logger.info("")

    logger.info("=== Schedule Builder ===")
    show_schedule()
    logger.info("")

    logger.info("=== Periods ===")
    show_periods()
    logger.info("")

    logger.info("=== Date Utilities ===")
    show_utils()
    logger.info("")

    logger.info("=== Market Data ===")
    show_market_data()


if __name__ == "__main__":
    main()
