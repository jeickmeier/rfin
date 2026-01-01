"""Title: Margin Calculation with CSA Terms
Persona: Risk Analyst
Complexity: Intermediate
Runtime: ~1 second.

Description:
Calculate initial and variation margin with netting.

Key Concepts:
- Netting set construction
- CSA terms (threshold, MTA, IM)
- Margin aggregation

Prerequisites:
- Portfolio basics
- Margin and collateral concepts
"""

from finstack import (
    Money,
    NettingSet,
    NettingSetId,
)


def main() -> None:

    # Create netting sets with margin terms
    NettingSet.bilateral(
        NettingSetId("BILATERAL-001", "JPM"),
        threshold=Money.from_code(1_000_000, "USD"),
        minimum_transfer_amount=Money.from_code(250_000, "USD"),
        initial_margin_pct=0.05,  # 5% IM
    )

    NettingSet.cleared(
        NettingSetId("CLEARED-001", "LCH"),
        threshold=Money.from_code(0, "USD"),  # No threshold for cleared
        minimum_transfer_amount=Money.from_code(0, "USD"),
        initial_margin_pct=0.10,  # 10% IM (higher for cleared)
    )

    # Note: Actual margin calculation requires marginable positions
    # See finstack-py/examples/portfolio/margin_example.py for full workflow


if __name__ == "__main__":
    main()
