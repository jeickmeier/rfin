"""Single-scenario and volatility-grid IRR analysis."""

from datetime import date
from typing import Any

import numpy as np

from .setup import (
    RevolvingCredit,
    calculate_irr_from_cashflows,
    create_deterministic_facility,
    create_stochastic_facility,
)

try:
    from finstack.core.market_data.context import MarketContext
except ImportError:
    MarketContext = Any  # type: ignore[assignment,misc]


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

    mc_as_of = commitment_date if as_of < commitment_date else as_of

    det_schedule = det_facility.cashflow_schedule(market, as_of)
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
    mc_result = stoch_facility.price_with_paths(market, mc_as_of)

    print(f"Monte Carlo paths: {mc_result.num_paths}")
    print(f"Mean PV: {mc_result.mean}")
    print(f"Std Error: ${mc_result.std_error:,.2f}")

    # Calculate IRR for each path
    irr_distribution = []
    utilization_paths = []
    credit_spread_paths = []
    path_irr_pairs = []

    for _i, path_result in enumerate(mc_result.path_results):
        path_irr = calculate_irr_from_cashflows(
            path_result.cashflows, commitment * initial_utilization, commitment_date
        )
        if path_irr is not None:
            irr_distribution.append(path_irr)
            path_irr_pairs.append((path_result, path_irr))

        if path_result.path_data:
            utilization_paths.append(path_result.path_data.utilization_path)
            credit_spread_paths.append(path_result.path_data.credit_spread_path)

    print(f"Valid IRRs calculated: {len(irr_distribution)}/{mc_result.num_paths}")

    if not irr_distribution:
        print("  Warning: No valid IRRs could be calculated for this scenario.")
        return {
            "irr_distribution": [],
            "deterministic_irr": det_irr,
            "utilization_paths": utilization_paths,
            "credit_spread_paths": credit_spread_paths,
            "time_points": None,
            "payment_dates": None,
            "mean_pv": mc_result.mean,
            "std_error": mc_result.std_error,
            "path_irr_pairs": path_irr_pairs,
        }

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
        "path_irr_pairs": path_irr_pairs,
    }


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

            stoch_spec = create_stochastic_facility(
                f"GRID-U{util_vol:.0%}-CS{cs_vol:.0%}",
                commitment=commitment,
                initial_utilization=initial_utilization,
                util_volatility=util_vol,
                credit_spread_volatility=cs_vol,
                num_paths=num_paths,
            )
            stoch_facility = RevolvingCredit.from_json(stoch_spec)
            loop_mc_as_of = commitment_date if as_of < commitment_date else as_of
            mc_result = stoch_facility.price_with_paths(market, loop_mc_as_of)

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
