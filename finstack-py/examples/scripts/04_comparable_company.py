"""Comparable Company Analysis (CCA) end-to-end demo.

Walks through the minimum-viable relative-value workflow exposed by
``finstack.analytics`` for comparable company analysis:

1.  Define a subject company's credit / valuation metrics.
2.  Build a peer set of 10 comparable companies with varying metrics.
3.  Compute percentile rank and z-score of the subject for each metric.
4.  Run a spread-vs-leverage OLS fit to estimate the subject's fair
    spread given its leverage.
5.  Summarize the peer distribution for each metric (``peer_stats``).
6.  Score composite relative value across four weighted dimensions.

Metric conventions
------------------
* ``ev_ebitda``         : EV / EBITDA multiple (turns).
* ``leverage``          : Debt / EBITDA (turns).
* ``interest_coverage`` : EBITDA / interest expense (turns).
* ``oas_bps``           : Option-adjusted spread in basis points.

For the composite rich/cheap score, the sign convention is:
``positive = cheap`` (trades wide / below fair), ``negative = rich``.
"""

from __future__ import annotations

from finstack.analytics import (
    compute_multiple,
    peer_stats,
    percentile_rank,
    regression_fair_value,
    score_relative_value,
    z_score,
)


def banner(title: str) -> None:
    print()
    print("=" * 72)
    print(title)
    print("=" * 72)


def main() -> None:
    # ------------------------------------------------------------------
    # 1. Subject company
    # ------------------------------------------------------------------
    # A BB-rated energy issuer trading at 8.5x EV/EBITDA with 3.5x
    # leverage, 4.2x interest coverage, and a 350bps OAS.
    subject = {
        "ev_ebitda": 8.5,
        "leverage": 3.5,
        "interest_coverage": 4.2,
        "oas_bps": 350.0,
    }

    banner("Subject company metrics")
    for k, v in subject.items():
        print(f"  {k:<20}: {v:>8.2f}")

    # Sanity-check canonical compute_multiple helper: EV 8500 / EBITDA 1000 = 8.5x.
    ev, ebitda = 8_500.0, 1_000.0
    subject_company = {"enterprise_value": ev, "ebitda": ebitda}
    print(
        f"  compute_multiple({subject_company}, 'EvEbitda') = "
        f"{compute_multiple(subject_company, 'EvEbitda'):.2f}x"
    )

    # ------------------------------------------------------------------
    # 2. Peer universe (10 companies)
    # ------------------------------------------------------------------
    # Each dict varies along the four dimensions in a realistic way:
    # higher leverage / lower coverage tends to correlate with wider
    # spread and (somewhat) richer EV/EBITDA. Intentional noise included.
    peers = [
        {"ev_ebitda": 7.2, "leverage": 2.5, "interest_coverage": 5.5, "oas_bps": 250.0},
        {"ev_ebitda": 8.0, "leverage": 3.0, "interest_coverage": 4.8, "oas_bps": 300.0},
        {"ev_ebitda": 8.8, "leverage": 3.8, "interest_coverage": 4.0, "oas_bps": 380.0},
        {"ev_ebitda": 9.5, "leverage": 4.5, "interest_coverage": 3.3, "oas_bps": 450.0},
        {"ev_ebitda": 7.8, "leverage": 2.8, "interest_coverage": 5.0, "oas_bps": 280.0},
        {"ev_ebitda": 8.3, "leverage": 3.3, "interest_coverage": 4.5, "oas_bps": 330.0},
        {"ev_ebitda": 9.0, "leverage": 4.0, "interest_coverage": 3.8, "oas_bps": 400.0},
        {"ev_ebitda": 10.2, "leverage": 5.0, "interest_coverage": 2.8, "oas_bps": 520.0},
        {"ev_ebitda": 7.5, "leverage": 2.2, "interest_coverage": 6.0, "oas_bps": 220.0},
        {"ev_ebitda": 8.6, "leverage": 3.6, "interest_coverage": 4.1, "oas_bps": 360.0},
    ]

    metrics = ("ev_ebitda", "leverage", "interest_coverage", "oas_bps")
    peer_series = {m: [p[m] for p in peers] for m in metrics}

    # ------------------------------------------------------------------
    # 3. Percentile rank + z-score per metric
    # ------------------------------------------------------------------
    banner("Subject vs peers (per-metric rank)")
    header = f"  {'metric':<20} {'subject':>10} {'pctile':>10} {'z-score':>10}"
    print(header)
    print(f"  {'-' * 20} {'-' * 10} {'-' * 10} {'-' * 10}")
    for m in metrics:
        pctile = percentile_rank(subject[m], peer_series[m])
        z = z_score(subject[m], peer_series[m])
        print(f"  {m:<20} {subject[m]:>10.2f} {pctile:>9.1f}% {z:>+10.2f}")

    # ------------------------------------------------------------------
    # 4. Regression: spread vs leverage
    # ------------------------------------------------------------------
    # Fit OAS = intercept + slope * leverage across peers, then evaluate
    # the fitted spread at the subject's leverage (3.5x). The residual
    # (subject actual - fitted) indicates rich/cheap versus the line.
    banner("Regression: OAS (bps) vs leverage (turns)")
    reg = regression_fair_value(
        peer_series["leverage"],
        peer_series["oas_bps"],
        subject["leverage"],
        subject["oas_bps"],
    )
    slope = reg["slope"]
    intercept = reg["intercept"]
    r_squared = reg["r_squared"]
    fitted = reg["fitted_value"]
    actual = subject["oas_bps"]
    residual = reg["residual"]

    print(f"  slope              : {slope:>8.2f} bps / turn")
    print(f"  intercept          : {intercept:>8.2f} bps")
    print(f"  R-squared          : {r_squared:>8.3f}")
    print(f"  fitted spread @3.5x: {fitted:>8.1f} bps")
    print(f"  actual spread      : {actual:>8.1f} bps")
    print(
        f"  residual           : {residual:>+8.1f} bps "
        f"({'cheap' if residual > 0 else 'rich'})"
    )

    # ------------------------------------------------------------------
    # 5. Peer stats summary
    # ------------------------------------------------------------------
    banner("Peer distribution summary")
    print(
        f"  {'metric':<20} {'n':>4} {'min':>8} {'q1':>8} "
        f"{'median':>8} {'mean':>8} {'q3':>8} {'max':>8} {'std':>8}"
    )
    print(f"  {'-' * 20} {'-' * 4} {'-' * 8} {'-' * 8} "
          f"{'-' * 8} {'-' * 8} {'-' * 8} {'-' * 8} {'-' * 8}")
    for m in metrics:
        s = peer_stats(peer_series[m])
        print(
            f"  {m:<20} {s['n']:>4} {s['min']:>8.2f} {s['q1']:>8.2f} "
            f"{s['median']:>8.2f} {s['mean']:>8.2f} {s['q3']:>8.2f} "
            f"{s['max']:>8.2f} {s['std_dev']:>8.2f}"
        )

    # ------------------------------------------------------------------
    # 6. Composite relative-value score (4 dimensions, custom weights)
    # ------------------------------------------------------------------
    # Weights (sum to 1.0):
    #   oas_bps         : 0.40  (credit spread is the primary signal)
    #   ev_ebitda       : 0.30  (valuation multiple)
    #   leverage        : 0.15  (balance sheet risk)
    #   interest_coverage: 0.15 (cash-flow coverage)
    banner("Composite relative value score (4 dimensions)")
    dimensions = [
        ("oas_bps", 0.40),
        ("ev_ebitda", 0.30),
        ("leverage", 0.15),
        ("interest_coverage", 0.15),
    ]
    score = score_relative_value(subject, peers, dimensions)

    print(f"  composite_score : {score['composite_score']:>+8.3f}")
    print(f"  confidence      : {score['confidence']:>8.3f}")
    print(f"  peer_count      : {score['peer_count']:>8d}")
    print()
    print(f"  {'dimension':<20} {'pctile':>10} {'z-score':>10} {'weight':>10}")
    print(f"  {'-' * 20} {'-' * 10} {'-' * 10} {'-' * 10}")
    for name, _ in dimensions:
        dim = score["by_dimension"].get(name)
        if dim is None:
            continue
        # `percentile` here is on the 0-1 scale from the core scorer.
        print(
            f"  {name:<20} {dim['percentile'] * 100:>9.1f}% "
            f"{dim['z_score']:>+10.2f} {dim['weight']:>10.2f}"
        )

    sign = "cheap" if score["composite_score"] > 0 else "rich"
    print()
    print(
        f"  Interpretation: composite > 0 => cheap; composite < 0 => rich. "
        f"Subject reads {sign}."
    )


if __name__ == "__main__":
    main()
