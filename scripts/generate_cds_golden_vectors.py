#!/usr/bin/env python3
"""Generate CDS golden vector fixtures from QuantLib reference implementation.

This script generates JSON fixtures for CDS pricing validation against
the ISDA CDS Standard Model via QuantLib.

Usage:
    uv run scripts/generate_cds_golden_vectors.py

The generated fixtures are saved to:
    finstack/valuations/tests/instruments/cds/golden/

Requirements:
    - QuantLib-Python (optional, falls back to analytical if not available)
"""

from dataclasses import dataclass
from datetime import date
import json
import math
from pathlib import Path
from typing import Any

# Try to import QuantLib, fall back to pure Python implementation if not available
try:
    import QuantLib as ql  # noqa: N813

    HAS_QUANTLIB = True
except ImportError:
    HAS_QUANTLIB = False
    print("Warning: QuantLib not available, using analytical approximations")


@dataclass
class CDSTestCase:
    """Test case parameters for CDS golden vector."""

    id: str
    as_of: date
    maturity: date
    notional: float
    spread_bp: float
    recovery_rate: float
    discount_rate: float  # Flat rate for simplicity
    hazard_rate: float  # Flat rate for simplicity


def create_flat_discount_curve(
    rate: float, as_of_date: "ql.Date", day_count: "ql.DayCounter"
) -> "ql.YieldTermStructureHandle":
    """Create a flat discount curve in QuantLib."""
    if not HAS_QUANTLIB:
        raise RuntimeError("QuantLib required for this function")

    flat_curve = ql.FlatForward(as_of_date, rate, day_count)
    return ql.YieldTermStructureHandle(flat_curve)


def create_flat_hazard_curve(
    hazard_rate: float, as_of_date: "ql.Date", day_count: "ql.DayCounter"
) -> "ql.DefaultProbabilityTermStructureHandle":
    """Create a flat hazard curve in QuantLib."""
    if not HAS_QUANTLIB:
        raise RuntimeError("QuantLib required for this function")

    flat_hazard = ql.FlatHazardRate(as_of_date, hazard_rate, day_count)
    return ql.DefaultProbabilityTermStructureHandle(flat_hazard)


def python_date_to_ql(d: date) -> "ql.Date":
    """Convert Python date to QuantLib date."""
    if not HAS_QUANTLIB:
        raise RuntimeError("QuantLib required for this function")
    return ql.Date(d.day, d.month, d.year)


def calculate_cds_metrics_quantlib(case: CDSTestCase) -> dict[str, float]:
    """Calculate CDS metrics using QuantLib."""
    if not HAS_QUANTLIB:
        raise RuntimeError("QuantLib not available")

    # Set evaluation date
    as_of_ql = python_date_to_ql(case.as_of)
    ql.Settings.instance().evaluationDate = as_of_ql

    # Day count convention (ISDA standard)
    day_count = ql.Actual360()

    # Create curves
    discount_handle = create_flat_discount_curve(case.discount_rate, as_of_ql, day_count)
    hazard_handle = create_flat_hazard_curve(case.hazard_rate, as_of_ql, day_count)

    # Create CDS schedule (quarterly, ISDA standard)
    maturity_ql = python_date_to_ql(case.maturity)
    schedule = ql.MakeSchedule(
        effectiveDate=as_of_ql,
        terminationDate=maturity_ql,
        tenor=ql.Period(3, ql.Months),
        calendar=ql.UnitedStates(ql.UnitedStates.GovernmentBond),
        convention=ql.Following,
        terminationDateConvention=ql.Following,
        rule=ql.DateGeneration.CDS,
    )

    # Create CDS instrument
    spread = case.spread_bp / 10000.0  # Convert to decimal
    cds = ql.CreditDefaultSwap(
        ql.Protection.Buyer,  # Protection buyer
        case.notional,
        spread,
        schedule,
        ql.Following,
        day_count,
    )

    # Create pricing engine
    engine = ql.MidPointCdsEngine(hazard_handle, case.recovery_rate, discount_handle)
    cds.setPricingEngine(engine)

    # Calculate metrics
    npv = cds.NPV()
    fair_spread = cds.fairSpread() * 10000  # Convert to bp

    return {
        "npv": npv,
        "par_spread_bp": fair_spread,
        "protection_leg_pv": cds.protectionLegNPV(),
        "premium_leg_pv": -cds.couponLegNPV(),  # QuantLib returns negative for buyer
    }


def calculate_cds_metrics_analytical(case: CDSTestCase) -> dict[str, float]:
    """Calculate CDS metrics using analytical approximations.

    Uses the "Credit Triangle" approximation and ISDA-style formulas.
    These are approximations - QuantLib values are more accurate.
    """
    r = case.discount_rate
    h = case.hazard_rate
    recovery = case.recovery_rate
    tenor = (case.maturity - case.as_of).days / 365.0

    # Loss given default
    lgd = 1.0 - recovery

    # Risky annuity (simplified, continuous approximation)
    risky_annuity_years = (1 - math.exp(-(r + h) * tenor)) / (r + h) if abs(r + h) > 1e-10 else tenor

    # Risky PV01 in currency units per bp
    risky_pv01 = risky_annuity_years * case.notional / 10000

    # Protection leg PV (expected loss)
    if abs(r + h) > 1e-10:
        protection_leg_pv = lgd * h / (r + h) * (1 - math.exp(-(r + h) * tenor)) * case.notional
    else:
        protection_leg_pv = lgd * h * tenor * case.notional

    # Par spread: spread where NPV = 0
    par_spread_bp = (protection_leg_pv / case.notional) / risky_annuity_years * 10000

    # Premium leg PV at stated spread
    spread_decimal = case.spread_bp / 10000.0
    premium_leg_pv = spread_decimal * risky_annuity_years * case.notional

    # NPV for protection buyer
    npv = protection_leg_pv - premium_leg_pv

    return {
        "npv": npv,
        "par_spread_bp": par_spread_bp,
        "risky_pv01": risky_pv01,
        "protection_leg_pv": protection_leg_pv,
        "premium_leg_pv": premium_leg_pv,
    }


def generate_golden_vector(case: CDSTestCase) -> dict[str, Any]:
    """Generate a complete golden vector JSON structure."""
    # Calculate metrics
    if HAS_QUANTLIB:
        try:
            metrics = calculate_cds_metrics_quantlib(case)
            source = f"QuantLib {ql.version()}"
        except Exception as e:
            print(f"Warning: QuantLib calculation failed for {case.id}: {e}")
            metrics = calculate_cds_metrics_analytical(case)
            source = "Analytical approximation (QuantLib unavailable)"
    else:
        metrics = calculate_cds_metrics_analytical(case)
        source = "Analytical approximation"

    return {
        "id": case.id,
        "source": source,
        "description": f"CDS {case.id} - {(case.maturity - case.as_of).days // 365}Y, "
        f"{case.spread_bp:.0f}bp spread, {case.recovery_rate * 100:.0f}% recovery",
        "contract": {
            "as_of": case.as_of.isoformat(),
            "start_date": case.as_of.isoformat(),
            "maturity_date": case.maturity.isoformat(),
            "notional": case.notional,
            "currency": "USD",
            "spread_bp": case.spread_bp,
            "recovery_rate": case.recovery_rate,
            "convention": "ISDA_NA",
            "side": "buy_protection",
        },
        "curves": {
            "discount": {"flat_rate": case.discount_rate},
            "hazard": {"flat_rate": case.hazard_rate},
        },
        "expected": {
            "par_spread_bp": {
                "value": round(metrics["par_spread_bp"], 4),
                "tolerance_bp": 0.5,
                "notes": f"Par spread from {source}",
            },
            "risky_pv01": {
                "value": round(metrics.get("risky_pv01", metrics["premium_leg_pv"] / case.spread_bp * 10000), 2),
                "tolerance_pct": 0.25,
                "notes": "Risky PV01 per bp in currency units",
            },
            "protection_leg_pv": {
                "value": round(metrics["protection_leg_pv"], 2),
                "tolerance_pct": 0.25,
                "notes": "LGD * probability-weighted default payments",
            },
            "premium_leg_pv": {
                "value": round(metrics["premium_leg_pv"], 2),
                "tolerance_pct": 0.25,
                "notes": f"Premium leg at {case.spread_bp:.0f}bp spread",
            },
            "npv": {
                "value": round(metrics["npv"], 2),
                "tolerance_abs": max(500.0, abs(metrics["npv"]) * 0.02),  # 2% or $500 min
                "notes": "NPV for protection buyer",
            },
        },
    }


def main():
    """Generate all golden vector fixtures.

    By default, only generates fixtures that don't already exist.
    Use --overwrite to regenerate all fixtures.
    """
    import sys

    overwrite = "--overwrite" in sys.argv

    # Define test cases
    test_cases = [
        CDSTestCase(
            id="isda_5y_flat_100bp",
            as_of=date(2024, 3, 20),
            maturity=date(2029, 3, 20),
            notional=10_000_000.0,
            spread_bp=60.0,  # At par for flat 1% hazard, 40% recovery
            recovery_rate=0.40,
            discount_rate=0.05,
            hazard_rate=0.01,
        ),
        CDSTestCase(
            id="isda_3y_150bp",
            as_of=date(2024, 6, 20),
            maturity=date(2027, 6, 20),
            notional=10_000_000.0,
            spread_bp=150.0,
            recovery_rate=0.35,
            discount_rate=0.06,
            hazard_rate=0.02,
        ),
        CDSTestCase(
            id="isda_5y_high_hazard",
            as_of=date(2024, 9, 20),
            maturity=date(2029, 9, 20),
            notional=10_000_000.0,
            spread_bp=300.0,
            recovery_rate=0.25,
            discount_rate=0.04,
            hazard_rate=0.04,
        ),
        CDSTestCase(
            id="isda_7y_investment_grade",
            as_of=date(2024, 12, 20),
            maturity=date(2031, 12, 20),
            notional=10_000_000.0,
            spread_bp=50.0,
            recovery_rate=0.40,
            discount_rate=0.03,
            hazard_rate=0.008,
        ),
    ]

    # Output directory
    output_dir = Path(__file__).parent.parent / "finstack" / "valuations" / "tests" / "instruments" / "cds" / "golden"
    output_dir.mkdir(parents=True, exist_ok=True)

    print(f"Generating CDS golden vectors to {output_dir}")
    print(f"QuantLib available: {HAS_QUANTLIB}")
    print()

    for case in test_cases:
        output_path = output_dir / f"{case.id}.json"

        if output_path.exists() and not overwrite:
            print(f"Skipped (exists): {output_path.name}")
            continue

        vector = generate_golden_vector(case)

        with open(output_path, "w") as f:
            json.dump(vector, f, indent=2)

        print(f"Generated: {output_path.name}")
        print(f"  Par spread: {vector['expected']['par_spread_bp']['value']:.4f} bp")
        print(f"  NPV: ${vector['expected']['npv']['value']:,.2f}")
        print()

    print("Done!")


if __name__ == "__main__":
    main()
