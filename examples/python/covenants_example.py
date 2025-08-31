#!/usr/bin/env python3
"""Example demonstrating covenant management in finstack."""

from finstack import (
    Date,
    Money,
    Currency,
    USD,
    Frequency,
    Covenant,
    CovenantType,
    CovenantConsequence,
    CovenantEngine,
    CovenantBreach,
)


def main():
    print("=" * 60)
    print("Covenant Management Example")
    print("=" * 60)

    # Create a covenant engine
    engine = CovenantEngine()

    # Define covenants
    leverage_covenant = Covenant(
        covenant_type=CovenantType("max_total_leverage", threshold=3.5),
        test_frequency=Frequency.Quarterly,
        cure_period_days=60,
    )

    # Add consequence for leverage breach
    leverage_consequence = CovenantConsequence("rate_increase", bp_increase=100)
    leverage_covenant.add_consequence(leverage_consequence)

    dscr_covenant = Covenant(
        covenant_type=CovenantType("min_interest_coverage", threshold=1.25),
        test_frequency=Frequency.Quarterly,
        cure_period_days=45,
    )

    # Add consequence for DSCR breach
    dscr_consequence = CovenantConsequence("cash_sweep", sweep_percentage=0.5)
    dscr_covenant.add_consequence(dscr_consequence)

    reporting_covenant = Covenant(
        covenant_type=CovenantType("affirmative", requirement="Monthly reporting"),
        test_frequency=Frequency.Monthly,
        cure_period_days=10,
    )

    # Add consequence for reporting breach
    reporting_consequence = CovenantConsequence("default")
    reporting_covenant.add_consequence(reporting_consequence)

    # Add covenants to engine
    engine.add_covenant(leverage_covenant)
    engine.add_covenant(dscr_covenant)
    engine.add_covenant(reporting_covenant)

    print("Added 3 covenants to engine")
    print()

    # Create covenant breaches
    leverage_breach = CovenantBreach(
        covenant_type="MaxTotalLeverage",
        breach_date=Date(2024, 3, 31),
        actual_value=4.2,
        threshold=3.5,
        cure_deadline=Date(2024, 5, 30),  # 60 days after breach
    )

    dscr_breach = CovenantBreach(
        covenant_type="MinInterestCoverage",
        breach_date=Date(2024, 3, 31),
        actual_value=1.15,
        threshold=1.25,
        cure_deadline=Date(2024, 5, 15),  # 45 days after breach
    )

    # Record breaches
    engine.add_breach(leverage_breach)
    engine.add_breach(dscr_breach)

    print("Recorded covenant breaches:")
    print(f"  - MaxTotalLeverage: 4.2 vs 3.5 (threshold)")
    print(f"  - MinInterestCoverage: 1.15 vs 1.25 (threshold)")
    print()

    # Simulate curing DSCR breach
    cured_dscr_breach = CovenantBreach(
        covenant_type="MinInterestCoverage",
        breach_date=Date(2024, 4, 15),
        actual_value=1.35,  # Now above threshold
        threshold=1.25,
        cure_deadline=Date(2024, 5, 15),
    )
    # Mark as cured
    cured_dscr_breach.mark_cured()
    engine.add_breach(cured_dscr_breach)
    print("DSCR breach cured - interest coverage now at 1.35")

    # Evaluate covenants (simplified demonstration)
    print()
    print("Covenant evaluation would check current metrics against thresholds")
    print("and apply consequences based on breach status and cure periods.")

    print()
    print("Example completed successfully!")


if __name__ == "__main__":
    main()
