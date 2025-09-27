#!/usr/bin/env python3
"""
Test the reorganized finstack-py structure.

This demonstrates:
- Core module (currency, money, dates, market_data)
- Valuations module (cashflow, instruments, results)
- Integration between modules
"""

from finstack import Currency, Money, Date, DayCount
from finstack.dates import Frequency, BusDayConvention, StubRule
from finstack.cashflow import CashflowBuilder, Amortization, CouponPaymentType
from finstack.instruments import Bond
from finstack.market_data import MarketContext


def test_core_modules():
    """Test core module functionality."""
    print("=" * 60)
    print("Testing Core Modules")
    print("=" * 60)

    # Currency
    usd = Currency("USD")
    eur = Currency("EUR")
    print(f"\nCurrencies: {usd}, {eur}")

    # Money
    amount1 = Money(1000.0, usd)
    amount2 = Money(500.0, usd)
    total = amount1 + amount2
    print(f"Money arithmetic: {amount1} + {amount2} = {total}")

    # Dates
    today = Date(2024, 1, 1)
    future = Date(2029, 1, 1)
    dc = DayCount.act360()
    yf = dc.year_fraction(today, future)
    print(f"Date calculation: {today} to {future} = {yf:.4f} years (ACT/360)")

    # Market data context
    context = MarketContext()
    print(f"Market context created: {context}")

    print("\n✓ Core modules working correctly")


def test_valuations_modules():
    """Test valuations module functionality."""
    print("\n" + "=" * 60)
    print("Testing Valuations Modules")
    print("=" * 60)

    # Cashflow builder
    print("\n1. Cashflow Builder:")
    builder = CashflowBuilder()
    builder.principal(
        Money(5_000_000, Currency("USD")), Date(2024, 1, 1), Date(2027, 1, 1)
    )
    builder.fixed_coupon(
        rate=0.06,
        frequency=Frequency.Quarterly,
        day_count=DayCount.thirty360(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None,
    )
    builder.with_amortization(Amortization.linear_to(Money(1_000_000, Currency("USD"))))
    schedule = builder.build()

    print(f"  Created schedule with {len(schedule.flows)} cashflows")
    print(f"  Total interest: ${schedule.total_interest():,.2f}")
    print(f"  Total principal: ${schedule.total_principal():,.2f}")

    # Bond instrument
    print("\n2. Bond Instrument:")
    bond = Bond(
        id="TEST-BOND",
        notional=Money(10_000_000, Currency("USD")),
        coupon=0.05,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        issue_date=Date(2023, 1, 1),
        maturity=Date(2028, 1, 1),
        discount_curve="USD-OIS",
    )
    print(f"  Created bond: {bond}")
    print(f"  Years to maturity: {bond.years_to_maturity(Date(2024, 1, 1)):.2f}")
    print(f"  Remaining coupons: {bond.num_coupons_remaining(Date(2024, 1, 1))}")

    print("\n✓ Valuations modules working correctly")


def test_integration():
    """Test integration between modules."""
    print("\n" + "=" * 60)
    print("Testing Module Integration")
    print("=" * 60)

    # Create a complex structure using both core and valuations
    print("\nCreating PIK/Toggle bond structure:")

    # Use core types
    currency = Currency("EUR")
    notional = Money(2_000_000, currency)
    issue = Date(2024, 1, 1)
    maturity = Date(2026, 12, 31)

    # Build with valuations
    builder = CashflowBuilder()
    builder.principal(notional, issue, maturity)

    # Split payment type
    split_type = CouponPaymentType.split(cash_pct=0.6, pik_pct=0.4)
    builder.fixed_coupon(
        rate=0.08,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act365f(),
        payment_type=split_type,
        business_day_conv=None,
        calendar=None,
        stub=None,
    )

    # Add payment windows
    builder.add_pik_period(Date(2024, 1, 1), Date(2024, 7, 1))
    builder.add_cash_period(Date(2024, 7, 1), Date(2026, 12, 31))

    schedule = builder.build()

    print(f"  Total flows: {len(schedule.flows)}")
    print(f"  PIK flows: {len(schedule.pik_flows())}")
    print(f"  Cash coupons: {len(schedule.coupons())}")

    # Show outstanding path
    path = schedule.outstanding_path()
    if len(path) >= 3:
        print("\n  Outstanding principal path (first 3):")
        for date, amount in path[:3]:
            print(f"    {date}: €{amount:,.2f}")

    print("\n✓ Module integration working correctly")


def main():
    """Run all tests."""
    print("Finstack-py Structure Test")
    print("=" * 60)

    # Test each module area
    test_core_modules()
    test_valuations_modules()
    test_integration()

    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    print("\nReorganization successful! The library now has:")
    print("\n📁 Core Module:")
    print("  - currency: Currency types and operations")
    print("  - money: Currency-safe monetary amounts")
    print("  - dates: Date handling, calendars, day counts")
    print("  - market_data: Curves, FX, market context")

    print("\n📁 Valuations Module:")
    print("  - cashflow: Comprehensive cashflow builder")
    print("  - instruments: Bond and other instruments")
    print("  - results: Valuation results with metrics")

    print("\nThis structure mirrors the Rust library organization,")
    print("making the codebase more maintainable and intuitive.")


if __name__ == "__main__":
    main()
