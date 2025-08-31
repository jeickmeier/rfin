#!/usr/bin/env python3
"""
Enhanced Loan Simulation Example

Demonstrates the new forward simulation methodology for DDTL and Revolver 
instruments, including Expected Exposure calculation and Monte Carlo 
enhancement for utilization fee tiers.

This example shows how the enhanced valuation methodology captures:
1. Dynamic balance evolution with expected draws/repayments
2. Proper PIK capitalization and floating rate projections  
3. Expected Exposure metrics for credit risk management
4. Monte Carlo modeling for utilization fee tier accuracy
"""

import finstack
from datetime import date, timedelta
from decimal import Decimal

def main():
    print("Enhanced Loan Simulation Example")
    print("=" * 50)
    
    # Setup market data
    base_date = date(2025, 1, 1)
    
    # Create discount curve
    usd_ois = finstack.DiscountCurve.builder("USD-OIS") \
        .base_date(base_date) \
        .knots([(0.0, 1.0), (1.0, 0.97), (3.0, 0.91), (5.0, 0.84)]) \
        .monotone_convex() \
        .build()
    
    # Create forward curve for floating rate loans
    usd_sofr_3m = finstack.ForwardCurve.builder("USD-SOFR-3M", 0.25) \
        .base_date(base_date) \
        .knots([(0.0, 0.045), (1.0, 0.048), (3.0, 0.050), (5.0, 0.052)]) \
        .flat_fwd() \
        .build()
    
    curves = finstack.MarketContext.new() \
        .with_discount(usd_ois) \
        .with_forecast(usd_sofr_3m)
    
    print("\n1. Delayed-Draw Term Loan (DDTL) Example")
    print("-" * 40)
    
    # Create DDTL with expected funding curve
    commitment_expiry = date(2026, 12, 31)
    maturity = date(2030, 1, 1)
    
    # Expected draw events with probabilities
    expected_draws = [
        finstack.DrawEvent(
            date=date(2025, 6, 1),
            amount=finstack.Money.new(3_000_000.0, finstack.Currency.USD),
            purpose="Working capital",
            conditional=False
        ),
        finstack.DrawEvent(
            date=date(2025, 12, 1), 
            amount=finstack.Money.new(2_000_000.0, finstack.Currency.USD),
            purpose="Expansion",
            conditional=False
        ),
        finstack.DrawEvent(
            date=date(2026, 6, 1),
            amount=finstack.Money.new(1_500_000.0, finstack.Currency.USD), 
            purpose="Acquisition",
            conditional=False
        )
    ]
    
    funding_curve = finstack.ExpectedFundingCurve.with_probabilities(
        expected_draws,
        [0.95, 0.80, 0.60]  # Decreasing probabilities over time
    )
    
    ddtl = finstack.DelayedDrawTermLoan.new(
        "DDTL_ENHANCED",
        finstack.Money.new(10_000_000.0, finstack.Currency.USD),
        commitment_expiry,
        maturity,
        finstack.InterestSpec.Floating(
            index_id="USD-SOFR-3M",
            spread_bp=275.0,  # 275 bps over SOFR
            spread_step_ups=None,
            gearing=1.0,
            reset_lag_days=2
        )
    ).with_expected_funding_curve(funding_curve) \
     .with_commitment_fee(0.0050)  # 50 bps commitment fee
    
    # Price with enhanced simulation
    result = ddtl.price(curves, base_date)
    print(f"DDTL Value: ${result.value.amount:,.2f} {result.value.currency}")
    
    # Show Expected Exposure metrics
    print(f"Expected Exposure (1Y): ${result.measures.get('expected_exposure_1y', 0):,.0f}")
    print(f"Commitment Fee PV: ${result.measures.get('commitment_fee_pv', 0):,.2f}")
    print(f"Incremental Interest PV: ${result.measures.get('incremental_interest_pv', 0):,.2f}")
    
    print("\n2. Revolving Credit Facility Example")
    print("-" * 40)
    
    # Create revolver with utilization fee tiers
    util_schedule = finstack.UtilizationFeeSchedule.new() \
        .with_tier(0.0, 0.33, 12.5)   # < 33%: 12.5 bps \
        .with_tier(0.33, 0.66, 25.0)  # 33-66%: 25 bps \
        .with_tier(0.66, 1.0, 50.0)   # > 66%: 50 bps
    
    # Expected draw/repay pattern for seasonal business
    expected_events = [
        finstack.DrawRepayEvent(
            date=date(2025, 3, 1),
            amount=finstack.Money.new(15_000_000.0, finstack.Currency.USD),
            mandatory=False,
            description="Spring inventory build"
        ),
        finstack.DrawRepayEvent(
            date=date(2025, 8, 1),
            amount=finstack.Money.new(-10_000_000.0, finstack.Currency.USD),
            mandatory=False, 
            description="Summer paydown"
        ),
        finstack.DrawRepayEvent(
            date=date(2025, 11, 1),
            amount=finstack.Money.new(20_000_000.0, finstack.Currency.USD),
            mandatory=False,
            description="Holiday inventory"
        )
    ]
    
    revolver_curve = finstack.RevolverFundingCurve.with_probabilities(
        expected_events,
        [0.90, 0.85, 0.95]  # High probability seasonal pattern
    )
    
    revolver = finstack.RevolvingCreditFacility.new(
        "RCF_ENHANCED",
        finstack.Money.new(50_000_000.0, finstack.Currency.USD),
        base_date,
        date(2028, 1, 1),  # Availability end
        date(2030, 1, 1)   # Final maturity
    ).with_interest(
        finstack.InterestSpec.Floating(
            index_id="USD-SOFR-3M",
            spread_bp=225.0,
            spread_step_ups=None,
            gearing=1.0,
            reset_lag_days=2
        )
    ).with_commitment_fee(0.0040) \
     .with_utilization_fees(util_schedule) \
     .with_expected_funding_curve(revolver_curve)
    
    # Price with standard deterministic simulation
    result_det = revolver.price(curves, base_date)
    print(f"Revolver Value (Deterministic): ${result_det.value.amount:,.2f}")
    print(f"Expected Exposure (1Y): ${result_det.measures.get('expected_exposure_1y', 0):,.0f}")
    print(f"Utilization Fee PV: ${result_det.measures.get('utilization_fee_pv', 0):,.2f}")
    
    # Price with Monte Carlo for utilization tier accuracy
    mc_metrics = [
        finstack.MetricId.custom("expected_exposure_mc_1y"),
        finstack.MetricId.custom("utilization_fee_pv")
    ]
    result_mc = revolver.price_with_metrics(curves, base_date, mc_metrics)
    print(f"Expected Exposure (MC 1Y): ${result_mc.measures.get('expected_exposure_mc_1y', 0):,.0f}")
    
    print("\n3. Comparison: Old vs New Methodology")
    print("-" * 40)
    
    # For demonstration, show how the old approximation would differ
    # (This would require accessing the old implementation)
    print("Enhanced simulation captures:")
    print("• Proper forward rate projections for floating-rate interest")
    print("• PIK capitalization effects on outstanding balances")
    print("• Mid-point averaging for more accurate fee accruals")
    print("• Utilization tier step function via Monte Carlo")
    print("• Complete amortization and principal flow modeling")
    
    print("\n4. Expected Exposure Term Structure")
    print("-" * 40)
    
    # Show expected exposure at multiple horizons
    horizons = [0.25, 0.5, 1.0, 2.0, 3.0]  # 3M, 6M, 1Y, 2Y, 3Y
    
    print("Horizon | Expected Exposure")
    print("--------|------------------")
    for horizon in horizons:
        ee_calc = finstack.ExpectedExposureCalculator.with_horizon(horizon)
        # In practice, would compute this through the metrics framework
        print(f"{horizon:>6.1f}Y | ${'TBD':>15}")  # Placeholder for actual calculation
    
    print("\nEnhanced loan valuation complete!")

if __name__ == "__main__":
    main()
