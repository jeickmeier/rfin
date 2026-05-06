"""Generate the QuantLib flat-curve Black CDS option benchmark."""

from __future__ import annotations

import QuantLib

NOTIONAL = 10_000_000
RECOVERY = 0.40
COUPON = 0.006
FLAT_RATE = 0.02
FLAT_HAZARD = 0.01
VOL = 0.50
BUMP_BP = 1e-4
VOL_BUMP = 0.01


def price(
    rate: float = FLAT_RATE,
    hazard: float = FLAT_HAZARD,
    vol: float = VOL,
) -> tuple[float, float, float]:
    """Return NPV, ATM forward spread, and risky annuity from QuantLib."""
    today = QuantLib.Date(5, QuantLib.January, 2026)
    QuantLib.Settings.instance().evaluationDate = today
    expiry = QuantLib.Date(20, QuantLib.March, 2026)

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
    schedule = QuantLib.Schedule(
        expiry,
        QuantLib.Date(20, QuantLib.December, 2030),
        QuantLib.Period(3, QuantLib.Months),
        QuantLib.WeekendsOnly(),
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
    option = QuantLib.CdsOption(cds, QuantLib.EuropeanExercise(expiry), True)
    option.setPricingEngine(
        QuantLib.BlackCdsOptionEngine(
            default_curve,
            RECOVERY,
            discount,
            QuantLib.QuoteHandle(QuantLib.SimpleQuote(vol)),
        )
    )
    return option.NPV(), option.atmRate() * 10_000.0, option.riskyAnnuity()


def main() -> None:
    """Print the QuantLib benchmark values for the CDS option fixture."""
    npv, atm_forward_bp, risky_annuity_notional = price()
    dv01 = (price(rate=FLAT_RATE + BUMP_BP)[0] - price(rate=FLAT_RATE - BUMP_BP)[0]) / 2.0
    cs01 = (price(hazard=FLAT_HAZARD + BUMP_BP)[0] - price(hazard=FLAT_HAZARD - BUMP_BP)[0]) / 2.0
    vega = (price(vol=VOL + VOL_BUMP)[0] - price(vol=VOL - VOL_BUMP)[0]) / 2.0
    print(f"npv = {npv:.12f}")
    print(f"atm_forward_bp = {atm_forward_bp:.12f}")
    print(f"risky_annuity_notional = {risky_annuity_notional:.12f}")
    print(f"risky_annuity_unit_notional = {risky_annuity_notional / NOTIONAL:.12f}")
    print(f"dv01_parallel_rate_bp = {dv01:.12f}")
    print(f"cs01_direct_hazard_bp = {cs01:.12f}")
    print(f"vega_1pct = {vega:.12f}")


if __name__ == "__main__":
    main()
