"""
Revolving Credit Python API Demonstration

Showcases the improved revolving credit bindings with:
- Standard pricing interface (npv, value)
- Convenient cashflow DataFrame extraction
- IRR distribution analysis
- Path visualization (utilization, credit spreads, cashflows)

This example demonstrates the full capabilities of the redesigned
revolving credit instrument and payoff implementation.
"""

from datetime import date
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.valuations.instruments import RevolvingCredit


def create_market_data() -> tuple[MarketContext, date]:
    """Create market data environment."""
    val_date = date(2025, 1, 1)
    
    # Discount curve (SOFR-based)
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [
            (0.0, 1.0000),
            (0.25, 0.9875),
            (0.5, 0.9750),
            (1.0, 0.9500),
            (2.0, 0.9000),
            (3.0, 0.8500),
        ],
    )
    
    # Hazard curve (BBB corporate credit)
    hazard_curve = HazardCurve(
        "CORP.BBB",
        val_date,
        [
            (1.0, 0.9850),  # ~150bp spread
            (3.0, 0.9550),
            (5.0, 0.9200),
        ],
    )
    
    market = MarketContext()
    market.insert_discount(disc_curve)
    market.insert_hazard(hazard_curve)
    
    return market, val_date


def example_1_standard_pricing():
    """Example 1: Standard pricing interface."""
    print("\n" + "=" * 80)
    print("EXAMPLE 1: STANDARD PRICING INTERFACE")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    # Create a simple revolving credit facility
    revolver = RevolvingCredit.builder(
        instrument_id="RC_STANDARD",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "upfront_fee": Money(50_000.0, USD),
            "commitment_fee_bp": 25.0,  # 25 bps on undrawn
            "usage_fee_bp": 50.0,        # 50 bps on drawn
            "facility_fee_bp": 10.0,      # 10 bps on commitment
        },
        draw_repay_spec={"deterministic": []},  # Constant utilization
        discount_curve="USD.SOFR",
    )
    
    print(f"\nFacility: {revolver}")
    print(f"Utilization Rate: {revolver.utilization_rate():.1%}")
    print(f"Undrawn Amount: {revolver.undrawn_amount()}")
    
    # Price using standard interface
    npv = revolver.npv(market, val_date)
    print(f"\nNet Present Value: {npv}")
    
    # value() method (alias for npv)
    value = revolver.value(market, val_date)
    print(f"Value (same as NPV): {value}")
    
    print("\n✓ Standard pricing interface works seamlessly")


def example_2_monte_carlo_with_paths():
    """Example 2: Monte Carlo simulation with path capture and analysis."""
    print("\n" + "=" * 80)
    print("EXAMPLE 2: MONTE CARLO WITH PATH ANALYSIS")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    # Create stochastic facility with multi-factor dynamics
    revolver = RevolvingCredit.builder(
        instrument_id="RC_STOCHASTIC",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.06},
        payment_frequency="quarterly",
        fees={
            "upfront_fee": Money(50_000.0, USD),
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 2.0,
                    "volatility": 0.35,
                },
                "num_paths": 5000,
                "seed": 42,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": "CORP.BBB",
                            "kappa": 0.5,
                            "implied_vol": 0.50,
                        }
                    },
                    "util_credit_corr": -0.3,  # Negative correlation: high util when credit worsens
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    print(f"\nFacility: {revolver}")
    print(f"Initial Utilization: {revolver.utilization_rate():.1%}")
    
    # Run Monte Carlo with path capture
    print("\nRunning Monte Carlo simulation (500 paths)...")
    result = revolver.mc_paths(
        market,
        val_date,
        capture_mode="sample",
        sample_count=500,  # Capture 50 paths for visualization
        seed=42
    )
    
    print(f"Estimated PV: {result.estimate}")
    print(f"Std Error: {result.stderr:.2f}")
    print(f"95% CI: {result.ci_95[0]} to {result.ci_95[1]}")
    print(f"Paths captured: {result.num_captured_paths()} / {result.num_paths}")
    
    # Extract paths for analysis
    paths = result.paths
    assert paths is not None, "Paths should be captured"
    
    print(f"\nPath dataset: {paths}")
    print(f"State variables available: {paths.state_var_keys()}")
    
    # Convert to DataFrame
    df = paths.to_dataframe()
    print(f"DataFrame shape: {df.shape}")
    print(f"Columns: {list(df.columns)}")
    print(df.head())
    
    return result, revolver, market, val_date


def example_3_cashflow_dataframe():
    """Example 3: Simple cashflow DataFrame extraction."""
    print("\n" + "=" * 80)
    print("EXAMPLE 3: CASHFLOW DATAFRAME EXTRACTION")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    revolver = RevolvingCredit.builder(
        instrument_id="RC_CASHFLOW_DEMO",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 40.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 2.0,
                    "volatility": 0.3,
                },
                "num_paths": 5000,
                "seed": 123,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {"constant": 0.015},
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    # One-line cashflow DataFrame extraction!
    print("\nExtracting cashflows to DataFrame...")
    df = revolver.cashflows_df(market, val_date, num_paths=5000, capture_mode="all", seed=123)
    
    print(f"Total cashflows: {len(df)}")
    print(f"\nCashflow breakdown by type:")
    print(df.groupby('cashflow_type')['amount'].agg(['count', 'sum', 'mean']))
    
    # Analyze principal flows
    print("\n" + "-" * 80)
    print("Principal Flow Analysis:")
    principal = df[df['cashflow_type'] == 'Principal']
    deployments = principal[principal['amount'] < 0]
    repayments = principal[principal['amount'] > 0]
    
    print(f"  Deployments: {len(deployments)} flows, total: ${-deployments['amount'].sum():,.2f}")
    print(f"  Repayments:  {len(repayments)} flows, total: ${repayments['amount'].sum():,.2f}")
    print(f"  Net:         ${principal['amount'].sum():,.2f}")
    
    return df, revolver, market, val_date


def example_4_irr_distribution():
    """Example 4: IRR distribution analysis."""
    print("\n" + "=" * 80)
    print("EXAMPLE 4: IRR DISTRIBUTION ANALYSIS")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    revolver = RevolvingCredit.builder(
        instrument_id="RC_IRR_DEMO",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.06},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 2.0,
                    "volatility": 0.30,
                },
                "num_paths": 5000,
                "seed": 456,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "cir": {
                            "kappa": 0.5,
                            "theta": 0.015,
                            "sigma": 0.03,
                            "initial": 0.015,
                        }
                    },
                    "util_credit_corr": -0.25,
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    # Calculate IRR distribution (simple one-liner!)
    print("\nCalculating IRR distribution across 500 paths...")
    irr_stats = revolver.irr_distribution(market, val_date, num_paths=5000, seed=456)
    
    print(f"\nIRR Distribution Statistics:")
    print(f"  Mean:     {irr_stats['mean']:.2%}")
    print(f"  Std Dev:  {irr_stats['std']:.2%}")
    print(f"\nPercentiles:")
    print(f"  P10:      {irr_stats['percentiles']['p10']:.2%}")
    print(f"  P25:      {irr_stats['percentiles']['p25']:.2%}")
    print(f"  Median:   {irr_stats['percentiles']['p50']:.2%}")
    print(f"  P75:      {irr_stats['percentiles']['p75']:.2%}")
    print(f"  P90:      {irr_stats['percentiles']['p90']:.2%}")
    
    return irr_stats


def create_visualizations():
    """Create comprehensive visualizations."""
    print("\n" + "=" * 80)
    print("CREATING VISUALIZATIONS")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    # Create facility with multi-factor dynamics
    revolver = RevolvingCredit.builder(
        instrument_id="RC_VIZ",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "upfront_fee": Money(50_000.0, USD),
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 2.0,
                    "volatility": 0.30,
                },
                "num_paths": 5000,
                "seed": 789,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "cir": {
                            "kappa": 0.6,
                            "theta": 0.018,
                            "sigma": 0.04,
                            "initial": 0.015,
                        }
                    },
                    "util_credit_corr": -0.30,
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    # Run MC with path capture
    print("\nRunning Monte Carlo (300 paths, capturing 30 for visualization)...")
    result = revolver.mc_paths(
        market,
        val_date,
        capture_mode="sample",
        sample_count=100,
        seed=789
    )
    
    print(f"Estimated PV: {result.estimate}")
    
    # Extract path data
    paths = result.paths
    df_paths = paths.to_dataframe()
    
    # Extract cashflows
    print("\nExtracting cashflow data...")
    df_cashflows = revolver.cashflows_df(market, val_date, num_paths=5000, capture_mode="sample", seed=789)
    
    # Create figure with subplots
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle('Revolving Credit Facility - Monte Carlo Analysis', fontsize=16, fontweight='bold')
    
    # --- Plot 1: Utilization Paths ---
    ax1 = axes[0, 0]
    for path_id in df_paths['path_id'].unique()[:30]:
        path_data = df_paths[df_paths['path_id'] == path_id]
        if 'spot' in path_data.columns:
            ax1.plot(path_data['time'], path_data['spot'], alpha=0.3, linewidth=0.8)
    
    ax1.axhline(y=0.50, color='r', linestyle='--', linewidth=1.5, label='Target (50%)')
    ax1.axhline(y=0.35, color='orange', linestyle=':', linewidth=1, alpha=0.7, label='Initial (35%)')
    ax1.set_xlabel('Time (years)', fontsize=11)
    ax1.set_ylabel('Utilization Rate', fontsize=11)
    ax1.set_title('Utilization Rate Paths (Mean-Reverting Process)', fontsize=12, fontweight='bold')
    ax1.grid(True, alpha=0.3)
    ax1.legend(loc='upper right')
    ax1.set_ylim([0, 1])
    
    # --- Plot 2: Credit Spread Paths ---
    ax2 = axes[0, 1]
    for path_id in df_paths['path_id'].unique()[:30]:
        path_data = df_paths[df_paths['path_id'] == path_id]
        if 'credit_spread' in path_data.columns:
            # Convert to basis points
            ax2.plot(path_data['time'], path_data['credit_spread'] * 10000, alpha=0.3, linewidth=0.8)
    
    ax2.axhline(y=180, color='r', linestyle='--', linewidth=1.5, label='Long-term Mean (180bp)')
    ax2.axhline(y=150, color='orange', linestyle=':', linewidth=1, alpha=0.7, label='Initial (150bp)')
    ax2.set_xlabel('Time (years)', fontsize=11)
    ax2.set_ylabel('Credit Spread (bps)', fontsize=11)
    ax2.set_title('Credit Spread Paths (CIR Process)', fontsize=12, fontweight='bold')
    ax2.grid(True, alpha=0.3)
    ax2.legend(loc='upper right')
    
    # --- Plot 3: Single Path Cashflow Waterfall ---
    ax3 = axes[1, 0]
    
    # Select first path
    first_path_id = df_cashflows['path_id'].iloc[0]
    path_cf = df_cashflows[df_cashflows['path_id'] == first_path_id].copy()
    
    # Aggregate cashflows by type and time
    cf_by_type = path_cf.groupby(['time_years', 'cashflow_type'])['amount'].sum().reset_index()
    
    # Create stacked bar chart
    cf_types = ['Interest', 'CommitmentFee', 'UsageFee', 'FacilityFee']
    cf_colors = {'Interest': '#2E86AB', 'CommitmentFee': '#A23B72', 
                 'UsageFee': '#F18F01', 'FacilityFee': '#C73E1D'}
    
    times = sorted(cf_by_type['time_years'].unique())
    bottoms = np.zeros(len(times))
    
    for cf_type in cf_types:
        type_data = cf_by_type[cf_by_type['cashflow_type'] == cf_type]
        amounts = []
        for t in times:
            matching = type_data[type_data['time_years'] == t]
            amounts.append(matching['amount'].sum() if len(matching) > 0 else 0.0)
        
        ax3.bar(times, amounts, bottom=bottoms, label=cf_type, 
                color=cf_colors.get(cf_type, 'gray'), alpha=0.8, width=0.06)
        bottoms += np.array(amounts)
    
    # Add principal flows separately (draws and repayments)
    principal_data = cf_by_type[cf_by_type['cashflow_type'] == 'Principal']
    
    # Separate draws (negative) and repayments (positive)
    draws = principal_data[principal_data['amount'] < 0]
    repayments = principal_data[principal_data['amount'] >= 0]
    
    # Plot draws (red bars)
    if len(draws) > 0:
        ax3.bar(draws['time_years'], draws['amount'], color='#D62246', alpha=0.7, 
                width=0.04, label='Principal Draw (deploy)')
    
    # Plot repayments (green bars)
    if len(repayments) > 0:
        ax3.bar(repayments['time_years'], repayments['amount'], color='#06A77D', alpha=0.7, 
                width=0.04, label='Principal Repay (return)')
    
    ax3.set_xlabel('Time (years)', fontsize=11)
    ax3.set_ylabel('Cashflow Amount ($)', fontsize=11)
    ax3.set_title(f'Single Path Cashflow Breakdown (Path {first_path_id})', fontsize=12, fontweight='bold')
    ax3.grid(True, alpha=0.3, axis='y')
    ax3.legend(loc='upper left', fontsize=9)
    ax3.axhline(y=0, color='black', linewidth=0.8)
    
    # --- Plot 4: Average Cumulative Cashflow by Type ---
    ax4 = axes[1, 1]
    
    # Separate principal into draws and repayments for clearer analysis
    df_cashflows_categorized = df_cashflows.copy()
    principal_mask = df_cashflows_categorized['cashflow_type'] == 'Principal'
    df_cashflows_categorized.loc[principal_mask & (df_cashflows_categorized['amount'] < 0), 'cashflow_type'] = 'Principal Draw'
    df_cashflows_categorized.loc[principal_mask & (df_cashflows_categorized['amount'] >= 0), 'cashflow_type'] = 'Principal Repay'
    
    # Step 1: Calculate cumulative cashflow by type for EACH path
    path_cumulative = df_cashflows_categorized.groupby(['path_id', 'cashflow_type'])['amount'].sum().reset_index()
    path_cumulative.columns = ['path_id', 'cashflow_type', 'cumulative_amount']
    
    # Step 2: Pivot to get all cashflow types as columns, filling missing with 0
    # This ensures paths without a specific type (e.g., no Recovery) get 0
    all_path_ids = df_cashflows_categorized['path_id'].unique()
    all_cf_types = path_cumulative['cashflow_type'].unique()
    
    # Create complete matrix: all paths × all cashflow types
    path_type_pivot = path_cumulative.pivot(index='path_id', columns='cashflow_type', values='cumulative_amount')
    path_type_pivot = path_type_pivot.fillna(0)  # Paths without a type get 0
    
    # Step 3: Calculate average cumulative across paths for each type (using absolute values)
    type_stats_data = []
    for cf_type in path_type_pivot.columns:
        values = path_type_pivot[cf_type].abs()  # Absolute for fair comparison
        mean_val = values.mean()
        num_nonzero = (values > 1e-6).sum()  # Count paths with this type
        type_stats_data.append({
            'cashflow_type': cf_type,
            'mean_cumulative': mean_val,
            'num_paths_with_type': num_nonzero,
            'total_paths': len(values)
        })
    
    type_stats = pd.DataFrame(type_stats_data).set_index('cashflow_type')
    type_stats = type_stats.sort_values('mean_cumulative', ascending=False)
    
    colors_map = {
        'Principal Draw': '#D62246',
        'Principal Repay': '#06A77D',
        'Interest': '#2E86AB',
        'CommitmentFee': '#A23B72',
        'UsageFee': '#F18F01',
        'FacilityFee': '#C73E1D',
        'Recovery': '#E85D75',
    }
    
    colors = [colors_map.get(cf_type, 'gray') for cf_type in type_stats.index]
    
    # Use average cumulative in thousands of dollars
    bars = ax4.barh(range(len(type_stats)), type_stats['mean_cumulative'] / 1_000, color=colors, alpha=0.8)
    ax4.set_yticks(range(len(type_stats)))
    ax4.set_yticklabels(type_stats.index)
    ax4.set_xlabel('Average Total per Path ($K)', fontsize=11)
    ax4.set_title('Average Cumulative Cashflow by Type (Across Paths)', fontsize=12, fontweight='bold')
    ax4.grid(True, alpha=0.3, axis='x')
    
    # Add value labels showing average cumulative and path participation
    for i, (idx, row) in enumerate(type_stats.iterrows()):
        mean_value = row['mean_cumulative'] / 1_000
        num_with_type = int(row['num_paths_with_type'])
        total_paths = int(row['total_paths'])
        pct = num_with_type / total_paths * 100
        ax4.text(mean_value + 100, i, f'${mean_value:.0f}K avg ({pct:.0f}% paths)', 
                va='center', fontsize=9)
    
    plt.tight_layout()
    plt.savefig('revolving_credit_api_demo.png', dpi=150, bbox_inches='tight')
    print(f"\n✓ Saved visualization to 'revolving_credit_api_demo.png'")
    
    return df_cashflows


def example_5_pandas_analysis():
    """Example 5: Advanced pandas analysis patterns."""
    print("\n" + "=" * 80)
    print("EXAMPLE 5: ADVANCED PANDAS ANALYSIS")
    print("=" * 80)
    
    market, val_date = create_market_data()
    
    revolver = RevolvingCredit.builder(
        instrument_id="RC_PANDAS_DEMO",
        commitment_amount=Money(10_000_000.0, USD),
        drawn_amount=Money(4_000_000.0, USD),
        commitment_date=val_date,
        maturity_date=date(2030, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
            "facility_fee_bp": 10.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 2.0,
                    "volatility": 0.25,
                },
                "num_paths": 5000,
                "seed": 111,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {"constant": 0.015},
                },
            }
        },
        discount_curve="USD.SOFR",
    )
    
    # Get cashflows
    df = revolver.cashflows_df(market, val_date, num_paths=5000, seed=111)
    
    print("\nPandas Analysis Examples:")
    print("-" * 80)
    
    # 1. Cashflows per path
    print("\n1. Total cashflows per path:")
    path_totals = df.groupby('path_id')['amount'].sum()
    print(f"   Mean per path: ${path_totals.mean():,.2f}")
    print(f"   Std dev:       ${path_totals.std():,.2f}")
    
    # 2. Time-series analysis
    print("\n2. Cashflows by quarter:")
    df['quarter'] = (df['time_years'] * 4).round().astype(int)
    quarterly = df.groupby('quarter')['amount'].sum()
    print(quarterly.head(8))
    
    # 3. Interest coverage
    print("\n3. Interest vs. Fee analysis:")
    interest_total = df[df['cashflow_type'] == 'Interest']['amount'].sum()
    fee_total = df[df['cashflow_type'].isin(['CommitmentFee', 'UsageFee', 'FacilityFee'])]['amount'].sum()
    print(f"   Total Interest: ${interest_total:,.2f}")
    print(f"   Total Fees:     ${fee_total:,.2f}")
    print(f"   Fee/Interest:   {fee_total/interest_total:.1%}")
    
    # 4. Principal flows
    print("\n4. Principal deployment patterns:")
    principal = df[df['cashflow_type'] == 'Principal']
    draws = principal[principal['amount'] < 0]
    repays = principal[principal['amount'] > 0]
    print(f"   Average draw per event:  ${-draws.groupby('path_id')['amount'].sum().mean():,.2f}")
    print(f"   Average repay per event: ${repays.groupby('path_id')['amount'].sum().mean():,.2f}")
    
    # 5. Default analysis
    print("\n5. Default event analysis:")
    recovery_flows = df[df['cashflow_type'] == 'Recovery']
    if len(recovery_flows) > 0:
        num_defaults = recovery_flows['path_id'].nunique()
        total_recovery = recovery_flows['amount'].sum()
        default_rate = num_defaults / 200.0
        print(f"   Paths with defaults: {num_defaults} / 200 ({default_rate:.1%})")
        print(f"   Total recovery:      ${total_recovery:,.2f}")
        print(f"   Avg recovery/event:  ${total_recovery/num_defaults:,.2f}")
    else:
        print(f"   No defaults in simulation")
    
    print("\n✓ Pandas analysis patterns demonstrated")


def main():
    """Run all examples."""
    print("=" * 80)
    print("REVOLVING CREDIT PYTHON API DEMONSTRATION")
    print("Showcasing the improved bindings with full Rust functionality access")
    print("=" * 80)
    
    # Run examples
    example_1_standard_pricing()
    example_2_monte_carlo_with_paths()
    df_cashflows = example_3_cashflow_dataframe()
    irr_stats = example_4_irr_distribution()
    create_visualizations()
    example_5_pandas_analysis()
    
    print("\n" + "=" * 80)
    print("DEMONSTRATION COMPLETE")
    print("=" * 80)
    print("\nKey Features Demonstrated:")
    print("  ✓ Standard pricing interface (npv, value)")
    print("  ✓ Monte Carlo with path capture")
    print("  ✓ One-line cashflow DataFrame extraction")
    print("  ✓ Built-in IRR distribution analysis")
    print("  ✓ Full access to state variables (utilization, credit spreads)")
    print("  ✓ Simple pandas integration patterns")
    print("  ✓ Comprehensive visualizations")
    print("\nThe API is simple, robust, and provides complete access to Rust functionality!")


if __name__ == "__main__":
    main()

