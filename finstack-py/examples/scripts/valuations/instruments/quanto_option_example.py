"""Quanto Option Example.

Demonstrates pricing and analysis of quanto options (currency-protected foreign equity options).
"""

from datetime import date

from finstack.core.currency import EUR, USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import QuantoOption
from finstack.valuations.pricer import create_standard_registry

from finstack import Money


def create_market_data(val_date: date) -> MarketContext:
    """Create market data for quanto option pricing."""
    market = MarketContext()

    # USD discount curve (times in years)
    usd_disc = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (0.5, 0.975), (1.0, 0.95)],
    )
    market.insert_discount(usd_disc)

    # EUR discount curve (not strictly required by the pricer but added for completeness)
    eur_disc = DiscountCurve(
        "EUR.ESTR",
        val_date,
        [(0.0, 1.0), (0.5, 0.98), (1.0, 0.96)],
    )
    market.insert_discount(eur_disc)

    # Equity volatility surface (expiries in years)
    eq_vol_surface = VolSurface(
        "SAP.VOL",
        expiries=[0.5, 1.0],
        strikes=[100.0, 120.0, 140.0, 160.0, 180.0],
        grid=[
            [0.30, 0.27, 0.25, 0.27, 0.30],  # 6M
            [0.32, 0.29, 0.27, 0.29, 0.32],  # 1Y
        ],
    )
    market.insert_surface(eq_vol_surface)

    # FX volatility surface (EUR/USD)
    fx_vol_surface = VolSurface(
        "EURUSD.VOL",
        expiries=[0.5, 1.0],
        strikes=[1.00, 1.05, 1.10, 1.15, 1.20],
        grid=[
            [0.08, 0.07, 0.06, 0.07, 0.08],  # 6M
            [0.09, 0.08, 0.07, 0.08, 0.09],  # 1Y
        ],
    )
    market.insert_surface(fx_vol_surface)

    # Spot and dividend
    market.insert_price("SAP", MarketScalar.price(Money(140.0, EUR)))
    market.insert_price("EURUSD", MarketScalar.unitless(1.10))
    market.insert_price("SAP.DIV", MarketScalar.unitless(0.02))

    return market


def example_quanto_call_positive_correlation():
    """Example: Quanto call with positive equity-FX correlation."""
    val_date = date(2025, 1, 1)
    # US investor wants exposure to SAP (German stock) without EUR/USD FX risk
    # Payoff in USD = max(S_T - K, 0) * fixed_rate (no FX conversion at maturity)
    option = QuantoOption.builder(
        instrument_id="QUANTO_001",
        ticker="SAP",
        equity_strike=140.0,  # Strike in EUR
        option_type="call",
        expiry=date(2025, 12, 31),
        notional=Money(10000.0, USD),  # Notional in USD
        domestic_currency=USD,  # Settlement currency
        foreign_currency=EUR,  # Underlying currency
        correlation=0.3,  # Positive correlation between SAP and EUR/USD
        domestic_discount_curve="USD.SOFR",
        foreign_discount_curve="EUR.ESTR",
        spot_id="SAP",
        vol_surface="SAP.VOL",
        div_yield_id="SAP.DIV",
        fx_rate_id="EURUSD",
        fx_vol_id="EURUSD.VOL",
    )

    # Price the quanto option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_quanto_put_negative_correlation():
    """Example: Quanto put with negative equity-FX correlation."""
    val_date = date(2025, 1, 1)
    # Hedging strategy: put on foreign stock with negative FX correlation
    option = QuantoOption.builder(
        instrument_id="QUANTO_002",
        ticker="SAP",
        equity_strike=150.0,  # Higher strike for put
        option_type="put",
        expiry=date(2025, 6, 30),
        notional=Money(25000.0, USD),
        domestic_currency=USD,
        foreign_currency=EUR,
        correlation=-0.2,  # Negative correlation (SAP falls when EUR strengthens)
        domestic_discount_curve="USD.SOFR",
        foreign_discount_curve="EUR.ESTR",
        spot_id="SAP",
        vol_surface="SAP.VOL",
        div_yield_id="SAP.DIV",
        fx_rate_id="EURUSD",
        fx_vol_id="EURUSD.VOL",
    )

    # Price the quanto option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def example_quanto_call_zero_correlation():
    """Example: Quanto call with zero equity-FX correlation."""
    val_date = date(2025, 1, 1)
    # Zero correlation case: equity and FX independent
    option = QuantoOption.builder(
        instrument_id="QUANTO_003",
        ticker="SAP",
        equity_strike=140.0,
        option_type="call",
        expiry=date(2026, 1, 1),
        notional=Money(50000.0, USD),
        domestic_currency=USD,
        foreign_currency=EUR,
        correlation=0.0,  # Zero correlation
        domestic_discount_curve="USD.SOFR",
        foreign_discount_curve="EUR.ESTR",
        spot_id="SAP",
        vol_surface="SAP.VOL",
        div_yield_id="SAP.DIV",
        fx_rate_id="EURUSD",
        fx_vol_id="EURUSD.VOL",
    )

    # Price the quanto option
    market = create_market_data(val_date)
    registry = create_standard_registry()
    result = registry.price(option, "monte_carlo_gbm", market, as_of=val_date)

    return option, result


def main() -> None:
    """Run all quanto option examples."""
    example_quanto_call_positive_correlation()
    example_quanto_put_negative_correlation()
    example_quanto_call_zero_correlation()


if __name__ == "__main__":
    main()
