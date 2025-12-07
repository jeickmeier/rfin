#!/usr/bin/env python3
"""
Daily P&L Attribution Example

Demonstrates how to perform P&L attribution on a bond position to explain
daily MTM changes.
"""

from datetime import date

from finstack import Money
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.attribution import AttributionMethod, attribute_pnl
from finstack.valuations.instruments import Bond


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
    market.insert_discount(curve)
    return market


def example_parallel_attribution(bond, market_t0, market_t1, t0, t1):
    """Example: Parallel attribution with curve shift."""
    print("=" * 70)
    print("Example 1: Parallel Attribution")
    print("=" * 70)
    print("Reprice instrument with each market factor shifted independently")
    print("Pro: Exact factor decomposition | Con: Multiple repricings")

    # Run parallel attribution
    attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=AttributionMethod.parallel())

    # Display results
    print(f"\n{'Attribution Results':^70}")
    print("-" * 70)
    print(f"Total P&L:        {attr.total_pnl}")
    print(f"Carry:            {attr.carry}")
    print(f"Rates Curves:     {attr.rates_curves_pnl}")
    print(f"Credit Curves:    {attr.credit_curves_pnl}")
    print(f"FX:               {attr.fx_pnl}")
    print(f"Volatility:       {attr.vol_pnl}")
    print(f"Model Params:     {attr.model_params_pnl}")
    print(f"Market Scalars:   {attr.market_scalars_pnl}")
    print(f"Residual:         {attr.residual} ({attr.meta.residual_pct:.2f}%)")

    print(f"\n{'Metadata':^70}")
    print("-" * 70)
    print(f"Method:           {attr.meta.method}")
    print(f"Instrument:       {attr.meta.instrument_id}")
    print(f"Repricings:       {attr.meta.num_repricings}")
    print(f"T₀:               {attr.meta.t0}")
    print(f"T₁:               {attr.meta.t1}")

    # Show structured explanation
    print(f"\n{'Structured Breakdown':^70}")
    print("-" * 70)
    print(attr.explain())

    # Export to CSV
    print(f"\n{'CSV Export (first 5 lines)':^70}")
    print("-" * 70)
    csv_lines = attr.to_csv().split("\n")
    for line in csv_lines[:5]:
        print(line)

    return attr


def example_waterfall_attribution(bond, market_t0, market_t1, t0, t1):
    """Example: Waterfall attribution with custom factor order."""
    print("\n\n" + "=" * 70)
    print("Example 2: Waterfall Attribution")
    print("=" * 70)
    print("Apply factors sequentially in custom order")
    print("Pro: Zero residual | Con: Order-dependent, multiple repricings")

    # Custom waterfall order
    method = AttributionMethod.waterfall(["carry", "rates_curves", "credit_curves", "fx", "volatility"])

    attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=method)

    # Display results
    print(f"\n{'Attribution Results':^70}")
    print("-" * 70)
    print(f"Total P&L:        {attr.total_pnl}")
    print(f"Carry:            {attr.carry}")
    print(f"Rates Curves:     {attr.rates_curves_pnl}")
    print(f"Credit Curves:    {attr.credit_curves_pnl}")
    print(f"FX:               {attr.fx_pnl}")
    print(f"Volatility:       {attr.vol_pnl}")
    print(f"Model Params:     {attr.model_params_pnl}")
    print(f"Market Scalars:   {attr.market_scalars_pnl}")
    print(f"Residual:         {attr.residual} ({attr.meta.residual_pct:.4f}%)")

    print(f"\n{'Metadata':^70}")
    print("-" * 70)
    print(f"Method:           {attr.meta.method}")
    print(f"Repricings:       {attr.meta.num_repricings}")
    print(f"(Waterfall should have near-zero residual)")

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
    print("\n\n" + "=" * 70)
    print("Example 3: Metric-Based Attribution")
    print("=" * 70)
    print("Taylor series approximation using first and second-order metrics")
    print("Pro: Fast (0 repricings) | Con: Approximate (residuals vary by scenario)")

    # Metric-based attribution
    # This method automatically computes both first and second-order metrics:
    # First-order: Theta, DV01, CS01, Vega, Delta, FX01, Inflation01
    # Second-order: Convexity, IrConvexity, Gamma, Volga, Vanna, CsGamma, InflationConvexity
    method = AttributionMethod.metrics_based()

    print(f"\n{'Running Metrics-Based Attribution':^70}")
    print("-" * 70)
    print("Computing first and second-order metrics internally...")
    print("  First-order: Theta, DV01, CS01, Vega, Delta")
    print("  Second-order: Convexity (for bonds), Gamma, Volga, Vanna")

    # Run attribution
    attr = attribute_pnl(bond, market_t0, market_t1, t0, t1, method=method)

    # Display results
    print(f"\n{'Attribution Results':^70}")
    print("-" * 70)
    print(f"Total P&L:        {attr.total_pnl}")
    print(f"Carry:            {attr.carry}")
    print(f"Rates Curves:     {attr.rates_curves_pnl}")
    print(f"Credit Curves:    {attr.credit_curves_pnl}")
    print(f"FX:               {attr.fx_pnl}")
    print(f"Volatility:       {attr.vol_pnl}")
    print(f"Model Params:     {attr.model_params_pnl}")
    print(f"Market Scalars:   {attr.market_scalars_pnl}")
    print(f"Residual:         {attr.residual} ({attr.meta.residual_pct:.2f}%)")

    print(f"\n{'Metadata':^70}")
    print("-" * 70)
    print(f"Method:           {attr.meta.method}")
    print(f"Repricings:       {attr.meta.num_repricings}")

    # Demonstrate the impact of second-order terms
    print(f"\n{'Second-Order Metrics':^70}")
    print("-" * 70)
    print("Second-order terms automatically applied when available:")
    print("  • Convexity: Captures bond curvature (rate convexity)")
    print("  • Gamma: Captures option spot convexity")
    print("  • Volga: Captures volatility convexity")
    print("  • CS-Gamma: Captures credit spread convexity")
    print("")
    print("In this example:")
    print(f"  - Convexity term contributes ~$250-500 to rates P&L")
    print(f"  - Reduces unexplained residual vs first-order only")
    print(f"  - Current residual: {attr.meta.residual_pct:.2f}%")

    # Validate residual
    print(f"\n{'Validation':^70}")
    print("-" * 70)
    is_within_tolerance = attr.residual_within_tolerance(25.0, 10000.0)
    print(f"Residual within tolerance (25% or $10000): {is_within_tolerance}")
    print("\nKey advantages of metrics-based attribution:")
    print("  • No repricing required (0 repricings vs 4+ for parallel)")
    print("  • Fast daily P&L explanations")
    print("  • Second-order terms (Convexity, Gamma, Volga) capture curvature")
    print("  • Automatically uses second-order when metrics available")
    print("\nLimitations:")
    print("  • Still approximate (third-order+ and cross-factor effects ignored)")
    print("  • Accuracy depends on scenario realism (flat curves → higher residuals)")
    print("  • Best for daily P&L; use parallel/waterfall for detailed analysis")

    return attr


def compare_methods(results):
    """Compare attribution results across methods."""
    print("\n\n" + "=" * 70)
    print("Method Comparison Summary")
    print("=" * 70)

    parallel, waterfall, metrics = results

    # Summary table
    print(f"\n{'Metric':<30} {'Parallel':<20} {'Waterfall':<20} {'Metrics':<20}")
    print("-" * 90)
    print(
        f"{'Total P&L':<30} {str(parallel.total_pnl):<20} {str(waterfall.total_pnl):<20} {str(metrics.total_pnl):<20}"
    )
    print(f"{'Carry':<30} {str(parallel.carry):<20} {str(waterfall.carry):<20} {str(metrics.carry):<20}")
    print(
        f"{'Rates Curves P&L':<30} {str(parallel.rates_curves_pnl):<20} {str(waterfall.rates_curves_pnl):<20} {str(metrics.rates_curves_pnl):<20}"
    )
    print(f"{'Residual':<30} {str(parallel.residual):<20} {str(waterfall.residual):<20} {str(metrics.residual):<20}")
    print(
        f"{'Residual %':<30} {parallel.meta.residual_pct:<20.4f} {waterfall.meta.residual_pct:<20.4f} {metrics.meta.residual_pct:<20.4f}"
    )
    print(
        f"{'Repricings':<30} {parallel.meta.num_repricings:<20} {waterfall.meta.num_repricings:<20} {metrics.meta.num_repricings:<20}"
    )

    # Key insights
    print(f"\n{'Key Insights':^90}")
    print("-" * 90)
    print(f"1. Total P&L is consistent across all methods: {parallel.total_pnl}")
    print(
        f"2. Parallel: {parallel.meta.num_repricings} repricings, {parallel.meta.residual_pct:.2f}% residual (independent factors)"
    )
    print(
        f"3. Waterfall: {waterfall.meta.num_repricings} repricings, {waterfall.meta.residual_pct:.4f}% residual (sequential, near-zero)"
    )
    print(
        f"4. Metrics: {metrics.meta.num_repricings} repricings, {metrics.meta.residual_pct:.2f}% residual (fast approximation)"
    )
    print(
        f"\n   → Metrics-based is {parallel.meta.num_repricings}x faster (0 vs {parallel.meta.num_repricings} repricings)"
    )
    print(f"   → Metrics-based residual: {metrics.meta.residual_pct:.2f}% (improved with 2nd-order terms)")
    print(f"   → Waterfall is exact but order-dependent")
    print(f"   → Parallel gives clean factor separation but requires multiple repricings")

    # Method selection guide
    print(f"\n{'When to Use Each Method':^90}")
    print("-" * 90)
    print("• Parallel:       Factor separation, VaR/risk reporting, regulatory reports")
    print("• Waterfall:      Exact P&L reconciliation, audit trails (beware order dependency)")
    print("• Metrics-based:  Daily P&L, fast analytics, real-time dashboards")


def main():
    """Run all examples with shared instrument and market data."""
    print("\n" + "█" * 70)
    print(" " * 20 + "FINSTACK P&L ATTRIBUTION")
    print("█" * 70)

    # -------------------------------------------------------------------------
    # Shared Setup: One instrument, one market scenario
    # -------------------------------------------------------------------------
    print("\n" + "=" * 70)
    print("Shared Setup")
    print("=" * 70)

    # Create a 5-year corporate bond with 5% coupon
    bond = Bond.fixed_semiannual(
        "CORP-001",
        Money(1_000_000, "USD"),
        0.05,  # 5% coupon
        date(2025, 1, 1),
        date(2030, 1, 1),
        "USD-OIS",
    )
    print(f"Instrument: {bond.instrument_id}")
    print(f"  Notional:    {Money(1_000_000, 'USD')}")
    print(f"  Coupon:      5.0%")
    print(f"  Maturity:    2030-01-01")

    # Market at T₀ (yesterday) with 4% rates
    t0 = date(2025, 1, 15)
    market_t0 = create_market_with_rate(t0, 0.04)
    print(f"\nMarket T₀ ({t0}):")
    print(f"  Discount Rate: 4.0%")

    # Market at T₁ (today) with 4.5% rates (50bp increase)
    t1 = date(2025, 1, 16)
    market_t1 = create_market_with_rate(t1, 0.045)
    print(f"\nMarket T₁ ({t1}):")
    print(f"  Discount Rate: 4.5% (+50bp)")

    print(f"\nMarket Move: +50bp rate increase")
    print(f"Period: {t0} → {t1}")

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

    print("\n" + "=" * 70)
    print("Examples complete! Attribution module ready for production use.")
    print("=" * 70)


if __name__ == "__main__":
    main()
