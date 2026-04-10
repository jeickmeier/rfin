"""CSV export utilities for cashflow schedules and Polars DataFrames."""

from datetime import date
import os
from typing import Any

import numpy as np
import pandas as pd


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
    import polars as pl

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

    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)
        df = df.with_columns([pl.lit(irr).alias("IRR"), pl.lit(f"Bottom_{idx}").alias("Path_Rank")])
        filename = os.path.join(output_dir, f"cashflows_polars_bottom_{idx}_irr_{irr:.4f}.csv")
        df.write_csv(filename)
        print(f"  \u2713 Bottom #{idx} (IRR={irr:.2%}): {filename}")
        print(f"    Columns: {df.columns}")
        print(f"    Rows: {df.height}")

    for idx, (path_result, irr) in enumerate(top_n, 1):
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)
        df = df.with_columns([pl.lit(irr).alias("IRR"), pl.lit(f"Top_{idx}").alias("Path_Rank")])
        filename = os.path.join(output_dir, f"cashflows_polars_top_{idx}_irr_{irr:.4f}.csv")
        df.write_csv(filename)
        print(f"  \u2713 Top #{idx} (IRR={irr:.2%}): {filename}")
        print(f"    Columns: {df.columns}")
        print(f"    Rows: {df.height}")

    print("\n\u2713 Raw Polars cashflows exported successfully!")
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
    if not path_irr_pairs:
        print("No path data available for CSV export")
        return

    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    bottom_n = sorted_pairs[:num_paths]
    top_n = sorted_pairs[-num_paths:]

    print("\nExporting cashflow schedules from Rust to_dataframe()...")
    print("Includes: Outstanding (drawn), Outstanding Undrawn - all from Rust!")

    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)
        df_pandas = df.to_pandas()
        df_pandas["IRR"] = irr
        df_pandas["Path_Rank"] = f"Bottom_{idx}"
        filename = os.path.join(output_dir, f"cashflows_bottom_{idx}_irr_{irr:.4f}.csv")
        df_pandas.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    for idx, (path_result, irr) in enumerate(top_n, 1):
        df = path_result.cashflows.to_dataframe(market=market, discount_curve_id="USD-OIS", as_of=as_of_date)
        df_pandas = df.to_pandas()
        df_pandas["IRR"] = irr
        df_pandas["Path_Rank"] = f"Top_{idx}"
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
    if not path_irr_pairs:
        print("No path data available for CSV export")
        return

    sorted_pairs = sorted(path_irr_pairs, key=lambda x: x[1])

    bottom_n = sorted_pairs[:num_paths]
    top_n = sorted_pairs[-num_paths:]

    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        records = _build_cashflow_records(path_result, irr, f"Bottom_{idx}")
        df = pd.DataFrame(records)
        filename = os.path.join(output_dir, f"cashflows_bottom_{idx}_irr_{irr:.4f}.csv")
        df.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    for idx, (path_result, irr) in enumerate(top_n, 1):
        records = _build_cashflow_records(path_result, irr, f"Top_{idx}")
        df = pd.DataFrame(records)
        filename = os.path.join(output_dir, f"cashflows_top_{idx}_irr_{irr:.4f}.csv")
        df.to_csv(filename, index=False)
        print(f"  Saved: {filename}")

    # Create summary CSV with aggregated data
    summary_records = []

    print("\nNote: Credit spreads may go to 0 in some paths due to CIR model dynamics.")
    print("      This is realistic when mean reversion is strong or volatility is low.")

    for idx, (path_result, irr) in enumerate(bottom_n, 1):
        summary_records.append(_build_summary_record(path_result, irr, "Bottom", idx))

    for idx, (path_result, irr) in enumerate(top_n, 1):
        summary_records.append(_build_summary_record(path_result, irr, "Top", idx))

    summary_df = pd.DataFrame(summary_records)
    summary_filename = os.path.join(output_dir, "cashflows_summary.csv")
    summary_df.to_csv(summary_filename, index=False)
    print(f"\nSummary saved: {summary_filename}")


def _build_cashflow_records(path_result, irr: float, path_rank: str) -> list[dict]:
    """Build detailed cashflow records for a single path."""
    cashflows = path_result.cashflows.flows()
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
            "Path_Rank": path_rank,
        }

        if hasattr(path_result, "path_data") and path_result.path_data:
            path_data = path_result.path_data
            if hasattr(path_data, "time_points") and path_data.time_points:
                days_since_start = (flow.date - date(2025, 1, 1)).days / 365.25
                if 0 <= days_since_start <= 2.0:
                    time_points = path_data.time_points
                    closest_idx = min(range(len(time_points)), key=lambda j: abs(time_points[j] - days_since_start))
                    if hasattr(path_data, "utilization_path") and len(path_data.utilization_path) > closest_idx:
                        record["MC_Utilization"] = path_data.utilization_path[closest_idx]
                    if hasattr(path_data, "credit_spread_path") and len(path_data.credit_spread_path) > closest_idx:
                        record["MC_Credit_Spread"] = path_data.credit_spread_path[closest_idx]

        records.append(record)

    return records


def _build_summary_record(path_result, irr: float, path_type: str, rank: int) -> dict:
    """Build a summary record for a single path."""
    cashflows = path_result.cashflows.flows()
    total_fees = sum(f.amount.amount for f in cashflows if "fee" in str(f.kind).lower())
    total_interest = sum(
        f.amount.amount for f in cashflows if "fixed" in str(f.kind).lower() or "float" in str(f.kind).lower()
    )
    total_notional = sum(f.amount.amount for f in cashflows if "notional" in str(f.kind).lower())
    total_cashflow = sum(f.amount.amount for f in cashflows)

    record: dict[str, Any] = {
        "Path_Type": path_type,
        "Rank": rank,
        "IRR": irr,
        "Total_Fees": total_fees,
        "Total_Interest": total_interest,
        "Total_Notional": total_notional,
        "Total_Cashflow": total_cashflow,
        "Num_Cashflows": len(list(cashflows)),
    }

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

    return record
