"""Credit events / restructuring end-to-end demo.

Walks through the three minimum-viable restructuring calculators exposed
by ``finstack.valuations``:

1.  Build a distressed capital stack (first lien, senior unsecured,
    subordinated, equity) and run a recovery waterfall under a
    $100M distributable value.
2.  Compare hold-vs-tender economics for a distressed exchange offer
    (old 8% bond at 45c vs new 6% secured at 80c with a 2c consent fee).
3.  Model an open-market repurchase LME at 60 cents on the dollar for
    40% of outstanding bonds.

All monetary amounts are in USD. Recovery rates and prices are
expressed as fractions of par (``0.45`` = 45 cents on the dollar).
"""

from __future__ import annotations

from finstack.valuations import (
    analyze_exchange_offer,
    analyze_lme,
    execute_recovery_waterfall,
)


def banner(title: str) -> None:
    print()
    print("=" * 72)
    print(title)
    print("=" * 72)


def fmt_money(x: float) -> str:
    return f"${x:>15,.0f}"


def fmt_pct(x: float) -> str:
    return f"{x * 100:>7.2f}%"


# ----------------------------------------------------------------------
# 1. Recovery waterfall on a distressed capital stack
# ----------------------------------------------------------------------
def demo_recovery_waterfall() -> None:
    banner("1. Recovery waterfall -- $100M available across capital stack")

    # Distressed issuer capital structure:
    #   $50M first-lien term loan (secured, collateral = $40M)
    #   $80M senior unsecured notes
    #   $40M subordinated notes
    #   Common equity (no principal owed; models residual claim only)
    claims = [
        {
            "id": "first_lien_tl",
            "label": "First Lien Term Loan",
            "seniority": "first_lien",
            "principal": 50_000_000.0,
            "accrued": 1_000_000.0,
            "collateral_value": 40_000_000.0,
            "haircut": 0.0,
        },
        {
            "id": "sr_unsec_notes",
            "label": "Senior Unsecured Notes",
            "seniority": "senior_unsecured",
            "principal": 80_000_000.0,
            "accrued": 2_000_000.0,
        },
        {
            "id": "sub_notes",
            "label": "Subordinated Notes",
            "seniority": "subordinated",
            "principal": 40_000_000.0,
            "accrued": 1_000_000.0,
        },
        {
            "id": "common_equity",
            "label": "Common Equity",
            "seniority": "equity",
            "principal": 0.01,  # placeholder so recovery_rate is well-defined
        },
    ]

    total_value = 100_000_000.0
    result = execute_recovery_waterfall(
        total_value=total_value,
        currency="USD",
        claims=claims,
        allocation_mode="pro_rata",
    )

    print(f"  distributable value : {fmt_money(total_value)}")
    print(f"  total distributed   : {fmt_money(result['total_distributed'])}")
    print(f"  residual            : {fmt_money(result['residual'])}")
    print(f"  APR satisfied       : {result['apr_satisfied']}")
    if result["apr_violations"]:
        print("  APR violations      :")
        for v in result["apr_violations"]:
            print(f"    - {v}")

    print()
    header = (
        f"  {'claim':<24} {'seniority':<20} {'claim $':>16} "
        f"{'recovery $':>16} {'rate':>8}"
    )
    print(header)
    print(f"  {'-' * 24} {'-' * 20} {'-' * 16} {'-' * 16} {'-' * 8}")
    for row in result["per_claim_recovery"]:
        print(
            f"  {row['id']:<24} {row['seniority']:<20} "
            f"{fmt_money(row['total_claim'])} "
            f"{fmt_money(row['total_recovery'])} "
            f"{fmt_pct(row['recovery_rate'])}"
        )

    print()
    print("  Interpretation:")
    print("    - First lien recovers 100% (collateral + pro-rata on deficiency).")
    print("    - Senior unsecured takes the impairment on the remaining pool.")
    print("    - Subordinated and equity recover $0 under strict APR.")


# ----------------------------------------------------------------------
# 2. Exchange offer: hold vs tender
# ----------------------------------------------------------------------
def demo_exchange_offer() -> None:
    banner("2. Exchange offer -- old 8% note @ 45c vs new 6% secured @ 80c")

    # Per $100 par:
    #   hold:   old 8% unsecured bond trades at 45c -> $45 PV
    #   tender: new 6% secured bond trades at 80c   -> $80 PV
    #           plus 2c consent fee                -> $2
    #           no equity sweetener                -> $0
    old_pv = 45.0
    new_pv = 80.0
    consent_fee = 2.0
    equity_sweetener = 0.0

    result = analyze_exchange_offer(
        old_pv=old_pv,
        new_pv=new_pv,
        consent_fee=consent_fee,
        equity_sweetener_value=equity_sweetener,
        exchange_type="discount",
    )

    print(f"  exchange_type         : {result['exchange_type']}")
    print(f"  old PV (hold)         : ${result['old_npv']:>8.2f}")
    print(f"  new PV (tender)       : ${result['new_npv']:>8.2f}")
    print(f"  consent fee           : ${result['consent_fee']:>8.2f}")
    print(f"  equity sweetener      : ${result['equity_sweetener_value']:>8.2f}")
    print(f"  tender total          : ${result['tender_total']:>8.2f}")
    print(f"  delta NPV (tender-hold): ${result['delta_npv']:>+8.2f}")
    print(f"  breakeven recovery    : {fmt_pct(result['breakeven_recovery'])}")
    print(f"  recommend tender?     : {result['tender_recommended']}")

    print()
    print("  Interpretation:")
    print("    - Tender total ($82) materially exceeds hold PV ($45): +$37/par.")
    print("    - Breakeven: holder needs >100% recovery to justify holding,")
    print("      i.e. the tender strictly dominates unless the holder expects")
    print("      the old note to appreciate past the tender consideration.")


# ----------------------------------------------------------------------
# 3. LME: open-market repurchase
# ----------------------------------------------------------------------
def demo_lme() -> None:
    banner("3. LME -- open-market repurchase @ 60c for 40% of bonds")

    # Issuer has $200M outstanding notes; wants to retire 40% at 60c/par.
    notional = 200_000_000.0
    repurchase_price = 0.60  # 60 cents on the dollar
    acceptance = 0.40        # 40% of holders tender
    ebitda = 25_000_000.0    # pro forma leverage calc

    result = analyze_lme(
        lme_type="open_market",
        notional=notional,
        repurchase_price_pct=repurchase_price,
        opt_acceptance_pct=acceptance,
        ebitda=ebitda,
    )

    print(f"  lme_type              : {result['lme_type']}")
    print(f"  outstanding           : {fmt_money(notional)}")
    print(f"  cash cost             : {fmt_money(result['cost'])}")
    print(f"  notional retired      : {fmt_money(result['notional_reduction'])}")
    print(f"  discount capture      : {fmt_money(result['discount_capture'])}")
    print(f"  discount capture pct  : {fmt_pct(result['discount_capture_pct'])}")
    print(f"  remaining-holder imp. : {fmt_pct(result['remaining_holder_impact_pct'])}")

    lev = result["leverage_impact"]
    if lev is not None:
        print()
        print("  Leverage impact:")
        print(f"    pre  debt           : {fmt_money(lev['pre_total_debt'])}")
        print(f"    post debt           : {fmt_money(lev['post_total_debt'])}")
        print(f"    pre  leverage       : {lev['pre_leverage']:>8.2f}x")
        print(f"    post leverage       : {lev['post_leverage']:>8.2f}x")
        print(f"    turns reduced       : {lev['leverage_reduction']:>8.2f}x")

    print()
    print("  Interpretation:")
    print("    - Issuer retires $80M of par for $48M cash.")
    print("    - $32M of discount captured (40% of par retired).")
    print("    - Leverage drops from 8.0x to 4.8x -- 3.2 turns of deleveraging.")


def main() -> None:
    demo_recovery_waterfall()
    demo_exchange_offer()
    demo_lme()
    print()


if __name__ == "__main__":
    main()
