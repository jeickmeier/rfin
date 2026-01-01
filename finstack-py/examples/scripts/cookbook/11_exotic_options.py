"""Title: Price Barrier, Asian, Lookback, and Quanto Options
Persona: Quantitative Researcher, Equity Analyst
Complexity: Advanced
Runtime: ~2 seconds.

Description:
Demonstrates exotic option pricing using analytical methods:
- Barrier options (Up-and-Out, Down-and-In) with continuous monitoring
- Asian options (arithmetic and geometric averaging)
- Lookback options (floating strike)
- Quanto options (cross-currency equity options)
- Comparison of analytical vs Monte Carlo pricing

Key Concepts:
- Analytical closed-form pricing (fast, accurate)
- Model selection via ModelKey
- Exotic payoff structures
- Cross-currency derivatives (quanto)

Prerequisites:
- Black-Scholes model understanding
- Exotic option payoff structures
- Monte Carlo basics (for comparison)
"""

from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import BarrierOption
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create a simple equity market for exotic option examples."""
    market = MarketContext()

    market.insert_discount(DiscountCurve("USD.SOFR", val_date, [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90)]))

    market.insert_surface(
        VolSurface(
            "SPY.VOL",
            expiries=[0.5, 1.0],
            strikes=[420.0, 450.0, 480.0, 510.0, 540.0],
            grid=[
                [0.22, 0.20, 0.19, 0.20, 0.22],
                [0.24, 0.22, 0.21, 0.22, 0.24],
            ],
        )
    )

    market.insert_price("SPY", MarketScalar.price(Money(480.0, USD)))
    market.insert_price("SPY.DIV", MarketScalar.unitless(0.015))

    return market


def main() -> None:
    """Construct and price a simple barrier option."""
    val_date = date(2025, 1, 1)
    market = create_market_data(val_date)
    registry = create_standard_registry()

    barrier_call = BarrierOption.builder(
        instrument_id="SPY.UAO.CALL",
        ticker="SPY",
        strike=500.0,
        barrier=550.0,
        option_type="call",
        barrier_type="up_and_out",
        expiry=date(2025, 12, 31),
        notional=Money(100_000.0, USD),
        discount_curve="USD.SOFR",
        spot_id="SPY",
        vol_surface="SPY.VOL",
        div_yield_id="SPY.DIV",
        use_gobet_miri=False,
    )

    # Price with Monte Carlo (kept small and fast by the engine defaults).
    registry.price(barrier_call, "monte_carlo_gbm", market, as_of=val_date)


if __name__ == "__main__":
    main()
