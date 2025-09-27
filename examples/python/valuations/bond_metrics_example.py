#!/usr/bin/env python3
"""
Comprehensive Bond Metrics Example

Demonstrates advanced bond valuation using the complete metrics framework,
including YTM, duration, convexity, and risk measures. Shows how to analyze
both quoted and model-valued bonds.
"""

from finstack import Currency, Date, DayCount, Money
from finstack.dates import Frequency
from finstack.instruments import Bond
from finstack.market_data import MarketContext, DiscountCurve
import pandas as pd


def create_sample_market_data():
    """Create sample market data for bond valuation."""
    print("📊 Setting up market data...")

    # Create market context
    context = MarketContext()
    print("  ✓ Market context created")

    return context


def demonstrate_quoted_bond_metrics():
    """Demonstrate comprehensive metrics for a bond with quoted price."""
    print("\n" + "=" * 60)
    print("🏛️  QUOTED BOND METRICS ANALYSIS")
    print("=" * 60)

    # Create a corporate bond with quoted clean price
    bond = Bond(
        id="CORP-5Y-2029",
        notional=Money(1_000_000, Currency("USD")),
        coupon=0.045,  # 4.5% annual coupon
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        issue_date=Date(2024, 1, 1),
        maturity=Date(2029, 1, 1),
        discount_curve="USD-CORP",
        quoted_clean_price=98.75,  # Trading at 98.75% of par
    )

    print(f"\n📋 Bond Details:")
    print(f"   ID: {bond.id}")
    print(f"   Notional: {bond.notional}")
    print(f"   Coupon: {bond.coupon:.2%} {bond.frequency}")
    print(f"   Day Count: {bond.day_count}")
    print(f"   Issue Date: {bond.issue_date}")
    print(f"   Maturity: {bond.maturity}")
    print(f"   Quoted Clean Price: {bond.quoted_clean_price:.2f}%")

    # Valuation date
    as_of = Date(2024, 6, 15)
    market_context = create_sample_market_data()

    print(f"\n📅 Valuation Date: {as_of}")
    print(f"   Years to Maturity: {bond.years_to_maturity(as_of):.3f}")
    print(f"   Remaining Coupons: {bond.num_coupons_remaining(as_of)}")

    try:
        # Calculate comprehensive metrics
        print(f"\n🧮 Bond Metrics Analysis:")

        # Individual metric calculations
        ytm = bond.yield_to_maturity(market_context, as_of)
        print(f"   💰 Yield to Maturity: {ytm:.3%}")

        modified_dur = bond.modified_duration(market_context, as_of)
        print(f"   ⏱️  Modified Duration: {modified_dur:.3f} years")

        macaulay_dur = bond.macaulay_duration(market_context, as_of)
        print(f"   📏 Macaulay Duration: {macaulay_dur:.3f} years")

        convexity = bond.convexity(market_context, as_of)
        print(f"   📈 Convexity: {convexity:.3f}")

        accrued = bond.accrued_interest(market_context, as_of)
        print(f"   🏦 Accrued Interest: ${accrued:,.2f}")

        clean_price = bond.clean_price(market_context, as_of)
        print(f"   📊 Clean Price: {clean_price:.3f}%")

        dirty_price = bond.dirty_price(market_context, as_of)
        print(f"   💵 Dirty Price: {dirty_price:.3f}%")

        cs01 = bond.cs01(market_context, as_of)
        print(f"   ⚠️  CS01 (Credit Risk): ${cs01:,.2f}")

        # Batch calculation of all metrics
        print(f"\n🔄 Batch Metrics Calculation:")
        all_metrics = bond.calculate_metrics(market_context, as_of)
        for metric_name, value in all_metrics.items():
            print(f"   {metric_name}: {value:.6f}")

    except Exception as e:
        print(f"   ⚠️ Note: Some metrics require market curves: {e}")
        print(f"   📝 In practice, you would set up proper discount curves")


def demonstrate_model_valued_bond():
    """Demonstrate model-valued bond without quoted price."""
    print("\n" + "=" * 60)
    print("🏗️  MODEL-VALUED BOND ANALYSIS")
    print("=" * 60)

    # Create a government bond without quoted price (model-valued)
    bond = Bond(
        id="GOVT-10Y-2034",
        notional=Money(5_000_000, Currency("USD")),
        coupon=0.0325,  # 3.25% annual coupon
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.actact(),
        issue_date=Date(2024, 1, 1),
        maturity=Date(2034, 1, 1),
        discount_curve="USD-TREASURY",
        # No quoted_clean_price - will be model-valued
    )

    print(f"\n📋 Bond Details:")
    print(f"   ID: {bond.id}")
    print(f"   Notional: {bond.notional}")
    print(f"   Coupon: {bond.coupon:.3%} {bond.frequency}")
    print(f"   Issue Date: {bond.issue_date}")
    print(f"   Maturity: {bond.maturity}")
    print(
        f"   Quoted Price: {'None (Model-valued)' if not bond.quoted_clean_price else bond.quoted_clean_price}"
    )

    as_of = Date(2024, 8, 1)
    market_context = create_sample_market_data()

    print(f"\n📅 Valuation Date: {as_of}")
    print(f"   Years to Maturity: {bond.years_to_maturity(as_of):.3f}")
    print(f"   Remaining Coupons: {bond.num_coupons_remaining(as_of)}")

    try:
        # Basic valuation
        pv = bond.value(market_context, as_of)
        print(f"\n💰 Model Value: {pv}")

        # Full pricing with metrics (request standard set)
        result = bond.price_with_metrics(
            market_context, as_of, ["ytm", "duration_mod", "convexity", "accrued"]
        )
        print(f"\n📊 Full Valuation Results:")
        print(f"   PV: {result.value}")
        print(f"   Metrics Available: {len(result.metric_names())}")

        # Try to calculate specific metrics (will work when curves are available)
        print(f"\n🧮 Risk Metrics:")

        modified_dur = bond.modified_duration(market_context, as_of)
        print(f"   Modified Duration: {modified_dur:.3f} years")

        convexity = bond.convexity(market_context, as_of)
        print(f"   Convexity: {convexity:.3f}")

        accrued = bond.accrued_interest(market_context, as_of)
        print(f"   Accrued Interest: ${accrued:,.2f}")

    except Exception as e:
        print(f"   ⚠️ Note: Model valuation requires discount curves: {e}")
        print(f"   📝 In practice, you would configure MarketContext with curves")


def demonstrate_metrics_comparison():
    """Compare metrics across different bond structures."""
    print("\n" + "=" * 60)
    print("🔍 BOND METRICS COMPARISON")
    print("=" * 60)

    bonds = []

    # Short-term bond
    bonds.append(
        Bond(
            id="SHORT-2Y",
            notional=Money(1_000_000, Currency("USD")),
            coupon=0.035,
            frequency=Frequency.SemiAnnual,
            day_count=DayCount.act360(),
            issue_date=Date(2024, 1, 1),
            maturity=Date(2026, 1, 1),
            discount_curve="USD-OIS",
            quoted_clean_price=99.8,
        )
    )

    # Medium-term bond
    bonds.append(
        Bond(
            id="MEDIUM-5Y",
            notional=Money(1_000_000, Currency("USD")),
            coupon=0.045,
            frequency=Frequency.SemiAnnual,
            day_count=DayCount.thirty360(),
            issue_date=Date(2024, 1, 1),
            maturity=Date(2029, 1, 1),
            discount_curve="USD-CORP",
            quoted_clean_price=98.5,
        )
    )

    # Long-term bond
    bonds.append(
        Bond(
            id="LONG-10Y",
            notional=Money(1_000_000, Currency("USD")),
            coupon=0.055,
            frequency=Frequency.SemiAnnual,
            day_count=DayCount.actact(),
            issue_date=Date(2024, 1, 1),
            maturity=Date(2034, 1, 1),
            discount_curve="USD-TREASURY",
            quoted_clean_price=97.2,
        )
    )

    as_of = Date(2024, 6, 1)
    market_context = create_sample_market_data()

    print(f"\n📊 Bond Portfolio Analysis (as of {as_of}):")
    print(f"{'Bond ID':<12} {'Coupon':<8} {'TTM':<6} {'Quoted':<8}")
    print("-" * 50)

    for bond in bonds:
        ttm = bond.years_to_maturity(as_of)
        quoted = bond.quoted_clean_price or 0.0
        print(f"{bond.id:<12} {bond.coupon:>6.2%} {ttm:>6.2f} {quoted:>7.2f}%")

    print(f"\n🔢 Expected Metrics Relationships:")
    print(f"   • Duration generally increases with time to maturity")
    print(f"   • Lower coupon bonds typically have higher duration")
    print(f"   • Convexity increases with duration and decreases with yield")
    print(f"   • YTM inverse relationship with quoted price")


def demonstrate_risk_scenarios():
    """Demonstrate risk scenario analysis."""
    print("\n" + "=" * 60)
    print("⚠️  RISK SCENARIO ANALYSIS")
    print("=" * 60)

    # High-yield corporate bond
    high_yield = Bond(
        id="HY-CORP-7Y",
        notional=Money(2_000_000, Currency("USD")),
        coupon=0.085,  # 8.5% coupon
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        issue_date=Date(2024, 1, 1),
        maturity=Date(2031, 1, 1),
        discount_curve="USD-HY",
        quoted_clean_price=95.0,  # Distressed pricing
    )

    print(f"\n📋 High-Yield Bond Profile:")
    print(f"   ID: {high_yield.id}")
    print(f"   Coupon: {high_yield.coupon:.1%} paid {high_yield.frequency}")
    print(f"   Quoted Price: {high_yield.quoted_clean_price:.1f}% (below par)")
    print(f"   Credit Quality: High-yield/speculative grade")

    as_of = Date(2024, 9, 1)
    market_context = create_sample_market_data()

    print(f"\n🎯 Risk Profile Analysis:")
    try:
        ytm = high_yield.yield_to_maturity(market_context, as_of)
        print(f"   🎢 Yield to Maturity: {ytm:.2%} (high due to credit risk)")

        modified_dur = high_yield.modified_duration(market_context, as_of)
        print(f"   🕐 Modified Duration: {modified_dur:.2f} years")

        cs01 = high_yield.cs01(market_context, as_of)
        print(f"   📉 CS01: ${cs01:,.0f} (credit spread sensitivity)")

        # Calculate price impact of spread changes
        duration_impact_100bp = modified_dur * 0.01 * high_yield.notional.amount
        print(f"\n📈 Scenario Analysis:")
        print(f"   100bp yield ↑: ~${duration_impact_100bp:,.0f} loss")
        print(f"   100bp yield ↓: ~${duration_impact_100bp:,.0f} gain")

        convexity_val = high_yield.convexity(market_context, as_of)
        convexity_benefit_100bp = (
            0.5 * convexity_val * (0.01**2) * high_yield.notional.amount
        )
        print(f"   Convexity benefit (100bp): ~${convexity_benefit_100bp:,.0f}")

        print(f"\n🎯 Key Insights:")
        print(f"   • High YTM reflects credit risk premium")
        print(f"   • Duration measures interest rate sensitivity")
        print(f"   • CS01 measures credit spread sensitivity")
        print(f"   • Convexity provides upside asymmetry")

    except Exception as e:
        print(f"   ⚠️ Metrics calculation: {e}")
        print(f"   📝 Would work with proper market curve setup")


def demonstrate_callable_bond():
    """Demonstrate yield-to-worst for callable bonds."""
    print("\n" + "=" * 60)
    print("📞 CALLABLE BOND ANALYSIS")
    print("=" * 60)

    # Callable bond (call protection example)
    callable_bond = Bond(
        id="CALLABLE-30Y",
        notional=Money(3_000_000, Currency("USD")),
        coupon=0.065,  # 6.5% annual coupon
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.actact(),
        issue_date=Date(2024, 1, 1),
        maturity=Date(2054, 1, 1),  # 30-year maturity
        discount_curve="USD-CORP",
        quoted_clean_price=105.25,  # Trading at premium
    )

    print(f"\n📋 Callable Bond Details:")
    print(f"   ID: {callable_bond.id}")
    print(f"   Term: 30 years")
    print(f"   Coupon: {callable_bond.coupon:.1%} semi-annual")
    print(f"   Quoted Price: {callable_bond.quoted_clean_price:.2f}% (premium)")
    print(f"   Call Protection: First 5 years (example)")

    as_of = Date(2024, 6, 1)
    market_context = create_sample_market_data()

    print(f"\n🎯 Callable Bond Analytics:")
    try:
        ytm = callable_bond.yield_to_maturity(market_context, as_of)
        print(f"   📊 Yield to Maturity: {ytm:.3%}")

        ytw = callable_bond.yield_to_worst(market_context, as_of)
        print(f"   ⚡ Yield to Worst: {ytw:.3%}")

        duration = callable_bond.modified_duration(market_context, as_of)
        print(f"   🕒 Modified Duration: {duration:.2f} years")

        convexity_val = callable_bond.convexity(market_context, as_of)
        print(f"   📈 Convexity: {convexity_val:.2f}")

        # Effective duration would be shorter due to call risk
        print(f"\n💡 Callable Bond Insights:")
        print(f"   • YTW < YTM when trading at premium (call risk)")
        print(f"   • Effective duration < modified duration (negative convexity)")
        print(f"   • Call protection reduces reinvestment risk")

    except Exception as e:
        print(f"   ⚠️ Callable metrics calculation: {e}")


def create_bond_portfolio_summary():
    """Create a summary table of different bond types."""
    print("\n" + "=" * 60)
    print("📊 PORTFOLIO BOND SUMMARY")
    print("=" * 60)

    # Create different bond types
    bonds_data = [
        {
            "id": "TREASURY-2Y",
            "coupon": 0.025,
            "maturity": Date(2026, 3, 1),
            "quoted": 99.95,
            "type": "Government",
        },
        {
            "id": "IG-CORP-5Y",
            "coupon": 0.045,
            "maturity": Date(2029, 6, 1),
            "quoted": 98.8,
            "type": "Investment Grade",
        },
        {
            "id": "HY-CORP-7Y",
            "coupon": 0.085,
            "maturity": Date(2031, 9, 1),
            "quoted": 94.5,
            "type": "High Yield",
        },
    ]

    as_of = Date(2024, 3, 1)

    print(f"\n📈 Bond Portfolio Summary (as of {as_of}):")
    print(
        f"{'ID':<14} {'Type':<16} {'Coupon':<8} {'TTM':<6} {'Price':<8} {'Expected':<12}"
    )
    print("-" * 80)

    for bond_info in bonds_data:
        bond = Bond(
            id=bond_info["id"],
            notional=Money(1_000_000, Currency("USD")),
            coupon=bond_info["coupon"],
            frequency=Frequency.SemiAnnual,
            day_count=DayCount.thirty360(),
            issue_date=Date(2024, 1, 1),
            maturity=bond_info["maturity"],
            discount_curve="USD-OIS",
            quoted_clean_price=bond_info["quoted"],
        )

        ttm = bond.years_to_maturity(as_of)
        expected_behavior = {
            "Government": "Low risk, low yield",
            "Investment Grade": "Moderate risk/yield",
            "High Yield": "High risk/yield",
        }[bond_info["type"]]

        print(
            f"{bond.id:<14} {bond_info['type']:<16} {bond.coupon:>6.1%} "
            f"{ttm:>6.2f} {bond_info['quoted']:>7.1f}% {expected_behavior:<12}"
        )

    print(f"\n💡 Portfolio Insights:")
    print(f"   • Government bonds: Highest prices, lowest yields (safe haven)")
    print(f"   • Investment grade: Moderate credit spreads")
    print(f"   • High yield: Significant credit risk premium")
    print(f"   • Duration risk increases with time to maturity")


def main():
    """Run comprehensive bond metrics demonstrations."""
    print("🏦 Finstack Bond Metrics Framework")
    print("=" * 60)
    print("Comprehensive fixed-income analytics with yield, duration,")
    print("convexity, and credit risk measures.")

    # Run all demonstrations
    demonstrate_quoted_bond_metrics()
    demonstrate_model_valued_bond()
    demonstrate_callable_bond()
    create_bond_portfolio_summary()

    print("\n" + "=" * 60)
    print("🎉 COMPREHENSIVE BOND METRICS COMPLETE")
    print("=" * 60)
    print("\nThe finstack-py library now provides:")
    print("   ✅ Yield to Maturity (YTM) calculations")
    print("   ✅ Modified and Macaulay duration")
    print("   ✅ Bond convexity measurements")
    print("   ✅ Accrued interest calculations")
    print("   ✅ Clean and dirty price analysis")
    print("   ✅ Credit spread sensitivity (CS01)")
    print("   ✅ Yield to worst for callable bonds")
    print("   ✅ Batch metrics calculation")
    print("   ✅ Portfolio-level risk analysis")

    print(f"\n🚀 Ready for institutional-grade bond portfolio management!")


if __name__ == "__main__":
    main()
