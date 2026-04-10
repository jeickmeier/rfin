"""Market setup, facility construction, and IRR calculation utilities."""

from datetime import date
import json
from pathlib import Path
import sys
from typing import Any

import numpy as np

OUTPUT_DIR = Path(__file__).parent.parent.parent.parent.parent.parent / "outputs"
OUTPUT_DIR.mkdir(exist_ok=True)

try:
    from finstack.core.cashflow import xirr
    from finstack.core.market_data.context import MarketContext
    from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve, HazardCurve
    from finstack.valuations.instruments import RevolvingCredit

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
    market.insert(discount_curve)
    market.insert(forward_curve)
    market.insert(hazard_curve)

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

    if not has_initial_draw:
        cash_flow_list.append((commitment_date, -initial_investment))  # Initial lending
        if debug:
            print(f"  Added initial investment manually: {commitment_date}: ${-initial_investment:,.2f}")
    elif debug:
        print("  Using initial draw from Rust cashflows (not adding manually)")

    # From lender perspective: interest and fees received are positive
    total_received = 0.0
    for flow in cashflows.flows():
        lender_flow = flow.amount.amount
        cash_flow_list.append((flow.date, lender_flow))
        total_received += lender_flow

        if debug and abs(lender_flow) > 0:
            print(f"  {flow.date}: {lender_flow:,.2f}")

    if debug:
        print(f"  Total flows: {len(cash_flow_list)}")
        print(f"  Total received: {total_received:,.2f}")
        print(f"  Net cash: {total_received - initial_investment:,.2f}")

    try:
        filtered_flows = [(d, a) for d, a in cash_flow_list if abs(a) > 0.01]
        if len(filtered_flows) < 2:
            return None
        irr = xirr(filtered_flows)
        return irr
    except Exception as e:
        if debug:
            print(f"  IRR calculation failed: {e}")
        return None
