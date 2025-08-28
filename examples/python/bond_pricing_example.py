#!/usr/bin/env python3
"""
Example: Bond valuation with market context.

Demonstrates:
- Creating bonds with various features
- Setting up market context with discount curves
- Pricing bonds and extracting metrics
- Analyzing results
"""

from finstack import Currency, Date, DayCount, Money
from finstack.dates import Frequency
from finstack.instruments import Bond
from finstack.market_data import MarketContext, DiscountCurve

def create_corporate_bond():
    """Create a typical corporate bond."""
    return Bond(
        id="CORP-5Y-4.5%",
        notional=Money(1_000_000, Currency("USD")),
        coupon=0.045,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        issue_date=Date(2023, 1, 15),
        maturity=Date(2028, 1, 15),
        discount_curve="USD-CORP-AA"
    )

def create_treasury_bond():
    """Create a treasury bond."""
    return Bond(
        id="UST-10Y-3.75%",
        notional=Money(1_000_000, Currency("USD")),
        coupon=0.0375,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.act365f(),
        issue_date=Date(2020, 3, 15),
        maturity=Date(2030, 3, 15),
        discount_curve="USD-TSY"
    )

def setup_market_context():
    """Set up market context with discount curves."""
    print("Setting up market context...")
    
    # Create market context
    context = MarketContext()
    
    # In practice, you would add actual discount curves here
    # For now, we'll use a simplified setup
    # context.add_discount_curve(
    #     DiscountCurve.from_rates("USD-TSY", base_date, rates)
    # )
    
    print("Market context ready (simplified for demo)")
    return context

def analyze_bond(bond, context, as_of):
    """Analyze a bond's valuation and metrics."""
    print(f"\n{'=' * 60}")
    print(f"Analyzing: {bond}")
    print(f"{'=' * 60}")
    
    # Basic properties
    print(f"\nBond Properties:")
    print(f"  Notional: {bond.notional}")
    print(f"  Coupon: {bond.coupon:.2%}")
    print(f"  Frequency: {bond.frequency}")
    print(f"  Day Count: {bond.day_count}")
    print(f"  Maturity: {bond.maturity}")
    
    # Calculate some basic metrics
    years_to_maturity = bond.years_to_maturity(as_of)
    print(f"\nTime Metrics (as of {as_of}):")
    print(f"  Years to Maturity: {years_to_maturity:.2f}")
    print(f"  Remaining Coupons: {bond.num_coupons_remaining(as_of)}")
    
    # Note: Full pricing would require complete market context
    # This demonstrates the API structure
    try:
        # This would work with a fully configured market context
        # result = bond.price(context, as_of)
        # print(f"\nValuation Results:")
        # print(f"  Present Value: ${result.value.amount:,.2f}")
        # print(f"  YTM: {result.get_metric('Ytm', 0):.2%}")
        # print(f"  Modified Duration: {result.get_metric('DurationMod', 0):.2f}")
        # print(f"  Convexity: {result.get_metric('Convexity', 0):.2f}")
        # print(f"  DV01: ${result.get_metric('Dv01', 0):,.2f}")
        
        print(f"\nNote: Full pricing requires complete market data setup")
        print(f"The pricing API is available as: bond.price(context, as_of)")
        
    except Exception as e:
        print(f"\nPricing not available in demo: {e}")

def compare_bonds():
    """Compare multiple bonds."""
    print("\n" + "=" * 60)
    print("Bond Comparison")
    print("=" * 60)
    
    # Create bonds
    corporate = create_corporate_bond()
    treasury = create_treasury_bond()
    
    as_of = Date(2024, 1, 1)
    
    # Compare basic features
    bonds = [corporate, treasury]
    
    print("\nComparison Table:")
    print(f"{'Feature':<20} {'Corporate':<20} {'Treasury':<20}")
    print("-" * 60)
    
    for label, getter in [
        ("ID", lambda b: b.id),
        ("Coupon", lambda b: f"{b.coupon:.2%}"),
        ("Maturity", lambda b: str(b.maturity)),
        ("Years to Mat.", lambda b: f"{b.years_to_maturity(as_of):.2f}"),
        ("Remaining Coupons", lambda b: str(b.num_coupons_remaining(as_of))),
    ]:
        corp_val = getter(corporate)
        tsy_val = getter(treasury)
        print(f"{label:<20} {corp_val:<20} {tsy_val:<20}")

def main():
    """Run all bond examples."""
    print("=" * 60)
    print("Bond Valuation Examples")
    print("=" * 60)
    
    # Set up market context
    context = setup_market_context()
    as_of = Date(2024, 1, 1)
    
    # Analyze individual bonds
    corporate = create_corporate_bond()
    treasury = create_treasury_bond()
    
    analyze_bond(corporate, context, as_of)
    analyze_bond(treasury, context, as_of)
    
    # Compare bonds
    compare_bonds()
    
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    print("\nThe bond pricing infrastructure is in place with:")
    print("- Bond instrument with full parameterization")
    print("- Market context for holding curves and market data")
    print("- Valuation results with metrics")
    print("- Python-friendly API with type hints")
    print("\nNext steps:")
    print("- Complete discount curve builders")
    print("- Add more instrument types (IRS, FRN, etc.)")
    print("- Implement risk metrics (DV01, CS01)")
    print("- Add scenario analysis capabilities")

if __name__ == "__main__":
    main()
