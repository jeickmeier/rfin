#!/usr/bin/env python3
"""
Example: Using the comprehensive cashflow builder for complex structures.

This demonstrates:
- Building fixed rate loans with amortization
- PIK/Cash toggle structures
- Converting results to DataFrames for analysis
"""

from finstack import Currency, Date, DayCount, Money
from finstack.dates import Frequency, BusDayConvention, StubRule
from finstack.cashflow import (
    CashflowBuilder, 
    CouponPaymentType, 
    Amortization,
    cashflows_to_dataframe
)

def simple_fixed_rate_loan():
    """Build a simple fixed rate loan with quarterly payments."""
    print("=" * 60)
    print("Simple Fixed Rate Loan")
    print("=" * 60)
    
    builder = CashflowBuilder()
    builder.principal(
        Money(10_000_000, Currency("USD")),
        Date(2024, 1, 1),
        Date(2029, 1, 1)
    )
    builder.fixed_coupon(
        rate=0.08,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    builder.with_amortization(Amortization.linear_to_zero(Currency("USD")))
    
    schedule = builder.build()
    
    print(f"Total flows: {len(schedule.flows)}")
    print(f"Total interest: ${schedule.total_interest():,.2f}")
    print(f"Total principal: ${schedule.total_principal():,.2f}")
    print()
    
    # Convert to DataFrame for analysis
    df = cashflows_to_dataframe(schedule)
    print("First 5 cashflows:")
    print(df.head())
    print()
    
    # Analyze by cashflow type
    print("Cashflows by type:")
    type_summary = df.groupby('kind')['amount'].agg(['sum', 'count'])
    print(type_summary)
    print()
    
    return schedule

def pik_toggle_structure():
    """Build a loan with PIK for first year, then cash payments."""
    print("=" * 60)
    print("PIK Toggle Structure")
    print("=" * 60)
    
    builder = CashflowBuilder()
    builder.principal(
        Money(5_000_000, Currency("EUR")),
        Date(2024, 1, 1),
        Date(2027, 1, 1)
    )
    builder.fixed_coupon(
        rate=0.10,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    
    # Add PIK period for first year
    builder.add_pik_period(Date(2024, 1, 1), Date(2025, 1, 1))
    
    # Switch to cash payments after first year
    builder.add_cash_period(Date(2025, 1, 1), Date(2027, 1, 1))
    
    schedule = builder.build()
    
    print(f"Total flows: {len(schedule.flows)}")
    
    # Separate PIK and cash flows
    pik_flows = schedule.pik_flows()
    coupon_flows = schedule.coupons()
    
    print(f"PIK flows: {len(pik_flows)}")
    if pik_flows:
        print(f"  Total PIK interest: €{sum(cf.amount for cf in pik_flows):,.2f}")
    
    print(f"Cash coupon flows: {len(coupon_flows)}")
    if coupon_flows:
        print(f"  Total cash interest: €{sum(cf.amount for cf in coupon_flows):,.2f}")
    print()
    
    # Show outstanding path
    print("Outstanding principal over time:")
    for date, amount in schedule.outstanding_path()[:5]:
        print(f"  {date}: €{amount:,.2f}")
    print("  ...")
    print()
    
    return schedule

def split_coupon_example():
    """Build a loan with split cash/PIK coupons."""
    print("=" * 60)
    print("Split Cash/PIK Coupon")
    print("=" * 60)
    
    builder = CashflowBuilder()
    builder.principal(
        Money(2_000_000, Currency("GBP")),
        Date(2024, 1, 1),
        Date(2026, 1, 1)
    )
    
    # 70% cash, 30% PIK split
    split_type = CouponPaymentType.split(cash_pct=0.7, pik_pct=0.3)
    
    builder.fixed_coupon(
        rate=0.12,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act365f(),
        payment_type=split_type,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    
    schedule = builder.build()
    
    print(f"Total flows: {len(schedule.flows)}")
    
    # Analyze the split
    df = cashflows_to_dataframe(schedule)

    print(df)
    
    print("\nCashflow breakdown:")
    for kind in df['kind'].unique():
        kind_df = df[df['kind'] == kind]
        total = kind_df['amount'].sum()
        print(f"  {kind}: £{total:,.2f} ({len(kind_df)} flows)")
    
    return schedule

def main():
    """Run all examples."""
    
    # Example 1: Simple fixed rate loan
    schedule1 = simple_fixed_rate_loan()
    
    # Example 2: PIK toggle structure
    schedule2 = pik_toggle_structure()
    
    # Example 3: Split coupon
    schedule3 = split_coupon_example()
    
    print("=" * 60)
    print("Summary")
    print("=" * 60)
    print("All examples completed successfully!")
    print()
    print("The cashflow builder supports:")
    print("- Fixed and floating rate coupons")
    print("- Various amortization schedules")
    print("- PIK/Cash/Toggle payment types")
    print("- Multiple fee types")
    print("- DataFrame export for analysis")
    print()
    print("Next steps:")
    print("- Add floating rate support")
    print("- Implement fee calculations")
    print("- Add covenant modeling")
    print("- Support prepayment/call features")

if __name__ == "__main__":
    main()
