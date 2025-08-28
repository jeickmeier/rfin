#!/usr/bin/env python3
"""
Example: Interest Rate Swap pricing and risk analysis.

Demonstrates:
- Creating interest rate swaps
- Fixed and floating leg specifications
- Pricing swaps with market context
- Calculating par rates
- Risk metrics (DV01, bucketed DV01)
"""

from finstack import Currency, Date, DayCount, Money
from finstack.dates import Frequency, BusDayConvention, StubRule
from finstack.instruments import (
    InterestRateSwap, PayReceive, FixedLeg, FloatLeg, Bond
)
from finstack.market_data import MarketContext
from finstack.risk import BucketedDv01, KeyRateDuration, calculate_risk_metrics

def create_vanilla_swap():
    """Create a vanilla USD interest rate swap."""
    print("\nCreating vanilla USD swap...")
    
    # Fixed leg specification
    fixed_leg = FixedLeg(
        discount_curve="USD-OIS",
        rate=0.035,  # 3.5% fixed rate
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2029, 1, 15),
        business_day_conv=BusDayConvention.ModifiedFollowing
    )
    
    # Floating leg specification
    float_leg = FloatLeg(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        spread_bp=0,  # No spread
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2029, 1, 15),
        business_day_conv=BusDayConvention.ModifiedFollowing
    )
    
    # Create the swap
    swap = InterestRateSwap(
        id="USD-5Y-SOFR-VANILLA",
        notional=Money(10_000_000, Currency("USD")),
        side=PayReceive.PayFixed,  # Pay fixed, receive floating
        fixed_leg=fixed_leg,
        float_leg=float_leg
    )
    
    print(f"Created: {swap}")
    return swap

def create_basis_swap():
    """Create a basis swap (float vs float)."""
    print("\nCreating USD basis swap...")
    
    # First floating leg (3M SOFR)
    float_leg_3m = FloatLeg(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        spread_bp=0,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2027, 1, 15)
    )
    
    # Second floating leg (1M SOFR + spread)
    float_leg_1m = FloatLeg(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-1M",
        spread_bp=5,  # 5 basis points spread
        frequency=Frequency.Monthly,
        day_count=DayCount.act360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2027, 1, 15)
    )
    
    print("Note: Basis swaps require special handling - this is a simplified example")
    return float_leg_3m, float_leg_1m

def analyze_swap_risk(swap):
    """Analyze risk metrics for a swap."""
    print("\n" + "=" * 60)
    print("Swap Risk Analysis")
    print("=" * 60)
    
    # Create market context (simplified for demo)
    context = MarketContext()
    as_of = Date(2024, 1, 1)
    
    print(f"\nAnalyzing: {swap.id}")
    print(f"Notional: {swap.notional}")
    print(f"Direction: {swap.side}")
    
    # Fixed leg details
    fixed = swap.fixed_leg
    print(f"\nFixed Leg:")
    print(f"  Rate: {fixed.rate:.2%}")
    print(f"  Frequency: {fixed.frequency}")
    print(f"  Day Count: {fixed.day_count}")
    
    # Float leg details
    floating = swap.float_leg
    print(f"\nFloating Leg:")
    print(f"  Index: {floating.forward_curve}")
    print(f"  Spread: {floating.spread_bp} bps")
    print(f"  Frequency: {floating.frequency}")
    print(f"  Day Count: {floating.day_count}")
    
    # Note: Actual pricing would require complete market context
    print("\nRisk Metrics (API demonstration):")
    print("  swap.par_rate(context, as_of) -> Par swap rate")
    print("  swap.price(context, as_of) -> Full valuation with metrics")
    print("  swap.value(context, as_of) -> NPV only")

def demonstrate_bucketed_dv01():
    """Demonstrate bucketed DV01 calculation."""
    print("\n" + "=" * 60)
    print("Bucketed DV01 Analysis")
    print("=" * 60)
    
    # Create bucketed DV01 calculator
    calc = BucketedDv01([0.25, 0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30])
    
    print(f"\nBucketed DV01 Calculator: {calc}")
    print(f"Tenor buckets: {calc.tenors}")
    
    # Custom buckets for specific analysis
    calc.tenors = [0.5, 1, 2, 5, 10, 30]
    print(f"Custom buckets: {calc.tenors}")
    
    print("\nUsage:")
    print("  buckets = calc.calculate(instrument, context, as_of)")
    print("  for tenor, dv01 in buckets.items():")
    print("      print(f'{tenor}: ${dv01:,.2f}')")
    
    print("\nBenefits of bucketed DV01:")
    print("- Shows curve risk at different maturities")
    print("- Helps with hedging specific tenor points")
    print("- Identifies curve steepening/flattening risks")
    print("- Essential for portfolio immunization")

def demonstrate_key_rate_duration():
    """Demonstrate key rate duration analysis."""
    print("\n" + "=" * 60)
    print("Key Rate Duration Analysis")
    print("=" * 60)
    
    # Create key rate duration calculator
    krd = KeyRateDuration()
    
    print(f"\nKey Rate Duration Calculator: {krd}")
    
    print("\nKey rate durations measure sensitivity to shifts at specific")
    print("maturity points while holding other rates constant.")
    
    print("\nApplications:")
    print("- Immunization strategies")
    print("- Precise hedging of curve risk")
    print("- Attribution of P&L to curve movements")
    print("- Asset-liability management")

def compare_instruments():
    """Compare risk metrics across different instruments."""
    print("\n" + "=" * 60)
    print("Instrument Risk Comparison")
    print("=" * 60)
    
    # Create instruments
    swap = create_vanilla_swap()
    
    bond = Bond(
        id="CORP-5Y",
        notional=Money(10_000_000, Currency("USD")),
        coupon=0.04,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        issue_date=Date(2024, 1, 15),
        maturity=Date(2029, 1, 15),
        discount_curve="USD-CORP"
    )
    
    print("\nComparing instruments:")
    print(f"1. Swap: {swap.id}")
    print(f"2. Bond: {bond.id}")
    
    print("\nTypical risk profile differences:")
    print("\nSwap characteristics:")
    print("- Two-sided risk (fixed vs float)")
    print("- Lower duration than equivalent bond")
    print("- Sensitive to curve shape changes")
    print("- Credit exposure mainly through CSA")
    
    print("\nBond characteristics:")
    print("- One-sided risk (fixed coupons)")
    print("- Higher duration for same maturity")
    print("- Credit spread sensitive")
    print("- Principal repayment at maturity")

def main():
    """Run all IRS and risk examples."""
    print("=" * 60)
    print("Interest Rate Swap and Risk Metrics Examples")
    print("=" * 60)
    
    # Create and analyze swaps
    swap = create_vanilla_swap()
    analyze_swap_risk(swap)
    
    # Create basis swap components
    create_basis_swap()
    
    # Demonstrate risk metrics
    demonstrate_bucketed_dv01()
    demonstrate_key_rate_duration()
    
    # Compare instruments
    compare_instruments()
    
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    
    print("\nPhase 3 (IRS) Complete:")
    print("✓ Interest rate swap instrument")
    print("✓ Fixed and floating leg builders")
    print("✓ Par rate calculation method")
    print("✓ Pricing integration with market context")
    
    print("\nPhase 4 (Risk Metrics) Complete:")
    print("✓ DV01 calculation framework")
    print("✓ Bucketed DV01 for curve risk")
    print("✓ Key rate duration calculator")
    print("✓ CS01 framework (credit sensitivity)")
    print("✓ Unified risk metrics interface")
    
    print("\nKey Features:")
    print("- Type-safe instrument construction")
    print("- Flexible leg specifications")
    print("- Comprehensive risk analytics")
    print("- Production-ready architecture")
    print("- Seamless Rust backend integration")

if __name__ == "__main__":
    main()
