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

from finstack.portfolio import NettingSet, NettingSetId


def main() -> None:
    # Create netting sets (group positions for margin aggregation).
    bilateral_id = NettingSetId.bilateral(counterparty_id="JPM", csa_id="BILATERAL-001")
    cleared_id = NettingSetId.cleared(ccp_id="LCH")

    bilateral = NettingSet(bilateral_id)
    cleared = NettingSet(cleared_id)

    # Attach some (example) position ids.
    bilateral.add_position("POS-IRS-1")
    bilateral.add_position("POS-FX-1")
    cleared.add_position("POS-IRS-CCP-1")

    # Note: Actual margin calculation requires marginable positions
    # See finstack-py/examples/portfolio/margin_example.py for full workflow


if __name__ == "__main__":
    main()
