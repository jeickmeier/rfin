#!/usr/bin/env python3
"""
Example demonstrating the finstack market_data module for financial curves and surfaces.

This example shows how to:
- Create various types of curves (discount, forward, hazard, inflation)
- Use different interpolation styles
- Create volatility surfaces
- Use the CurveSet container for managing multiple curves
- Work with numpy arrays for batch operations
"""

import numpy as np
from finstack import Date, DayCount
from finstack.market_data import (
    InterpStyle,
    DiscountCurve,
    ForwardCurve,
    HazardCurve,
    InflationCurve,
    VolSurface,
    CurveSet,
)


def example_discount_curve():
    """Example: Creating and using a discount curve."""
    print("=== Discount Curve Example ===")

    # Create a USD OIS discount curve with monotone-convex interpolation
    base_date = Date(2025, 1, 1)
    times = [0.0, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0]
    discount_factors = [1.0, 0.9925, 0.985, 0.97, 0.94, 0.85, 0.70]

    curve = DiscountCurve(
        id="USD-OIS",
        base_date=base_date,
        times=times,
        discount_factors=discount_factors,
        interpolation=InterpStyle.MonotoneConvex,
    )

    # Single point evaluation
    t = 1.5
    df = curve.df(t)
    zero_rate = curve.zero(t)
    print(f"At t={t}:")
    print(f"  Discount factor: {df:.6f}")
    print(f"  Zero rate: {zero_rate:.4%}")

    # Batch evaluation with numpy
    eval_times = np.linspace(0, 5, 11)
    dfs = curve.df_batch(eval_times)
    print(f"\nBatch evaluation at {len(eval_times)} points:")
    for i in range(5):
        print(f"  t={eval_times[i]:.1f}: DF={dfs[i]:.6f}")

    # Access curve properties
    print("\nCurve properties:")
    print(f"  ID: {curve.id}")
    print(f"  Base date: {curve.base_date}")
    print()


def example_forward_curve():
    """Example: Creating and using a forward curve."""
    print("=== Forward Curve Example ===")

    # Create a 3-month SOFR forward curve
    base_date = Date(2025, 1, 1)
    times = [0.0, 0.5, 1.0, 2.0, 5.0]
    forward_rates = [0.035, 0.038, 0.040, 0.042, 0.045]  # 3.5%, 3.8%, etc.

    curve = ForwardCurve(
        id="USD-SOFR3M",
        tenor=0.25,  # 3 months = 0.25 years
        base_date=base_date,
        times=times,
        forward_rates=forward_rates,
        interpolation=InterpStyle.Linear,
        reset_lag=2,  # 2 business days
        day_count=DayCount.act360(),
    )

    # Get forward rate at specific time
    t = 1.5
    rate = curve.rate(t)
    print(f"3M SOFR forward rate at t={t}: {rate:.4%}")

    # Average rate over a period
    t1, t2 = 0.5, 1.5
    avg_rate = curve.rate_period(t1, t2)
    print(f"Average rate from {t1} to {t2}: {avg_rate:.4%}")

    print("\nCurve properties:")
    print(f"  Tenor: {curve.tenor} years")
    print(f"  Reset lag: {curve.reset_lag} days")
    print(f"  Day count: {curve.day_count}")
    print()


def example_hazard_curve():
    """Example: Creating and using a hazard curve for credit risk."""
    print("=== Hazard Curve Example ===")

    # Create a credit hazard curve
    base_date = Date(2025, 1, 1)
    times = [0.0, 1.0, 3.0, 5.0, 10.0]
    hazard_rates = [0.01, 0.015, 0.02, 0.025, 0.03]  # 1%, 1.5%, etc.

    curve = HazardCurve(
        id="CORP-A-USD",
        base_date=base_date,
        times=times,
        hazard_rates=hazard_rates,
    )

    # Survival probability
    t = 2.0
    sp = curve.sp(t)
    print(f"Survival probability to t={t}: {sp:.6f}")

    # Default probability between two times
    t1, t2 = 1.0, 3.0
    default_prob = curve.default_probability(t1, t2)
    print(f"Default probability between t={t1} and t={t2}: {default_prob:.6f}")
    print()


def example_inflation_curve():
    """Example: Creating and using an inflation curve."""
    print("=== Inflation Curve Example ===")

    # Create a CPI inflation curve
    base_cpi = 300.0  # Current CPI level
    times = [0.0, 1.0, 2.0, 5.0, 10.0]
    cpi_levels = [300.0, 306.0, 312.24, 331.5, 366.0]  # ~2% inflation

    curve = InflationCurve(
        id="US-CPI",
        base_cpi=base_cpi,
        times=times,
        cpi_levels=cpi_levels,
        interpolation=InterpStyle.LogLinear,  # Exponential growth
    )

    # CPI at future time
    t = 3.0
    cpi = curve.cpi(t)
    print(f"CPI at t={t}: {cpi:.2f}")

    # Inflation rate between two times
    t1, t2 = 0.0, 5.0
    inflation_rate = curve.inflation_rate(t1, t2)
    print(f"Annualized inflation rate from t={t1} to t={t2}: {inflation_rate:.4%}")

    # Year-over-year inflation
    for year in range(1, 4):
        rate = curve.inflation_rate(year - 1, year)
        print(f"Year {year} inflation: {rate:.4%}")
    print()


def example_volatility_surface():
    """Example: Creating and using a volatility surface."""
    print("=== Volatility Surface Example ===")

    # Create an equity volatility surface
    expiries = [0.25, 0.5, 1.0, 2.0]  # Time to expiry in years
    strikes = [80.0, 90.0, 100.0, 110.0, 120.0]  # Strike prices

    # Volatility data (expiries x strikes)
    # Shows volatility smile effect
    vol_data = [
        [0.25, 0.22, 0.20, 0.22, 0.25],  # 3M expiry
        [0.24, 0.21, 0.19, 0.21, 0.24],  # 6M expiry
        [0.23, 0.20, 0.18, 0.20, 0.23],  # 1Y expiry
        [0.22, 0.19, 0.17, 0.19, 0.22],  # 2Y expiry
    ]

    surface = VolSurface(
        id="SPX-IV",
        expiries=expiries,
        strikes=strikes,
        values=vol_data,
    )

    # Interpolate volatility at specific point
    expiry, strike = 0.75, 95.0
    vol = surface.value(expiry, strike)
    print(f"Implied vol at expiry={expiry}, strike={strike}: {vol:.4f}")

    # Get volatility for all strikes at a specific expiry
    expiry_idx = 1  # 6M expiry
    vols = surface.get_expiry_slice(expiry_idx)
    print(f"\nVolatilities at {expiries[expiry_idx]}Y expiry:")
    for k, v in zip(strikes, vols):
        print(f"  Strike {k}: {v:.4f}")

    # Access surface data as numpy arrays
    print(f"\nSurface shape: {surface.data.shape}")
    print()


def example_curve_set():
    """Example: Using CurveSet to manage multiple curves."""
    print("=== CurveSet Container Example ===")

    # Create various curves
    base_date = Date(2025, 1, 1)

    # Discount curves
    usd_ois = DiscountCurve(
        id="USD-OIS",
        base_date=base_date,
        times=[0.0, 1.0, 5.0],
        discount_factors=[1.0, 0.97, 0.85],
    )

    eur_ois = DiscountCurve(
        id="EUR-OIS",
        base_date=base_date,
        times=[0.0, 1.0, 5.0],
        discount_factors=[1.0, 0.98, 0.88],
    )

    # Forward curve
    usd_sofr3m = ForwardCurve(
        id="USD-SOFR3M",
        tenor=0.25,
        base_date=base_date,
        times=[0.0, 1.0, 5.0],
        forward_rates=[0.035, 0.04, 0.045],
    )

    # Credit curve
    corp_hazard = HazardCurve(
        id="CORP-A-USD",
        base_date=base_date,
        times=[0.0, 5.0],
        hazard_rates=[0.01, 0.02],
    )

    # Create CurveSet and add all curves using dictionary interface
    curves = CurveSet()
    curves["USD-OIS"] = usd_ois
    curves["EUR-OIS"] = eur_ois
    curves["USD-SOFR3M"] = usd_sofr3m
    curves["CORP-A-USD"] = corp_hazard

    # Map collateral agreements to discount curves
    curves.map_collateral("CSA-USD", "USD-OIS")
    curves.map_collateral("CSA-EUR", "EUR-OIS")

    # Access curves by ID
    print("Accessing curves from CurveSet:")

    # Generic access (returns appropriate type)
    usd_curve = curves["USD-OIS"]
    print(f"USD OIS DF at t=1: {usd_curve.df(1.0):.6f}")

    # Type-specific access
    fwd_curve = curves.forward_curve("USD-SOFR3M")
    print(f"USD SOFR 3M at t=1: {fwd_curve.rate(1.0):.4%}")

    # Access via collateral mapping
    csa_curve = curves.collateral_curve("CSA-USD")
    print(f"CSA-USD DF at t=1: {csa_curve.df(1.0):.6f}")

    # List all curves
    print(f"\nAll curve IDs: {list(curves.keys())}")

    # Check containment
    print(f"Contains USD-OIS: {'USD-OIS' in curves}")
    print(f"Contains GBP-OIS: {'GBP-OIS' in curves}")
    print()


def example_interpolation_styles():
    """Example: Comparing different interpolation styles."""
    print("=== Interpolation Styles Comparison ===")

    # Same data, different interpolation methods
    base_date = Date(2025, 1, 1)
    times = [0.0, 1.0, 2.0, 5.0]
    dfs = [1.0, 0.97, 0.93, 0.82]

    styles = [
        ("Linear", InterpStyle.Linear),
        ("LogLinear", InterpStyle.LogLinear),
        ("MonotoneConvex", InterpStyle.MonotoneConvex),
        ("CubicHermite", InterpStyle.CubicHermite),
        ("FlatForward", InterpStyle.FlatForward),
    ]

    # Create curves with different interpolation
    curves = []
    for name, style in styles:
        curve = DiscountCurve(
            id=f"TEST-{name}",
            base_date=base_date,
            times=times,
            discount_factors=dfs,
            interpolation=style,
        )
        curves.append((name, curve))

    # Compare interpolated values
    print("Comparing DF values at t=1.5:")
    for name, curve in curves:
        df = curve.df(1.5)
        zero = curve.zero(1.5)
        print(f"  {name:15s}: DF={df:.6f}, Zero={zero:.4%}")
    print()


def main():
    """Run all examples."""
    print("Finstack Market Data Python Examples")
    print("=" * 50)
    print()

    example_discount_curve()
    example_forward_curve()
    example_hazard_curve()
    example_inflation_curve()
    example_volatility_surface()
    example_curve_set()
    example_interpolation_styles()

    print("All examples completed successfully!")


if __name__ == "__main__":
    main()
