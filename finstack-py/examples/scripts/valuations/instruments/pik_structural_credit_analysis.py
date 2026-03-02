#!/usr/bin/env python3
"""PIK Structural Credit Analysis: Breakeven Spreads Across Issuer Credit Profiles.

Demonstrates the full Merton structural credit + Monte Carlo PIK pricing
pipeline.  Compares cash-pay, full-PIK, and PIK-toggle bonds for issuers
ranging from investment-grade to deeply stressed.

Barriers are calibrated from historical annual PDs via
``MertonModel.from_target_pd``.  The pricer registry is used for both:

- **Merton MC** pricing (``registry.price(bond, "merton_mc", market)``)
- **Hazard-rate** pricing (``registry.price(bond, "hazard_rate", market)``)

Usage:
    python pik_structural_credit_analysis.py
"""

from __future__ import annotations

import math
from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.valuations.instruments import (
    Bond,
    MertonModel,
    MertonAssetDynamics,
    MertonBarrierType,
    EndogenousHazardSpec,
    DynamicRecoverySpec,
    ToggleExerciseModel,
    MertonMcConfig,
)
from finstack.valuations.pricer import create_standard_registry

# ── Global parameters ──────────────────────────────────────────────────────

RISK_FREE_RATE = 0.045
COUPON_RATE = 0.085
MATURITY_YEARS = 5
NOTIONAL = 100.0
AS_OF = date(2025, 6, 15)
MATURITY_DATE = AS_OF + timedelta(days=int(MATURITY_YEARS * 365.25))
NUM_PATHS = 25_000
SEED = 42
REGISTRY = create_standard_registry()

# ── h0 calibration ────────────────────────────────────────────────────────


def calibrate_h0(target_spread: float, recovery: float) -> float:
    """Solve for the flat hazard rate whose reduced-form bond price
    matches the price implied by *target_spread* (Z-spread).

    Uses bisection over the survival-weighted PV formula:
      PV(h) = Σ cpn·D(t)·S(t) + N·D(T)·S(T) + R·N·Σ D(t)·[S(t-1)-S(t)]
    where D(t) = exp(-r·t) and S(t) = exp(-h·t).
    """
    n_periods = int(MATURITY_YEARS * 2)
    times = [i / 2.0 for i in range(1, n_periods + 1)]
    cpn = COUPON_RATE / 2 * NOTIONAL

    target = sum(cpn * math.exp(-(RISK_FREE_RATE + target_spread) * t) for t in times)
    target += NOTIONAL * math.exp(-(RISK_FREE_RATE + target_spread) * MATURITY_YEARS)

    def _hr_pv(h: float) -> float:
        pv = 0.0
        prev_s = 1.0
        for t in times:
            df = math.exp(-RISK_FREE_RATE * t)
            s = math.exp(-h * t)
            pv += cpn * df * s
            pv += recovery * NOTIONAL * df * (prev_s - s)
            prev_s = s
        pv += NOTIONAL * math.exp(-RISK_FREE_RATE * MATURITY_YEARS) * math.exp(-h * MATURITY_YEARS)
        return pv

    lo, hi = 0.0, 5.0
    for _ in range(200):
        mid = (lo + hi) / 2
        pv = _hr_pv(mid)
        if abs(pv - target) < 1e-6:
            return mid
        if pv > target:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


# ── Issuer profiles ───────────────────────────────────────────────────────
#
# Barriers are calibrated from historical annual PDs via from_target_pd.
# Base hazard (h0) is calibrated from observed market spreads via
# calibrate_h0, or set directly when USE_MARKET_SPREADS is False.

USE_MARKET_SPREADS = True

ISSUER_PROFILES: list[dict] = [
    {"name": "BB+ (Solid HY)",       "asset_value": 200.0, "asset_vol": 0.20,
     "annual_pd": 0.0020, "market_spread": 0.0085,  "base_recovery": 0.45},
    {"name": "BB- (Mid HY)",         "asset_value": 165.0, "asset_vol": 0.25,
     "annual_pd": 0.0100, "market_spread": 0.0210,  "base_recovery": 0.40},
    {"name": "B (Weak HY)",          "asset_value": 140.0, "asset_vol": 0.30,
     "annual_pd": 0.0250, "market_spread": 0.0390,  "base_recovery": 0.35},
    {"name": "B- (Stressed)",        "asset_value": 125.0, "asset_vol": 0.35,
     "annual_pd": 0.0550, "market_spread": 0.0630,  "base_recovery": 0.30},
    {"name": "CCC (Deeply Stressed)","asset_value": 115.0, "asset_vol": 0.40,
     "annual_pd": 0.1000, "market_spread": 0.1050,  "base_recovery": 0.25},
]

if USE_MARKET_SPREADS:
    for _p in ISSUER_PROFILES:
        _p["base_hazard"] = calibrate_h0(_p["market_spread"], _p["base_recovery"])
else:
    _DIRECT_H0 = [0.015, 0.035, 0.060, 0.090, 0.140]
    for _p, _h in zip(ISSUER_PROFILES, _DIRECT_H0):
        _p["base_hazard"] = _h

# ── Helpers ────────────────────────────────────────────────────────────────

def build_market() -> MarketContext:
    """Build a MarketContext with a flat discount curve."""
    market = MarketContext()
    market.insert_discount(DiscountCurve(
        "USD-OIS", AS_OF,
        [(t, math.exp(-RISK_FREE_RATE * t))
         for t in [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]],
    ))
    return market


def build_merton(profile: dict) -> MertonModel:
    """Calibrate Merton barrier from the issuer's historical annual PD."""
    five_year_pd = 1.0 - math.exp(-profile["annual_pd"] * MATURITY_YEARS)
    return MertonModel.from_target_pd(
        asset_value=profile["asset_value"],
        asset_vol=profile["asset_vol"],
        risk_free_rate=RISK_FREE_RATE,
        target_pd=five_year_pd,
        maturity=MATURITY_YEARS,
    )


def build_mc_config(
    merton: MertonModel,
    profile: dict,
    pik_schedule: str | list | None = None,
    toggle: ToggleExerciseModel | None = None,
) -> MertonMcConfig:
    """Build an MC config with endogenous hazard + dynamic recovery."""
    endo = EndogenousHazardSpec.power_law(
        base_hazard=profile["base_hazard"],
        base_leverage=merton.debt_barrier / profile["asset_value"],
        exponent=2.0,
    )
    dyn_rec = DynamicRecoverySpec.floored_inverse(
        base_recovery=profile["base_recovery"],
        base_notional=NOTIONAL,
        floor=0.10,
    )
    kwargs: dict = dict(
        merton=merton,
        pik_schedule=pik_schedule,
        endogenous_hazard=endo,
        dynamic_recovery=dyn_rec,
        num_paths=NUM_PATHS,
        seed=SEED,
        antithetic=True,
        time_steps_per_year=12,
    )
    if toggle is not None:
        kwargs["toggle_model"] = toggle
    return MertonMcConfig(**kwargs)


def build_bond(coupon_type: str, mc_config: MertonMcConfig) -> Bond:
    """Build a bond with the MC config attached for registry pricing."""
    return (
        Bond.builder(f"PIK-{coupon_type.upper()}")
        .money(Money(NOTIONAL, USD))
        .coupon_rate(COUPON_RATE)
        .coupon_type(coupon_type)
        .issue(AS_OF)
        .maturity(MATURITY_DATE)
        .frequency(2)
        .disc_id("USD-OIS")
        .credit_curve("CREDIT")
        .merton_mc(mc_config)
        .build()
    )


def build_plain_bond(coupon_type: str) -> Bond:
    """Build a bond without MC config (for hazard-rate pricing)."""
    return (
        Bond.builder(f"HR-{coupon_type.upper()}")
        .money(Money(NOTIONAL, USD))
        .coupon_rate(COUPON_RATE)
        .coupon_type(coupon_type)
        .issue(AS_OF)
        .maturity(MATURITY_DATE)
        .frequency(2)
        .disc_id("USD-OIS")
        .credit_curve("CREDIT")
        .build()
    )


def mc_price(bond: Bond, market: MarketContext) -> dict:
    """Price via the Merton MC pricer in the registry.

    The pricer computes cash-equivalent Z-spread and YTM internally,
    so result.measures already contains 'z_spread' and 'ytm' (decimal).
    """
    result = REGISTRY.price(bond, "merton_mc", market, as_of=AS_OF)
    pv = result.value.amount
    m = result.measures
    return {
        "price_pct": pv / NOTIONAL * 100.0,
        "z_spread_bp": m.get("z_spread", 0.0) * 10_000,
        "ytm_pct": m.get("ytm", 0.0) * 100.0,
        "expected_loss": m.get("expected_loss", 0.0),
        "default_rate": m.get("default_rate", 0.0),
        "pik_fraction": m.get("pik_fraction", 0.0),
        "avg_terminal_notional": m.get("avg_terminal_notional", NOTIONAL),
        "mc_standard_error": m.get("mc_standard_error", 0.0),
    }


def hr_price_bond(bond: Bond, hazard: float, recovery: float) -> float:
    """Price a bond using the library's hazard-rate engine."""
    market = build_market()
    market.insert_hazard(HazardCurve(
        "CREDIT", AS_OF, [(0.0, hazard), (10.0, hazard)],
        recovery_rate=recovery,
    ))
    return REGISTRY.price(bond, "hazard_rate", market, as_of=AS_OF).value.amount


def hr_find_implied_hazard(bond: Bond, target_pv: float, recovery: float) -> float:
    """Bisect for the flat hazard rate that reprices to target_pv."""
    lo, hi = 0.0, 5.0
    for _ in range(200):
        mid = (lo + hi) / 2.0
        pv = hr_price_bond(bond, mid, recovery)
        if abs(pv - target_pv) < 1e-6:
            return mid
        if pv > target_pv:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2.0


# ── Main ───────────────────────────────────────────────────────────────────

def main() -> None:
    market = build_market()
    toggle = ToggleExerciseModel.threshold("hazard_rate", 0.10, "above")
    all_results: dict[str, dict[str, dict]] = {}

    print("=" * 110)
    print("PIK Structural Credit Analysis: Breakeven Spreads by Issuer Credit Profile")
    print("=" * 110)
    print(f"Bond:     {MATURITY_YEARS}Y  {COUPON_RATE:.1%} semi-annual  |  "
          f"Risk-free: {RISK_FREE_RATE:.2%}  |  "
          f"MC paths: {NUM_PATHS:,}  |  As-of: {AS_OF}")
    print(f"h0 mode:  {'calibrated from market spreads' if USE_MARKET_SPREADS else 'direct specification'}")
    print()

    # ── Issuer profiles summary ────────────────────────────────────────
    print(f"  {'Issuer':<25s}  {'Assets':>6s}  {'Vol':>5s}  "
          f"{'Mkt Sprd':>8s}  {'h0 (cal)':>8s}  {'h≈s/(1-R)':>9s}  {'R0':>5s}")
    print("  " + "-" * 78)
    for profile in ISSUER_PROFILES:
        s = profile["market_spread"]
        approx = s / (1 - profile["base_recovery"])
        print(f"  {profile['name']:<25s}  {profile['asset_value']:>6.0f}  "
              f"{profile['asset_vol']:>5.0%}  {s * 1e4:>7.0f}bp  "
              f"{profile['base_hazard']:>8.2%}  {approx:>9.2%}  "
              f"{profile['base_recovery']:>5.0%}")
    print()

    for profile in ISSUER_PROFILES:
        merton = build_merton(profile)

        dd = merton.distance_to_default(MATURITY_YEARS)
        pd_val = merton.default_probability(MATURITY_YEARS)
        impl_spread = merton.implied_spread(MATURITY_YEARS, profile["base_recovery"])

        print(f"── {profile['name']} ──────────────────────────────────────────")
        print(f"  Assets: {profile['asset_value']:.0f}  |  "
              f"Barrier: {merton.debt_barrier:.1f}  |  "
              f"Vol: {profile['asset_vol']:.0%}  |  "
              f"Annual PD: {profile['annual_pd']:.2%}")
        print(f"  Merton DD: {dd:.2f}  |  "
              f"PD({MATURITY_YEARS}Y): {pd_val:.2%}  |  "
              f"Implied Spread: {impl_spread * 10_000:.0f} bp")
        print()

        base_cfg = build_mc_config(merton, profile)
        toggle_cfg = build_mc_config(merton, profile, pik_schedule="toggle", toggle=toggle)

        cash_bond = build_bond("cash", base_cfg)
        pik_bond = build_bond("pik", base_cfg)
        toggle_bond = build_bond("cash", toggle_cfg)

        cash_r = mc_price(cash_bond, market)
        pik_r = mc_price(pik_bond, market)
        toggle_r = mc_price(toggle_bond, market)
        all_results[profile["name"]] = {"Cash": cash_r, "PIK": pik_r, "Toggle": toggle_r}

        header = (
            f"  {'Structure':<15s}  "
            f"{'Price':>7s}  "
            f"{'Z-Sprd':>7s}  "
            f"{'E[Loss]':>7s}  "
            f"{'DefRate':>7s}  "
            f"{'PIK%':>6s}  "
            f"{'TermNtl':>8s}  "
            f"{'SE':>6s}"
        )
        print(header)
        print("  " + "-" * (len(header) - 2))
        for label, r in [("Cash-Pay", cash_r), ("Full PIK", pik_r), ("PIK Toggle", toggle_r)]:
            print(
                f"  {label:<15s}  "
                f"{r['price_pct']:7.2f}  "
                f"{r['z_spread_bp']:6.0f}bp  "
                f"{r['expected_loss']:7.2%}  "
                f"{r['default_rate']:7.2%}  "
                f"{r['pik_fraction']:5.1%}  "
                f"{r['avg_terminal_notional']:8.1f}  "
                f"{r['mc_standard_error']:6.4f}"
            )

        pik_prem = pik_r["z_spread_bp"] - cash_r["z_spread_bp"]
        tog_prem = toggle_r["z_spread_bp"] - cash_r["z_spread_bp"]
        print()
        print(f"  PIK Z-spread premium:     {pik_prem:+.0f} bp")
        print(f"  Toggle Z-spread premium:  {tog_prem:+.0f} bp")
        print()

    # ── Summary table ─────────────────────────────────────────────────
    print("=" * 110)
    print("SUMMARY: Z-Spread (bp) by Structure and Credit Quality")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'Cash':>8s}  {'Full PIK':>8s}  {'Toggle':>8s}  "
          f"{'PIK-Cash':>8s}  {'Tog-Cash':>8s}")
    print("  " + "-" * 80)

    for profile in ISSUER_PROFILES:
        r = all_results[profile["name"]]
        c = r["Cash"]["z_spread_bp"]
        p = r["PIK"]["z_spread_bp"]
        t = r["Toggle"]["z_spread_bp"]
        print(
            f"  {profile['name']:<25s}  "
            f"{c:7.0f}bp {p:7.0f}bp {t:7.0f}bp  "
            f"{p - c:+8.0f}  {t - c:+8.0f}"
        )

    print()
    print("Key: All values are CASH-EQUIVALENT Z-spreads: the Z-spread on a standard")
    print("     cash-pay bond that reproduces each structure's MC price.")
    print("     PIK-Cash = PIK premium in Z-spread terms (positive = PIK is riskier)")
    print("     Tog-Cash = toggle premium in Z-spread terms")
    print()

    # ── Toggle > PIK explanation ──────────────────────────────────────
    print("NOTE ON TOGGLE vs PIK ORDERING")
    print("-" * 60)
    print("  It is possible for toggle to trade CHEAPER than full PIK.")
    print("  This reflects adverse selection, not a model error:")
    print()
    print("  - Full PIK distributes accrual uniformly across ALL paths,")
    print("    including healthy ones where extra notional barely matters.")
    print("  - Toggle concentrates PIK on the WORST paths (high hazard")
    print("    triggers the toggle), seeding the leverage spiral where")
    print("    it does the most damage.")
    print("  - The feedback loop (PIK -> higher leverage -> higher hazard")
    print("    -> more PIK) is more intense on already-stressed paths.")
    print()
    print("  A practitioner would recognise this as the 'adverse selection")
    print("  cost' of the toggle option.")
    print()

    # ── Hazard-rate comparison ────────────────────────────────────────
    print("=" * 110)
    print("HAZARD-RATE PRICES: Cash-Pay vs PIK at Each Issuer's Flat Hazard Rate")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'λ (bp)':>7s}  "
          f"{'HR Cash':>9s}  {'HR PIK':>9s}  {'Δ Price':>8s}  "
          f"{'MC Cash':>9s}  {'MC PIK':>9s}  {'Δ Price':>8s}")
    print("  " + "-" * 100)

    cash_hr = build_plain_bond("cash")
    pik_hr = build_plain_bond("pik")

    for profile in ISSUER_PROFILES:
        lam, rec = profile["base_hazard"], profile["base_recovery"]

        hr_cash_px = hr_price_bond(cash_hr, lam, rec)
        hr_pik_px = hr_price_bond(pik_hr, lam, rec)
        hr_delta = hr_pik_px - hr_cash_px

        r = all_results[profile["name"]]
        mc_delta = r["PIK"]["price_pct"] - r["Cash"]["price_pct"]

        print(
            f"  {profile['name']:<25s}  "
            f"{lam * 10_000:7.0f}  "
            f"{hr_cash_px:9.2f}  "
            f"{hr_pik_px:9.2f}  "
            f"{hr_delta:+8.2f}  "
            f"{r['Cash']['price_pct']:9.2f}  "
            f"{r['PIK']['price_pct']:9.2f}  "
            f"{mc_delta:+8.2f}"
        )

    print()
    print("  Δ Price = PIK price minus Cash price (negative = PIK trades cheaper)")
    print("  HR captures timing + notional; MC adds endogenous feedback.")
    print()

    # ── Implied hazard rates ──────────────────────────────────────────
    print("=" * 110)
    print("IMPLIED HAZARD RATES: Flat λ Backing Out Each Merton MC Price")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'Base λ':>7s}  "
          f"{'λ Cash':>7s}  {'λ PIK':>7s}  {'Δλ':>7s}  "
          f"{'Z Cash':>7s}  {'Z PIK':>7s}  {'ΔZ':>7s}")
    print("  " + "-" * 90)

    for profile in ISSUER_PROFILES:
        rec = profile["base_recovery"]
        r = all_results[profile["name"]]

        cash_target = r["Cash"]["price_pct"] / 100 * NOTIONAL
        pik_target = r["PIK"]["price_pct"] / 100 * NOTIONAL

        lam_cash = hr_find_implied_hazard(cash_hr, cash_target, rec)
        lam_pik = hr_find_implied_hazard(pik_hr, pik_target, rec)
        delta_lam = lam_pik - lam_cash

        print(
            f"  {profile['name']:<25s}  "
            f"{profile['base_hazard'] * 10_000:7.0f}  "
            f"{lam_cash * 10_000:7.0f}  "
            f"{lam_pik * 10_000:7.0f}  "
            f"{delta_lam * 10_000:+7.0f}  "
            f"{r['Cash']['z_spread_bp']:7.0f}  "
            f"{r['PIK']['z_spread_bp']:7.0f}  "
            f"{r['PIK']['z_spread_bp'] - r['Cash']['z_spread_bp']:+7.0f}"
        )

    print()
    print("Key: Base λ  = issuer's input hazard rate (bp)")
    print("     λ Cash  = flat hazard rate reproducing the MC cash-pay price")
    print("     λ PIK   = flat hazard rate reproducing the MC full-PIK price")
    print("     Δλ      = PIK hazard premium (structural feedback cost)")
    print("     Z Cash / Z PIK = Z-spread from Merton MC pricing")
    print("     ΔZ      = PIK Z-spread premium")
    print()


if __name__ == "__main__":
    main()
