"""Real Estate Asset Example.
==========================

Demonstrates real estate valuation using direct capitalization and
discounted cashflow (DCF) methods.
"""

from finstack.core.market_data import (
    DiscountCurve,
    InterpolationStyle,
    MarketContext,
)
from finstack.valuations.instruments import RealEstateAsset
from finstack.valuations.pricer import create_standard_registry

from finstack import Date


def main():
    """Value a commercial office property using:
    1. Direct Capitalization (stabilized NOI / cap rate)
    2. DCF (5-year hold with exit cap rate).
    """
    # Market data setup
    as_of = Date(2024, 1, 1)
    base_date = as_of

    # USD discount curve (for DCF valuation)
    tenors = [0.0, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]
    dfs = [1.0, 0.9400, 0.8900, 0.8450, 0.7700, 0.7000, 0.6000]

    discount_curve = DiscountCurve(
        curve_id="USD-OIS",
        base_date=base_date,
        tenors=tenors,
        values=dfs,
        interpolation=InterpolationStyle.LogLinear(),
    )

    market = MarketContext()
    market.insert_discount(discount_curve)

    # Method 1: Direct Capitalization
    print("=" * 70)
    print("Real Estate Valuation - Direct Capitalization")
    print("=" * 70)

    direct_cap_asset = RealEstateAsset.create_direct_cap(
        "OFFICE-NYC-123",
        currency="USD",
        valuation_date=as_of,
        stabilized_noi=5_000_000.0,  # $5M annual stabilized NOI
        cap_rate=0.06,  # 6% cap rate
        discount_curve_id="USD-OIS",
    )

    registry = create_standard_registry()

    result_direct = registry.price_real_estate_asset_with_metrics(
        direct_cap_asset,
        "direct_cap",
        market,
        ["cap_rate", "noi_yield"],
    )

    print(f"Property ID:        {direct_cap_asset.instrument_id()}")
    print(f"Valuation Date:     {direct_cap_asset.valuation_date()}")
    print("Method:             Direct Capitalization")
    print(f"Stabilized NOI:     ${direct_cap_asset.stabilized_noi():,.2f}")
    print(f"Cap Rate:           {direct_cap_asset.cap_rate() * 100:.2f}%")
    print(f"\nProperty Value:     ${result_direct.present_value.amount:,.2f}")
    print(f"Implied Value/SF:   ${result_direct.present_value.amount / 100_000:.2f} (assuming 100k SF)")
    print("\nMetrics:")
    print(f"  Cap Rate:          {result_direct.metric('cap_rate') or 0:.4f}")
    print(f"  NOI Yield:         {result_direct.metric('noi_yield') or 0:.4f}")
    print("=" * 70)

    # Method 2: DCF with projected NOI schedule
    print("\n" + "=" * 70)
    print("Real Estate Valuation - Discounted Cashflow (DCF)")
    print("=" * 70)

    # NOI projection: 5-year hold with growth
    noi_schedule = [
        (Date(2024, 12, 31), 4_500_000.0),  # Year 1: Below stabilized
        (Date(2025, 12, 31), 4_800_000.0),  # Year 2: Approaching stabilized
        (Date(2026, 12, 31), 5_000_000.0),  # Year 3: Stabilized
        (Date(2027, 12, 31), 5_150_000.0),  # Year 4: 3% growth
        (Date(2028, 12, 31), 5_300_000.0),  # Year 5: 3% growth
    ]

    dcf_asset = RealEstateAsset.create_dcf(
        "OFFICE-NYC-123-DCF",
        currency="USD",
        valuation_date=as_of,
        noi_schedule=noi_schedule,
        exit_date=Date(2029, 1, 1),  # Exit after 5 years
        exit_cap_rate=0.065,  # 6.5% exit cap (higher than going-in)
        discount_curve_id="USD-OIS",
    )

    result_dcf = registry.price_real_estate_asset_with_metrics(
        dcf_asset,
        "dcf",
        market,
        ["exit_value", "noi_pv", "total_noi", "irr"],
    )

    print(f"Property ID:        {dcf_asset.instrument_id()}")
    print(f"Valuation Date:     {dcf_asset.valuation_date()}")
    print("Method:             Discounted Cashflow")
    print(f"Exit Date:          {dcf_asset.exit_date()}")
    print(f"Exit Cap Rate:      {dcf_asset.exit_cap_rate() * 100:.2f}%")
    print("\nNOI Schedule:")
    for date, noi in noi_schedule:
        print(f"  {date}:  ${noi:,.2f}")
    print(f"\nProperty Value:     ${result_dcf.present_value.amount:,.2f}")
    print("\nMetrics:")
    print(f"  Exit Value (Y5):   ${result_dcf.metric('exit_value') or 0:,.2f}")
    print(f"  NOI PV:            ${result_dcf.metric('noi_pv') or 0:,.2f}")
    print(f"  Total NOI (5Y):    ${result_dcf.metric('total_noi') or 0:,.2f}")
    print(f"  IRR:               {(result_dcf.metric('irr') or 0) * 100:.2f}%")
    print("=" * 70)

    # Sensitivity analysis - vary cap rate
    print("\nSensitivity Analysis - Direct Cap Method")
    print(f"{'Cap Rate':<12} {'Value':<15} {'Value/SF'}")
    print("-" * 50)

    for cap_rate in [0.05, 0.055, 0.06, 0.065, 0.07]:
        sens_asset = RealEstateAsset.create_direct_cap(
            "SENS-ANALYSIS",
            currency="USD",
            valuation_date=as_of,
            stabilized_noi=5_000_000.0,
            cap_rate=cap_rate,
            discount_curve_id="USD-OIS",
        )
        sens_result = registry.price_real_estate_asset(sens_asset, "direct_cap", market)
        value = sens_result.present_value.amount
        value_per_sf = value / 100_000
        print(f"{cap_rate * 100:>6.1f}%      ${value:>12,.2f}  ${value_per_sf:>8.2f}")

    print("=" * 70)

    # Comparison
    direct_value = result_direct.present_value.amount
    dcf_value = result_dcf.present_value.amount
    difference = dcf_value - direct_value
    difference_pct = (difference / direct_value) * 100

    print("\nValuation Comparison:")
    print(f"  Direct Cap Value:  ${direct_value:,.2f}")
    print(f"  DCF Value:         ${dcf_value:,.2f}")
    print(f"  Difference:        ${difference:,.2f} ({difference_pct:+.1f}%)")
    print("\nDCF accounts for:")
    print("  - Below-market NOI in early years")
    print("  - Growth to stabilization")
    print("  - Higher exit cap rate risk")
    print("=" * 70)


if __name__ == "__main__":
    main()
