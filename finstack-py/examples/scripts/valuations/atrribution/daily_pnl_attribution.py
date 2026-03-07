#!/usr/bin/env python3
"""Daily P&L Attribution Example.

Demonstrates how to perform P&L attribution on a bond position to explain
daily MTM changes.
"""

from datetime import date

from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.valuations.attribution import AttributionMethod, attribute_pnl
from finstack.valuations.instruments import Bond

from finstack import FinstackError, Money

HAZARD_ID = "USD-CREDIT"


def create_market_with_rate(as_of_date, rate):
    """Helper to create a market context with a flat discount curve."""
    # Create a flat discount curve using multiple knots for accurate interpolation
    # Use continuous compounding: DF(t) = exp(-rate * t)
    import math

    knots = []
    for t in [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]:
        df = math.exp(-rate * t)
        knots.append((t, df))

    curve = DiscountCurve("USD-OIS", as_of_date, knots)

    market = MarketContext()
    market.insert(curve)
    # Add a flat hazard curve so credit metrics (e.g., CS01) are available.
    hazard = HazardCurve(HAZARD_ID, as_of_date, [(0.0, 0.01), (30.0, 0.01)])
    market.insert(hazard)
    return market


def example_parallel_attribution(bond, market_t0, market_t1, t0, t1):
    """Example: Parallel attribution with curve shift."""
    # Run parallel attribution
    attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=AttributionMethod.parallel())

    # Display results

    # Show structured explanation

    # Export to CSV
    csv_lines = attr.to_csv().split("\n")
    for _line in csv_lines[:5]:
        pass

    return attr


def example_waterfall_attribution(bond, market_t0, market_t1, t0, t1):
    """Example: Waterfall attribution with custom factor order."""
    # Custom waterfall order
    method = AttributionMethod.waterfall(["carry", "rates_curves", "credit_curves", "fx", "volatility"])

    attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=method)

    # Display results

    return attr


def example_metric_based_attribution(bond, market_t0, market_t1, t0, t1):
    """Example: Metric-based attribution using first and second-order metrics.

    Metrics-based attribution uses Taylor series approximation to explain P&L:
    - Carry = Theta × time
    - Rates = DV01 × Δrates + ½ × P × Convexity × (Δrates)²
    - Credit = CS01 × Δspreads + ½ × P × CS-Gamma × (Δspreads)²
    - Vol = Vega × Δvol + ½ × Volga × (Δvol)² + Vanna × Δspot × Δvol
    - Spot = Delta × Δspot + ½ × Gamma × (Δspot)²

    Advantages:
    - Fast (0 repricings - uses pre-computed metrics)
    - Second-order terms (Convexity, Gamma, Volga) capture curvature effects
    - Graceful degradation: Works with or without second-order metrics
    - Convenient for daily P&L explanations

    Limitations:
    - Still approximate (third-order+ and cross-factor effects ignored)
    - Residuals depend on scenario complexity (realistic curves → better accuracy)
    """
    # Metric-based attribution
    # This method automatically computes both first and second-order metrics:
    # First-order: Theta, DV01, CS01, Vega, Delta, FX01, Inflation01
    # Second-order: Convexity, IrConvexity, Gamma, Volga, Vanna, CsGamma, InflationConvexity
    method = AttributionMethod.metrics_based()

    # Run attribution
    try:
        attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=method)
    except FinstackError:
        fallback = AttributionMethod.parallel()
        attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=fallback)

    # Display results

    # Demonstrate the impact of second-order terms

    # Validate residual
    attr.residual_within_tolerance(25.0, 10000.0)

    return attr


def compare_methods(results) -> None:
    """Compare attribution results across methods."""
    _parallel, _waterfall, _metrics = results

    # Summary table

    # Key insights

    # Method selection guide


def main() -> None:
    """Run all examples with shared instrument and market data."""
    # -------------------------------------------------------------------------
    # Shared Setup: One instrument, one market scenario
    # -------------------------------------------------------------------------

    # Create a 5-year corporate bond with 5% coupon
    bond = (
        Bond.builder("CORP-001")
        .money(Money(1_000_000, "USD"))
        .issue(date(2025, 1, 1))
        .maturity(date(2030, 1, 1))
        .coupon_rate(0.05)
        .frequency("SemiAnnual")
        .day_count("30/360")
        .bdc("Following")
        .disc_id("USD-OIS")
        .credit_curve(HAZARD_ID)
        .build()
    )

    # Market at T₀ (yesterday) with 4% rates
    t0 = date(2025, 1, 15)
    market_t0 = create_market_with_rate(t0, 0.04)

    # Market at T₁ (today) with 4.5% rates (50bp increase)
    t1 = date(2025, 1, 16)
    market_t1 = create_market_with_rate(t1, 0.045)

    # -------------------------------------------------------------------------
    # Run all three attribution methods on the same data
    # -------------------------------------------------------------------------
    results = []

    attr_parallel = example_parallel_attribution(bond, market_t0, market_t1, t0, t1)
    results.append(attr_parallel)

    attr_waterfall = example_waterfall_attribution(bond, market_t0, market_t1, t0, t1)
    results.append(attr_waterfall)

    attr_metrics = example_metric_based_attribution(bond, market_t0, market_t1, t0, t1)
    results.append(attr_metrics)

    # -------------------------------------------------------------------------
    # Compare results
    # -------------------------------------------------------------------------
    compare_methods(results)


if __name__ == "__main__":
    main()
