"""
Revolving Credit Example

Demonstrates pricing and analysis of revolving credit facilities.
"""

from datetime import date
from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.pricer import create_standard_registry


def create_market_data() -> MarketContext:
    """Create market data for revolving credit pricing."""
    val_date = date(2025, 1, 1)

    # Discount curve (times in years)
    disc_curve = DiscountCurve(
        "USD.SOFR",
        val_date,
        [(0.0, 1.0), (1.0, 0.95), (2.0, 0.90), (3.0, 0.85)],
    )

    market = MarketContext()
    market.insert_discount(disc_curve)

    return market


def example_fixed_rate_revolver():
    """Example: Fixed rate revolving credit facility."""
    print("\n" + "=" * 80)
    print("FIXED RATE REVOLVING CREDIT")
    print("=" * 80)
    
    
    revolver = RevolvingCredit.builder(
        instrument_id="REVOLVER_001",
        commitment_amount=Money(5000000.0, USD),
        drawn_amount=Money(2000000.0, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={"type": "fixed", "rate": 0.055},
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
        },
        draw_repay_spec={"deterministic": []},
        discount_curve="USD.SOFR",
    )
    
    print(f"\nInstrument: {revolver}")
    print(f"  Commitment: {revolver.commitment_amount}")
    print(f"  Drawn: {revolver.drawn_amount}")
    print(f"  Maturity: {revolver.maturity_date}")
    
    market = create_market_data()
    registry = create_standard_registry()
    result = registry.price(revolver, "discounting", market)
    
    print(f"\nPricing Results:")
    print(f"  Present Value: {result.value}")
    
    return revolver, result


def main():
    """Run revolving credit examples."""
    print("\n" + "=" * 80)
    print("REVOLVING CREDIT EXAMPLES")
    print("=" * 80)
    
    example_fixed_rate_revolver()
    
    print("\n" + "=" * 80)
    print("Examples completed!")
    print("=" * 80 + "\n")


if __name__ == "__main__":
    main()

