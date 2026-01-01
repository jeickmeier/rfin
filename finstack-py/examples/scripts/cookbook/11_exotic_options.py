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

from finstack import (
    AsianOption,
    BarrierOption,
    Date,
    DiscountCurve,
    FxMatrix,
    LookbackOption,
    MarketContext,
    QuantoOption,
    VolSurface,
    create_standard_registry,
)


def create_market_data():
    """Create market with vol surfaces and FX rates."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))

    # USD discount curve
    usd_curve = DiscountCurve.flat(id="USD.OIS", base_date=Date(2024, 1, 15), rate=0.045, day_count="Act360")
    market.insert_discount(usd_curve)

    # EUR discount curve (for quanto)
    eur_curve = DiscountCurve.flat(id="EUR.OIS", base_date=Date(2024, 1, 15), rate=0.035, day_count="Act360")
    market.insert_discount(eur_curve)

    # Equity spot prices
    market.set_equity("SPY", 480.0)  # S&P 500
    market.set_equity("EUROSTOXX", 4200.0)  # EuroStoxx 50

    # Vol surfaces (flat for simplicity)
    spy_vol = VolSurface.flat(
        id="SPY.VOL",
        value=0.18,  # 18% vol
        surface_type="lognormal",
    )
    market.set_vol_surface(spy_vol)

    eurostoxx_vol = VolSurface.flat(
        id="EUROSTOXX.VOL",
        value=0.22,  # 22% vol
        surface_type="lognormal",
    )
    market.set_vol_surface(eurostoxx_vol)

    # FX rates and FX vol (for quanto)
    fx = FxMatrix()
    fx.set_spot("USD", "EUR", 0.92)
    market.set_fx_matrix(fx)

    # FX vol for quanto adjustment
    fx_vol = VolSurface.flat(
        id="USDEUR.VOL",
        value=0.10,  # 10% FX vol
        surface_type="lognormal",
    )
    market.set_vol_surface(fx_vol)

    # Dividend yield (for equity options)
    market.set_scalar("SPY.DIVIDEND_YIELD", 0.015)  # 1.5% dividend yield
    market.set_scalar("EUROSTOXX.DIVIDEND_YIELD", 0.025)  # 2.5% dividend yield

    # Correlation (for quanto)
    market.set_scalar("SPY_USDEUR_CORRELATION", -0.3)  # -30% correlation

    return market


def main() -> None:
    """Price exotic options with analytical methods."""
    # Create market
    market = create_market_data()
    registry = create_standard_registry()

    base_date = Date(2024, 1, 15)
    expiry = Date(2024, 7, 15)  # 6M expiry

    spot = market.get_equity("SPY")
    market.get_vol_surface("SPY.VOL").value(0.5, spot)  # 6M vol

    # 1. Barrier Options (Up-and-Out Call)

    barrier_call = BarrierOption.up_and_out_call(
        id="SPY.UAO.CALL",
        strike=500.0,
        barrier=550.0,  # Knocked out if SPY >= 550
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS",
        monitoring="continuous",  # Continuous barrier monitoring
    )

    # Price with analytical method (Black-Scholes barrier formula)
    result_barrier = registry.price_barrier_option_with_metrics(
        barrier_call,
        model="barrier_bs_continuous",  # Analytical continuous barrier
        market=market,
        metrics=["delta", "gamma", "vega", "theta"],
    )

    pv_barrier = result_barrier.present_value.amount
    result_barrier.metric("delta")

    # 2. Asian Option (Arithmetic Average)

    asian_call = AsianOption.arithmetic_call(
        id="SPY.ASIAN.CALL",
        strike=480.0,
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        averaging_start=base_date,
        averaging_frequency="daily",  # Daily averaging
        discount_curve_id="USD.OIS",
    )

    # Price with Turnbull-Wakeman approximation (fast analytical method)
    result_asian = registry.price_asian_option_with_metrics(
        asian_call,
        model="asian_turnbull_wakeman",  # Analytical approximation
        market=market,
        metrics=["delta", "vega"],
    )

    result_asian.metric("delta")

    # 3. Lookback Option (Floating Strike Call)

    lookback_call = LookbackOption.floating_strike_call(
        id="SPY.LOOKBACK.CALL",
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS",
        monitoring="continuous",  # Continuous monitoring of min/max
    )

    # Price with analytical Black-Scholes lookback formula
    result_lookback = registry.price_lookback_option_with_metrics(
        lookback_call,
        model="lookback_bs_continuous",  # Analytical continuous lookback
        market=market,
        metrics=["delta", "gamma"],
    )

    result_lookback.metric("delta")

    # 4. Quanto Option (Cross-Currency)

    # Call on SPY (USD equity) settled in EUR at fixed FX rate
    quanto_call = QuantoOption.call(
        id="SPY.QUANTO.EUR",
        strike=500.0,
        expiry=expiry,
        underlying="SPY",
        quantity=100.0,
        domestic_currency="EUR",  # Settle in EUR
        foreign_currency="USD",  # SPY is USD asset
        discount_curve_id="EUR.OIS",
        fx_correlation=-0.3,  # Correlation between SPY and USD/EUR
    )

    # Price with quanto-adjusted Black-Scholes
    result_quanto = registry.price_quanto_option_with_metrics(
        quanto_call,
        model="quanto_bs",  # Analytical quanto adjustment
        market=market,
        metrics=["delta", "vega"],
    )

    pv_quanto = result_quanto.present_value.amount
    result_quanto.metric("delta")

    # Convert EUR PV to USD for comparison
    fx_rate = market.fx_matrix().rate("EUR", "USD")
    pv_quanto * fx_rate

    # 5. Comparison: Vanilla vs Exotic

    # Vanilla call for comparison
    from finstack import EquityOption

    vanilla_call = EquityOption.european(
        id="SPY.CALL.VANILLA",
        strike=500.0,
        expiry=expiry,
        is_call=True,
        underlying="SPY",
        quantity=100.0,
        discount_curve_id="USD.OIS",
    )

    registry.price_equity_option(vanilla_call, model="black_scholes", market=market)

    # 6. Analytical vs Monte Carlo comparison

    # Price same barrier option with Monte Carlo
    result_mc = registry.price_barrier_option_with_metrics(
        barrier_call,
        model="monte_carlo_gbm",  # Monte Carlo simulation
        market=market,
        metrics=["delta"],
        mc_config={"num_paths": 100_000, "seed": 42, "antithetic": True},
    )

    pv_mc = result_mc.present_value.amount
    diff = pv_mc - pv_barrier
    (diff / pv_barrier) * 100

    # 7. Summary


if __name__ == "__main__":
    main()
