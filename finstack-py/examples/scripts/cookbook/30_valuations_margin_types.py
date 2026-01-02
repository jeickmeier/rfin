"""
Title: Valuations Margin Types (CSA / VM / IM)
Persona: Risk Analyst
Complexity: Beginner
Runtime: ~1 second

Description:
Construct CSA specifications and margin parameter objects from the valuations margin module.

Key Concepts:
- VmParameters / ImParameters
- EligibleCollateralSchedule
- CsaSpec
"""

from __future__ import annotations


def main() -> None:
    from finstack.core.money import Money
    from finstack.valuations.margin import (
        CsaSpec,
        EligibleCollateralSchedule,
        ImMethodology,
        ImParameters,
        MarginCallTiming,
        MarginTenor,
        VmParameters,
    )

    vm = VmParameters(
        threshold=Money(0.0, "USD"),
        mta=Money(500_000.0, "USD"),
        frequency=MarginTenor.DAILY,
        settlement_lag=1,
    )
    im = ImParameters(
        methodology=ImMethodology.SIMM,
        mpor_days=10,
        threshold=Money(50_000_000.0, "USD"),
        mta=Money(0.0, "USD"),
        segregated=True,
    )

    eligible = EligibleCollateralSchedule.bcbs_standard()
    timing = MarginCallTiming.regulatory_standard()
    csa = CsaSpec(
        id="USD-CSA-DEMO",
        base_currency="USD",
        vm_params=vm,
        im_params=im,
        eligible_collateral=eligible,
        call_timing=timing,
        collateral_curve_id="USD-OIS",
    )

    print("csa:", csa)
    print("requires_im:", csa.requires_im())
    print("vm_threshold:", csa.vm_threshold())
    print("im_threshold:", csa.im_threshold())


if __name__ == "__main__":
    main()
