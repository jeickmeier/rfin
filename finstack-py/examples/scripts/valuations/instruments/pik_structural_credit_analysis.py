#!/usr/bin/env python3
"""PIK Structural Credit Analysis: Breakeven Spreads Across Issuer Credit Profiles.

Demonstrates the full Merton structural credit + Monte Carlo PIK pricing
pipeline.  Compares cash-pay, full-PIK, and PIK-toggle bonds for issuers
ranging from investment-grade to deeply stressed.

Usage:
    python pik_structural_credit_analysis.py
"""

from __future__ import annotations

import math
from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.valuations.instruments import (
    Bond,
    BondBuilder,
    MertonModel,
    MertonAssetDynamics,
    MertonBarrierType,
    EndogenousHazardSpec,
    DynamicRecoverySpec,
    ToggleExerciseModel,
    MertonMcConfig,
    MertonMcResult,
)

# ── Global parameters ──────────────────────────────────────────────────────

RISK_FREE_RATE = 0.045  # 4.5% base rate
COUPON_RATE = 0.085  # 8.5% coupon (typical HY PIK)
MATURITY_YEARS = 5
NOTIONAL = 100.0
AS_OF = date(2025, 6, 15)
MATURITY_DATE = AS_OF + timedelta(days=int(MATURITY_YEARS * 365.25))
NUM_PATHS = 25_000
SEED = 42

# ── Issuer profiles ───────────────────────────────────────────────────────
#
# Each profile represents a firm at a different point on the credit spectrum.
# We specify asset value, asset vol, and debt barrier.  The debt barrier is
# normalised to 100 (matching our bond notional), so asset_value / 100
# gives the asset coverage ratio.

ISSUER_PROFILES: list[dict] = [
    {
        "name": "BB+ (Solid HY)",
        "asset_value": 200.0,  # 2.0x coverage
        "asset_vol": 0.20,
        "debt_barrier": 100.0,
        "base_hazard": 0.015,
        "base_recovery": 0.45,
    },
    {
        "name": "BB- (Mid HY)",
        "asset_value": 165.0,  # 1.65x coverage
        "asset_vol": 0.25,
        "debt_barrier": 100.0,
        "base_hazard": 0.035,
        "base_recovery": 0.40,
    },
    {
        "name": "B (Weak HY)",
        "asset_value": 140.0,  # 1.40x coverage
        "asset_vol": 0.30,
        "debt_barrier": 100.0,
        "base_hazard": 0.06,
        "base_recovery": 0.35,
    },
    {
        "name": "B- (Stressed)",
        "asset_value": 125.0,  # 1.25x coverage
        "asset_vol": 0.35,
        "debt_barrier": 100.0,
        "base_hazard": 0.09,
        "base_recovery": 0.30,
    },
    {
        "name": "CCC (Deeply Stressed)",
        "asset_value": 115.0,  # 1.15x coverage
        "asset_vol": 0.40,
        "debt_barrier": 100.0,
        "base_hazard": 0.14,
        "base_recovery": 0.25,
    },
]


# ── Helpers ────────────────────────────────────────────────────────────────

def build_bond() -> Bond:
    """Build a 5Y semi-annual fixed-rate bond."""
    return (
        Bond.builder("PIK-ANALYSIS")
        .money(Money(NOTIONAL, USD))
        .coupon_rate(COUPON_RATE)
        .issue(AS_OF)
        .maturity(MATURITY_DATE)
        .frequency(2)
        .disc_id("USD-OIS")
        .build()
    )


def build_merton(profile: dict) -> MertonModel:
    """Build a Merton model from an issuer profile."""
    return MertonModel(
        asset_value=profile["asset_value"],
        asset_vol=profile["asset_vol"],
        debt_barrier=profile["debt_barrier"],
        risk_free_rate=RISK_FREE_RATE,
        barrier_type=MertonBarrierType.first_passage(0.0),
        dynamics=MertonAssetDynamics.GEOMETRIC_BROWNIAN,
    )


def build_configs(
    merton: MertonModel,
    profile: dict,
) -> tuple[MertonMcConfig, MertonMcConfig, MertonMcConfig]:
    """Return (cash_config, pik_config, toggle_config) for one issuer."""
    # Endogenous hazard: hazard rate climbs as leverage grows
    endo = EndogenousHazardSpec.power_law(
        base_hazard=profile["base_hazard"],
        base_leverage=profile["debt_barrier"] / profile["asset_value"],
        exponent=2.0,
    )

    # Dynamic recovery: recovery falls as PIK accrual inflates notional
    dyn_rec = DynamicRecoverySpec.floored_inverse(
        base_recovery=profile["base_recovery"],
        base_notional=NOTIONAL,
        floor=0.10,
    )

    # Toggle model: borrower PIKs when hazard rate > 10%
    toggle = ToggleExerciseModel.threshold(
        variable="hazard_rate",
        threshold=0.10,
        direction="above",
    )

    base_kwargs = dict(
        num_paths=NUM_PATHS,
        seed=SEED,
        antithetic=True,
        time_steps_per_year=12,
    )

    # Cash-pay: no toggle, no endogenous feedback (plain MC)
    cash_config = MertonMcConfig(merton=merton, **base_kwargs)

    # Full PIK: endogenous hazard + dynamic recovery, no toggle
    pik_config = MertonMcConfig(
        merton=merton,
        endogenous_hazard=endo,
        dynamic_recovery=dyn_rec,
        **base_kwargs,
    )

    # PIK Toggle: all three extensions
    toggle_config = MertonMcConfig(
        merton=merton,
        endogenous_hazard=endo,
        dynamic_recovery=dyn_rec,
        toggle_model=toggle,
        **base_kwargs,
    )

    return cash_config, pik_config, toggle_config


def format_result_row(label: str, result: MertonMcResult) -> str:
    """Format one row of the results table."""
    return (
        f"  {label:<20s}  "
        f"{result.clean_price_pct:7.2f}  "
        f"{result.effective_spread_bp:7.0f}  "
        f"{result.expected_loss:7.2%}  "
        f"{result.default_rate:7.2%}  "
        f"{result.average_pik_fraction:7.2%}  "
        f"{result.avg_terminal_notional:8.1f}  "
        f"{result.standard_error:7.4f}"
    )


# ── Hazard-rate pricing helpers (library curves) ─────────────────────────
#
# Use the library's HazardCurve and DiscountCurve term structures to price
# bonds under a reduced-form flat hazard rate.  Instead of raw math.exp(),
# we use HazardCurve.survival(t), HazardCurve.default_prob(t1, t2), and
# DiscountCurve.df(t) — the library handles interpolation, extrapolation,
# and day-count conventions.
#
# Cash-pay: coupons + principal weighted by survival × discount factor.
# Full-PIK: no coupons; inflated notional N×(1+c/f)^n at maturity.
# Recovery leg: R × outstanding_notional × default_prob(t-dt, t) × df(t).


def _build_curves(
    hazard: float, recovery: float,
) -> tuple[HazardCurve, DiscountCurve]:
    """Build flat hazard and discount curves using the library."""
    dc = DiscountCurve(
        "USD-OIS", AS_OF,
        [(t, math.exp(-RISK_FREE_RATE * t))
         for t in [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]],
    )
    hc = HazardCurve(
        "CREDIT", AS_OF, [(0.0, hazard), (10.0, hazard)],
        recovery_rate=recovery,
    )
    return hc, dc


def hr_price_cash(hc: HazardCurve, dc: DiscountCurve) -> float:
    """PV of a cash-pay bond using library HazardCurve and DiscountCurve."""
    n_periods = MATURITY_YEARS * 2  # semi-annual
    dt = 0.5
    cpn = COUPON_RATE / 2 * NOTIONAL
    pv = 0.0
    for i in range(1, n_periods + 1):
        t = i * dt
        surv = hc.survival(t)
        df = dc.df(t)
        cf = cpn + (NOTIONAL if i == n_periods else 0.0)
        pv += cf * surv * df
        # Recovery leg: R × N × P(default in [t-dt, t]) × D(t)
        pv += hc.recovery_rate * NOTIONAL * hc.default_prob(t - dt, t) * df
    return pv


def hr_price_pik(hc: HazardCurve, dc: DiscountCurve) -> float:
    """PV of a full-PIK bond using library HazardCurve and DiscountCurve.

    PIK bond: no coupons paid; notional grows by c/f each period.
    At maturity the investor receives N × (1 + c/f)^n.  On default the
    recovery is based on the *inflated* notional at that point.
    """
    n_periods = MATURITY_YEARS * 2
    dt = 0.5
    growth = 1.0 + COUPON_RATE / 2
    pv = 0.0
    for i in range(1, n_periods + 1):
        t = i * dt
        surv = hc.survival(t)
        df = dc.df(t)
        ntl_i = NOTIONAL * growth ** i
        if i == n_periods:
            pv += ntl_i * surv * df
        pv += hc.recovery_rate * ntl_i * hc.default_prob(t - dt, t) * df
    return pv


def hr_find_implied_hazard(
    price_fn, target_pv: float, recovery: float, dc: DiscountCurve,
) -> float:
    """Bisect for the flat hazard rate λ that reprices to *target_pv*."""
    lo, hi = 0.0, 5.0
    for _ in range(200):
        mid = (lo + hi) / 2.0
        hc = HazardCurve(
            "CREDIT", AS_OF, [(0.0, mid), (10.0, mid)],
            recovery_rate=recovery,
        )
        pv = price_fn(hc, dc)
        if abs(pv - target_pv) < 1e-6:
            return mid
        if pv > target_pv:
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2.0


# ── Main ───────────────────────────────────────────────────────────────────

def main() -> None:
    bond = build_bond()
    mc_results: dict[str, tuple[MertonMcResult, MertonMcResult, MertonMcResult]] = {}

    print("=" * 110)
    print("PIK Structural Credit Analysis: Breakeven Spreads by Issuer Credit Profile")
    print("=" * 110)
    print(f"Bond:     {MATURITY_YEARS}Y  {COUPON_RATE:.1%} semi-annual  |  "
          f"Risk-free: {RISK_FREE_RATE:.2%}  |  "
          f"MC paths: {NUM_PATHS:,}  |  As-of: {AS_OF}")
    print()

    for profile in ISSUER_PROFILES:
        merton = build_merton(profile)

        # Analytical credit metrics
        dd = merton.distance_to_default(MATURITY_YEARS)
        pd = merton.default_probability(MATURITY_YEARS)
        impl_spread = merton.implied_spread(MATURITY_YEARS, profile["base_recovery"])

        print(f"── {profile['name']} ──────────────────────────────────────────")
        print(f"  Assets: {profile['asset_value']:.0f}  |  "
              f"Debt: {profile['debt_barrier']:.0f}  |  "
              f"Vol: {profile['asset_vol']:.0%}  |  "
              f"Coverage: {profile['asset_value']/profile['debt_barrier']:.2f}x")
        print(f"  Merton DD: {dd:.2f}  |  "
              f"PD({MATURITY_YEARS}Y): {pd:.2%}  |  "
              f"Implied Spread: {impl_spread:.0f} bp")
        print()

        # Build configurations
        cash_cfg, pik_cfg, toggle_cfg = build_configs(merton, profile)

        # Price all three structures
        cash_result = bond.price_merton_mc(config=cash_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)
        pik_result = bond.price_merton_mc(config=pik_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)
        toggle_result = bond.price_merton_mc(config=toggle_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)
        mc_results[profile["name"]] = (cash_result, pik_result, toggle_result)

        header = (
            f"  {'Structure':<20s}  "
            f"{'Price':>7s}  "
            f"{'Sprd bp':>7s}  "
            f"{'E[Loss]':>7s}  "
            f"{'DefRate':>7s}  "
            f"{'PIK Frac':>7s}  "
            f"{'Term Ntl':>8s}  "
            f"{'SE':>7s}"
        )
        print(header)
        print("  " + "-" * (len(header) - 2))
        print(format_result_row("Cash-Pay", cash_result))
        print(format_result_row("Full PIK", pik_result))
        print(format_result_row("PIK Toggle", toggle_result))

        # Spread differential
        pik_premium = pik_result.effective_spread_bp - cash_result.effective_spread_bp
        toggle_premium = toggle_result.effective_spread_bp - cash_result.effective_spread_bp
        print()
        print(f"  PIK premium over cash:     {pik_premium:+.0f} bp")
        print(f"  Toggle premium over cash:  {toggle_premium:+.0f} bp")
        print()

    # ── Summary table (Merton MC) ────────────────────────────────────
    print("=" * 110)
    print("SUMMARY: Merton MC Breakeven Spreads (bp) by Structure and Credit Quality")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'Cash':>8s}  {'Full PIK':>8s}  {'Toggle':>8s}  "
          f"{'PIK-Cash':>8s}  {'Toggle-Cash':>11s}")
    print("  " + "-" * 80)

    for profile in ISSUER_PROFILES:
        cash_r, pik_r, toggle_r = mc_results[profile["name"]]
        pik_d = pik_r.effective_spread_bp - cash_r.effective_spread_bp
        tog_d = toggle_r.effective_spread_bp - cash_r.effective_spread_bp
        print(
            f"  {profile['name']:<25s}  "
            f"{cash_r.effective_spread_bp:8.0f}  "
            f"{pik_r.effective_spread_bp:8.0f}  "
            f"{toggle_r.effective_spread_bp:8.0f}  "
            f"{pik_d:+8.0f}  "
            f"{tog_d:+11.0f}"
        )

    print()
    print("Key: PIK-Cash = additional spread required for full-PIK vs cash-pay bond")
    print("     Toggle-Cash = additional spread for PIK-toggle vs cash-pay bond")
    print()

    # ── Hazard-rate-only pricing (library curves) ─────────────────────
    #
    # Reduced-form model: flat hazard rate λ, priced via the library's
    # HazardCurve.survival(t), HazardCurve.default_prob(t1, t2), and
    # DiscountCurve.df(t).
    #
    # Cash-pay: coupons + principal weighted by survival(t) × df(t),
    #   plus a recovery leg using default_prob() for each period.
    # Full-PIK: zero coupons; inflated notional N × (1+c/f)^n at
    #   maturity, with recovery on the growing notional per period.
    #
    # Two views:
    #   1. Price at issuer's base λ:  shows how PIK shifts value to
    #      maturity (duration extension + notional inflation).
    #   2. Implied λ from MC prices:  backs out the flat hazard rate
    #      that reproduces each Merton MC model price.  The PIK implied
    #      λ is higher than cash — that gap is the structural feedback
    #      premium that a flat hazard model can't capture.

    # Build a shared discount curve (same risk-free rate for all issuers)
    dc = DiscountCurve(
        "USD-OIS", AS_OF,
        [(t, math.exp(-RISK_FREE_RATE * t))
         for t in [0.0, 0.5, 1.0, 2.0, 3.0, 5.0, 7.0, 10.0]],
    )

    # ── View 1: prices at each issuer's base hazard rate ──────────────
    print("=" * 110)
    print("HAZARD-RATE PRICES: Cash-Pay vs PIK at Each Issuer's Flat Hazard Rate")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'λ (bp)':>7s}  "
          f"{'HR Cash':>9s}  {'HR PIK':>9s}  {'Δ Price':>8s}  "
          f"{'MC Cash':>9s}  {'MC PIK':>9s}  {'Δ Price':>8s}")
    print("  " + "-" * 100)

    for profile in ISSUER_PROFILES:
        lam = profile["base_hazard"]
        rec = profile["base_recovery"]

        hc = HazardCurve(
            "CREDIT", AS_OF, [(0.0, lam), (10.0, lam)],
            recovery_rate=rec,
        )
        hr_cash_px = hr_price_cash(hc, dc)
        hr_pik_px = hr_price_pik(hc, dc)
        hr_delta = hr_pik_px - hr_cash_px

        cash_r, pik_r, _ = mc_results[profile["name"]]
        mc_delta = pik_r.clean_price_pct - cash_r.clean_price_pct

        print(
            f"  {profile['name']:<25s}  "
            f"{lam * 10_000:7.0f}  "
            f"{hr_cash_px:9.2f}  "
            f"{hr_pik_px:9.2f}  "
            f"{hr_delta:+8.2f}  "
            f"{cash_r.clean_price_pct:9.2f}  "
            f"{pik_r.clean_price_pct:9.2f}  "
            f"{mc_delta:+8.2f}"
        )

    print()
    print("  Δ Price = PIK price minus Cash price (negative = PIK trades cheaper)")
    print("  The HR model captures timing + notional effects only.")
    print("  The MC model adds endogenous feedback → larger PIK discount for weak credits.")
    print()

    # ── View 2: implied hazard rates from MC model prices ─────────────
    print("=" * 110)
    print("IMPLIED HAZARD RATES: Flat λ Backing Out Each Merton MC Price")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'Base λ':>7s}  "
          f"{'λ Cash':>7s}  {'λ PIK':>7s}  {'Δλ':>7s}  "
          f"{'MC Sprd':>7s}  {'MC PIK':>7s}  {'Δ Sprd':>7s}")
    print("  " + "-" * 90)

    for profile in ISSUER_PROFILES:
        rec = profile["base_recovery"]
        cash_r, pik_r, _ = mc_results[profile["name"]]

        # Target PV from MC prices (convert percentage to currency units)
        cash_target = cash_r.clean_price_pct / 100 * NOTIONAL
        pik_target = pik_r.clean_price_pct / 100 * NOTIONAL

        lam_cash = hr_find_implied_hazard(hr_price_cash, cash_target, rec, dc)
        lam_pik = hr_find_implied_hazard(hr_price_pik, pik_target, rec, dc)
        delta_lam = lam_pik - lam_cash

        print(
            f"  {profile['name']:<25s}  "
            f"{profile['base_hazard'] * 10_000:7.0f}  "
            f"{lam_cash * 10_000:7.0f}  "
            f"{lam_pik * 10_000:7.0f}  "
            f"{delta_lam * 10_000:+7.0f}  "
            f"{cash_r.effective_spread_bp:7.0f}  "
            f"{pik_r.effective_spread_bp:7.0f}  "
            f"{pik_r.effective_spread_bp - cash_r.effective_spread_bp:+7.0f}"
        )

    print()
    print("Key: Base λ  = issuer's input hazard rate (bp)")
    print("     λ Cash  = flat hazard rate reproducing the MC cash-pay price")
    print("     λ PIK   = flat hazard rate reproducing the MC full-PIK price")
    print("     Δλ      = PIK hazard premium: extra hazard the market charges for PIK")
    print("     MC Sprd = Merton MC effective spread (bp)")
    print("     Δ Sprd  = MC PIK spread minus MC Cash spread")
    print()


if __name__ == "__main__":
    main()
