#!/usr/bin/env python3
"""
ISDA 2014 CDS Standard Model Compliance Example

Demonstrates the full ISDA 2014 standard-compliant CDS pricing implementation:
- Exact integration points (not simplified midpoint)
- Standard coupon dates (20th of Mar/Jun/Sep/Dec) 
- Proper stub accrual handling
- Standard recovery assumptions (40% senior unsecured)
"""

import finstack as fs
from datetime import date
from finstack import Currency

def main():
    print("=" * 60)
    print("ISDA 2014 CDS Standard Model Compliance Example")
    print("=" * 60)
    
    # Create market context with discount and credit curves
    context = fs.MarketContext()
    
    # Add discount curve (USD OIS)
    disc_curve = fs.DiscountCurve.builder("USD-OIS") \
        .base_date(date(2025, 1, 1)) \
        .knots([
            (0.0, 1.0),
            (0.25, 0.9975),
            (0.5, 0.995),
            (1.0, 0.99),
            (2.0, 0.975),
            (3.0, 0.96),
            (5.0, 0.93),
            (10.0, 0.85)
        ]) \
        .build()
    context = context.insert_discount(disc_curve)
    
    # Add credit curve with moderate credit risk
    hazard_curve = fs.HazardCurve.builder("CORP-CREDIT") \
        .base_date(date(2025, 1, 1)) \
        .recovery_rate(0.40) \
        .knots([
            (0.0, 0.01),
            (1.0, 0.015),
            (2.0, 0.02),
            (3.0, 0.025),
            (5.0, 0.03),
            (10.0, 0.035)
        ]) \
        .build()
    context = context.insert_hazard(hazard_curve)
    
    # 1. Create CDS with ISDA standard configuration
    print("\n1. ISDA Standard CDS Configuration")
    print("-" * 40)
    
    # Use ISDA standard senior unsecured recovery (40%)
    credit_params = fs.CreditParams.senior_unsecured("ABC Corp", "CORP-CREDIT")
    
    cds = fs.CreditDefaultSwap.builder() \
        .id("CDS-ABC-5Y") \
        .notional(fs.Money(10_000_000, Currency.USD)) \
        .side(fs.PayReceive.PayProtection) \
        .spread_bp(150.0) \
        .credit_params(credit_params) \
        .dates(date(2025, 1, 15), date(2030, 1, 15)) \
        .market_refs(fs.MarketRefs(disc_id="USD-OIS")) \
        .convention(fs.CDSConvention.IsdaNa) \
        .build()
    
    print(f"Reference Entity: {credit_params.reference_entity}")
    print(f"Recovery Rate: {credit_params.recovery_rate:.1%} (ISDA senior unsecured)")
    print(f"Spread: {cds.premium.spread_bp:.0f} bps")
    print(f"Convention: {cds.convention}")
    
    # 2. Price with ISDA-compliant pricer (default configuration)
    print("\n2. ISDA-Compliant Pricing")
    print("-" * 40)
    
    # Default pricer uses ISDA standard configuration
    pricer = fs.CDSPricer()
    
    # Calculate protection and premium legs
    protection_pv = pricer.pv_protection_leg(cds, context, date(2025, 1, 1))
    premium_pv = pricer.pv_premium_leg(cds, context, date(2025, 1, 1))
    npv = pricer.npv(cds, context, date(2025, 1, 1))
    
    print(f"Protection Leg PV: {protection_pv:,.2f}")
    print(f"Premium Leg PV: {premium_pv:,.2f}")
    print(f"NPV (buyer perspective): {npv:,.2f}")
    
    # Calculate key metrics
    par_spread = pricer.par_spread(cds, context, date(2025, 1, 1))
    risky_pv01 = pricer.risky_pv01(cds, context, date(2025, 1, 1))
    cs01 = pricer.cs01(cds, context, date(2025, 1, 1))
    
    print(f"\nPar Spread: {par_spread:.1f} bps")
    print(f"Risky PV01: {risky_pv01:,.2f}")
    print(f"CS01: {cs01:,.2f}")
    
    # 3. Compare ISDA exact vs simplified integration
    print("\n3. ISDA Exact vs Simplified Integration")
    print("-" * 40)
    
    # ISDA exact integration (default)
    config_exact = fs.CDSPricerConfig.isda_standard()
    pricer_exact = fs.CDSPricer.with_config(config_exact)
    
    # Simplified midpoint integration
    config_simple = fs.CDSPricerConfig.simplified()
    pricer_simple = fs.CDSPricer.with_config(config_simple)
    
    # Compare protection leg calculations
    protection_exact = pricer_exact.pv_protection_leg(cds, context, date(2025, 1, 1))
    protection_simple = pricer_simple.pv_protection_leg(cds, context, date(2025, 1, 1))
    
    diff = abs(protection_exact.amount() - protection_simple.amount())
    diff_pct = (diff / protection_simple.amount()) * 100
    
    print(f"ISDA Exact Integration: {protection_exact:,.2f}")
    print(f"Simplified Midpoint: {protection_simple:,.2f}")
    print(f"Difference: {fs.Money(diff, Currency.USD):,.2f} ({diff_pct:.3f}%)")
    
    # 4. Verify ISDA standard coupon dates
    print("\n4. ISDA Standard Coupon Dates")
    print("-" * 40)
    
    schedule = pricer.generate_isda_schedule(cds)
    print("Payment Schedule (20th of Mar/Jun/Sep/Dec):")
    for i, date in enumerate(schedule):
        if i == 0:
            print(f"  Start: {date}")
        elif i == len(schedule) - 1:
            print(f"  Maturity: {date}")
        else:
            print(f"  Coupon {i}: {date} (day={date.day})")
    
    # 5. Test different seniority levels
    print("\n5. Recovery Rates by Seniority")
    print("-" * 40)
    
    # Senior unsecured (40%)
    senior_params = fs.CreditParams.senior_unsecured("ABC Corp", "CORP-CREDIT")
    print(f"Senior Unsecured: {senior_params.recovery_rate:.1%}")
    
    # Subordinated (20%)
    sub_params = fs.CreditParams.subordinated("ABC Corp", "CORP-CREDIT")
    print(f"Subordinated: {sub_params.recovery_rate:.1%}")
    
    # High yield (30%)
    hy_params = fs.CreditParams.high_yield("ABC Corp", "CORP-CREDIT")
    print(f"High Yield: {hy_params.recovery_rate:.1%}")
    
    # 6. Configuration comparison
    print("\n6. Configuration Comparison")
    print("-" * 40)
    
    isda_config = fs.CDSPricerConfig.isda_standard()
    simple_config = fs.CDSPricerConfig.simplified()
    
    print("ISDA Standard Configuration:")
    print(f"  Integration Method: IsdaExact")
    print(f"  Integration Points/Year: {isda_config.steps_per_year}")
    print(f"  ISDA Coupon Dates: {isda_config.use_isda_coupon_dates}")
    print(f"  Tolerance: {isda_config.tolerance}")
    print(f"  Include Accrual: {isda_config.include_accrual}")
    
    print("\nSimplified Configuration:")
    print(f"  Integration Method: Midpoint")
    print(f"  Integration Points/Year: {simple_config.steps_per_year}")
    print(f"  ISDA Coupon Dates: {simple_config.use_isda_coupon_dates}")
    print(f"  Tolerance: {simple_config.tolerance}")
    print(f"  Include Accrual: {simple_config.include_accrual}")
    
    print("\n" + "=" * 60)
    print("ISDA 2014 CDS Standard Model fully implemented!")
    print("=" * 60)

if __name__ == "__main__":
    main()
