"""CMS Option Example.

Demonstrates pricing and analysis of Constant Maturity Swap (CMS) options.
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import CmsOption
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for CMS option pricing."""
    market = MarketContext()

    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (5.0, 0.80), (10.0, 0.65)],
    )
    market.insert(disc_curve)

    # Swaption volatility surface (toy)
    vol_surface = VolSurface(
        "USD.SWAPTION.VOL",
        expiries=[1.0, 2.0, 5.0],
        strikes=[0.02, 0.03, 0.04, 0.05, 0.06],
        grid=[
            [0.60, 0.50, 0.45, 0.50, 0.60],
            [0.55, 0.45, 0.40, 0.45, 0.55],
            [0.50, 0.40, 0.35, 0.40, 0.50],
        ],
    )
    market.insert_surface(vol_surface)

    return market


def example_cms_cap():
    """Example: CMS cap (call options on swap rate)."""
    # Quarterly fixings over 2 years on 10Y CMS rate
    fixing_dates = [
        date(2025, 4, 1),
        date(2025, 7, 1),
        date(2025, 10, 1),
        date(2026, 1, 1),
        date(2026, 4, 1),
        date(2026, 7, 1),
        date(2026, 10, 1),
        date(2027, 1, 1),
    ]

    # Accrual fractions (quarterly ~0.25)
    accrual_fractions = [0.25] * 8

    cms_cap = CmsOption.builder(
        instrument_id="CMS_CAP_001",
        strike=0.04,  # 4% strike
        cms_tenor=10.0,  # 10-year CMS rate
        fixing_dates=fixing_dates,
        accrual_fractions=accrual_fractions,
        option_type="call",  # Cap = call on rate
        notional=Money(10000000.0, USD),  # $10M notional
        discount_curve="USD.SOFR",
        forward_curve="USD.SOFR",
        vol_surface="USD.SWAPTION.VOL",
    )

    # Price the CMS cap
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(cms_cap, "monte_carlo_hull_white_1f", market, as_of=val_date)

    return cms_cap, result


def example_cms_floor():
    """Example: CMS floor (put options on swap rate)."""
    # Semi-annual fixings over 5 years on 5Y CMS rate
    fixing_dates = [
        date(2025, 7, 1),
        date(2026, 1, 1),
        date(2026, 7, 1),
        date(2027, 1, 1),
        date(2027, 7, 1),
        date(2028, 1, 1),
        date(2028, 7, 1),
        date(2029, 1, 1),
        date(2029, 7, 1),
        date(2030, 1, 1),
    ]

    # Accrual fractions (semi-annual ~0.5)
    accrual_fractions = [0.5] * 10

    cms_floor = CmsOption.builder(
        instrument_id="CMS_FLOOR_001",
        strike=0.025,  # 2.5% strike
        cms_tenor=5.0,  # 5-year CMS rate
        fixing_dates=fixing_dates,
        accrual_fractions=accrual_fractions,
        option_type="put",  # Floor = put on rate
        notional=Money(25000000.0, USD),  # $25M notional
        discount_curve="USD.SOFR",
        forward_curve="USD.SOFR",
        vol_surface="USD.SWAPTION.VOL",
    )

    # Price the CMS floor
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.get_price(cms_floor, "monte_carlo_hull_white_1f", market, as_of=val_date)

    return cms_floor, result


def example_cms_spread_option():
    """Example: CMS spread option (difference between two CMS rates)."""
    # Annual fixings over 3 years
    # In practice, would use two CMS options to create a spread
    fixing_dates = [
        date(2026, 1, 1),
        date(2027, 1, 1),
        date(2028, 1, 1),
    ]

    accrual_fractions = [1.0] * 3

    # Call on 10Y CMS rate
    cms_long = CmsOption.builder(
        instrument_id="CMS_SPREAD_LONG",
        strike=0.035,  # 3.5% strike
        cms_tenor=10.0,  # 10-year rate
        fixing_dates=fixing_dates,
        accrual_fractions=accrual_fractions,
        option_type="call",
        notional=Money(50000000.0, USD),
        discount_curve="USD.SOFR",
        forward_curve="USD.SOFR",
        vol_surface="USD.SWAPTION.VOL",
    )

    # Put on 2Y CMS rate (to get positive spread exposure)
    cms_short = CmsOption.builder(
        instrument_id="CMS_SPREAD_SHORT",
        strike=0.03,  # 3% strike
        cms_tenor=2.0,  # 2-year rate
        fixing_dates=fixing_dates,
        accrual_fractions=accrual_fractions,
        option_type="put",
        notional=Money(50000000.0, USD),
        discount_curve="USD.SOFR",
        forward_curve="USD.SOFR",
        vol_surface="USD.SWAPTION.VOL",
    )

    # Price both positions
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result_long = registry.get_price(cms_long, "monte_carlo_hull_white_1f", market, as_of=val_date)
    result_short = registry.get_price(cms_short, "monte_carlo_hull_white_1f", market, as_of=val_date)

    return (cms_long, cms_short), (result_long, result_short)


def main() -> None:
    """Run all CMS option examples."""
    example_cms_cap()
    example_cms_floor()
    example_cms_spread_option()


if __name__ == "__main__":
    main()
