#!/usr/bin/env python3
"""
Revolving Credit Period Analysis Example

Demonstrates the new per-period PV and DataFrame functionality for revolving credit facilities
with credit risk pricing.

This example creates a deterministic facility with:
- 30% initial utilization (drawn)
- Floating rate + 150bp margin on drawn amount
- 50bp commitment fee on undrawn amount
- 5 year term
- Quarterly payments
- Credit risk via hazard curve (BORROWER-A, 40% recovery)

Credit risk is incorporated via:
- Hazard curve parameter in facility builder
- Survival probabilities applied to all cashflow PVs
- Credit-adjusted NPV reflects default probability

Shows how to:
1. Build the facility with hazard curve for credit risk
2. Generate cashflow schedule
3. Compute per-period present values (credit-adjusted)
4. Export to DataFrame with credit adjustment columns
5. Verify PV sum equals total NPV
"""

from datetime import date
import pandas as pd
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve
from finstack.core.dates.periods import build_periods
from finstack.valuations.instruments import RevolvingCredit


def create_market_data() -> tuple[MarketContext, date]:
    """Create market data with discount, forward, and hazard curves."""
    val_date = date(2025, 1, 1)
    
    # Discount curve (SOFR-based, ~4% rate)
    import math
    rate = 0.04
    disc_curve = DiscountCurve(
        "USD-OIS",
        val_date,
        [
            (0.0, 1.0),
            (0.25, math.exp(-rate * 0.25)),
            (0.5, math.exp(-rate * 0.5)),
            (1.0, math.exp(-rate * 1.0)),
            (2.0, math.exp(-rate * 2.0)),
            (3.0, math.exp(-rate * 3.0)),
            (5.0, math.exp(-rate * 5.0)),
        ],
    )
    
    # Forward curve for floating rate (SOFR 3M, ~4.5% forward rates)
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,  # 3-month tenor
        [
            (0.0, 0.0450),
            (1.0, 0.0475),
            (2.0, 0.0500),
            (3.0, 0.0525),
            (5.0, 0.0550),
        ],
        base_date=val_date,
    )
    
    # Hazard curve for credit risk (survival probabilities)
    # Using moderate credit quality: 99% at 1Y, 96% at 3Y, 92% at 5Y
    # Implies ~1% annual default probability initially
    hazard_curve = HazardCurve(
        "BORROWER-A",
        val_date,
        [
            (0.0, 1.0),
            (1.0, 0.99),
            (2.0, 0.975),
            (3.0, 0.96),
            (5.0, 0.92),
        ],
    )
    
    market = MarketContext()
    market.insert_discount(disc_curve)
    market.insert_forward(forward_curve)
    market.insert_hazard(hazard_curve)
    
    return market, val_date


def main():
    """Main example demonstrating revolving credit period analysis."""
    print("=" * 80)
    print("REVOLVING CREDIT PERIOD ANALYSIS EXAMPLE")
    print("=" * 80)
    
    # Create market data
    market, val_date = create_market_data()
    maturity_date = date(2030, 1, 1)  # 5 year term
    
    # Facility parameters
    commitment_amount = Money(10_000_000.0, USD)
    drawn_amount = Money(3_000_000.0, USD)  # 30% utilization
    margin_bp = 150.0  # 150bp margin over floating rate
    commitment_fee_bp = 50.0  # 50bp on undrawn amount
    
    # Recovery rate of 40% on default
    recovery_rate = 0.40
    
    print(f"\nFacility Details:")
    print(f"  Commitment: {commitment_amount}")
    print(f"  Drawn: {drawn_amount} ({drawn_amount.amount / commitment_amount.amount * 100:.1f}%)")
    print(f"  Undrawn: {commitment_amount.checked_sub(drawn_amount)}")
    print(f"  Term: {val_date} to {maturity_date} (5 years)")
    print(f"  Rate: Floating + {margin_bp}bp")
    print(f"  Commitment Fee: {commitment_fee_bp}bp on undrawn")
    print(f"  Credit Risk: Hazard curve BORROWER-A, Recovery {recovery_rate:.0%}")
    
    # Create revolving credit facility with credit risk
    # Credit risk is now directly supported in deterministic facilities via hazard_curve
    
    revolver = RevolvingCredit.builder(
        instrument_id="RC_FLOATING_30PCT",
        commitment_amount=commitment_amount,
        drawn_amount=drawn_amount,
        commitment_date=val_date,
        maturity_date=maturity_date,
        base_rate_spec={
            "type": "floating",
            "index_id": "USD-SOFR-3M",
            "margin_bp": margin_bp,
            "reset_freq": "quarterly",
        },
        payment_frequency="quarterly",
        fees={
            "upfront_fee": None,
            "commitment_fee_bp": commitment_fee_bp,
            "usage_fee_bp": 0.0,
            "facility_fee_bp": 0.0,
        },
        draw_repay_spec={"deterministic": []},  # No draws/repays, constant utilization
        discount_curve="USD-OIS",
        hazard_curve="BORROWER-A",  # Credit risk modeling
        recovery_rate=recovery_rate,  # 40% recovery on default
    )
    
    print(f"\n✓ Facility created: {revolver}")
    print(f"  Utilization: {revolver.utilization_rate():.1%}")
    
    # Build periods for analysis (quarterly periods covering the facility term)
    start_year = val_date.year
    end_year = maturity_date.year
    period_range = f"{start_year}Q1..{end_year}Q4"
    periods = build_periods(period_range, None)
    
    # Filter to periods within facility term
    # Note: periods.periods returns PyPeriod objects, we can access .start directly
    periods_list = list(periods.periods)
    periods_filtered = [p for p in periods_list if p.start < maturity_date]
    print(f"\n✓ Built {len(periods_filtered)} periods for analysis")
    
    # Example 1: Build cashflow schedule
    print("\n" + "-" * 80)
    print("EXAMPLE 1: Build Cashflow Schedule")
    print("-" * 80)
    
    schedule = revolver.build_schedule(market, val_date)
    print(f"✓ Schedule created with {len(schedule.flows())} cashflows")
    
    # Show first few cashflows
    flows_list = schedule.flows()
    print(f"\nFirst 5 cashflows:")
    for i, cf in enumerate(flows_list[:5]):
        print(f"  {i+1}. {cf.date}: {cf.amount} ({cf.kind.name})")
    
    # Example 2: Per-period PV analysis
    print("\n" + "-" * 80)
    print("EXAMPLE 2: Per-Period Present Values")
    print("-" * 80)
    
    pv_by_period = revolver.per_period_pv(
        periods_filtered,
        market,
        discount_curve_id="USD-OIS",
        as_of=val_date,
    )
    
    print(f"\n✓ Computed PVs for {len(pv_by_period)} periods")
    print(f"\nPer-Period PVs (first 8 periods):")
    total_pv_from_periods = 0.0
    for i, (period_code, pv) in enumerate(list(pv_by_period.items())[:8]):
        total_pv_from_periods += pv
        print(f"  {period_code}: ${pv:,.2f}")
    
    # Sum all period PVs
    total_pv_from_periods = sum(pv_by_period.values())
    print(f"\n  ... (showing first 8 of {len(pv_by_period)} periods)")
    print(f"\n  Total PV from periods: ${total_pv_from_periods:,.2f}")
    
    # Compare with total NPV
    total_npv = revolver.npv(market, val_date)
    print(f"  Total NPV (from pricing): ${total_npv.amount:,.2f}")
    print(f"  Difference: ${abs(total_pv_from_periods - total_npv.amount):,.2f}")
    
    if abs(total_pv_from_periods - total_npv.amount) < 1.0:
        print("  ✓ PV sum matches total NPV (within tolerance)")
    else:
        print("  ⚠ Warning: PV sum differs from total NPV")
    
    # Example 3: Period-aligned DataFrame
    print("\n" + "-" * 80)
    print("EXAMPLE 3: Period-Aligned DataFrame Export")
    print("-" * 80)
    
    df_dict = revolver.to_period_dataframe(
        periods_filtered,
        market,
        discount_curve_id="USD-OIS",
        hazard_curve_id="BORROWER-A",  # Credit risk adjustment via hazard curve
        forward_curve_id="USD-SOFR-3M",  # For floating rate decomposition
        as_of=val_date,
        facility_limit=commitment_amount,  # For unfunded amount calculation
        include_floating_decomposition=True,  # Include Base Rate + Spread columns
    )
    
    # Convert to pandas DataFrame
    df = pd.DataFrame(df_dict)
    
    print(f"\n✓ DataFrame created with {len(df)} rows and {len(df.columns)} columns")
    print(f"\nColumns: {list(df.columns)}")
    
    # Display summary statistics
    print(f"\nDataFrame Summary:")
    print(f"  Total rows: {len(df)}")
    print(f"  Date range: {df['Start Date'].min()} to {df['End Date'].max()}")
    print(f"  Total PV: ${df['PV'].sum():,.2f}")
    print(f"  Total Amount: ${df['Amount'].sum():,.2f}")
    
    # Show sample rows
    print(f"\nSample DataFrame (first 5 rows):")
    pd.set_option('display.max_columns', None)
    pd.set_option('display.width', None)
    pd.set_option('display.max_colwidth', 20)
    print(df.head().to_string())
    
    # Analysis by cashflow type
    print(f"\n" + "-" * 80)
    print("Cashflow Type Breakdown:")
    print("-" * 80)
    if 'CFType' in df.columns:
        cf_summary = df.groupby('CFType').agg({
            'Amount': 'sum',
            'PV': 'sum',
            'PayDate': 'count'
        }).rename(columns={'PayDate': 'Count'})
        print(cf_summary.to_string())
    
    # Verify PV sum equals total NPV
    df_pv_sum = df['PV'].sum()
    print(f"\n" + "-" * 80)
    print("PV Verification:")
    print("-" * 80)
    print(f"  DataFrame PV sum: ${df_pv_sum:,.2f}")
    print(f"  Total NPV: ${total_npv.amount:,.2f}")
    print(f"  Difference: ${abs(df_pv_sum - total_npv.amount):,.2f}")
    
    if abs(df_pv_sum - total_npv.amount) < 1.0:
        print("  ✓ DataFrame PV sum matches total NPV")
    else:
        print("  ⚠ Warning: DataFrame PV sum differs from total NPV")
    
    # Show notional and unfunded amounts over time
    if 'Notional' in df.columns and 'Unfunded Amount' in df.columns:
        print(f"\n" + "-" * 80)
        print("Notional and Unfunded Amounts (sample):")
        print("-" * 80)
        sample_cols = ['PayDate', 'Notional', 'Unfunded Amount', 'Amount', 'PV']
        available_cols = [c for c in sample_cols if c in df.columns]
        print(df[available_cols].head(8).to_string())
    
    # Show floating rate decomposition if available
    if 'Base Rate' in df.columns and 'Spread' in df.columns:
        print(f"\n" + "-" * 80)
        print("Floating Rate Decomposition (interest flows only):")
        print("-" * 80)
        interest_rows = df[df['CFType'].isin(['FloatReset', 'Fixed'])].copy()
        if len(interest_rows) > 0:
            print(f"  Found {len(interest_rows)} interest cashflows")
            print(f"  Average Base Rate: {interest_rows['Base Rate'].mean():.4f} ({interest_rows['Base Rate'].mean() * 10000:.0f}bp)")
            print(f"  Average Spread: {interest_rows['Spread'].mean():.4f} ({interest_rows['Spread'].mean() * 10000:.0f}bp)")
            print(f"  Expected Spread: {margin_bp / 10000:.4f} ({margin_bp:.0f}bp)")
            print(f"\n  Sample interest flows:")
            rate_cols = ['PayDate', 'Notional', 'Base Rate', 'Spread', 'Rate', 'Amount', 'PV']
            available_rate_cols = [c for c in rate_cols if c in df.columns]
            print(interest_rows[available_rate_cols].head(5).to_string())
    
    # Show credit risk adjustment if survival probabilities are present
    if 'SurvivalProb' in df.columns:
        print(f"\n" + "-" * 80)
        print("Credit Risk Adjustment (via Hazard Curve):")
        print("-" * 80)
        print(f"  Survival probabilities incorporated into PV calculations")
        print(f"  Average Survival Prob: {df['SurvivalProb'].mean():.4f} ({df['SurvivalProb'].mean() * 100:.2f}%)")
        print(f"  Final Survival Prob (5Y): {df['SurvivalProb'].iloc[-1]:.4f} ({df['SurvivalProb'].iloc[-1] * 100:.2f}%)")
        print(f"  Expected from hazard curve: 92.0% at 5Y")
        print(f"\n  Sample cashflows with survival probabilities:")
        surv_cols = ['PayDate', 'Amount', 'SurvivalProb', 'PV']
        available_surv_cols = [c for c in surv_cols if c in df.columns]
        print(df[available_surv_cols].head(8).to_string())
    
    print("\n" + "=" * 80)
    print("Example completed successfully!")
    print("=" * 80)


if __name__ == "__main__":
    main()

