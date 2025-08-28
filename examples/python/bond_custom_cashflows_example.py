#!/usr/bin/env python3
"""
Example demonstrating Bond instruments with custom cashflow schedules.

This example shows how to create bonds with complex cashflow patterns
using the cashflow builder and pass them to bond instruments.
"""

import finstack as fs
from finstack.instruments import Bond
from finstack.cashflow import (
    CashflowBuilder, 
    CouponPaymentType, 
    Amortization,
    cashflows_to_dataframe
)
import pandas as pd


def example_stepup_bond():
    """Create a bond with step-up coupons using custom cashflows."""
    print("\n=== Step-Up Bond with Custom Cashflows ===")
    
    # Create dates
    issue = fs.Date(2025, 1, 15)
    maturity = fs.Date(2028, 1, 15)
    
    # Build custom cashflow schedule
    # Note: Step-up rates would typically be implemented with multiple
    # fixed_coupon calls or custom windows (pending full API implementation)
    builder = CashflowBuilder()
    builder.principal(
        fs.Money(1_000_000, fs.Currency("USD")),
        issue,
        maturity
    )
    
    # For this example, using a single rate
    # In full implementation, would use builder.fixed_stepup() or similar
    builder.fixed_coupon(
        rate=0.04,  # Starting at 4%
        frequency=fs.Frequency.SemiAnnual,
        day_count=fs.DayCount.act365f(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    
    custom_schedule = builder.build()
    
    # Create bond from custom cashflows
    bond = Bond.from_cashflows(
        id="STEPUP_BOND_2028",
        schedule=custom_schedule,
        discount_curve="USD-OIS",
        quoted_clean_price=98.5
    )
    
    print(f"Created bond: {bond.id}")
    print(f"Issue date: {bond.issue_date}")
    print(f"Maturity: {bond.maturity}")
    print(f"Quoted clean price: {bond.quoted_clean_price}")
    
    # Analyze cashflows
    print(f"\nTotal cashflows: {len(custom_schedule.flows)}")
    df = cashflows_to_dataframe(custom_schedule)
    print("\nCashflow breakdown by type:")
    print(df.groupby('kind')['amount'].agg(['count', 'sum']))


def example_pik_toggle_bond():
    """Create a bond with PIK toggle features."""
    print("\n=== PIK Toggle Bond ===")
    
    issue = fs.Date(2025, 3, 1)
    maturity = fs.Date(2027, 3, 1)
    
    # Build schedule with PIK toggle
    builder = CashflowBuilder()
    builder.principal(
        fs.Money(10_000_000, fs.Currency("USD")),
        issue,
        maturity
    )
    
    # Create split payment type: 50% cash, 50% PIK
    split_type = CouponPaymentType.split(cash_pct=0.5, pik_pct=0.5)
    
    builder.fixed_coupon(
        rate=0.08,
        frequency=fs.Frequency.Quarterly,
        day_count=fs.DayCount.thirty360(),
        payment_type=split_type,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    
    custom_schedule = builder.build()
    
    # Create bond with custom cashflows in constructor
    bond = Bond(
        id="PIK_TOGGLE_2027",
        notional=fs.Money(10_000_000, fs.Currency("USD")),
        coupon=0.08,
        frequency=fs.Frequency.Quarterly,
        day_count=fs.DayCount.thirty360(),
        issue_date=issue,
        maturity=maturity,
        discount_curve="USD-OIS",
        custom_cashflows=custom_schedule  # Pass custom cashflows
    )
    
    print(f"Created PIK toggle bond: {bond.id}")
    print(f"Notional: {bond.notional}")
    print(f"Coupon rate: {bond.coupon:.2%}")
    
    # Analyze PIK vs cash flows
    pik_flows = custom_schedule.pik_flows()
    cash_flows = custom_schedule.coupons()
    
    print(f"\nCashflow breakdown:")
    print(f"  PIK flows: {len(pik_flows)}")
    if pik_flows:
        total_pik = sum(cf.amount for cf in pik_flows)
        print(f"  Total PIK amount: ${total_pik:,.2f}")
    
    print(f"  Cash coupon flows: {len(cash_flows)}")
    if cash_flows:
        total_cash = sum(cf.amount for cf in cash_flows)
        print(f"  Total cash coupons: ${total_cash:,.2f}")


def example_amortizing_bond():
    """Create an amortizing bond."""
    print("\n=== Amortizing Bond ===")
    
    issue = fs.Date(2025, 6, 1)
    maturity = fs.Date(2030, 6, 1)
    
    # Build cashflow schedule with amortization
    builder = CashflowBuilder()
    builder.principal(
        fs.Money(50_000_000, fs.Currency("EUR")),
        issue,
        maturity
    )
    
    # Linear amortization to 20% of original
    builder.with_amortization(
        Amortization.linear_to(
            fs.Money(10_000_000, fs.Currency("EUR"))
        )
    )
    
    # Fixed coupon
    builder.fixed_coupon(
        rate=0.045,
        frequency=fs.Frequency.SemiAnnual,
        day_count=fs.DayCount.act360(),
        payment_type=None,
        business_day_conv=fs.BusDayConvention.ModifiedFollowing,
        calendar="EUR",
        stub=None
    )
    
    custom_schedule = builder.build()
    
    # Create bond from cashflows
    bond = Bond.from_cashflows(
        id="AMORT_BOND_2030",
        schedule=custom_schedule,
        discount_curve="EUR-OIS",
        quoted_clean_price=None
    )
    
    print(f"Created amortizing bond: {bond.id}")
    print(f"Initial notional: {custom_schedule.notional}")
    
    # Show amortization profile
    principal_flows = custom_schedule.principal_flows()
    amort_flows = [cf for cf in principal_flows if cf.kind == "Amortization"]
    
    print(f"\nAmortization profile:")
    print(f"  Number of amortizations: {len(amort_flows)}")
    if amort_flows:
        total_amort = sum(abs(cf.amount) for cf in amort_flows)
        print(f"  Total amortization: EUR {total_amort:,.2f}")
    
    # Show outstanding path
    print("\nOutstanding principal over time (first 5):")
    for i, (date, amount) in enumerate(custom_schedule.outstanding_path()[:5], 1):
        print(f"  {i}. {date}: EUR {amount}")


def example_comparison_regular_vs_custom():
    """Compare regular bond vs bond with custom cashflows."""
    print("\n=== Regular Bond vs Custom Cashflow Bond ===")
    
    issue = fs.Date(2025, 1, 1)
    maturity = fs.Date(2026, 1, 1)
    
    # Create regular bond
    regular_bond = Bond(
        id="REGULAR_BOND",
        notional=fs.Money(1_000_000, fs.Currency("USD")),
        coupon=0.05,
        frequency=fs.Frequency.Annual,
        day_count=fs.DayCount.act365f(),
        issue_date=issue,
        maturity=maturity,
        discount_curve="USD-OIS"
    )
    
    # Create custom cashflow schedule with higher frequency
    builder = CashflowBuilder()
    builder.principal(
        fs.Money(1_000_000, fs.Currency("USD")),
        issue,
        maturity
    )
    builder.fixed_coupon(
        rate=0.05,
        frequency=fs.Frequency.SemiAnnual,  # Higher frequency
        day_count=fs.DayCount.act365f(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    custom_schedule = builder.build()
    
    # Apply custom cashflows to create new bond
    custom_bond = regular_bond.with_cashflows(custom_schedule)
    
    print(f"Regular bond: {regular_bond.id}")
    print(f"  Frequency: {regular_bond.frequency}")
    print(f"  Coupon: {regular_bond.coupon:.2%}")
    
    print(f"\nBond with custom cashflows: {custom_bond.id}")
    print(f"  Original frequency: {custom_bond.frequency} (overridden)")
    print(f"  Custom schedule flows: {len(custom_schedule.flows)}")
    
    # Compare cashflow structures
    df = cashflows_to_dataframe(custom_schedule)
    print("\nCustom cashflow summary:")
    print(df[['date', 'amount', 'currency', 'kind']].head())


def example_dataframe_analysis():
    """Show how to analyze bond cashflows using DataFrames."""
    print("\n=== Cashflow DataFrame Analysis ===")
    
    issue = fs.Date(2025, 1, 1)
    maturity = fs.Date(2027, 1, 1)
    
    # Build a cashflow schedule
    builder = CashflowBuilder()
    builder.principal(
        fs.Money(5_000_000, fs.Currency("USD")),
        issue,
        maturity
    )
    builder.fixed_coupon(
        rate=0.06,
        frequency=fs.Frequency.Quarterly,
        day_count=fs.DayCount.act360(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None
    )
    
    schedule = builder.build()
    
    # Create bond from schedule
    bond = Bond.from_cashflows(
        id="ANALYSIS_BOND",
        schedule=schedule,
        discount_curve="USD-OIS",
        quoted_clean_price=99.5
    )
    
    # Convert to DataFrame for analysis
    df = cashflows_to_dataframe(schedule)
    
    print("Cashflow DataFrame (first 5 rows):")
    print(df[['date', 'amount', 'currency', 'kind', 'accrual_factor']].head())
    
    # Summary statistics
    print("\nCashflow Summary:")
    print(f"  Total rows: {len(df)}")
    print(f"  Date range: {df['date'].min()} to {df['date'].max()}")
    print(f"  Total amount: ${df['amount'].sum():,.2f}")
    
    # Group by kind
    print("\nCashflows by type:")
    summary = df.groupby('kind')['amount'].agg(['count', 'sum', 'mean'])
    print(summary)
    
    # Timeline analysis
    print("\nQuarterly cashflow timeline:")
    df['year'] = pd.DatetimeIndex(df['date']).year
    df['quarter'] = pd.DatetimeIndex(df['date']).quarter
    quarterly = df.groupby(['year', 'quarter'])['amount'].sum()
    print(quarterly.head(8))


if __name__ == "__main__":
    print("=" * 60)
    print("Bond Instruments with Custom Cashflow Schedules")
    print("=" * 60)
    
    try:
        # Run examples
        example_stepup_bond()
        example_pik_toggle_bond()
        example_amortizing_bond()
        example_comparison_regular_vs_custom()
        example_dataframe_analysis()
        
        print("\n" + "=" * 60)
        print("Examples completed successfully!")
        print("=" * 60)
        print("\nNote: Full pricing with market context is pending")
        print("complete Python bindings implementation.")
        
    except Exception as e:
        print(f"\nError running examples: {e}")
        import traceback
        traceback.print_exc()