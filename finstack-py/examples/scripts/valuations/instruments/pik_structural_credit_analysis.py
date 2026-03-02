#!/usr/bin/env python3
"""PIK Structural Credit Analysis: Breakeven Spreads Across Issuer Credit Profiles.

Demonstrates the full Merton structural credit + Monte Carlo PIK pricing
pipeline.  Compares cash-pay, full-PIK, and PIK-toggle bonds for issuers
ranging from investment-grade to deeply stressed.

Usage:
    python pik_structural_credit_analysis.py
"""

from __future__ import annotations

from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
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


# ── Main ───────────────────────────────────────────────────────────────────

def main() -> None:
    bond = build_bond()

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

    # ── Summary table ──────────────────────────────────────────────────
    print("=" * 110)
    print("SUMMARY: Breakeven Spreads (bp) by Structure and Credit Quality")
    print("=" * 110)
    print(f"  {'Issuer':<25s}  {'Cash':>8s}  {'Full PIK':>8s}  {'Toggle':>8s}  "
          f"{'PIK-Cash':>8s}  {'Toggle-Cash':>11s}")
    print("  " + "-" * 80)

    for profile in ISSUER_PROFILES:
        merton = build_merton(profile)
        cash_cfg, pik_cfg, toggle_cfg = build_configs(merton, profile)

        cash_r = bond.price_merton_mc(config=cash_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)
        pik_r = bond.price_merton_mc(config=pik_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)
        toggle_r = bond.price_merton_mc(config=toggle_cfg, discount_rate=RISK_FREE_RATE, as_of=AS_OF)

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


if __name__ == "__main__":
    main()
