#!/usr/bin/env python3
"""
Example demonstrating expected funding curves for DDTL and RCF pricing.

This shows how to add expected future draws to delayed-draw term loans
and revolving credit facilities for more accurate pricing and valuation.
"""

from datetime import date
from finstack import Date, Money, Currency


def get_amount_value(amount_obj):
    """Helper to extract numeric amount from Money object or float."""
    if hasattr(amount_obj, "amount"):
        # It's a Money object, check if amount is a method or property
        amount_attr = getattr(amount_obj, "amount")
        return amount_attr() if callable(amount_attr) else amount_attr
    else:
        # It's already a numeric value
        return amount_obj


from finstack.instruments import (
    DelayedDrawTermLoan,
    RevolvingCreditFacility,
    DrawEvent,
    ExpectedFundingCurve,
)


def demo_ddtl_with_expected_funding():
    """Demonstrate DDTL with expected future draws."""
    print("=" * 60)
    print("DELAYED-DRAW TERM LOAN WITH EXPECTED FUNDING CURVE")
    print("=" * 60)

    # Create a $50M DDTL commitment
    ddtl = DelayedDrawTermLoan(
        id="DDTL-ACME-001",
        commitment=Money(50_000_000, Currency("usd")),
        commitment_expiry=Date(2026, 12, 31),  # Can draw until end of 2026
        maturity=Date(2030, 12, 31),  # Final maturity in 2030
    )

    print(f"Initial DDTL: {ddtl}")
    print(f"  Undrawn: {ddtl.undrawn()}")

    # Simulate initial draw
    ddtl.draw(10_000_000)
    print(f"\nAfter initial $10M draw:")
    print(f"  Drawn: {ddtl.drawn}")
    print(f"  Undrawn: {ddtl.undrawn()}")

    # Add expected future draws for pricing
    expected_draws = [
        DrawEvent(
            date=Date(2025, 6, 1),
            amount=Money(15_000_000, Currency("usd")),
            purpose="Expansion project",
        ),
        DrawEvent(
            date=Date(2025, 12, 1),
            amount=Money(10_000_000, Currency("usd")),
            purpose="Working capital",
        ),
        DrawEvent(
            date=Date(2026, 6, 1),
            amount=Money(5_000_000, Currency("usd")),
            purpose="Equipment purchase",
        ),
    ]

    ddtl = ddtl.with_expected_draws(expected_draws)

    print(f"\nExpected funding curve added:")
    print(
        f"  Number of expected draws: {len(ddtl.expected_funding_curve.expected_draws)}"
    )
    for draw in ddtl.expected_funding_curve.expected_draws:
        amount_value = get_amount_value(draw.amount)
        print(f"    - {draw.date}: ${amount_value:,.0f} ({draw.purpose})")

    # Alternative: Create with probabilities
    print("\n" + "-" * 40)
    print("With draw probabilities:")

    funding_curve = ExpectedFundingCurve(
        expected_draws=[
            DrawEvent(
                Date(2025, 9, 1), Money(20_000_000, Currency("usd")), "Acquisition"
            ),
            DrawEvent(
                Date(2026, 3, 1), Money(10_000_000, Currency("usd")), "Contingent"
            ),
        ],
        draw_probabilities=[0.95, 0.60],  # 95% and 60% probability
    )

    ddtl_with_probs = DelayedDrawTermLoan(
        "DDTL-ACME-002",
        Money(50_000_000, Currency("usd")),
        Date(2026, 12, 31),
        Date(2030, 12, 31),
    ).with_expected_funding_curve(funding_curve)

    print(f"  Expected draws with probabilities:")
    for i, draw in enumerate(funding_curve.expected_draws):
        prob = funding_curve.draw_probabilities[i]
        amount_value = get_amount_value(draw.amount)
        print(f"    - {draw.date}: ${amount_value:,.0f} " f"@ {prob:.0%} probability")


def demo_rcf_with_expected_funding():
    """Demonstrate RCF with expected draws and repayments."""
    print("\n" + "=" * 60)
    print("REVOLVING CREDIT FACILITY WITH EXPECTED FUNDING CURVE")
    print("=" * 60)

    # Create a $100M revolving credit facility
    rcf = RevolvingCreditFacility(
        id="RCF-ACME-001",
        commitment=Money(100_000_000, Currency("usd")),
        availability_start=Date(2025, 1, 1),
        maturity=Date(2028, 12, 31),
    )

    print(f"Initial RCF: {rcf}")
    print(f"  Utilization: {rcf.utilization():.1%}")

    # Simulate initial activity
    rcf.draw(25_000_000)
    print(f"\nAfter $25M draw:")
    print(f"  Drawn: {rcf.drawn}")
    print(f"  Undrawn: {rcf.undrawn()}")
    print(f"  Utilization: {rcf.utilization():.1%}")

    # Add expected future funding activity
    # Positive amounts are draws, negative are repayments
    expected_events = [
        DrawEvent(
            date=Date(2025, 3, 1),
            amount=Money(20_000_000, Currency("usd")),  # Draw $20M
            purpose="Q1 seasonal needs",
        ),
        DrawEvent(
            date=Date(2025, 6, 1),
            amount=Money(-15_000_000, Currency("usd")),  # Repay $15M
            purpose="Q2 cash flow",
        ),
        DrawEvent(
            date=Date(2025, 9, 1),
            amount=Money(30_000_000, Currency("usd")),  # Draw $30M
            purpose="Inventory buildup",
        ),
        DrawEvent(
            date=Date(2025, 12, 1),
            amount=Money(-25_000_000, Currency("usd")),  # Repay $25M
            purpose="Year-end deleveraging",
        ),
        DrawEvent(
            date=Date(2026, 3, 1),
            amount=Money(15_000_000, Currency("usd")),  # Draw $15M
            purpose="Growth investment",
        ),
    ]

    rcf = rcf.with_expected_events(expected_events)

    print(f"\nExpected funding activity:")
    current_balance = 25_000_000  # Current drawn amount
    for event in rcf.expected_funding_curve.expected_draws:
        amount_value = get_amount_value(event.amount)
        if amount_value > 0:
            action = "Draw"
            amount = amount_value
        else:
            action = "Repay"
            amount = abs(amount_value)
        current_balance += amount_value
        utilization = (current_balance / 100_000_000) * 100

        print(
            f"  {event.date}: {action:5} ${amount:,.0f} -> "
            f"Balance: ${current_balance:,.0f} ({utilization:.1f}% util)"
        )


def demo_complex_funding_scenarios():
    """Demonstrate more complex funding scenarios."""
    print("\n" + "=" * 60)
    print("COMPLEX FUNDING SCENARIOS")
    print("=" * 60)

    # Scenario 1: Seasonal RCF with predictable pattern
    print("\n1. Seasonal Business RCF Pattern:")

    seasonal_events = []
    months = [3, 6, 9, 12]  # Quarterly pattern

    for year in [2025, 2026]:
        for i, month in enumerate(months):
            if i % 2 == 0:  # Q1 and Q3: draw for inventory
                amount = 30_000_000 if month == 9 else 20_000_000
                seasonal_events.append(
                    DrawEvent(
                        Date(year, month, 1),
                        Money(amount, Currency("usd")),
                        f"Q{i+1} inventory buildup",
                    )
                )
            else:  # Q2 and Q4: repay from sales
                amount = -25_000_000 if month == 12 else -15_000_000
                seasonal_events.append(
                    DrawEvent(
                        Date(year, month, 1),
                        Money(amount, Currency("usd")),
                        f"Q{i+1} collections",
                    )
                )

    seasonal_rcf = RevolvingCreditFacility(
        "RCF-SEASONAL-001",
        Money(75_000_000, Currency("usd")),
        Date(2025, 1, 1),
        Date(2027, 12, 31),
    ).with_expected_events(seasonal_events)

    print(f"  Created {len(seasonal_events)} expected events over 2 years")
    print("  Pattern: Draw in Q1/Q3, Repay in Q2/Q4")

    # Scenario 2: DDTL with decreasing probability draws
    print("\n2. DDTL with Uncertain Future Draws:")

    uncertain_draws = [
        DrawEvent(Date(2025, 6, 1), Money(10_000_000, Currency("usd")), "Phase 1"),
        DrawEvent(Date(2025, 12, 1), Money(15_000_000, Currency("usd")), "Phase 2"),
        DrawEvent(Date(2026, 6, 1), Money(20_000_000, Currency("usd")), "Phase 3"),
    ]

    # Probabilities decrease over time due to uncertainty
    probabilities = [0.90, 0.70, 0.40]

    uncertain_curve = ExpectedFundingCurve(uncertain_draws, probabilities)

    uncertain_ddtl = DelayedDrawTermLoan(
        "DDTL-UNCERTAIN-001",
        Money(50_000_000, Currency("usd")),
        Date(2027, 6, 30),
        Date(2032, 6, 30),
    ).with_expected_funding_curve(uncertain_curve)

    print("  Expected draws with decreasing probability:")
    expected_value = 0
    for i, (draw, prob) in enumerate(zip(uncertain_draws, probabilities)):
        amount_value = get_amount_value(draw.amount)
        expected_draw = amount_value * prob
        expected_value += expected_draw
        print(
            f"    {draw.purpose}: ${amount_value:,.0f} @ {prob:.0%} "
            f"= ${expected_draw:,.0f} expected"
        )
    print(f"  Total expected draws: ${expected_value:,.0f}")


def main():
    """Run all demonstrations."""
    demo_ddtl_with_expected_funding()
    demo_rcf_with_expected_funding()
    demo_complex_funding_scenarios()

    print("\n" + "=" * 60)
    print("SUMMARY")
    print("=" * 60)
    print(
        """
Expected funding curves enable more accurate pricing by incorporating:

1. Future Draw Expectations:
   - Planned capital calls
   - Seasonal working capital needs
   - Growth investments
   
2. Probability Weighting:
   - Risk-adjusted valuations
   - Scenario analysis
   - Contingent funding
   
3. Dynamic Utilization:
   - Better commitment fee calculations
   - Accurate interest projections
   - Utilization-based pricing

This improves valuation accuracy for both lenders and borrowers.
    """
    )


if __name__ == "__main__":
    main()
