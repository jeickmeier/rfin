"""
Revolving Credit Cashflow Analysis with Pandas DataFrames

Working example demonstrating cashflow tracking and DataFrame features
for revolving credit Monte Carlo simulations using ACTUAL cashflows from Rust.
"""

from datetime import date
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.common.mc import CashflowType


def create_market_data() -> tuple[MarketContext, date]:
    """Create comprehensive market data."""
    val_date = date(2025, 1, 1)
    
    # Discount curve
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85), (5.0, 0.75)],
    )
    
    # Forward curve for floating rates (3M tenor = 0.25 years)
    fwd_curve = ForwardCurve(
        "USD.SOFR.3M",
        0.25,  # 3-month tenor
        [(0.0, 0.05), (1.0, 0.052), (2.0, 0.054), (3.0, 0.055)],
    )
    
    # Hazard curve for credit risk
    hazard_curve = HazardCurve(
        "CORP.BBB",
        val_date,
        [(1.0, 0.98), (3.0, 0.92), (5.0, 0.85)],
    )
    
    market = MarketContext()
    market.insert_discount(disc_curve)
    market.insert_forward(fwd_curve)
    market.insert_hazard(hazard_curve)
    
    return market, val_date


def example_multi_factor_mc_with_cashflows():
    """Run multi-factor MC simulation and analyze actual cashflows from Rust."""
    print("\n" + "=" * 80)
    print("MULTI-FACTOR MONTE CARLO WITH CASHFLOW TRACKING")
    print("=" * 80)
    
    # Create facility with multi-factor stochastic dynamics
    revolver = RevolvingCredit.builder(
        instrument_id="RC_MULTI_FACTOR",
        commitment_amount=Money(10000000.0, USD),
        drawn_amount=Money(3000000.0, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2026, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.06},
        payment_frequency="quarterly",
        fees={
            "upfront_fee": Money(50000.0, USD),
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.50,
                    "speed": 1.0,
                    "volatility": 0.30,
                },
                "num_paths": 500,
                "seed": 42,
                "antithetic": False,
                "use_sobol_qmc": False,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": "CORP.BBB",
                            "kappa": 0.5,
                            "implied_vol": 0.50,
                        }
                    },
                    "util_credit_corr": -0.3,
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    print(f"\nFacility Configuration:")
    print(f"  ID: {revolver.instrument_id}")
    print(f"  Commitment: ${revolver.commitment_amount.amount / 1e6:.1f}M")
    print(f"  Initial Draw: ${revolver.drawn_amount.amount / 1e6:.1f}M")
    print(f"  Maturity: 1 year")
    print(f"  Base Rate: 6.00% fixed")
    print(f"  Upfront Fee: $50k")
    print(f"  Commitment Fee: 25 bps")
    print(f"  Usage Fee: 50 bps")
    print(f"  Facility Fee: 10 bps")
    
    print(f"\nStochastic Factors:")
    print(f"  • Utilization: Mean-reverting (target=50%, vol=30%)")
    print(f"  • Credit Spread: CIR process (market-anchored)")
    print(f"  • Correlation: Util-Credit = -0.3")
    
    market, as_of = create_market_data()
    
    print(f"\nRunning Monte Carlo simulation...")
    print(f"  Paths: 500")
    print(f"  Capturing: 100 sample paths")
    
    # Use mc_paths() to get actual cashflows from Rust!
    result = revolver.mc_paths(
        market=market,
        as_of=as_of,
        capture_mode="sample",
        sample_count=100,
        seed=42
    )
    
    print(f"\n✓ Simulation complete!")
    print(f"\nResults:")
    print(f"  Mean PV: {result.estimate}")
    print(f"  Stderr: ${result.stderr:,.2f}")
    print(f"  Paths captured: {result.num_captured_paths()}")
    
    # Get the captured paths dataset
    paths_dataset = result.paths
    
    if paths_dataset is None:
        print("\nERROR: No paths captured")
        return None
    
    print(f"\n✓ Got PathDataset with {len(paths_dataset)} paths")
    
    return paths_dataset


def analyze_single_path(dataset):
    """Analyze cashflows from a single Monte Carlo path."""
    print("\n" + "=" * 80)
    print("SINGLE PATH CASHFLOW ANALYSIS")
    print("=" * 80)
    
    # Get first path
    path = dataset.paths[0]
    
    print(f"\nPath {path.path_id}:")
    print(f"  Final Value: ${path.final_value:,.2f}")
    print(f"  Steps: {path.num_steps()}")
    
    if path.irr is not None:
        print(f"  IRR: {path.irr:.2%} annualized")
    
    # Convert to DataFrame
    df = path.to_dataframe()
    
    print(f"\n✓ Converted to DataFrame: {len(df)} cashflow records")
    print("\nFirst 10 cashflows:")
    print(df.head(10).to_string(index=False))
    
    # Aggregate by type
    print("\n" + "-" * 80)
    print("Cashflows by Type:")
    print("-" * 80)
    by_type = df.groupby('cashflow_type')['amount'].agg(['sum', 'count', 'mean'])
    print(by_type.to_string())
    
    # Show principal flows
    print("\n" + "-" * 80)
    print("Principal Flows:")
    print("-" * 80)
    principal_df = df[df['cashflow_type'] == 'Principal']
    if not principal_df.empty:
        print(principal_df.to_string(index=False))
    else:
        print("No principal flows recorded")
    
    # Show interest flows
    print("\n" + "-" * 80)
    print("Interest Flows:")
    print("-" * 80)
    interest_df = df[df['cashflow_type'] == 'Interest']
    if not interest_df.empty:
        print(interest_df.to_string(index=False))
    else:
        print("No interest flows recorded")
    
    return df


def analyze_all_paths(dataset):
    """Analyze cashflows across all captured paths."""
    print("\n" + "=" * 80)
    print("MULTI-PATH CASHFLOW ANALYSIS")
    print("=" * 80)
    
    # Convert all cashflows to DataFrame
    all_cashflows = dataset.cashflows_to_dataframe()
    
    print(f"\n✓ Converted {len(dataset)} paths to DataFrame")
    print(f"  Total cashflow records: {len(all_cashflows)}")
    
    # Show structure
    print("\nDataFrame structure:")
    print(all_cashflows.head(15).to_string(index=False))
    
    # Aggregate by cashflow type across ALL paths
    print("\n" + "-" * 80)
    print("Total Cashflows by Type (All Paths):")
    print("-" * 80)
    total_by_type = all_cashflows.groupby('cashflow_type')['amount'].agg([
        ('total', 'sum'),
        ('count', 'count'),
        ('mean', 'mean'),
        ('std', 'std'),
    ])
    print(total_by_type.to_string())
    
    # Time series analysis
    print("\n" + "-" * 80)
    print("Average Cashflows by Time and Type:")
    print("-" * 80)
    time_type = all_cashflows.groupby(['time_years', 'cashflow_type'])['amount'].mean().unstack(fill_value=0)
    print(time_type.to_string())
    
    # Per-path statistics
    print("\n" + "-" * 80)
    print("Per-Path Total Cashflows:")
    print("-" * 80)
    path_totals = all_cashflows.groupby('path_id')['amount'].sum()
    print(f"  Mean:   ${path_totals.mean():>12,.2f}")
    print(f"  Median: ${path_totals.median():>12,.2f}")
    print(f"  Std:    ${path_totals.std():>12,.2f}")
    print(f"  Min:    ${path_totals.min():>12,.2f}")
    print(f"  Max:    ${path_totals.max():>12,.2f}")
    
    return all_cashflows


def analyze_irr_distribution(dataset):
    """Analyze IRR distribution across paths."""
    print("\n" + "=" * 80)
    print("IRR DISTRIBUTION ANALYSIS")
    print("=" * 80)
    
    # Extract IRRs
    irrs = [p.irr for p in dataset.paths if p.irr is not None]
    
    if not irrs:
        print("\nNo IRRs calculated yet")
        return
    
    print(f"\n✓ Extracted IRRs from {len(irrs)} paths")
    
    # Statistics
    print(f"\nIRR Statistics:")
    print(f"  Mean:     {np.mean(irrs):>8.2%}")
    print(f"  Median:   {np.median(irrs):>8.2%}")
    print(f"  Std Dev:  {np.std(irrs):>8.2%}")
    print(f"  Min:      {np.min(irrs):>8.2%}")
    print(f"  Max:      {np.max(irrs):>8.2%}")
    print(f"  25th pct: {np.percentile(irrs, 25):>8.2%}")
    print(f"  75th pct: {np.percentile(irrs, 75):>8.2%}")
    
    # Create IRR histogram
    plt.figure(figsize=(10, 6))
    plt.hist(irrs, bins=30, alpha=0.7, edgecolor='black', color='steelblue')
    plt.axvline(np.mean(irrs), color='red', linestyle='--', linewidth=2, label=f'Mean: {np.mean(irrs):.2%}')
    plt.axvline(np.median(irrs), color='green', linestyle='--', linewidth=2, label=f'Median: {np.median(irrs):.2%}')
    plt.xlabel('IRR (Annualized)')
    plt.ylabel('Frequency')
    plt.title('Distribution of Path IRRs\n(Lender Perspective)')
    plt.legend()
    plt.grid(alpha=0.3)
    plt.tight_layout()
    plt.savefig('revolving_credit_irr_distribution.png', dpi=150, bbox_inches='tight')
    print(f"\n✓ Saved plot: revolving_credit_irr_distribution.png")
    
    return irrs


def visualize_cashflows(all_cashflows_df):
    """Create comprehensive cashflow visualizations."""
    print("\n" + "=" * 80)
    print("CASHFLOW VISUALIZATIONS")
    print("=" * 80)
    
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    
    # Plot 1: Total cashflows by type
    ax = axes[0, 0]
    by_type = all_cashflows_df.groupby('cashflow_type')['amount'].sum() / 1e6
    by_type.plot(kind='barh', ax=ax, color='steelblue', edgecolor='black')
    ax.set_xlabel('Total Amount ($M)')
    ax.set_title('Total Cashflows by Type (All Paths)')
    ax.grid(axis='x', alpha=0.3)
    
    # Plot 2: Time series of cashflows
    ax = axes[0, 1]
    time_pivot = all_cashflows_df.pivot_table(
        values='amount',
        index='time_years',
        columns='cashflow_type',
        aggfunc='mean'
    ) / 1e3
    
    # Stack positive cashflows
    positive_cols = [col for col in time_pivot.columns if col != 'Principal']
    if positive_cols:
        time_pivot[positive_cols].plot(kind='area', stacked=True, ax=ax, alpha=0.7)
        ax.set_xlabel('Time (years)')
        ax.set_ylabel('Average Cashflow ($k)')
        ax.set_title('Average Cashflow Composition Over Time')
        ax.legend(bbox_to_anchor=(1.05, 1), loc='upper left', fontsize=8)
        ax.grid(alpha=0.3)
    
    # Plot 3: Distribution of total cashflows per path
    ax = axes[1, 0]
    path_totals = all_cashflows_df.groupby('path_id')['amount'].sum() / 1e6
    ax.hist(path_totals, bins=25, alpha=0.7, edgecolor='black', color='steelblue')
    ax.axvline(path_totals.mean(), color='red', linestyle='--', linewidth=2,
               label=f'Mean: ${path_totals.mean():.2f}M')
    ax.axvline(path_totals.median(), color='green', linestyle='--', linewidth=2,
               label=f'Median: ${path_totals.median():.2f}M')
    ax.set_xlabel('Total Cashflow per Path ($M)')
    ax.set_ylabel('Frequency')
    ax.set_title('Distribution of Total Cashflows Across Paths')
    ax.legend()
    ax.grid(alpha=0.3)
    
    # Plot 4: Cashflow breakdown pie chart
    ax = axes[1, 1]
    positive_cfs = all_cashflows_df[all_cashflows_df['amount'] > 0]
    pie_data = positive_cfs.groupby('cashflow_type')['amount'].sum() / 1e6
    colors = plt.cm.Set3(range(len(pie_data)))
    pie_data.plot(kind='pie', ax=ax, autopct='%1.1f%%', colors=colors, startangle=90)
    ax.set_ylabel('')
    ax.set_title('Composition of Positive Cashflows ($M)')
    
    plt.tight_layout()
    plt.savefig('revolving_credit_cashflow_analysis.png', dpi=150, bbox_inches='tight')
    print(f"\n✓ Saved visualizations: revolving_credit_cashflow_analysis.png")
    
    return fig


def demonstrate_filtering_and_aggregation(df):
    """Demonstrate various DataFrame operations."""
    print("\n" + "=" * 80)
    print("DATAFRAME FILTERING & AGGREGATION")
    print("=" * 80)
    
    # 1. Filter by cashflow type
    print("\n1. Filter by Cashflow Type:")
    print("-" * 80)
    principal = df[df['cashflow_type'] == 'Principal']
    interest = df[df['cashflow_type'] == 'Interest']
    all_fees = df[df['cashflow_type'].str.contains('Fee')]
    
    print(f"  Principal flows: {len(principal)} records, Total: ${principal['amount'].sum() / 1e6:.2f}M")
    print(f"  Interest flows:  {len(interest)} records, Total: ${interest['amount'].sum() / 1e6:.2f}M")
    print(f"  All fees:        {len(all_fees)} records, Total: ${all_fees['amount'].sum() / 1e6:.2f}M")
    
    # 2. Aggregate by path and type
    print("\n2. Aggregate by Path and Type:")
    print("-" * 80)
    path_type_agg = df.groupby(['path_id', 'cashflow_type'])['amount'].sum() / 1e3
    first_path_id = df['path_id'].iloc[0]
    print(f"Sample (path_id={first_path_id}):")
    print(path_type_agg.loc[first_path_id].to_string())
    
    # 3. Time-based analysis
    print("\n3. Cashflows by Timestep:")
    print("-" * 80)
    by_step = df.groupby(['step', 'cashflow_type'])['amount'].mean() / 1e3
    print("Average cashflows per step ($k):")
    print(by_step.unstack(fill_value=0).to_string())
    
    # 4. Summary statistics
    print("\n4. Summary Statistics by Type:")
    print("-" * 80)
    summary = df.groupby('cashflow_type')['amount'].describe() / 1e3
    print(summary.to_string())


def main():
    """Run comprehensive cashflow tracking demonstration."""
    print("\n" + "=" * 80)
    print("REVOLVING CREDIT CASHFLOW TRACKING")
    print("Using ACTUAL Cashflows from Rust Monte Carlo Engine")
    print("=" * 80)
    
    # Run MC simulation and get paths
    dataset = example_multi_factor_mc_with_cashflows()
    
    if dataset is None:
        print("\nCould not get path dataset")
        return
    
    # Analyze single path
    single_path_df = analyze_single_path(dataset)
    
    # Analyze all paths
    all_cashflows_df = analyze_all_paths(dataset)
    
    # Demonstrate filtering and aggregation
    demonstrate_filtering_and_aggregation(all_cashflows_df)
    
    # Analyze IRR distribution
    irrs = analyze_irr_distribution(dataset)
    
    # Create visualizations
    fig = visualize_cashflows(all_cashflows_df)
    
    print("\n" + "=" * 80)
    print("SUMMARY")
    print("=" * 80)
    
    print("\n✓ Successfully demonstrated:")
    print("  1. Multi-factor Monte Carlo simulation (utilization + credit)")
    print("  2. Real cashflow extraction from Rust engine")
    print("  3. Typed cashflow categorization (9 types)")
    print("  4. DataFrame conversion (PathPoint, SimulatedPath, PathDataset)")
    print("  5. Per-path IRR calculation")
    print("  6. Pandas aggregation and filtering")
    print("  7. Comprehensive visualizations")
    
    print("\nGenerated Files:")
    print("  • revolving_credit_cashflow_analysis.png")
    print("  • revolving_credit_irr_distribution.png")
    
    print("\nKey API Methods Used:")
    print("  • revolver.mc_paths() - Run MC with path capture")
    print("  • path.to_dataframe() - Convert path cashflows to DataFrame")
    print("  • dataset.cashflows_to_dataframe() - Convert all cashflows")
    print("  • path.extract_cashflows_by_type(CashflowType) - Filter by type")
    print("  • path.irr - Get annualized IRR")
    
    print("\nCashflow Types Tracked:")
    for cf_type in ['Principal', 'Interest', 'CommitmentFee', 'UsageFee', 
                    'FacilityFee', 'UpfrontFee', 'Recovery']:
        count = len(all_cashflows_df[all_cashflows_df['cashflow_type'] == cf_type])
        total = all_cashflows_df[all_cashflows_df['cashflow_type'] == cf_type]['amount'].sum()
        if count > 0:
            print(f"  • {cf_type:16s}: {count:4d} records, Total: ${total/1e6:>8.2f}M")
    
    print("\n" + "=" * 80 + "\n")


if __name__ == "__main__":
    main()
