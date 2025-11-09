#!/usr/bin/env python3
"""Revolving Credit Stochastic Path Analysis.

Demonstrates comprehensive analysis of Monte Carlo paths for revolving credit facilities.

This script provides a complete workflow for investigating stochastic pricing:

1. **State Variable Path Visualization** (100 sample paths)
   - Utilization rate evolution (mean-reverting Ornstein-Uhlenbeck process)
   - Credit spread dynamics (CIR process, market-anchored calibration)
   - Interest rate paths (Hull-White 1F model)
   - Mean paths overlaid in red for reference

2. **Worst-Path Identification**
   - Identifies paths with lowest final values (worst PV)
   - Identifies paths with lowest IRRs (worst returns)
   - Useful for stress testing and tail risk analysis

3. **Cashflow Decomposition by Path**
   - Plots cashflows for worst PV paths with color-coded types
   - Plots cashflows for worst IRR paths with color-coded types
     * Principal (blue): draws and repayments
     * Interest (red): interest on drawn amounts
     * CommitmentFee (green): fees on undrawn commitment
     * Recovery (brown): recovery proceeds on default
   - Shows how different scenarios produce different cashflow patterns

4. **IRR Distribution Analysis**
   - Computes Internal Rate of Return for each path
   - Displays histogram with statistics (mean, std, percentiles)
   - Captures path-dependent return variability

**Multi-Factor Modeling Features:**
- Correlated state variables (utilization, credit, rates)
- Market-anchored credit spread calibration from hazard curve
- Hull-White short rate dynamics for floating-rate instruments
- Default modeling with first-passage time methodology

**Usage:**
    python revolving_credit_path_analysis.py

**Output Files (in finstack-py/examples/outputs/):**
- revolving_credit_state_paths.png: 3-panel plot of state variables
- revolving_credit_worst_pv_cashflows.png: Cashflows for worst PV paths
- revolving_credit_worst_irr_cashflows.png: Cashflows for worst IRR paths
- revolving_credit_irr_distribution.png: IRR histogram with statistics

**Key Configuration:**
- 5,000 Monte Carlo paths
- 5-year facility term
- Quarterly payment frequency
- 20% utilization volatility
- 60% correlation between utilization and credit spread
"""

from datetime import date
import math

import matplotlib.pyplot as plt
import numpy as np
import pandas as pd

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve
from finstack.valuations.instruments import RevolvingCredit


def create_market_data() -> tuple[MarketContext, date]:
    """Create market data with discount, forward, and hazard curves."""
    val_date = date(2025, 1, 1)

    # Discount curve (SOFR-based, ~4% rate)
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

    # Hazard curve for credit risk (cumulative default probabilities)
    # Using survival probabilities: 99% at 1Y, 96% at 3Y, 92% at 5Y
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


def create_stochastic_facility(
    instrument_id: str,
    commitment: Money,
    drawn: Money,
    commitment_date: date,
    maturity_date: date,
    margin_bp: float,
    commitment_fee_bp: float,
    volatility: float,
    num_paths: int = 10000,
    seed: int = 42,
) -> RevolvingCredit:
    """Create a stochastic revolving credit facility with credit risk."""
    initial_util = drawn.amount / commitment.amount

    return RevolvingCredit.builder(
        instrument_id=instrument_id,
        commitment_amount=commitment,
        drawn_amount=drawn,
        commitment_date=commitment_date,
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
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": initial_util,
                    "speed": 0.5,
                    "volatility": volatility,
                },
                "num_paths": num_paths,
                "seed": seed,
                "antithetic": True,
                "use_sobol_qmc": False,
                "default_model": None,
                # Enable multi-factor MC with credit spread and interest rate dynamics
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": "BORROWER-A",
                            "kappa": 0.3,
                            "implied_vol": 0.50,
                            "tenor_years": None,
                        }
                    },
                    "interest_rate_process": {
                        "hull_white_1f": {
                            "kappa": 0.1,
                            "sigma": 0.01,
                            "initial": 0.045,
                            "theta": 0.045,
                        }
                    },
                    "util_credit_corr": 0.6,  # Positive correlation: high util → wider spreads
                },
            }
        },
        discount_curve="USD-OIS",
    )


def extract_path_data(mc_result) -> pd.DataFrame:
    """Extract state variables from MC paths into DataFrame."""
    paths_dataset = mc_result.paths
    if paths_dataset is None:
        raise ValueError("No paths captured in MC result")

    # Convert to DataFrame with state variables
    path_data = paths_dataset.to_dict()
    df = pd.DataFrame(path_data)

    return df


def plot_state_variable_paths(df: pd.DataFrame, sample_size: int = 100):
    """Plot utilization, credit spread, and interest rate paths."""
    # Sample paths for visualization
    all_path_ids = df["path_id"].unique()
    np.random.seed(42)
    sampled_ids = np.random.choice(all_path_ids, size=min(sample_size, len(all_path_ids)), replace=False)
    df_sample = df[df["path_id"].isin(sampled_ids)]

    fig, axes = plt.subplots(3, 1, figsize=(12, 10))

    # Plot 1: Utilization paths (stored in 'spot' column)
    ax1 = axes[0]
    for path_id in sampled_ids:
        path_df = df_sample[df_sample["path_id"] == path_id]
        ax1.plot(path_df["time"], path_df["spot"], alpha=0.15, color="blue", linewidth=0.5)

    # Add mean path
    mean_util = df.groupby("time")["spot"].mean()
    ax1.plot(mean_util.index, mean_util.values, color="red", linewidth=2, label="Mean")
    ax1.set_xlabel("Time (years)")
    ax1.set_ylabel("Utilization Rate")
    ax1.set_title(f"Utilization Paths (n={sample_size} sampled)")
    ax1.grid(True, alpha=0.3)
    ax1.legend()

    # Plot 2: Credit spread paths
    ax2 = axes[1]
    for path_id in sampled_ids:
        path_df = df_sample[df_sample["path_id"] == path_id]
        # Convert spread to bps for readability
        ax2.plot(path_df["time"], path_df["credit_spread"] * 10000, alpha=0.15, color="green", linewidth=0.5)

    mean_spread = df.groupby("time")["credit_spread"].mean() * 10000
    ax2.plot(mean_spread.index, mean_spread.values, color="red", linewidth=2, label="Mean")
    ax2.set_xlabel("Time (years)")
    ax2.set_ylabel("Credit Spread (bps)")
    ax2.set_title(f"Credit Spread Paths (n={sample_size} sampled)")
    ax2.grid(True, alpha=0.3)
    ax2.legend()

    # Plot 3: Interest rate paths (stored in 'variance' column as fallback for short_rate)
    ax3 = axes[2]
    # Use 'variance' column which contains the short rate
    for path_id in sampled_ids:
        path_df = df_sample[df_sample["path_id"] == path_id]
        # Convert rate to bps
        ax3.plot(path_df["time"], path_df["variance"] * 10000, alpha=0.15, color="purple", linewidth=0.5)

    mean_rate = df.groupby("time")["variance"].mean() * 10000
    ax3.plot(mean_rate.index, mean_rate.values, color="red", linewidth=2, label="Mean")
    ax3.set_xlabel("Time (years)")
    ax3.set_ylabel("Short Rate (bps)")
    ax3.set_title(f"Interest Rate Paths (n={sample_size} sampled)")
    ax3.grid(True, alpha=0.3)
    ax3.legend()

    plt.tight_layout()
    plt.savefig("finstack-py/examples/outputs/revolving_credit_state_paths.png", dpi=150)
    print("✓ Saved state variable paths plot")
    plt.close()


def identify_worst_paths(df: pd.DataFrame, num_worst: int = 5) -> list[int]:
    """Identify paths with worst (most negative) final values."""
    # Get final value for each path
    path_values = df.groupby("path_id")["final_value"].first().sort_values()
    worst_paths = path_values.head(num_worst).index.tolist()

    print(f"\n{'='*80}")
    print(f"WORST {num_worst} PATHS BY FINAL VALUE")
    print(f"{'='*80}")
    for i, path_id in enumerate(worst_paths, 1):
        final_val = path_values[path_id]
        print(f"  {i}. Path {path_id}: Final Value = ${final_val:,.2f}")

    return worst_paths


def plot_worst_path_cashflows(
    facility: RevolvingCredit,
    market: MarketContext,
    val_date: date,
    worst_paths: list[int],
    num_paths: int,
    df: pd.DataFrame,
):
    """Plot cashflows for worst paths by type with IRR annotations."""
    # Get cashflows for all paths
    cf_df = facility.cashflows_df(
        market,
        as_of=val_date,
        num_paths=num_paths,
        capture_mode="all",
        seed=42,
    )

    # Get IRR distribution to annotate each path
    irr_stats = facility.irr_distribution(market, as_of=val_date, num_paths=num_paths, seed=42)
    irrs = irr_stats["irrs"]

    # Get final values for context
    path_values = df.groupby("path_id")["final_value"].first()

    # Filter to worst paths
    cf_worst = cf_df[cf_df["path_id"].isin(worst_paths)]

    # Create stacked bar chart by cashflow type
    fig, axes = plt.subplots(len(worst_paths), 1, figsize=(14, 3 * len(worst_paths)))
    if len(worst_paths) == 1:
        axes = [axes]

    cashflow_colors = {
        "Principal": "blue",
        "Interest": "red",
        "CommitmentFee": "green",
        "UsageFee": "orange",
        "FacilityFee": "purple",
        "Recovery": "brown",
        "MarkToMarket": "pink",
        "Other": "gray",
    }

    for idx, path_id in enumerate(worst_paths):
        ax = axes[idx]
        path_cf = cf_worst[cf_worst["path_id"] == path_id]

        # Group by time and cashflow type
        cf_types = path_cf["cashflow_type"].unique()

        for cf_type in cf_types:
            type_cf = path_cf[path_cf["cashflow_type"] == cf_type]
            color = cashflow_colors.get(cf_type, "gray")
            ax.scatter(
                type_cf["time_years"],
                type_cf["amount"],
                label=cf_type,
                alpha=0.7,
                s=50,
                color=color,
            )

        ax.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax.set_xlabel("Time (years)")
        ax.set_ylabel("Cashflow ($)")

        # Add IRR and final value to title if available
        irr_val = irrs[path_id] if path_id < len(irrs) else None
        final_val = path_values[path_id] if path_id in path_values.index else None

        if irr_val is not None and final_val is not None:
            ax.set_title(f"Path {path_id} | IRR: {irr_val*100:.2f}% | Final Value: ${final_val:,.0f}")
        elif irr_val is not None:
            ax.set_title(f"Path {path_id} | IRR: {irr_val*100:.2f}%")
        else:
            ax.set_title(f"Path {path_id} Cashflows by Type")

        ax.legend(loc="best", fontsize=8)
        ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig("finstack-py/examples/outputs/revolving_credit_worst_pv_cashflows.png", dpi=150)
    print("✓ Saved worst PV path cashflows plot")
    plt.close()


def plot_worst_irr_cashflows(
    facility: RevolvingCredit,
    market: MarketContext,
    val_date: date,
    num_paths: int,
    df: pd.DataFrame,
    num_worst: int = 5,
):
    """Plot cashflows for paths with worst IRRs."""
    # Get IRR distribution
    irr_stats = facility.irr_distribution(market, as_of=val_date, num_paths=num_paths, seed=42)
    irrs = irr_stats["irrs"]

    # Create DataFrame of path_id and IRR
    irr_data = []
    for path_id, irr_val in enumerate(irrs):
        if irr_val is not None:
            irr_data.append({"path_id": path_id, "irr": irr_val})

    irr_df = pd.DataFrame(irr_data)

    # Sort by IRR and get worst paths
    worst_irr_paths = irr_df.nsmallest(num_worst, "irr")["path_id"].tolist()

    print(f"\n{'='*80}")
    print(f"WORST {num_worst} PATHS BY IRR")
    print(f"{'='*80}")

    # Get final values for context
    path_values = df.groupby("path_id")["final_value"].first()

    for i, path_id in enumerate(worst_irr_paths, 1):
        irr_val = irrs[path_id]
        final_val = path_values[path_id] if path_id in path_values.index else None
        if final_val is not None:
            print(f"  {i}. Path {path_id}: IRR = {irr_val*100:.2f}% | Final Value = ${final_val:,.2f}")
        else:
            print(f"  {i}. Path {path_id}: IRR = {irr_val*100:.2f}%")

    # Get cashflows for all paths
    cf_df = facility.cashflows_df(
        market,
        as_of=val_date,
        num_paths=num_paths,
        capture_mode="all",
        seed=42,
    )

    # Filter to worst IRR paths
    cf_worst = cf_df[cf_df["path_id"].isin(worst_irr_paths)]

    # Create bar chart by cashflow type
    fig, axes = plt.subplots(len(worst_irr_paths), 1, figsize=(14, 3 * len(worst_irr_paths)))
    if len(worst_irr_paths) == 1:
        axes = [axes]

    cashflow_colors = {
        "Principal": "blue",
        "Interest": "red",
        "CommitmentFee": "green",
        "UsageFee": "orange",
        "FacilityFee": "purple",
        "Recovery": "brown",
        "MarkToMarket": "pink",
        "Other": "gray",
    }

    for idx, path_id in enumerate(worst_irr_paths):
        ax = axes[idx]
        path_cf = cf_worst[cf_worst["path_id"] == path_id]

        # Group by time and cashflow type
        cf_types = path_cf["cashflow_type"].unique()

        for cf_type in cf_types:
            type_cf = path_cf[path_cf["cashflow_type"] == cf_type]
            color = cashflow_colors.get(cf_type, "gray")
            ax.scatter(
                type_cf["time_years"],
                type_cf["amount"],
                label=cf_type,
                alpha=0.7,
                s=50,
                color=color,
            )

        ax.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax.set_xlabel("Time (years)")
        ax.set_ylabel("Cashflow ($)")

        # Add IRR and final value to title
        irr_val = irrs[path_id]
        final_val = path_values[path_id] if path_id in path_values.index else None

        if final_val is not None:
            ax.set_title(f"Path {path_id} | IRR: {irr_val*100:.2f}% | Final Value: ${final_val:,.0f}")
        else:
            ax.set_title(f"Path {path_id} | IRR: {irr_val*100:.2f}%")

        ax.legend(loc="best", fontsize=8)
        ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig("finstack-py/examples/outputs/revolving_credit_worst_irr_cashflows.png", dpi=150)
    print("✓ Saved worst IRR path cashflows plot")
    plt.close()


def plot_irr_distribution(
    facility: RevolvingCredit,
    market: MarketContext,
    val_date: date,
    num_paths: int,
):
    """Calculate and plot IRR distribution across all paths."""
    # Get IRR distribution
    irr_stats = facility.irr_distribution(market, as_of=val_date, num_paths=num_paths, seed=42)

    irrs = irr_stats["irrs"]
    # Filter out None values (paths without valid IRR)
    valid_irrs = [irr for irr in irrs if irr is not None]

    if len(valid_irrs) == 0:
        print("⚠ No valid IRRs computed (no sign changes in cashflows)")
        return

    # Convert to percentage
    irrs_pct = np.array(valid_irrs) * 100

    # Plot histogram
    fig, ax = plt.subplots(figsize=(12, 6))

    ax.hist(irrs_pct, bins=50, alpha=0.7, color="steelblue", edgecolor="black")

    # Add vertical lines for statistics
    mean_irr = irr_stats["mean"] * 100 if irr_stats["mean"] is not None else np.mean(irrs_pct)
    ax.axvline(mean_irr, color="red", linestyle="--", linewidth=2, label=f"Mean: {mean_irr:.2f}%")

    if irr_stats.get("percentiles"):
        p50 = irr_stats["percentiles"].get("p50")
        if p50 is not None:
            ax.axvline(p50 * 100, color="green", linestyle="--", linewidth=2, label=f"Median (p50): {p50*100:.2f}%")

        p10 = irr_stats["percentiles"].get("p10")
        p90 = irr_stats["percentiles"].get("p90")
        if p10 is not None and p90 is not None:
            ax.axvline(p10 * 100, color="orange", linestyle=":", linewidth=1.5, label=f"p10: {p10*100:.2f}%")
            ax.axvline(p90 * 100, color="orange", linestyle=":", linewidth=1.5, label=f"p90: {p90*100:.2f}%")

    ax.set_xlabel("IRR (%)")
    ax.set_ylabel("Frequency")
    ax.set_title(f"IRR Distribution Across Paths (n={len(valid_irrs)} valid IRRs)")
    ax.legend()
    ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plt.savefig("finstack-py/examples/outputs/revolving_credit_irr_distribution.png", dpi=150)
    print("✓ Saved IRR distribution plot")
    plt.close()

    # Print statistics
    print(f"\n{'='*80}")
    print("IRR DISTRIBUTION STATISTICS")
    print(f"{'='*80}")
    print(f"  Total paths: {len(irrs)}")
    print(f"  Valid IRRs: {len(valid_irrs)} ({len(valid_irrs)/len(irrs)*100:.1f}%)")
    print(f"  Mean IRR: {mean_irr:.2f}%")
    if irr_stats.get("std") is not None:
        print(f"  Std Dev: {irr_stats['std']*100:.2f}%")
    if irr_stats.get("percentiles"):
        pcts = irr_stats["percentiles"]
        print("  Percentiles:")
        for pct_name, pct_val in sorted(pcts.items()):
            if pct_val is not None:
                print(f"    {pct_name}: {pct_val*100:.2f}%")


def main():
    """Main analysis workflow."""
    print("=" * 80)
    print("REVOLVING CREDIT STOCHASTIC PATH ANALYSIS")
    print("=" * 80)

    # Create market data
    market, val_date = create_market_data()
    maturity_date = date(2030, 1, 1)  # 5 year term

    # Facility parameters
    commitment_amount = Money(10_000_000.0, USD)
    drawn_amount = Money(3_000_000.0, USD)  # 30% utilization
    margin_bp = 150.0
    commitment_fee_bp = 50.0
    volatility = 0.20  # 20% annualized utilization volatility

    print("\nFacility Configuration:")
    print(f"  Commitment: {commitment_amount}")
    print(f"  Initial Draw: {drawn_amount} ({drawn_amount.amount/commitment_amount.amount*100:.1f}%)")
    print(f"  Term: {val_date} to {maturity_date} (5 years)")
    print(f"  Margin: {margin_bp}bp over SOFR")
    print(f"  Commitment Fee: {commitment_fee_bp}bp on undrawn")
    print(f"  Utilization Volatility: {volatility:.0%}")

    # Create stochastic facility
    num_paths = 5000  # Use smaller number for faster execution
    facility = create_stochastic_facility(
        instrument_id="RC_STOCH_ANALYSIS",
        commitment=commitment_amount,
        drawn=drawn_amount,
        commitment_date=val_date,
        maturity_date=maturity_date,
        margin_bp=margin_bp,
        commitment_fee_bp=commitment_fee_bp,
        volatility=volatility,
        num_paths=num_paths,
        seed=42,
    )

    print(f"\n✓ Facility created with {num_paths} Monte Carlo paths")

    # =========================================================================
    # Step 1: Run MC and capture paths
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 1: Running Monte Carlo Simulation")
    print(f"{'-'*80}")

    mc_result = facility.mc_paths(
        market,
        as_of=val_date,
        capture_mode="all",  # Capture all paths
        sample_count=num_paths,
        seed=42,
    )

    print("✓ MC simulation complete")
    print(f"  Estimate: {mc_result.estimate}")
    print(f"  Std Error: ${mc_result.stderr:,.2f}")
    print(f"  95% CI: ({mc_result.ci_95[0]}, {mc_result.ci_95[1]})")
    print(f"  Paths captured: {mc_result.num_captured_paths()}")

    # =========================================================================
    # Step 2: Extract and visualize state variable paths
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 2: Extracting State Variable Paths")
    print(f"{'-'*80}")

    df = extract_path_data(mc_result)
    print(f"✓ Extracted {len(df)} path points across {df['path_id'].nunique()} paths")
    print(f"  State variables: {[col for col in df.columns if col not in ['path_id', 'step', 'time', 'final_value']]}")

    plot_state_variable_paths(df, sample_size=100)

    # =========================================================================
    # Step 3: Identify worst paths
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 3: Identifying Worst Paths")
    print(f"{'-'*80}")

    worst_paths = identify_worst_paths(df, num_worst=5)

    # =========================================================================
    # Step 4: Plot cashflows for worst PV paths
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 4: Analyzing Cashflows for Worst PV Paths")
    print(f"{'-'*80}")

    plot_worst_path_cashflows(facility, market, val_date, worst_paths, num_paths, df)

    # =========================================================================
    # Step 4b: Plot cashflows for worst IRR paths
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 4b: Analyzing Cashflows for Worst IRR Paths")
    print(f"{'-'*80}")

    plot_worst_irr_cashflows(facility, market, val_date, num_paths, df, num_worst=5)

    # =========================================================================
    # Step 5: Calculate and plot IRR distribution
    # =========================================================================
    print(f"\n{'-'*80}")
    print("STEP 5: IRR Distribution Analysis")
    print(f"{'-'*80}")

    plot_irr_distribution(facility, market, val_date, num_paths)

    # =========================================================================
    # Summary
    # =========================================================================
    print(f"\n{'='*80}")
    print("ANALYSIS COMPLETE")
    print(f"{'='*80}")
    print("\nGenerated outputs:")
    print("  1. revolving_credit_state_paths.png - Utilization, spread, rate paths")
    print("  2. revolving_credit_worst_pv_cashflows.png - Cashflows for worst PV paths")
    print("  3. revolving_credit_worst_irr_cashflows.png - Cashflows for worst IRR paths")
    print("  4. revolving_credit_irr_distribution.png - IRR histogram with statistics")
    print("\nKey insights:")
    print("  • State variable paths show mean-reverting behavior with volatility")
    print("  • Worst PV paths may differ from worst IRR paths")
    print("  • Worst IRR paths show most challenging return scenarios")
    print("  • IRR distribution captures path-dependent return variability")
    print("  • Multi-factor model captures correlation between util/credit/rates")


if __name__ == "__main__":
    main()

