"""FX Variance Swap Example.
=========================

Demonstrates pricing an FX variance swap with realized and implied variance
components.
"""

from finstack.core.market_data import (
    DiscountCurve,
    FxMatrix,
    InterpolationStyle,
    MarketContext,
    VolSurface,
)
from finstack.valuations.instruments import FxVarianceSwap
from finstack.valuations.pricer import create_standard_registry

from finstack import Date, Money


def main():
    """Price a 1-year EUR/USD variance swap:
    - Notional: $1M (variance notional)
    - Strike variance: 0.04 (4% vol squared = 20% vol)
    - Side: Receive variance (long vol)
    - Start date: Jan 2, 2024
    - Maturity: Jan 2, 2025
    - Daily observations.
    """
    # Market data setup
    as_of = Date(2024, 6, 15)  # Midway through the swap
    base_date = as_of

    # USD discount curve
    usd_tenors = [0.0, 0.5, 1.0, 2.0, 5.0]
    usd_dfs = [1.0, 0.9750, 0.9500, 0.9100, 0.8200]

    usd_discount = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=usd_tenors,
        values=usd_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )

    # EUR discount curve
    eur_tenors = [0.0, 0.5, 1.0, 2.0, 5.0]
    eur_dfs = [1.0, 0.9800, 0.9600, 0.9300, 0.8500]

    eur_discount = DiscountCurve(
        curve_id="EUR-OIS",
        base_date=base_date,
        tenors=eur_tenors,
        values=eur_dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )

    # FX volatility surface (implied vols for forward variance)
    vol_tenors = [0.25, 0.5, 0.75, 1.0, 2.0]  # Years
    vol_strikes = [0.95, 1.00, 1.05, 1.10, 1.15]  # Moneyness strikes

    # Vol matrix (ATM vol ~ 22%)
    vol_matrix = [
        [0.25, 0.23, 0.22, 0.23, 0.25],  # 3M
        [0.24, 0.22, 0.21, 0.22, 0.24],  # 6M
        [0.23, 0.21, 0.20, 0.21, 0.23],  # 9M
        [0.22, 0.20, 0.19, 0.20, 0.22],  # 1Y
        [0.21, 0.19, 0.18, 0.19, 0.21],  # 2Y
    ]

    vol_surface = VolSurface(
        surface_id="EURUSD-VOL",
        base_date=base_date,
        tenors=vol_tenors,
        strikes=vol_strikes,
        vol_matrix=vol_matrix,
        interpolation=InterpolationStyle.Linear(),
    )

    # FX spot rate: EUR/USD = 1.09
    fx_matrix = FxMatrix()
    fx_matrix.set_rate("EUR", "USD", 1.09)

    # Build market context
    market = MarketContext()
    market.insert_discount(usd_discount)
    market.insert_discount(eur_discount)
    market.insert_vol_surface(vol_surface)
    market.set_fx_provider(fx_matrix)

    # Add spot FX rate
    market.insert_scalar("EURUSD-SPOT", 1.09)

    # Historical FX rates for realized variance calculation
    # (Assume daily observations from Jan 2 to Jun 14, 2024 - about 120 business days)
    # For simplicity, we'll provide summary realized variance
    market.insert_scalar("EURUSD-REALIZED-VAR", 0.038)  # Partial realized variance

    # Create FX variance swap
    var_swap = (
        FxVarianceSwap.builder("FXVAR-EURUSD-1Y")
        .base_currency("EUR")
        .quote_currency("USD")
        .notional(Money.from_code(1_000_000, "USD"))
        .strike_variance(0.04)  # 20% vol squared
        .start_date(Date(2024, 1, 2))
        .maturity(Date(2025, 1, 2))
        .observation_freq("daily")
        .realized_method("close_to_close")
        .side("receive")  # Receive realized variance (long vol)
        .domestic_discount_curve("USD-OIS")
        .foreign_discount_curve("EUR-OIS")
        .vol_surface("EURUSD-VOL")
        .spot_id("EURUSD-SPOT")
        .build()
    )

    # Price the variance swap
    registry = create_standard_registry()

    result = registry.price_fx_variance_swap_with_metrics(
        var_swap,
        "variance_replication",
        market,
        ["realized_var", "implied_var", "total_var", "vega", "theta"],
    )

    # Display results
    print("=" * 70)
    print("FX Variance Swap Valuation Results")
    print("=" * 70)
    print(f"Instrument ID:      {var_swap.instrument_id()}")
    print(f"Currency Pair:      {var_swap.base_currency().code}/{var_swap.quote_currency().code}")
    print(f"Notional:           {var_swap.notional().amount:,.2f} {var_swap.notional().currency.code}")
    print(f"Strike Variance:    {var_swap.strike_variance():.4f} ({var_swap.strike_variance() ** 0.5 * 100:.2f}% vol)")
    print(f"Side:               {var_swap.side()}")
    print(f"Start Date:         {var_swap.start_date()}")
    print(f"Maturity Date:      {var_swap.maturity()}")
    print(f"Observation Freq:   {var_swap.observation_freq()}")
    print(f"Realized Method:    {var_swap.realized_method()}")
    print(f"\nPresent Value:      {result.present_value.amount:,.2f} {result.present_value.currency.code}")
    print("\nMetrics:")
    print(f"  Realized Variance:  {result.metric('realized_var') or 0:.4f}")
    print(f"  Implied Variance:   {result.metric('implied_var') or 0:.4f}")
    print(f"  Total Variance:     {result.metric('total_var') or 0:.4f}")
    print(f"  Vega:               {result.metric('vega') or 0:,.2f}")
    print(f"  Theta (1d):         {result.metric('theta') or 0:,.2f}")
    print("=" * 70)

    # Payoff interpretation
    total_var = result.metric("total_var") or 0.04
    payoff_at_maturity = var_swap.notional().amount * (total_var - var_swap.strike_variance())

    print("\nPayoff at Maturity:")
    print(f"  Total Variance:     {total_var:.4f}")
    print(f"  Strike Variance:    {var_swap.strike_variance():.4f}")
    print(f"  Variance Diff:      {total_var - var_swap.strike_variance():.4f}")
    print(f"  Notional:           ${var_swap.notional().amount:,.2f}")
    print(f"  Expected Payoff:    ${payoff_at_maturity:,.2f}")
    print("=" * 70)


if __name__ == "__main__":
    main()
