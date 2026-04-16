"""ECL / IFRS 9 / CECL end-to-end demo.

This script walks through the minimum-viable ECL workflow exposed by
``finstack.statements_analytics``:

1.  Define a synthetic corporate loan exposure.
2.  Classify it into an IFRS 9 stage under three scenarios (performing,
    SICR, default).
3.  Compute 12-month (Stage 1) and lifetime (Stage 2) ECL.
4.  Apply IFRS 9 B5.5.42 probability weighting across base / upside /
    downside macro scenarios.

All monetary amounts are in the exposure's base currency (USD in this
example). PDs are cumulative and in decimal form (0.02 = 2%).
"""

from __future__ import annotations

from finstack.statements_analytics import (
    Exposure,
    classify_stage,
    compute_ecl,
    compute_ecl_weighted,
)


def banner(title: str) -> None:
    print()
    print("=" * 72)
    print(title)
    print("=" * 72)


def main() -> None:
    # ------------------------------------------------------------------
    # 1. Synthetic corporate loan exposure
    # ------------------------------------------------------------------
    # A 5-year senior unsecured term loan to a BBB-rated corporate.
    #   EAD = $1,000,000
    #   LGD = 45%   (typical senior unsecured recovery ~55%)
    #   EIR = 6%    (IFRS 9 discount rate)
    #   Lifetime PD (5y horizon) at origination = 3%, current = 3.2%
    base_exposure = Exposure(
        id="CORP-LOAN-001",
        ead=1_000_000.0,
        lgd=0.45,
        eir=0.06,
        remaining_maturity=5.0,
        current_pd=0.032,
        origination_pd=0.030,
        dpd=0,
    )

    banner("Synthetic exposure")
    print(base_exposure)

    # ------------------------------------------------------------------
    # 2. Stage classification across three scenarios
    # ------------------------------------------------------------------
    banner("Stage classification")

    # 2a. Performing: small PD drift, no delinquency -> Stage 1
    stage, reason = classify_stage(
        base_exposure,
        pd_delta_stage2=0.01,
        dpd_30_trigger=True,
        dpd_90_trigger=True,
    )
    print(f"  Performing   -> {stage:<8} | trigger: {reason}")

    # 2b. SICR: lifetime PD has more than doubled (1.5pp absolute delta)
    sicr_exposure = Exposure(
        id="CORP-LOAN-001-SICR",
        ead=1_000_000.0,
        lgd=0.45,
        eir=0.06,
        remaining_maturity=5.0,
        current_pd=0.045,      # PD rose from 3.0% to 4.5%
        origination_pd=0.030,
        dpd=0,
    )
    stage, reason = classify_stage(
        sicr_exposure,
        pd_delta_stage2=0.01,
        dpd_30_trigger=True,
        dpd_90_trigger=True,
    )
    print(f"  SICR         -> {stage:<8} | trigger: {reason}")

    # 2c. Default: more than 90 DPD -> Stage 3
    default_exposure = Exposure(
        id="CORP-LOAN-001-NPL",
        ead=1_000_000.0,
        lgd=0.45,
        eir=0.06,
        remaining_maturity=5.0,
        current_pd=0.25,
        origination_pd=0.030,
        dpd=120,
    )
    stage, reason = classify_stage(
        default_exposure,
        pd_delta_stage2=0.01,
        dpd_30_trigger=True,
        dpd_90_trigger=True,
    )
    print(f"  90+ DPD      -> {stage:<8} | trigger: {reason}")

    # ------------------------------------------------------------------
    # 3. ECL computation
    # ------------------------------------------------------------------
    # Cumulative PD schedule for the base scenario (annual knots).
    # A (0.0, 0.0) knot is added automatically by the binding.
    base_pd_schedule = [
        (1.0, 0.008),
        (2.0, 0.015),
        (3.0, 0.022),
        (4.0, 0.028),
        (5.0, 0.032),
    ]
    sicr_pd_schedule = [
        (1.0, 0.015),
        (2.0, 0.025),
        (3.0, 0.033),
        (4.0, 0.040),
        (5.0, 0.045),
    ]

    banner("ECL computation")

    ecl_12m = compute_ecl(
        ead=base_exposure.ead,
        pd_schedule=base_pd_schedule,
        lgd=base_exposure.lgd,
        eir=base_exposure.eir,
        max_horizon_years=base_exposure.remaining_maturity,
        bucket_width_years=0.25,
        stage="stage1",
    )
    ecl_lifetime = compute_ecl(
        ead=sicr_exposure.ead,
        pd_schedule=sicr_pd_schedule,
        lgd=sicr_exposure.lgd,
        eir=sicr_exposure.eir,
        max_horizon_years=sicr_exposure.remaining_maturity,
        bucket_width_years=0.25,
        stage="stage2",
    )

    print(f"  Stage 1 (12-month) ECL       : ${ecl_12m:>12,.2f}")
    print(f"  Stage 2 (lifetime, SICR) ECL : ${ecl_lifetime:>12,.2f}")
    print(
        f"  Lifetime / 12m multiple       : {ecl_lifetime / ecl_12m:>12.1f}x"
    )

    # ------------------------------------------------------------------
    # 4. Macro scenario weighting (IFRS 9 B5.5.42)
    # ------------------------------------------------------------------
    # Three probability-weighted scenarios covering the 5-year horizon.
    upside_pd_schedule = [
        (1.0, 0.004),
        (2.0, 0.008),
        (3.0, 0.012),
        (4.0, 0.016),
        (5.0, 0.020),
    ]
    downside_pd_schedule = [
        (1.0, 0.025),
        (2.0, 0.045),
        (3.0, 0.065),
        (4.0, 0.085),
        (5.0, 0.100),
    ]

    scenarios = [
        (0.60, base_pd_schedule),       # base
        (0.20, upside_pd_schedule),     # upside
        (0.20, downside_pd_schedule),   # downside
    ]

    weighted_12m = compute_ecl_weighted(
        ead=base_exposure.ead,
        scenarios=scenarios,
        lgd=base_exposure.lgd,
        eir=base_exposure.eir,
        max_horizon=base_exposure.remaining_maturity,
        stage="stage1",
    )
    weighted_lifetime = compute_ecl_weighted(
        ead=base_exposure.ead,
        scenarios=scenarios,
        lgd=base_exposure.lgd,
        eir=base_exposure.eir,
        max_horizon=base_exposure.remaining_maturity,
        stage="stage2",
    )

    banner("Macro-scenario weighted ECL (base 60 / upside 20 / downside 20)")
    print(f"  Weighted Stage 1 (12-month) ECL : ${weighted_12m:>12,.2f}")
    print(f"  Weighted Stage 2 (lifetime) ECL : ${weighted_lifetime:>12,.2f}")

    # Per-scenario breakdown for transparency.
    print()
    print("  Per-scenario Stage 1 ECL (unweighted):")
    for label, (w, sched) in zip(("base", "upside", "downside"), scenarios):
        ecl_s = compute_ecl(
            ead=base_exposure.ead,
            pd_schedule=sched,
            lgd=base_exposure.lgd,
            eir=base_exposure.eir,
            max_horizon_years=base_exposure.remaining_maturity,
            bucket_width_years=0.25,
            stage="stage1",
        )
        print(f"    {label:<8} (w={w:.0%}) : ${ecl_s:>12,.2f}")


if __name__ == "__main__":
    main()
