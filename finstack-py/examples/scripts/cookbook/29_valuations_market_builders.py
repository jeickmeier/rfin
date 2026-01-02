"""
Title: Valuations Market Builders (Quotes -> Instruments)
Persona: Quantitative Researcher
Complexity: Intermediate
Runtime: ~2 seconds

Description:
Use market quote schemas + BuildCtx to construct calibration-ready instruments.

Key Concepts:
- BuildCtx (as_of, notional, curve role mapping)
- RateQuote -> built instrument
- CDS tranche quote -> built instrument (with overrides)
"""

from __future__ import annotations

from datetime import date


def main() -> None:
    from finstack.valuations.conventions import CdsConventionKey, CdsDocClause, ConventionRegistry
    from finstack.valuations.market import (
        BuildCtx,
        CdsTrancheBuildOverrides,
        CdsTrancheQuote,
        RateQuote,
        build_cds_tranche_instrument,
        build_rate_instrument,
    )

    # Ensure global conventions are loaded
    _ = ConventionRegistry.global_instance()

    # Rates: deposit quote -> instrument
    ctx = BuildCtx(
        as_of=date(2024, 1, 2),
        notional=1_000_000.0,
        curve_ids={"discount": "USD-OIS", "forward": "USD-SOFR"},
    )
    q = RateQuote.deposit(
        id="USD-SOFR-DEP-1M",
        index="USD-SOFR-1M",
        pillar="1M",
        rate=0.0525,
    )
    inst = build_rate_instrument(q, ctx)
    print("rate instrument:", inst.id, inst.instrument_type.name)

    # Credit: CDS tranche quote -> instrument
    tranche_ctx = BuildCtx(
        as_of=date(2024, 1, 2),
        notional=100_000_000.0,
        curve_ids={"discount": "USD-OIS", "credit": "CDX.NA.IG"},
    )
    tranche_quote = CdsTrancheQuote.cds_tranche(
        id="CDX-IG-3-7",
        index="CDX.NA.IG",
        attachment=0.03,
        detachment=0.07,
        maturity=date(2029, 6, 20),
        upfront_pct=-2.5,
        running_spread_bp=500.0,
        convention=CdsConventionKey(currency="USD", doc_clause=CdsDocClause.ISDA_NA),
    )
    overrides = CdsTrancheBuildOverrides(series=42)
    tranche = build_cds_tranche_instrument(tranche_quote, tranche_ctx, overrides)
    print("tranche instrument:", tranche.id, tranche.instrument_type.name)


if __name__ == "__main__":
    main()
