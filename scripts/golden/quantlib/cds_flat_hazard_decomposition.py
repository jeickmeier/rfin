"""Generate the QuantLib flat-hazard CDS decomposition benchmark.

This script is intentionally small and prints the values copied into
`cds_quantlib_flat_hazard_decomposition.json`.
"""

from __future__ import annotations

import QuantLib

NOTIONAL = 10_000_000
RECOVERY = 0.40
COUPON = 0.01
FLAT_RATE = 0.02
FLAT_HAZARD = 0.01
BUMP_BP = 1e-4


def build_cds(
    rate: float = FLAT_RATE,
    hazard: float = FLAT_HAZARD,
) -> QuantLib.CreditDefaultSwap:
    """Build the flat-curve QuantLib CDS used for decomposition goldens."""
    today = QuantLib.Date(5, QuantLib.January, 2026)
    QuantLib.Settings.instance().evaluationDate = today

    calendar = QuantLib.WeekendsOnly()
    schedule = QuantLib.Schedule(
        today,
        QuantLib.Date(20, QuantLib.December, 2030),
        QuantLib.Period(3, QuantLib.Months),
        calendar,
        QuantLib.Following,
        QuantLib.Unadjusted,
        QuantLib.DateGeneration.CDS,
        False,
    )
    cds = QuantLib.CreditDefaultSwap(
        QuantLib.Protection.Buyer,
        NOTIONAL,
        COUPON,
        schedule,
        QuantLib.Following,
        QuantLib.Actual360(True),
        True,
        True,
    )
    discount = QuantLib.YieldTermStructureHandle(
        QuantLib.FlatForward(
            today,
            rate,
            QuantLib.Actual365Fixed(),
            QuantLib.Continuous,
            QuantLib.Annual,
        )
    )
    default_curve = QuantLib.DefaultProbabilityTermStructureHandle(
        QuantLib.FlatHazardRate(
            today,
            QuantLib.QuoteHandle(QuantLib.SimpleQuote(hazard)),
            QuantLib.Actual365Fixed(),
        )
    )
    cds.setPricingEngine(
        QuantLib.IsdaCdsEngine(
            default_curve,
            RECOVERY,
            discount,
            False,
            QuantLib.IsdaCdsEngine.Taylor,
            QuantLib.IsdaCdsEngine.HalfDayBias,
            QuantLib.IsdaCdsEngine.Piecewise,
        )
    )
    return cds


def main() -> None:
    """Print the QuantLib benchmark values for the CDS decomposition fixture."""
    cds = build_cds()
    premium_leg_pv = abs(cds.couponLegNPV())
    print(f"npv = {cds.NPV():.12f}")
    print(f"par_spread = {cds.fairSpread() * 10_000:.12f}")
    print(f"protection_leg_pv = {cds.defaultLegNPV():.12f}")
    print(f"premium_leg_pv = {premium_leg_pv:.12f}")
    print(f"risky_annuity = {premium_leg_pv / (NOTIONAL * COUPON):.12f}")
    print(f"risky_pv01 = {premium_leg_pv / 100.0:.12f}")

    dv01 = (build_cds(FLAT_RATE + BUMP_BP).NPV() - build_cds(FLAT_RATE - BUMP_BP).NPV()) / 2.0
    direct_hazard_cs01 = (
        build_cds(hazard=FLAT_HAZARD + BUMP_BP).NPV() - build_cds(hazard=FLAT_HAZARD - BUMP_BP).NPV()
    ) / 2.0
    spread_equivalent_bump = BUMP_BP / (1.0 - RECOVERY)
    spread_equivalent_cs01 = (
        build_cds(hazard=FLAT_HAZARD + spread_equivalent_bump).NPV()
        - build_cds(hazard=FLAT_HAZARD - spread_equivalent_bump).NPV()
    ) / 2.0
    print(f"dv01_parallel_rate_bp = {dv01:.12f}")
    print(f"cs01_direct_hazard_bp = {direct_hazard_cs01:.12f}")
    print(f"cs01_spread_equivalent_bp = {spread_equivalent_cs01:.12f}")


if __name__ == "__main__":
    main()
