#!/usr/bin/env python3
"""
Daily P&L Attribution Example

Demonstrates how to perform P&L attribution on a bond position to explain
daily MTM changes.
"""

from datetime import date
from finstack import Money
from finstack.valuations import Bond
from finstack.valuations.attribution import AttributionMethod, attribute_pnl
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve


def create_market_with_rate(as_of_date, rate):
    """Helper to create a market context with a flat discount curve."""
    # Create a flat discount curve using direct constructor
    # DiscountCurve(id, base_date, knots)
    curve = DiscountCurve(
        "USD-OIS",
        as_of_date,
        [
            (0.0, 1.0),
            (5.0, (1.0 + rate) ** -5)  # Approximate discount factor
        ]
    )
    
    market = MarketContext()
    market.insert_discount(curve)
    return market


def example_bond_attribution():
    """Example: Bond P&L attribution with curve shift."""
    print("=" * 70)
    print("Example 1: Bond Attribution with Curve Shift")
    print("=" * 70)
    
    # Create a 5-year corporate bond
    bond = Bond.fixed_semiannual(
        "CORP-001",
        Money(1_000_000, "USD"),
        0.05,  # 5% coupon
        date(2025, 1, 1),
        date(2030, 1, 1),
        "USD-OIS"
    )
    
    # Market at T₀ (yesterday) with 4% rates
    market_t0 = create_market_with_rate(date(2025, 1, 15), 0.04)
    
    # Market at T₁ (today) with 4.5% rates (rates increased)
    market_t1 = create_market_with_rate(date(2025, 1, 16), 0.045)
    
    # Run parallel attribution
    attr = attribute_pnl(
        bond,
        market_t0,
        market_t1,
        date(2025, 1, 15),
        date(2025, 1, 16),
        method=AttributionMethod.parallel()
    )
    
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
    
    # Check tolerance
    print(f"\n{'Validation':^70}")
    print("-" * 70)
    is_within_tolerance = attr.residual_within_tolerance(0.1, 100.0)
    print(f"Residual within tolerance (0.1% or $100): {is_within_tolerance}")
    
    # Export to CSV
    print(f"\n{'CSV Export (first 5 lines)':^70}")
    print("-" * 70)
    csv_lines = attr.to_csv().split('\n')
    for line in csv_lines[:5]:
        print(line)
    
    return attr


def example_waterfall_attribution():
    """Example: Waterfall attribution with custom factor order."""
    print("\n\n" + "=" * 70)
    print("Example 2: Waterfall Attribution with Custom Order")
    print("=" * 70)
    
    bond = Bond.fixed_semiannual(
        "CORP-002",
        Money(500_000, "USD"),
        0.04,
        date(2025, 1, 1),
        date(2028, 1, 1),
        "USD-OIS"
    )
    
    market = create_market_with_rate(date(2025, 1, 15), 0.04)
    
    # Custom waterfall order
    method = AttributionMethod.waterfall([
        "carry",
        "rates_curves",
        "credit_curves",
        "fx",
        "volatility"
    ])
    
    attr = attribute_pnl(
        bond,
        market,
        market,
        date(2025, 1, 15),
        date(2025, 1, 16),
        method=method
    )
    
    print(f"\nWaterfall residual: {attr.residual} ({attr.meta.residual_pct:.4f}%)")
    print(f"(Waterfall should have near-zero residual)")


def main():
    """Run all examples."""
    print("\n" + "█" * 70)
    print(" " * 20 + "FINSTACK P&L ATTRIBUTION")
    print("█" * 70)
    
    # Run examples
    attr = example_bond_attribution()
    example_waterfall_attribution()
    
    print("\n" + "=" * 70)
    print("Examples complete! Attribution module ready for production use.")
    print("=" * 70)


if __name__ == "__main__":
    main()


