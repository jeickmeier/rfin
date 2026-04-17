"""FRTB SBA and SA-CCR regulatory capital example.

Demonstrates the ``finstack.margin`` regulatory bindings:

1. Build a small FRTB sensitivity portfolio (GIRR delta at four tenors plus a
   single equity delta).
2. Compute the Sensitivity-Based Approach charge under each correlation
   scenario (low / medium / high) and print per-risk-class charges.
3. Build a simple 5-year interest-rate swap as a :class:`SaCcrTrade` and
   compute SA-CCR Exposure at Default (RC, PFE, EAD).

Run standalone:

    python finstack-py/examples/11_frtb_saccr.py
"""

from __future__ import annotations

from finstack.margin import (
    FrtbSensitivities,
    SaCcrTrade,
    frtb_sba_charge,
    saccr_ead,
)


def build_frtb_portfolio() -> FrtbSensitivities:
    """Construct a small multi-asset FRTB sensitivity portfolio.

    GIRR delta at 4 tenors across USD (vanilla IR exposure) plus a single
    equity delta in bucket 11 (large-cap indices).
    """
    sens = FrtbSensitivities(base_currency="USD")

    # GIRR delta -- USD curve, classic IR swap risk profile.
    # Units: currency per 1bp (see FrtbSensitivities Rust docs).
    sens.add_girr_delta(tenor="1Y", amount=25_000.0)
    sens.add_girr_delta(tenor="2Y", amount=40_000.0)
    sens.add_girr_delta(tenor="5Y", amount=80_000.0)
    sens.add_girr_delta(tenor="10Y", amount=60_000.0)

    # Equity delta -- index bucket (bucket 11 carries the large-cap RW).
    sens.add_equity_delta(underlier="SPX", bucket=11, amount=150_000.0)

    return sens


def run_frtb_scenarios(sens: FrtbSensitivities) -> None:
    """Run FRTB SBA under each correlation scenario and print breakdowns."""
    print("=" * 72)
    print("FRTB Sensitivity-Based Approach -- scenario sweep")
    print("=" * 72)

    for scenario in ("low", "medium", "high"):
        total, breakdown = frtb_sba_charge(sens, correlation_scenario=scenario)
        print(f"\n[{scenario.upper()} correlation scenario]")
        print(f"  Total charge (SBA + DRC + RRAO): {total:,.2f}")

        delta = breakdown["delta"]
        if delta:
            print("  Delta charge by risk class:")
            for rc, amt in sorted(delta.items()):
                print(f"    {rc:<18} {amt:>14,.2f}")

        vega = breakdown["vega"]
        if vega:
            print("  Vega charge by risk class:")
            for rc, amt in sorted(vega.items()):
                print(f"    {rc:<18} {amt:>14,.2f}")

        curvature = breakdown["curvature"]
        if curvature:
            print("  Curvature charge by risk class:")
            for rc, amt in sorted(curvature.items()):
                print(f"    {rc:<18} {amt:>14,.2f}")

        print(f"  DRC:  {breakdown['drc']:,.2f}")
        print(f"  RRAO: {breakdown['rrao']:,.2f}")

    # Headline: default call runs all three and takes the max.
    total_max, breakdown_max = frtb_sba_charge(sens)
    print("\n[All scenarios, max-binding -- headline capital]")
    print(f"  Binding scenario: {breakdown_max['binding_scenario']}")
    print(f"  Total charge:     {total_max:,.2f}")
    print("  Scenario SBA charges:")
    for name, amt in sorted(breakdown_max["scenario_charges"].items()):
        print(f"    {name:<8} {amt:>14,.2f}")


def run_saccr_irs() -> None:
    """Build a vanilla 5Y pay-fixed USD IRS and compute SA-CCR EAD."""
    print()
    print("=" * 72)
    print("SA-CCR -- single interest-rate swap")
    print("=" * 72)

    # 100M notional, 5Y maturity, +$2.5M MTM (receiver is in the money).
    trade = SaCcrTrade(
        trade_id="IRS-001",
        asset_class="ir",
        notional=100_000_000.0,
        start_year=2024,
        start_month=1,
        start_day=15,
        end_year=2029,
        end_month=1,
        end_day=15,
        underlier="USD",
        hedging_set="USD-IR",
        direction=1.0,
        mtm=2_500_000.0,
    )

    # Unmargined netting set, zero collateral held.
    rc, pfe, ead = saccr_ead([trade], margined=False, collateral=0.0)
    print("\n[Unmargined, no collateral]")
    print(f"  Trade:            {trade}")
    print(f"  Replacement cost: {rc:,.2f}")
    print(f"  PFE:              {pfe:,.2f}")
    print(f"  EAD (alpha=1.4):  {ead:,.2f}")

    # Margined netting set at 10-day MPoR.
    rc_m, pfe_m, ead_m = saccr_ead([trade], margined=True, collateral=0.0)
    print("\n[Margined (10-day MPoR), no collateral]")
    print(f"  Replacement cost: {rc_m:,.2f}")
    print(f"  PFE:              {pfe_m:,.2f}")
    print(f"  EAD:              {ead_m:,.2f}")
    print(f"  Margining impact: EAD reduction = {ead - ead_m:,.2f}")


def main() -> None:
    sens = build_frtb_portfolio()
    run_frtb_scenarios(sens)
    run_saccr_irs()


if __name__ == "__main__":
    main()
