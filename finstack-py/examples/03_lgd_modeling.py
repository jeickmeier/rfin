"""Loss-given-default (LGD) modeling example.

Demonstrates the LGD primitives exposed from the Rust ``finstack-core``
crate through the ``finstack.core.credit.lgd`` submodule:

1. Seniority-based recovery statistics (Moody's historical).
2. Beta-distributed recovery sampling and quantiles.
3. Workout (collateral-waterfall) LGD with costs and discounting.
4. Frye-Jacobs downturn LGD adjustment.
5. Regulatory-floor downturn LGD adjustment.
6. Exposure at default for term loans vs. revolvers (with CCF).

Run:

    python finstack-py/examples/03_lgd_modeling.py
"""

from __future__ import annotations

import math
import statistics

from finstack.core.credit import lgd


def banner(title: str) -> None:
    """Print a labelled section separator."""
    print()
    print("=" * 72)
    print(f"  {title}")
    print("=" * 72)


def main() -> None:
    # -----------------------------------------------------------------
    # 1. Seniority recovery statistics (Moody's 1982-2023 calibration)
    # -----------------------------------------------------------------
    banner("Seniority recovery statistics (Moody's historical)")

    classes = ["senior_secured", "senior_unsecured", "subordinated"]
    print(f"  {'Seniority':<20s} {'Mean':>8s} {'Std':>8s} {'Alpha':>10s} {'Beta':>10s}")
    print("  " + "-" * 58)
    stats_by_class: dict[str, dict[str, float]] = {}
    for c in classes:
        s = lgd.seniority_recovery_stats(c, rating_agency="moodys")
        stats_by_class[c] = dict(s)
        print(
            f"  {c:<20s} {s['mean']:>8.2%} {s['std']:>8.2%} "
            f"{s['alpha']:>10.4f} {s['beta']:>10.4f}"
        )

    # -----------------------------------------------------------------
    # 2. Beta-recovery sampling and quantile summary
    # -----------------------------------------------------------------
    banner("Beta recovery sampling - Senior Unsecured, N = 10,000")

    unsec = stats_by_class["senior_unsecured"]
    samples = lgd.beta_recovery_sample(
        mean=unsec["mean"], std=unsec["std"], n_samples=10_000, seed=42
    )

    sample_mean = statistics.fmean(samples)
    sample_std = statistics.pstdev(samples)
    sample_min = min(samples)
    sample_max = max(samples)

    # Empirical quantiles (nearest-rank, good enough for a summary)
    sorted_samples = sorted(samples)
    def empirical_q(q: float) -> float:
        idx = min(len(sorted_samples) - 1, max(0, int(math.floor(q * len(sorted_samples)))))
        return sorted_samples[idx]

    print(f"  Target mean / std          : {unsec['mean']:.4f} / {unsec['std']:.4f}")
    print(f"  Sample mean / std          : {sample_mean:.4f} / {sample_std:.4f}")
    print(f"  Sample min / max           : {sample_min:.4f} / {sample_max:.4f}")
    print(f"  Empirical  q10 / q50 / q90 : "
          f"{empirical_q(0.10):.4f} / {empirical_q(0.50):.4f} / {empirical_q(0.90):.4f}")

    # Analytic Beta quantile function, for comparison.
    q10 = lgd.beta_recovery_quantile(unsec["mean"], unsec["std"], 0.10)
    q50 = lgd.beta_recovery_quantile(unsec["mean"], unsec["std"], 0.50)
    q90 = lgd.beta_recovery_quantile(unsec["mean"], unsec["std"], 0.90)
    print(f"  Analytic   q10 / q50 / q90 : {q10:.4f} / {q50:.4f} / {q90:.4f}")

    # -----------------------------------------------------------------
    # 3. Workout LGD - $10M EAD, RE ($8M) + Equipment ($2M)
    # -----------------------------------------------------------------
    banner("Workout LGD - $10M EAD with collateral waterfall")

    ead = 10_000_000.0
    collateral = [
        # (type, book value, haircut)
        ("real_estate", 8_000_000.0, 0.30),   # 30% RE haircut
        ("equipment",   2_000_000.0, 0.40),   # 40% equipment haircut
    ]
    direct_cost_pct = 0.05     # 5% of EAD in legal/admin
    indirect_cost_pct = 0.03   # 3% of EAD in opportunity cost
    workout_years = 2.0
    discount_rate = 0.05

    net_recovery, workout_lgd_val = lgd.workout_lgd(
        ead=ead,
        collateral=collateral,
        direct_cost_pct=direct_cost_pct,
        indirect_cost_pct=indirect_cost_pct,
        time_to_resolution_years=workout_years,
        discount_rate=discount_rate,
    )

    gross = sum(v * (1.0 - h) for (_t, v, h) in collateral)
    df = (1.0 + discount_rate) ** (-workout_years)
    print(f"  EAD                        : ${ead:>14,.0f}")
    print(f"  Collateral (post-haircut)  : ${gross:>14,.0f}")
    print(f"  Discount factor ({workout_years:.0f}y @ {discount_rate:.0%}) : {df:>14.4f}")
    print(f"  Direct costs ({direct_cost_pct:.0%} of EAD)   : ${direct_cost_pct * ead:>14,.0f}")
    print(f"  Indirect costs ({indirect_cost_pct:.0%} of EAD) : ${indirect_cost_pct * ead:>14,.0f}")
    print(f"  Net recovery (discounted)  : ${net_recovery:>14,.0f}")
    print(f"  Workout LGD                : {workout_lgd_val:>14.4%}")

    base_lgd = workout_lgd_val

    # -----------------------------------------------------------------
    # 4. Frye-Jacobs downturn adjustment
    # -----------------------------------------------------------------
    banner("Frye-Jacobs downturn LGD")

    rho = 0.15
    stress_q = 0.999
    fj_lgd = lgd.downturn_lgd_frye_jacobs(
        base_lgd=base_lgd,
        asset_correlation=rho,
        stress_quantile=stress_q,
    )
    print(f"  Base LGD                   : {base_lgd:>14.4%}")
    print(f"  Asset correlation (rho)    : {rho:>14.2f}")
    print(f"  Stress quantile            : {stress_q:>14.3f}")
    print(f"  Frye-Jacobs downturn LGD   : {fj_lgd:>14.4%}")
    print(f"  Uplift over base           : {(fj_lgd - base_lgd):>14.4%}")

    # -----------------------------------------------------------------
    # 5. Regulatory floor adjustment (Basel-style secured)
    # -----------------------------------------------------------------
    banner("Regulatory-floor downturn LGD (secured-style)")

    add_on = 0.08
    floor = 0.10
    reg_lgd = lgd.downturn_lgd_regulatory_floor(
        base_lgd=base_lgd, add_on=add_on, floor=floor
    )
    print(f"  Base LGD                   : {base_lgd:>14.4%}")
    print(f"  Flat add-on                : {add_on:>14.4%}")
    print(f"  Absolute floor             : {floor:>14.4%}")
    print(f"  Regulatory downturn LGD    : {reg_lgd:>14.4%}")

    # Illustrate the floor actually binding on a lower base LGD.
    tiny_base = 0.05
    reg_lgd_tiny = lgd.downturn_lgd_regulatory_floor(
        base_lgd=tiny_base, add_on=add_on, floor=floor
    )
    print(
        f"  (Floor binding example: base={tiny_base:.2%} + add_on={add_on:.2%} "
        f"-> max(0.13, {floor:.0%}) = {reg_lgd_tiny:.2%})"
    )

    # -----------------------------------------------------------------
    # 6. EAD: term loan vs. revolver (with CCF)
    # -----------------------------------------------------------------
    banner("Exposure at default: term loan vs. revolver")

    term_principal = 10_000_000.0
    term_ead = lgd.ead_term_loan(term_principal)
    print(f"  Term loan principal        : ${term_principal:>14,.0f}")
    print(f"  Term loan EAD              : ${term_ead:>14,.0f}")

    drawn = 6_000_000.0
    undrawn = 4_000_000.0
    ccf_basel = 0.75
    ccf_full = 1.00
    rev_ead_basel = lgd.ead_revolver(drawn=drawn, undrawn=undrawn, ccf=ccf_basel)
    rev_ead_full = lgd.ead_revolver(drawn=drawn, undrawn=undrawn, ccf=ccf_full)

    print(f"  Revolver drawn             : ${drawn:>14,.0f}")
    print(f"  Revolver undrawn           : ${undrawn:>14,.0f}")
    print(
        f"  Revolver EAD (CCF={ccf_basel:.2f} Basel): ${rev_ead_basel:>14,.0f}  "
        f"(= drawn + undrawn x CCF)"
    )
    print(
        f"  Revolver EAD (CCF={ccf_full:.2f} full ): ${rev_ead_full:>14,.0f}"
    )


if __name__ == "__main__":
    main()
