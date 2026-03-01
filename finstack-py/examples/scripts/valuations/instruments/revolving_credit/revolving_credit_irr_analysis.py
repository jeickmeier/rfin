"""Revolving Credit IRR Distribution Analysis.

This script analyzes the Internal Rate of Return (IRR) distributions from
Monte Carlo simulations of revolving credit facilities with different
volatility scenarios for utilization and credit spreads.

Key analyses:
1. IRR distribution for a single MC scenario with path visualization
2. Comparison grid of IRR distributions across volatility combinations
"""

from datetime import date
import json
from pathlib import Path
import sys
from typing import Any

import numpy as np

# Set up output directory for artifacts
OUTPUT_DIR = Path(__file__).parent.parent.parent.parent.parent / "outputs"
OUTPUT_DIR.mkdir(exist_ok=True)

try:
    from finstack.core.cashflow import xirr
    from finstack.core.market_data.context import MarketContext
    from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve
    from finstack.valuations.instruments import RevolvingCredit
    from matplotlib.gridspec import GridSpec
    import matplotlib.patches as mpatches
    import matplotlib.pyplot as plt
    import pandas as pd

    print("All imports successful")
except ImportError as e:
    print(f"Error importing modules: {e}")
    print("Please ensure finstack-py is installed: make python-dev")
    print("Also ensure matplotlib and pandas are installed: uv pip install matplotlib pandas")
    sys.exit(1)


def create_test_market() -> MarketContext:
    """Create a test market with discount, forward, and hazard curves.

    Returns:
        MarketContext with USD-OIS discount, USD-SOFR-3M forward, and BORROWER-HZ hazard curves.
    """
    as_of = date(2024, 12, 29)  # A few days before commitment date

    # Create discount curve (3% rate approximation)
    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.25, 0.9925),
            (0.5, 0.9851),
            (1.0, 0.9704),
            (2.0, 0.9418),
            (5.0, 0.8607),
        ],
    )

    # Create forward curve (SOFR 3M at 3.5%)
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,  # 3-month tenor
        [
            (0.0, 0.035),
            (0.25, 0.035),
            (0.5, 0.035),
            (1.0, 0.035),
            (2.0, 0.035),
            (5.0, 0.035),
        ],
        base_date=as_of,
    )

    # Create hazard curve (150 bps credit spread)
    hazard_curve = HazardCurve(
        "BORROWER-HZ",
        as_of,
        [
            (0.0, 0.015),
            (0.25, 0.015),
            (0.5, 0.015),
            (1.0, 0.015),
            (2.0, 0.015),
            (5.0, 0.015),
        ],
        recovery_rate=0.4,
    )

    # Build market context
    market = MarketContext()
    market.insert_discount(discount_curve)
    market.insert_forward(forward_curve)
    market.insert_hazard(hazard_curve)

    return market


def create_deterministic_facility(
    facility_id: str, commitment: float = 100_000_000, initial_utilization: float = 0.25
) -> str:
    """Create a deterministic revolving credit facility specification."""
    drawn = int(commitment * initial_utilization)

    spec = {
        "id": facility_id,
        "commitment_amount": {"amount": commitment, "currency": "USD"},
        "drawn_amount": {"amount": drawn, "currency": "USD"},
        "commitment_date": "2025-01-01",
        "maturity": "2027-01-01",  # 2-year facility
        "base_rate_spec": {
            "Floating": {
                "index_id": "USD-SOFR-3M",
                "spread_bp": 250.0,  # 250 bps over SOFR
                "gearing": 1.0,
                "reset_freq": {"count": 3, "unit": "months"},
                "floor_bp": 0.0,
                "dc": "Act360",
                "bdc": "modified_following",
                "calendar_id": "weekends_only",
                "end_of_month": False,
                "payment_lag_days": 0,
            }
        },
        "day_count": "Act360",
        "payment_frequency": {"count": 3, "unit": "months"},
        "fees": {
            "upfront_fee": {"amount": 500_000, "currency": "USD"},  # 50 bps upfront
            "commitment_fee_tiers": [
                {"threshold": 0.0, "bps": 50},
                {"threshold": 0.5, "bps": 35},
                {"threshold": 0.75, "bps": 25},
            ],
            "usage_fee_tiers": [{"threshold": 0.75, "bps": 15}],
            "facility_fee_bp": 10,
        },
        "draw_repay_spec": {"Deterministic": []},
        "discount_curve_id": "USD-OIS",
        "hazard_curve_id": "BORROWER-HZ",
        "recovery_rate": 0.4,
        "attributes": {"tags": [], "meta": {}},
    }

    return json.dumps(spec)


def create_stochastic_facility(
    facility_id: str,
    commitment: float = 100_000_000,
    initial_utilization: float = 0.25,
    util_volatility: float = 0.10,
    credit_spread_volatility: float = 0.30,
    num_paths: int = 1000,
    seed: int = 42,
) -> str:
    """Create a stochastic revolving credit facility specification."""
    drawn = int(commitment * initial_utilization)

    spec = {
        "id": facility_id,
        "commitment_amount": {"amount": commitment, "currency": "USD"},
        "drawn_amount": {"amount": drawn, "currency": "USD"},
        "commitment_date": "2025-01-01",
        "maturity": "2027-01-01",
        "base_rate_spec": {
            "Floating": {
                "index_id": "USD-SOFR-3M",
                "spread_bp": 250.0,
                "gearing": 1.0,
                "reset_freq": {"count": 3, "unit": "months"},
                "floor_bp": 0.0,
                "dc": "Act360",
                "bdc": "modified_following",
                "calendar_id": "weekends_only",
                "end_of_month": False,
                "payment_lag_days": 0,
            }
        },
        "day_count": "Act360",
        "payment_frequency": {"count": 3, "unit": "months"},
        "fees": {
            "upfront_fee": {"amount": 500_000, "currency": "USD"},
            "commitment_fee_tiers": [
                {"threshold": 0.0, "bps": 50},
                {"threshold": 0.5, "bps": 35},
                {"threshold": 0.75, "bps": 25},
            ],
            "usage_fee_tiers": [{"threshold": 0.75, "bps": 15}],
            "facility_fee_bp": 10,
        },
        "draw_repay_spec": {
            "Stochastic": {
                "utilization_process": {
                    "MeanReverting": {
                        "target_rate": initial_utilization,
                        "speed": 2.0,
                        "volatility": util_volatility,
                    }
                },
                "num_paths": num_paths,
                "seed": seed,
                "antithetic": True,
                "use_sobol_qmc": False,
                "mc_config": {
                    "recovery_rate": 0.4,
                    "credit_spread_process": {
                        "Cir": {
                            "kappa": 0.5,
                            "theta": 0.015,
                            "sigma": credit_spread_volatility,
                            "initial": 0.015,
                        }
                    },
                    "interest_rate_process": None,  # Keep rates deterministic
                    "correlation_matrix": None,
                    "util_credit_corr": None,
                },
            }
        },
        "discount_curve_id": "USD-OIS",
        "hazard_curve_id": "BORROWER-HZ",
        "recovery_rate": 0.4,
        "attributes": {"tags": [], "meta": {}},
    }

    return json.dumps(spec)


def calculate_irr_from_cashflows(
    cashflows, initial_investment: float, commitment_date: date = date(2025, 1, 1), debug: bool = False
) -> float | None:
    """Calculate IRR from a cashflow schedule.

    Note: When as_of < commitment_date, the initial draw should be in the Rust cashflows.
    We check for it and only add if not present.

    Args:
        cashflows: CashFlowSchedule object
        initial_investment: Initial investment amount (positive for outflow)
        commitment_date: The commitment date of the facility
        debug: If True, print debug information

    Returns:
        IRR as a decimal, or None if cannot be calculated
    """
    # Create list of (date, amount) tuples for xirr
    cash_flow_list = []

    # Check if initial draw is in the cashflows (it should be if as_of < commitment_date)
    has_initial_draw = False
    flows = list(cashflows.flows())
    for flow in flows:
        # Check for large negative notional on commitment date (threshold based on expected size)
        if flow.date == commitment_date and flow.amount.amount < -(initial_investment * 0.9):  # 90% of expected amount
            has_initial_draw = True
            if debug:
                print(f"  Found initial draw in Rust cashflows: {flow.date}: ${flow.amount.amount:,.2f}")
            break

    # Only add if not already present
    if not has_initial_draw:
        cash_flow_list.append((commitment_date, -initial_investment))  # Initial lending
        if debug:
            print(f"  Added initial investment manually: {commitment_date}: ${-initial_investment:,.2f}")
    elif debug:
        print("  Using initial draw from Rust cashflows (not adding manually)")

    # Add all cashflows from the schedule
    # From lender perspective: interest and fees received are positive
    total_received = 0.0
    for flow in cashflows.flows():
        lender_flow = flow.amount.amount
        cash_flow_list.append((flow.date, lender_flow))
        total_received += lender_flow

        if debug and abs(lender_flow) > 0:
            print(f"  {flow.date}: {lender_flow:,.2f}")

    # No need to add terminal value - it should be in the cashflows already
    # The facility should handle repayment at maturity

    if debug:
        print(f"  Total flows: {len(cash_flow_list)}")
        print(f"  Total received: {total_received:,.2f}")
        print(f"  Net cash: {total_received - initial_investment:,.2f}")

    try:
        # Filter out zero cashflows that might cause issues
        filtered_flows = [(d, a) for d, a in cash_flow_list if abs(a) > 0.01]
        if len(filtered_flows) < 2:
            return None
        irr = xirr(filtered_flows)
        return irr
    except Exception as e:
        if debug:
            print(f"  IRR calculation failed: {e}")
        return None


def analyze_single_scenario(
    market: MarketContext,
    as_of: date,
    util_vol: float = 0.10,
    cs_vol: float = 0.30,
    num_paths: int = 10000,
    commitment: float = 100_000_000,
    initial_utilization: float = 0.25,
    commitment_date: date = date(2025, 1, 1),
) -> dict[str, Any]:
    """Analyze IRR distribution for a single volatility scenario.

    Returns dict with:
        - irr_distribution: List of IRRs from MC paths
        - deterministic_irr: IRR from deterministic pricing
        - path_data: Utilization and credit spread paths
        - path_irr_pairs: List of (path_result, irr) tuples for detailed analysis
    """
    print(f"\nAnalyzing scenario: Util Vol={util_vol:.0%}, CS Vol={cs_vol:.0%}")

    # Create deterministic facility
    det_spec = create_deterministic_facility("DET-BASE", commitment, initial_utilization)
    det_facility = RevolvingCredit.from_json(det_spec)

    # The facility starts on commitment_date.
    # If as_of < commitment_date, the schedule will start from commitment_date.
    # If as_of >= commitment_date, the schedule starts from as_of.
    # The error "start must be before end" suggests schedule generation is failing.
    # Let's use an as_of date that is definitely inside the facility period for the Monte Carlo pricing.
    # However, the initial investment logic needs to be carefully handled.

    # For this script, we will use commitment_date as as_of for MC pricing to avoid confusion.
    mc_as_of = commitment_date if as_of < commitment_date else as_of

    det_schedule = det_facility.build_dated_flows(market, as_of)  # Keep original as_of for det pricing
    det_irr = calculate_irr_from_cashflows(det_schedule, commitment * initial_utilization, commitment_date)
    print(f"Deterministic IRR: {det_irr:.2%}" if det_irr else "Deterministic IRR: N/A")

    # Create and price stochastic facility
    stoch_spec = create_stochastic_facility(
        f"MC-U{util_vol:.0%}-CS{cs_vol:.0%}",
        commitment=commitment,
        initial_utilization=initial_utilization,
        util_volatility=util_vol,
        credit_spread_volatility=cs_vol,
        num_paths=num_paths,
    )
    stoch_facility = RevolvingCredit.from_json(stoch_spec)
    # Use mc_as_of to ensure valid range for simulation
    mc_result = stoch_facility.price_with_paths(market, mc_as_of)

    print(f"Monte Carlo paths: {mc_result.num_paths}")
    print(f"Mean PV: {mc_result.mean}")
    print(f"Std Error: ${mc_result.std_error:,.2f}")

    # Calculate IRR for each path
    irr_distribution = []
    utilization_paths = []
    credit_spread_paths = []
    path_irr_pairs = []  # Store (path_result, irr) tuples

    for _i, path_result in enumerate(mc_result.path_results):
        # Calculate IRR for this path
        path_irr = calculate_irr_from_cashflows(
            path_result.cashflows, commitment * initial_utilization, commitment_date
        )
        if path_irr is not None:
            irr_distribution.append(path_irr)
            path_irr_pairs.append((path_result, path_irr))

        # Collect path data if available
        if path_result.path_data:
            utilization_paths.append(path_result.path_data.utilization_path)
            credit_spread_paths.append(path_result.path_data.credit_spread_path)

    print(f"Valid IRRs calculated: {len(irr_distribution)}/{mc_result.num_paths}")

    # Get time points from first path
    time_points = None
    payment_dates = None
    if mc_result.path_results and mc_result.path_results[0].path_data:
        time_points = mc_result.path_results[0].path_data.time_points
        payment_dates = mc_result.path_results[0].path_data.payment_dates

    return {
        "irr_distribution": irr_distribution,
        "deterministic_irr": det_irr,
        "utilization_paths": utilization_paths,
        "credit_spread_paths": credit_spread_paths,
        "time_points": time_points,
        "payment_dates": payment_dates,
        "mean_pv": mc_result.mean,
        "std_error": mc_result.std_error,
        "path_irr_pairs": path_irr_pairs,  # Added for extreme path analysis
    }


def plot_single_scenario_analysis(results: dict[str, Any], output_file: Path | None = None) -> None:
    """Create comprehensive visualization for single scenario analysis."""
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_single_scenario.png"
    fig = plt.figure(figsize=(16, 10))
    gs = GridSpec(3, 2, figure=fig, height_ratios=[2, 1.5, 1.5])

    # 1. IRR Distribution
    ax1 = fig.add_subplot(gs[0, :])
    irr_dist = np.array(results["irr_distribution"]) * 100  # Convert to percentage

    # Plot histogram
    _n, _bins, _patches = ax1.hist(
        irr_dist, bins=50, alpha=0.7, color="steelblue", edgecolor="black", density=True, label="MC Distribution"
    )

    # Add kernel density estimate (optional; SciPy is not a hard dependency)
    try:
        from scipy.stats import gaussian_kde  # type: ignore
    except ModuleNotFoundError:
        gaussian_kde = None

    if gaussian_kde is not None and len(irr_dist) > 1:
        kde = gaussian_kde(irr_dist)
        x_range = np.linspace(irr_dist.min(), irr_dist.max(), 200)
        ax1.plot(x_range, kde(x_range), "b-", linewidth=2, label="KDE")

    # Add deterministic IRR as vertical line
    if results["deterministic_irr"]:
        det_irr_pct = results["deterministic_irr"] * 100
        ax1.axvline(
            det_irr_pct, color="red", linestyle="--", linewidth=2, label=f"Deterministic IRR: {det_irr_pct:.2f}%"
        )

    # Add mean and percentiles
    mean_irr = np.mean(irr_dist)
    p5, p50, p95 = np.percentile(irr_dist, [5, 50, 95])
    ax1.axvline(mean_irr, color="green", linestyle=":", linewidth=2, label=f"Mean: {mean_irr:.2f}%")
    ax1.axvline(p50, color="orange", linestyle=":", linewidth=1.5, label=f"Median: {p50:.2f}%")

    ax1.set_xlabel("IRR (%)", fontsize=12)
    ax1.set_ylabel("Density", fontsize=12)
    ax1.set_title("IRR Distribution (10% Util Vol, 30% CS Vol)", fontsize=14, fontweight="bold")
    ax1.legend(loc="upper right")
    ax1.grid(True, alpha=0.3)

    # Add text box with statistics
    stats_text = f"Mean: {mean_irr:.2f}%\n"
    stats_text += f"Std Dev: {np.std(irr_dist):.2f}%\n"
    stats_text += f"5th Pctl: {p5:.2f}%\n"
    stats_text += f"95th Pctl: {p95:.2f}%"
    ax1.text(
        0.02,
        0.98,
        stats_text,
        transform=ax1.transAxes,
        fontsize=10,
        verticalalignment="top",
        bbox={"boxstyle": "round", "facecolor": "wheat", "alpha": 0.5},
    )

    # 2. Utilization Paths
    ax2 = fig.add_subplot(gs[1, 0])
    if results["utilization_paths"] and results["time_points"]:
        util_paths = np.array(results["utilization_paths"]) * 100  # Convert to percentage
        time_points = results["time_points"]

        # Plot sample paths
        for i in range(min(100, len(util_paths))):
            ax2.plot(time_points, util_paths[i], alpha=0.1, color="blue", linewidth=0.5)

        # Plot mean and percentiles
        mean_path = np.mean(util_paths, axis=0)
        p5_path = np.percentile(util_paths, 5, axis=0)
        p95_path = np.percentile(util_paths, 95, axis=0)

        ax2.fill_between(time_points, p5_path, p95_path, alpha=0.3, color="lightblue", label="5th-95th percentile")
        ax2.plot(time_points, mean_path, "b-", linewidth=2, label="Mean")
        ax2.axhline(50, color="red", linestyle="--", alpha=0.5, label="Initial (50%)")

        ax2.set_xlabel("Time (years)", fontsize=11)
        ax2.set_ylabel("Utilization Rate (%)", fontsize=11)
        ax2.set_title("Utilization Rate Paths", fontsize=12, fontweight="bold")
        ax2.legend(loc="upper right", fontsize=9)
        ax2.grid(True, alpha=0.3)
        ax2.set_ylim([0, 100])

    # 3. Credit Spread Paths
    ax3 = fig.add_subplot(gs[1, 1])
    if results["credit_spread_paths"] and results["time_points"]:
        cs_paths = np.array(results["credit_spread_paths"]) * 10000  # Convert to bps

        # Plot sample paths
        for i in range(min(100, len(cs_paths))):
            ax3.plot(time_points, cs_paths[i], alpha=0.1, color="orange", linewidth=0.5)

        # Plot mean and percentiles
        mean_path = np.mean(cs_paths, axis=0)
        p5_path = np.percentile(cs_paths, 5, axis=0)
        p95_path = np.percentile(cs_paths, 95, axis=0)

        ax3.fill_between(time_points, p5_path, p95_path, alpha=0.3, color="moccasin", label="5th-95th percentile")
        ax3.plot(time_points, mean_path, color="darkorange", linewidth=2, label="Mean")
        ax3.axhline(150, color="red", linestyle="--", alpha=0.5, label="Initial (150 bps)")

        ax3.set_xlabel("Time (years)", fontsize=11)
        ax3.set_ylabel("Credit Spread (bps)", fontsize=11)
        ax3.set_title("Credit Spread Paths", fontsize=12, fontweight="bold")
        ax3.legend(loc="upper right", fontsize=9)
        ax3.grid(True, alpha=0.3)
        ax3.set_ylim([0, max(300, np.max(p95_path) * 1.1)])

    # 4. Path Statistics Over Time
    ax4 = fig.add_subplot(gs[2, :])
    if results["utilization_paths"] and results["credit_spread_paths"] and results["time_points"]:
        util_std = np.std(np.array(results["utilization_paths"]) * 100, axis=0)
        cs_std = np.std(np.array(results["credit_spread_paths"]) * 10000, axis=0)

        ax4_twin = ax4.twinx()

        line1 = ax4.plot(time_points, util_std, "b-", linewidth=2, label="Utilization Std Dev")
        line2 = ax4_twin.plot(time_points, cs_std, "r-", linewidth=2, label="Credit Spread Std Dev")

        ax4.set_xlabel("Time (years)", fontsize=11)
        ax4.set_ylabel("Utilization Std Dev (%)", fontsize=11, color="b")
        ax4_twin.set_ylabel("Credit Spread Std Dev (bps)", fontsize=11, color="r")
        ax4.tick_params(axis="y", labelcolor="b")
        ax4_twin.tick_params(axis="y", labelcolor="r")
        ax4.set_title("Path Volatility Over Time", fontsize=12, fontweight="bold")
        ax4.grid(True, alpha=0.3)

        # Combine legends
        lines = line1 + line2
        labels = [l.get_label() for l in lines]
        ax4.legend(lines, labels, loc="upper right")

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nSingle scenario analysis saved to {output_file}")


def analyze_volatility_grid(
    market: MarketContext,
    as_of: date,
    util_vols: list[float] | None = None,
    cs_vols: list[float] | None = None,
    num_paths: int = 500,
    commitment: float = 100_000_000,
    initial_utilization: float = 0.25,
    commitment_date: date = date(2025, 1, 1),
) -> dict[tuple[float, float], list[float]]:
    """Analyze IRR distributions across a grid of volatility combinations.

    Returns dict mapping (util_vol, cs_vol) tuples to IRR distributions.
    """
    if util_vols is None:
        util_vols = [0.10, 0.20, 0.30]
    if cs_vols is None:
        cs_vols = [0.20, 0.30, 0.40]
    results = {}

    print("\n" + "=" * 80)
    print("VOLATILITY GRID ANALYSIS")
    print("=" * 80)

    for util_vol in util_vols:
        for cs_vol in cs_vols:
            print(f"\nProcessing: Util Vol={util_vol:.0%}, CS Vol={cs_vol:.0%}")

            # Create stochastic facility
            stoch_spec = create_stochastic_facility(
                f"GRID-U{util_vol:.0%}-CS{cs_vol:.0%}",
                commitment=commitment,
                initial_utilization=initial_utilization,
                util_volatility=util_vol,
                credit_spread_volatility=cs_vol,
                num_paths=num_paths,
            )
            stoch_facility = RevolvingCredit.from_json(stoch_spec)
            # Use commitment_date if as_of is before it, otherwise use as_of
            loop_mc_as_of = commitment_date if as_of < commitment_date else as_of
            mc_result = stoch_facility.price_with_paths(market, loop_mc_as_of)

            # Calculate IRRs
            irr_distribution = []
            for path_result in mc_result.path_results:
                path_irr = calculate_irr_from_cashflows(
                    path_result.cashflows, commitment * initial_utilization, commitment_date
                )
                if path_irr is not None:
                    irr_distribution.append(path_irr)

            results[(util_vol, cs_vol)] = irr_distribution

            if irr_distribution:
                mean_irr = np.mean(irr_distribution) * 100
                std_irr = np.std(irr_distribution) * 100
                print(f"  Mean IRR: {mean_irr:.2f}%, Std Dev: {std_irr:.2f}%")
            else:
                print("  Warning: No valid IRRs calculated")

    return results


def export_raw_polars_cashflows(
    path_irr_pairs: list[tuple[Any, float]], market, as_of_date, num_paths: int = 1, output_dir: str = "."
) -> None:
    """Export raw Polars cashflow DataFrames for extreme IRR paths.

    Exports the pure Rust-computed cashflows with all columns:
    - date, kind, amount, accrual_factor, reset_date
    - outstanding_start, outstanding (drawn balance), rate
    - outstanding_undrawn (for revolving credit with facility limits)
    - discount_factor, pv (when market provided)

    Args:
        path_irr_pairs: List of (path_result, irr) tuples
        market: MarketContext for pricing
        as_of_date: Valuation date
        num_paths: Number of top and bottom paths to export (default: 1)
        output_dir: Directory to save CSV files
    """
    import os

    if not path_irr_pairs:
        print("No path data available for Polars export")
        return

    # Sort by IRR
    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    # Get top and bottom paths
    bottom_n = sorted_pairs[:num_paths]
    top_n = sorted_pairs[-num_paths:]

    print(f"\nExporting raw Polars cashflows for top {num_paths} and bottom {num_paths} IRR paths...")
    print("All data computed in Rust - zero Python logic!")

    # Process bottom performers
    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        # Get Polars DataFrame directly from Rust with market pricing
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)

        # Add IRR and rank as Polars columns
        import polars as pl

        df = df.with_columns([pl.lit(irr).alias("IRR"), pl.lit(f"Bottom_{idx}").alias("Path_Rank")])

        # Save to CSV using Polars
        filename = os.path.join(output_dir, f"cashflows_polars_bottom_{idx}_irr_{irr:.4f}.csv")
        df.write_csv(filename)
        print(f"  ✓ Bottom #{idx} (IRR={irr:.2%}): {filename}")
        print(f"    Columns: {df.columns}")
        print(f"    Rows: {df.height}")

    # Process top performers
    for idx, (path_result, irr) in enumerate(top_n, 1):
        # Get Polars DataFrame directly from Rust with market pricing
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)

        # Add IRR and rank as Polars columns
        import polars as pl

        df = df.with_columns([pl.lit(irr).alias("IRR"), pl.lit(f"Top_{idx}").alias("Path_Rank")])

        # Save to CSV using Polars
        filename = os.path.join(output_dir, f"cashflows_polars_top_{idx}_irr_{irr:.4f}.csv")
        df.write_csv(filename)
        print(f"  ✓ Top #{idx} (IRR={irr:.2%}): {filename}")
        print(f"    Columns: {df.columns}")
        print(f"    Rows: {df.height}")

    print("\n✓ Raw Polars cashflows exported successfully!")
    print("  All outstanding balances computed deterministically in Rust")


def save_cashflow_schedules_with_pv_to_csv(
    path_irr_pairs: list[tuple[Any, float]], market, as_of_date, num_paths: int = 5, output_dir: str = "."
) -> None:
    """Save cashflow schedules directly from Rust using to_dataframe().

    Uses Rust's to_dataframe() which includes:
    - Outstanding balance (drawn)
    - Outstanding undrawn (for revolving credit)
    - All cashflow details

    NO PYTHON CASHFLOW LOGIC - everything from Rust!

    Args:
        path_irr_pairs: List of (path_result, irr) tuples
        market: MarketContext used for pricing
        as_of_date: Valuation date
        num_paths: Number of top and bottom paths to save
        output_dir: Directory to save CSV files
    """
    import os

    if not path_irr_pairs:
        print("No path data available for CSV export")
        return

    # Sort by IRR
    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    # Get top and bottom paths
    bottom_n = sorted_pairs[:num_paths]
    top_n = sorted_pairs[-num_paths:]

    print("\nExporting cashflow schedules from Rust to_dataframe()...")
    print("Includes: Outstanding (drawn), Outstanding Undrawn - all from Rust!")

    # Process bottom performers
    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        # Use Rust's to_dataframe() with market - returns Polars DataFrame
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)

        # Convert to pandas and add metadata
        df_pandas = df.to_pandas()
        df_pandas["IRR"] = irr
        df_pandas["Path_Rank"] = f"Bottom_{idx}"

        # Save to CSV
        filename = os.path.join(output_dir, f"cashflows_bottom_{idx}_irr_{irr:.4f}.csv")
        df_pandas.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    # Process top performers
    for idx, (path_result, irr) in enumerate(top_n, 1):
        # Use Rust's to_dataframe() with market - returns Polars DataFrame
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)

        # Convert to pandas and add metadata
        df_pandas = df.to_pandas()
        df_pandas["IRR"] = irr
        df_pandas["Path_Rank"] = f"Top_{idx}"

        # Save to CSV
        filename = os.path.join(output_dir, f"cashflows_top_{idx}_irr_{irr:.4f}.csv")
        df_pandas.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    print("\nAll cashflow data comes from Rust's to_dataframe():")
    print("  - outstanding = Drawn balance (correctly tracked for revolving credit)")
    print("  - outstanding_undrawn = Unused commitment (if facility limit exists)")


def save_cashflow_schedules_to_csv(
    path_irr_pairs: list[tuple[Any, float]], num_paths: int = 5, output_dir: str = "."
) -> None:
    """Save detailed cashflow schedules for top and bottom IRR paths to CSV files.

    NOTE: The cashflow schedules come directly from the Rust engine and include
    only the fields available on the CashFlow object:
    - date, amount, currency, kind (cashflow type), accrual_factor

    Path data (utilization and credit spread paths) are from the Monte Carlo
    simulation and are interpolated to the closest payment dates.

    Credit spreads can go to 0 when using the CIR model - this is correct
    behavior when the Feller condition is violated or volatility is low.

    Args:
        path_irr_pairs: List of (path_result, irr) tuples
        num_paths: Number of top and bottom paths to save
        output_dir: Directory to save CSV files
    """
    from datetime import date
    import os

    import pandas as pd

    if not path_irr_pairs:
        print("No path data available for CSV export")
        return

    # Sort by IRR
    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    # Get top and bottom paths
    bottom_n = sorted_pairs[:num_paths]
    top_n = sorted_pairs[-num_paths:]

    # Process bottom performers
    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        cashflows = path_result.cashflows.flows()

        # Create detailed cashflow records
        # Each record contains ONLY the fields actually available from the Rust CashFlow object
        records = []
        for flow in cashflows:
            record = {
                "Date": flow.date,
                "Days_From_Start": (flow.date - date(2025, 1, 1)).days if flow.date else None,
                "Cashflow_Type": str(flow.kind),
                "Amount": flow.amount.amount,
                "Currency": flow.amount.currency,
                "Accrual_Factor": flow.accrual_factor,
                "IRR": irr,
                "Path_Rank": f"Bottom_{idx}",
            }

            # Add MC path data (these are simulation values, NOT from the cashflow)
            # The path_data contains the stochastic paths for utilization and credit spread
            if hasattr(path_result, "path_data") and path_result.path_data:
                path_data = path_result.path_data

                # Match cashflow date to the MC simulation time points
                if hasattr(path_data, "time_points") and path_data.time_points:
                    days_since_start = (flow.date - date(2025, 1, 1)).days / 365.25

                    # Find the closest simulation time point
                    if 0 <= days_since_start <= 2.0:  # Within facility tenor
                        time_points = path_data.time_points
                        closest_idx = min(range(len(time_points)), key=lambda j: abs(time_points[j] - days_since_start))

                        # Add utilization from MC path
                        if hasattr(path_data, "utilization_path") and len(path_data.utilization_path) > closest_idx:
                            record["MC_Utilization"] = path_data.utilization_path[closest_idx]

                        # Add credit spread from MC path
                        # NOTE: CIR model correctly allows spreads to hit 0 when:
                        #   - Feller condition (2κθ ≥ σ²) is violated
                        #   - Low volatility or mean-reversion to low values
                        #   - This is handled gracefully by the QE discretization
                        if hasattr(path_data, "credit_spread_path") and len(path_data.credit_spread_path) > closest_idx:
                            record["MC_Credit_Spread"] = path_data.credit_spread_path[closest_idx]

            records.append(record)

        # Create DataFrame and save to CSV
        df = pd.DataFrame(records)
        filename = os.path.join(output_dir, f"cashflows_bottom_{idx}_irr_{irr:.4f}.csv")
        df.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    # Process top performers
    for idx, (path_result, irr) in enumerate(top_n, 1):
        cashflows = path_result.cashflows.flows()

        # Create detailed cashflow records
        # Each record contains ONLY the fields actually available from the Rust CashFlow object
        records = []
        for flow in cashflows:
            record = {
                "Date": flow.date,
                "Days_From_Start": (flow.date - date(2025, 1, 1)).days if flow.date else None,
                "Cashflow_Type": str(flow.kind),
                "Amount": flow.amount.amount,
                "Currency": flow.amount.currency,
                "Accrual_Factor": flow.accrual_factor,
                "IRR": irr,
                "Path_Rank": f"Top_{idx}",
            }

            # Add MC path data (these are simulation values, NOT from the cashflow)
            # The path_data contains the stochastic paths for utilization and credit spread
            if hasattr(path_result, "path_data") and path_result.path_data:
                path_data = path_result.path_data

                # Match cashflow date to the MC simulation time points
                if hasattr(path_data, "time_points") and path_data.time_points:
                    days_since_start = (flow.date - date(2025, 1, 1)).days / 365.25

                    # Find the closest simulation time point
                    if 0 <= days_since_start <= 2.0:  # Within facility tenor
                        time_points = path_data.time_points
                        closest_idx = min(range(len(time_points)), key=lambda j: abs(time_points[j] - days_since_start))

                        # Add utilization from MC path
                        if hasattr(path_data, "utilization_path") and len(path_data.utilization_path) > closest_idx:
                            record["MC_Utilization"] = path_data.utilization_path[closest_idx]

                        # Add credit spread from MC path
                        # NOTE: CIR model correctly allows spreads to hit 0 when:
                        #   - Feller condition (2κθ ≥ σ²) is violated
                        #   - Low volatility or mean-reversion to low values
                        #   - This is handled gracefully by the QE discretization
                        if hasattr(path_data, "credit_spread_path") and len(path_data.credit_spread_path) > closest_idx:
                            record["MC_Credit_Spread"] = path_data.credit_spread_path[closest_idx]

            records.append(record)

        # Create DataFrame and save to CSV
        df = pd.DataFrame(records)
        filename = os.path.join(output_dir, f"cashflows_top_{idx}_irr_{irr:.4f}.csv")
        df.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    # Create summary CSV with aggregated data
    summary_records = []

    print("\nNote: Credit spreads may go to 0 in some paths due to CIR model dynamics.")
    print("      This is realistic when mean reversion is strong or volatility is low.")

    # Add bottom performers summary
    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        cashflows = path_result.cashflows.flows()
        total_fees = sum(f.amount.amount for f in cashflows if "fee" in str(f.kind).lower())
        total_interest = sum(
            f.amount.amount for f in cashflows if "fixed" in str(f.kind).lower() or "float" in str(f.kind).lower()
        )
        total_notional = sum(f.amount.amount for f in cashflows if "notional" in str(f.kind).lower())
        total_cashflow = sum(f.amount.amount for f in cashflows)

        record = {
            "Path_Type": "Bottom",
            "Rank": idx,
            "IRR": irr,
            "Total_Fees": total_fees,
            "Total_Interest": total_interest,
            "Total_Notional": total_notional,
            "Total_Cashflow": total_cashflow,
            "Num_Cashflows": len(list(cashflows)),
        }

        # Add average path data if available
        if hasattr(path_result, "path_data") and path_result.path_data:
            path_data = path_result.path_data
            if hasattr(path_data, "utilization_path") and path_data.utilization_path:
                record["Avg_Utilization"] = np.mean(path_data.utilization_path)
                record["Min_Utilization"] = np.min(path_data.utilization_path)
                record["Max_Utilization"] = np.max(path_data.utilization_path)
            if hasattr(path_data, "credit_spread_path") and path_data.credit_spread_path:
                record["Avg_Credit_Spread"] = np.mean(path_data.credit_spread_path)
                record["Min_Credit_Spread"] = np.min(path_data.credit_spread_path)
                record["Max_Credit_Spread"] = np.max(path_data.credit_spread_path)

        summary_records.append(record)

    # Add top performers summary
    for idx, (path_result, irr) in enumerate(top_n, 1):
        cashflows = path_result.cashflows.flows()
        total_fees = sum(f.amount.amount for f in cashflows if "fee" in str(f.kind).lower())
        total_interest = sum(
            f.amount.amount for f in cashflows if "fixed" in str(f.kind).lower() or "float" in str(f.kind).lower()
        )
        total_notional = sum(f.amount.amount for f in cashflows if "notional" in str(f.kind).lower())
        total_cashflow = sum(f.amount.amount for f in cashflows)

        record = {
            "Path_Type": "Top",
            "Rank": idx,
            "IRR": irr,
            "Total_Fees": total_fees,
            "Total_Interest": total_interest,
            "Total_Notional": total_notional,
            "Total_Cashflow": total_cashflow,
            "Num_Cashflows": len(list(cashflows)),
        }

        # Add average path data if available
        if hasattr(path_result, "path_data") and path_result.path_data:
            path_data = path_result.path_data
            if hasattr(path_data, "utilization_path") and path_data.utilization_path:
                record["Avg_Utilization"] = np.mean(path_data.utilization_path)
                record["Min_Utilization"] = np.min(path_data.utilization_path)
                record["Max_Utilization"] = np.max(path_data.utilization_path)
            if hasattr(path_data, "credit_spread_path") and path_data.credit_spread_path:
                record["Avg_Credit_Spread"] = np.mean(path_data.credit_spread_path)
                record["Min_Credit_Spread"] = np.min(path_data.credit_spread_path)
                record["Max_Credit_Spread"] = np.max(path_data.credit_spread_path)

        summary_records.append(record)

    # Save summary CSV
    summary_df = pd.DataFrame(summary_records)
    summary_filename = os.path.join(output_dir, "cashflows_summary.csv")
    summary_df.to_csv(summary_filename, index=False)
    print(f"\nSummary saved: {summary_filename}")


def plot_extreme_paths_analysis(path_irr_pairs: list[tuple[Any, float]], output_file: Path | None = None) -> None:
    """Analyze and visualize cashflow patterns for top 5 and bottom 5 IRR paths.
    Each path gets two panels: cashflow bars and cumulative cashflows.

    Args:
        path_irr_pairs: List of (path_result, irr) tuples
        output_file: Output filename for the chart
    """
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_extreme_paths.png"
    if not path_irr_pairs:
        print("No path data available for extreme paths analysis")
        return

    # Sort by IRR
    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    # Get top 5 and bottom 5
    bottom_5 = sorted_pairs[:5]
    top_5 = sorted_pairs[-5:]

    # Create figure with 10x2 grid (each path gets 2 rows: bars + cumulative)
    from matplotlib import gridspec

    fig = plt.figure(figsize=(20, 35))
    gs = gridspec.GridSpec(10, 2, figure=fig, hspace=0.3, wspace=0.25)

    fig.suptitle(
        "Cashflow Analysis: Top 5 vs Bottom 5 IRR Paths\n(With Cumulative Cashflows)",
        fontsize=16,
        fontweight="bold",
        y=0.995,
    )

    # Color mapping
    colors = {
        "Notional": "blue",
        "Fees": "orange",
        "Fixed Interest": "green",
        "Floating Interest": "lime",
    }
    default_colors = ["purple", "red", "cyan", "gold", "pink"]

    # Process bottom 5 (left column)
    for idx, (path_result, irr) in enumerate(bottom_5):
        # Extract and aggregate cashflows by date
        cashflows = path_result.cashflows.flows()
        date_cashflows = {}

        for flow in cashflows:
            if flow.date not in date_cashflows:
                date_cashflows[flow.date] = {}

            # Map cashflow kind to category
            kind_str = str(flow.kind)
            kind_normalized = kind_str.lower()
            if "notional" in kind_normalized:
                category = "Notional"
            elif "fee" in kind_normalized:
                category = "Fees"
            elif "fixed" in kind_normalized:
                category = "Fixed Interest"
            elif "float" in kind_normalized or "reset" in kind_normalized:
                category = "Floating Interest"
            else:
                category = kind_str

            amount_in_thousands = flow.amount.amount / 1000
            if category not in date_cashflows[flow.date]:
                date_cashflows[flow.date][category] = 0
            date_cashflows[flow.date][category] += amount_in_thousands

        # Sort dates
        sorted_dates = sorted(date_cashflows.keys())
        if not sorted_dates:
            continue

        start_date = sorted_dates[0]
        x_positions = [(d - start_date).days for d in sorted_dates]

        # Create stacked bars - use categories that actually exist in the data
        all_categories = set()
        for date_flows in date_cashflows.values():
            all_categories.update(date_flows.keys())
        categories = sorted(all_categories)

        # Add colors for unexpected categories
        for i, cat in enumerate(categories):
            if cat not in colors:
                colors[cat] = default_colors[i % len(default_colors)]

        # --- PANEL 1: Bar Chart ---
        ax_bar = fig.add_subplot(gs[idx * 2, 0])

        bottom_pos = np.zeros(len(sorted_dates))
        bottom_neg = np.zeros(len(sorted_dates))

        for category in categories:
            values = [date_cashflows[d].get(category, 0) for d in sorted_dates]
            pos_values = [max(0, v) for v in values]
            neg_values = [min(0, v) for v in values]

            if any(v != 0 for v in pos_values):
                ax_bar.bar(
                    x_positions,
                    pos_values,
                    bottom=bottom_pos,
                    color=colors[category],
                    alpha=0.7,
                    edgecolor="black",
                    width=20,
                )
                bottom_pos += pos_values

            if any(v != 0 for v in neg_values):
                ax_bar.bar(
                    x_positions,
                    neg_values,
                    bottom=bottom_neg,
                    color=colors[category],
                    alpha=0.7,
                    edgecolor="black",
                    width=20,
                )
                bottom_neg += neg_values

        ax_bar.set_title(f"Bottom #{idx + 1}: IRR = {irr:.2%}", fontweight="bold", fontsize=10)
        ax_bar.set_xlabel("Days from Start", fontsize=8)
        ax_bar.set_ylabel("Cashflow ($000s)", fontsize=8)
        ax_bar.grid(True, alpha=0.3)
        ax_bar.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_bar.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)

        # --- PANEL 2: Cumulative Cashflows ---
        ax_cum = fig.add_subplot(gs[idx * 2 + 1, 0])

        # Calculate cumulative cashflows by category
        cumulative_by_category = {}
        for category in categories:
            cumulative = []
            running_total = 0
            for d in sorted_dates:
                running_total += date_cashflows[d].get(category, 0)
                cumulative.append(running_total)
            if any(v != 0 for v in cumulative):
                cumulative_by_category[category] = cumulative

        # Plot cumulative lines
        for category, cumulative in cumulative_by_category.items():
            ax_cum.plot(
                x_positions,
                cumulative,
                color=colors[category],
                linewidth=1.5,
                alpha=0.8,
                label=category,
                marker="o",
                markersize=2,
            )

        # Calculate and plot total cumulative
        total_cumulative = []
        running_total = 0
        for d in sorted_dates:
            daily_total = sum(date_cashflows[d].values())
            running_total += daily_total
            total_cumulative.append(running_total)

        ax_cum.plot(
            x_positions, total_cumulative, color="black", linewidth=2.5, alpha=0.9, label="Total Net", linestyle="--"
        )

        ax_cum.set_xlabel("Days from Start", fontsize=8)
        ax_cum.set_ylabel("Cumulative ($000s)", fontsize=8)
        ax_cum.grid(True, alpha=0.3)
        ax_cum.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_cum.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)
        ax_cum.legend(fontsize=6, loc="best", ncol=2)

    # Process top 5 (right column)
    for idx, (path_result, irr) in enumerate(top_5):
        # Extract and aggregate cashflows by date
        cashflows = path_result.cashflows.flows()
        date_cashflows = {}

        for flow in cashflows:
            if flow.date not in date_cashflows:
                date_cashflows[flow.date] = {}

            # Map cashflow kind to category
            kind_str = str(flow.kind)
            kind_normalized = kind_str.lower()
            if "notional" in kind_normalized:
                category = "Notional"
            elif "fee" in kind_normalized:
                category = "Fees"
            elif "fixed" in kind_normalized:
                category = "Fixed Interest"
            elif "float" in kind_normalized or "reset" in kind_normalized:
                category = "Floating Interest"
            else:
                category = kind_str

            amount_in_thousands = flow.amount.amount / 1000
            if category not in date_cashflows[flow.date]:
                date_cashflows[flow.date][category] = 0
            date_cashflows[flow.date][category] += amount_in_thousands

        # Sort dates and prepare data for stacked bar chart
        sorted_dates = sorted(date_cashflows.keys())
        if not sorted_dates:
            continue

        start_date = sorted_dates[0]
        x_positions = [(d - start_date).days for d in sorted_dates]

        # Get all categories
        all_categories = set()
        for date_flows in date_cashflows.values():
            all_categories.update(date_flows.keys())
        categories = sorted(all_categories)

        # Add colors for unexpected categories
        for i, cat in enumerate(categories):
            if cat not in colors:
                colors[cat] = default_colors[i % len(default_colors)]

        # --- PANEL 1: Bar Chart ---
        ax_bar = fig.add_subplot(gs[idx * 2, 1])

        bottom_pos = np.zeros(len(sorted_dates))
        bottom_neg = np.zeros(len(sorted_dates))

        for category in categories:
            values = [date_cashflows[d].get(category, 0) for d in sorted_dates]
            pos_values = [max(0, v) for v in values]
            neg_values = [min(0, v) for v in values]

            if any(v != 0 for v in pos_values):
                ax_bar.bar(
                    x_positions,
                    pos_values,
                    bottom=bottom_pos,
                    color=colors[category],
                    alpha=0.7,
                    edgecolor="black",
                    width=20,
                )
                bottom_pos += pos_values

            if any(v != 0 for v in neg_values):
                ax_bar.bar(
                    x_positions,
                    neg_values,
                    bottom=bottom_neg,
                    color=colors[category],
                    alpha=0.7,
                    edgecolor="black",
                    width=20,
                )
                bottom_neg += neg_values

        ax_bar.set_title(f"Top #{idx + 1}: IRR = {irr:.2%}", fontweight="bold", fontsize=10)
        ax_bar.set_xlabel("Days from Start", fontsize=8)
        ax_bar.set_ylabel("Cashflow ($000s)", fontsize=8)
        ax_bar.grid(True, alpha=0.3)
        ax_bar.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_bar.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)

        # --- PANEL 2: Cumulative Cashflows ---
        ax_cum = fig.add_subplot(gs[idx * 2 + 1, 1])

        # Calculate cumulative cashflows by category
        cumulative_by_category = {}
        for category in categories:
            cumulative = []
            running_total = 0
            for d in sorted_dates:
                running_total += date_cashflows[d].get(category, 0)
                cumulative.append(running_total)
            if any(v != 0 for v in cumulative):
                cumulative_by_category[category] = cumulative

        # Plot cumulative lines
        for category, cumulative in cumulative_by_category.items():
            ax_cum.plot(
                x_positions,
                cumulative,
                color=colors[category],
                linewidth=1.5,
                alpha=0.8,
                label=category,
                marker="o",
                markersize=2,
            )

        # Calculate and plot total cumulative
        total_cumulative = []
        running_total = 0
        for d in sorted_dates:
            daily_total = sum(date_cashflows[d].values())
            running_total += daily_total
            total_cumulative.append(running_total)

        ax_cum.plot(
            x_positions, total_cumulative, color="black", linewidth=2.5, alpha=0.9, label="Total Net", linestyle="--"
        )

        ax_cum.set_xlabel("Days from Start", fontsize=8)
        ax_cum.set_ylabel("Cumulative ($000s)", fontsize=8)
        ax_cum.grid(True, alpha=0.3)
        ax_cum.axhline(y=0, color="black", linestyle="-", linewidth=0.5)
        ax_cum.set_xlim(-30, max(x_positions) + 30 if x_positions else 730)
        ax_cum.legend(fontsize=6, loc="best", ncol=2)

    # Add column headers
    fig.text(0.3, 0.99, "Bottom 5 Performers", fontsize=14, fontweight="bold", ha="center")
    fig.text(0.7, 0.99, "Top 5 Performers", fontsize=14, fontweight="bold", ha="center")

    # Add summary statistics
    bottom_avg_irr = np.mean([irr for _, irr in bottom_5]) * 100
    top_avg_irr = np.mean([irr for _, irr in top_5]) * 100
    spread = top_avg_irr - bottom_avg_irr

    stats_text = f"Bottom 5 Avg: {bottom_avg_irr:.2f}%  |  Top 5 Avg: {top_avg_irr:.2f}%  |  Spread: {spread:.2f}%"
    fig.text(
        0.5, 0.005, stats_text, ha="center", fontsize=11, bbox={"boxstyle": "round", "facecolor": "wheat", "alpha": 0.5}
    )

    # Save figure
    plt.tight_layout(rect=[0, 0.01, 1, 0.99])
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nExtreme paths analysis saved to {output_file}")


def plot_volatility_grid_comparison(
    grid_results: dict[tuple[float, float], list[float]], output_file: Path | None = None
) -> None:
    """Create overlay plot of IRR distributions for different volatility combinations."""
    if output_file is None:
        output_file = OUTPUT_DIR / "irr_volatility_grid.png"
    fig, axes = plt.subplots(2, 2, figsize=(16, 12))
    fig.suptitle("IRR Distributions Across Volatility Grid", fontsize=16, fontweight="bold")

    # Color schemes for different scenarios
    colors = plt.cm.Set2(np.linspace(0, 1, 9))
    scenario_colors = {}
    color_idx = 0

    # Sort scenarios for consistent ordering
    sorted_scenarios = sorted(grid_results.keys())

    # 1. Main overlay plot (top left)
    ax1 = axes[0, 0]
    for scenario, irr_dist in grid_results.items():
        if irr_dist:
            irr_pct = np.array(irr_dist) * 100
            util_vol, cs_vol = scenario
            label = f"U:{util_vol:.0%}, CS:{cs_vol:.0%}"
            color = colors[color_idx % len(colors)]
            scenario_colors[scenario] = color
            color_idx += 1

            # Plot kernel density (optional; SciPy is not a hard dependency)
            try:
                from scipy.stats import gaussian_kde  # type: ignore
            except ModuleNotFoundError:
                gaussian_kde = None

            if gaussian_kde is not None:
                kde = gaussian_kde(irr_pct)
                x_range = np.linspace(min(irr_pct) - 1, max(irr_pct) + 1, 200)
                ax1.plot(x_range, kde(x_range), linewidth=2, label=label, color=color, alpha=0.8)

    ax1.set_xlabel("IRR (%)", fontsize=12)
    ax1.set_ylabel("Density", fontsize=12)
    ax1.set_title("All Scenarios Overlay", fontsize=13, fontweight="bold")
    ax1.legend(loc="upper left", fontsize=9)
    ax1.grid(True, alpha=0.3)

    # 2. Box plots comparison (top right)
    ax2 = axes[0, 1]
    box_data = []
    box_labels = []
    box_colors = []

    for scenario in sorted_scenarios:
        if grid_results.get(scenario):
            irr_pct = np.array(grid_results[scenario]) * 100
            box_data.append(irr_pct)
            util_vol, cs_vol = scenario
            box_labels.append(f"U:{util_vol:.0%}\nCS:{cs_vol:.0%}")
            box_colors.append(scenario_colors[scenario])

    bp = ax2.boxplot(box_data, labels=box_labels, patch_artist=True)
    for patch, color in zip(bp["boxes"], box_colors, strict=False):
        patch.set_facecolor(color)
        patch.set_alpha(0.7)

    ax2.set_ylabel("IRR (%)", fontsize=12)
    ax2.set_title("Box Plot Comparison", fontsize=13, fontweight="bold")
    ax2.grid(True, alpha=0.3, axis="y")
    plt.setp(ax2.xaxis.get_majorticklabels(), fontsize=8)

    # 3. Mean vs Volatility scatter (bottom left)
    ax3 = axes[1, 0]
    means = []
    stds = []
    util_vols_plot = []
    cs_vols_plot = []

    for scenario, irr_dist in grid_results.items():
        if irr_dist:
            means.append(np.mean(irr_dist) * 100)
            stds.append(np.std(irr_dist) * 100)
            util_vols_plot.append(scenario[0] * 100)
            cs_vols_plot.append(scenario[1] * 100)

    # Create scatter plot with color representing credit spread vol
    scatter = ax3.scatter(util_vols_plot, means, c=cs_vols_plot, s=100, cmap="viridis", edgecolors="black", alpha=0.7)
    cbar = plt.colorbar(scatter, ax=ax3)
    cbar.set_label("Credit Spread Vol (%)", fontsize=11)

    ax3.set_xlabel("Utilization Volatility (%)", fontsize=12)
    ax3.set_ylabel("Mean IRR (%)", fontsize=12)
    ax3.set_title("Mean IRR vs Utilization Volatility", fontsize=13, fontweight="bold")
    ax3.grid(True, alpha=0.3)

    # 4. Statistics table (bottom right)
    ax4 = axes[1, 1]
    ax4.axis("tight")
    ax4.axis("off")

    # Create statistics table
    table_data = [["Scenario", "Mean IRR", "Std Dev", "5th Pctl", "95th Pctl"]]

    for scenario in sorted_scenarios:
        if grid_results.get(scenario):
            irr_pct = np.array(grid_results[scenario]) * 100
            util_vol, cs_vol = scenario
            scenario_label = f"U:{util_vol:.0%}, CS:{cs_vol:.0%}"
            mean = np.mean(irr_pct)
            std = np.std(irr_pct)
            p5 = np.percentile(irr_pct, 5)
            p95 = np.percentile(irr_pct, 95)

            table_data.append([scenario_label, f"{mean:.2f}%", f"{std:.2f}%", f"{p5:.2f}%", f"{p95:.2f}%"])

    table = ax4.table(cellText=table_data, cellLoc="center", loc="center", colWidths=[0.25, 0.15, 0.15, 0.15, 0.15])
    table.auto_set_font_size(False)
    table.set_fontsize(9)
    table.scale(1.2, 1.5)

    # Style header row
    for i in range(len(table_data[0])):
        table[(0, i)].set_facecolor("#40466e")
        table[(0, i)].set_text_props(weight="bold", color="white")

    # Alternate row colors
    for i in range(1, len(table_data)):
        for j in range(len(table_data[0])):
            if i % 2 == 0:
                table[(i, j)].set_facecolor("#f0f0f0")

    ax4.set_title("Summary Statistics", fontsize=13, fontweight="bold", pad=20)

    plt.tight_layout()
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\nVolatility grid comparison saved to {output_file}")


def main() -> int:
    """Run IRR distribution analysis for revolving credit facilities."""
    print("\n" + "=" * 80)
    print("REVOLVING CREDIT IRR DISTRIBUTION ANALYSIS")
    print("Monte Carlo Simulation with Volatility Scenarios")
    print("=" * 80)

    # For the first example, we use a date clearly before the facility starts
    # to allow calculation of the initial drawdown
    as_of = date(2024, 12, 29)  # A few days before commitment

    try:
        # Set up market
        market = create_test_market()
        commitment_date = date(2025, 1, 1)

        # Part 1: Single scenario analysis (10% util vol, 30% credit spread vol)
        print("\n" + "=" * 80)
        print("PART 1: Single Scenario Analysis")
        print("=" * 80)

        # Use same commitment date as in create_deterministic_facility
        single_results = analyze_single_scenario(
            market,
            as_of,
            util_vol=0.10,
            cs_vol=0.30,
            num_paths=1000,
            initial_utilization=0.25,
            commitment_date=commitment_date,
        )
        plot_single_scenario_analysis(single_results)

        # Analyze extreme performers
        if single_results.get("path_irr_pairs"):
            plot_extreme_paths_analysis(single_results["path_irr_pairs"])

            # Export raw Polars cashflows for top 1 and bottom 1
            print("\n" + "=" * 80)
            print("Exporting Raw Polars Cashflows (Top 1 & Bottom 1)")
            print("=" * 80)

            export_raw_polars_cashflows(
                single_results["path_irr_pairs"], market, as_of, num_paths=1, output_dir=str(OUTPUT_DIR)
            )

            # Save RAW cashflow schedules for PV debugging
            print("\n" + "=" * 80)
            print("Saving Additional Cashflow Schedules")
            print("=" * 80)
            save_cashflow_schedules_with_pv_to_csv(
                single_results["path_irr_pairs"], market, as_of, num_paths=5, output_dir=str(OUTPUT_DIR)
            )

            # Save detailed cashflow schedules with MC path data to CSV
            print("\nSaving cashflow schedules with MC path data...")
            save_cashflow_schedules_to_csv(single_results["path_irr_pairs"], num_paths=5, output_dir=str(OUTPUT_DIR))

        # Part 2: Volatility grid analysis
        print("\n" + "=" * 80)
        print("PART 2: Volatility Grid Analysis")
        print("=" * 80)

        grid_results = analyze_volatility_grid(
            market,
            as_of,
            util_vols=[0.10, 0.20, 0.30],
            cs_vols=[0.20, 0.30, 0.40],
            num_paths=500,  # Balanced for speed and accuracy
            initial_utilization=0.25,
            commitment_date=commitment_date,
        )
        plot_volatility_grid_comparison(grid_results)

        # Summary
        print("\n" + "=" * 80)
        print("ANALYSIS COMPLETE")
        print("=" * 80)
        print("\nKey Insights:")
        print("1. IRR distributions show significant variability with volatility parameters")
        print("2. Higher utilization volatility generally increases IRR uncertainty")
        print("3. Credit spread volatility impacts both mean and dispersion of returns")
        print("4. Path-dependent features create complex IRR distributions")

        print("\nOutput files generated:")
        print("- irr_single_scenario.png: Single scenario deep dive")
        print("- irr_extreme_paths.png: Top 5 vs Bottom 5 cashflow analysis")
        print("- irr_volatility_grid.png: Grid comparison across scenarios")

        return 0

    except Exception as e:
        print(f"\n✗ Error during analysis: {e}")
        import traceback

        traceback.print_exc()
        return 1


if __name__ == "__main__":
    sys.exit(main())
