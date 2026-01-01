"""Title: Constrained Portfolio Optimization
Persona: Portfolio Manager
Complexity: Intermediate
Runtime: ~2 seconds.

Description:
Demonstrates portfolio optimization with rating constraints.

Key Concepts:
- Optimization problem setup
- Objective functions (maximize yield)
- Constraints (rating limits, concentration)
- Trade universe definition

Prerequisites:
- Portfolio construction basics
- Understanding of optimization
"""

from datetime import date

from finstack.core.currency import Currency
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit
from finstack.portfolio import optimize_max_yield_with_ccc_limit
from finstack.valuations.instruments import Bond


def main() -> None:
    # Build a small bond portfolio and run the built-in optimizer helper.
    # This keeps the example fast while exercising the real optimization engine.
    as_of = date(2024, 1, 15)

    market = MarketContext()
    market.insert_discount(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))

    fund = Entity("FUND-OPT-001").with_name("Optimization Demo Fund")

    bonds = [
        ("BOND.AAA.5Y", 0.040, "AAA"),
        ("BOND.AA.5Y", 0.045, "AA"),
        ("BOND.BBB.5Y", 0.050, "BBB"),
        ("BOND.BB.5Y", 0.070, "BB"),
        ("BOND.CCC.3Y", 0.100, "CCC"),
    ]

    positions = []
    for pos_id, coupon, rating in bonds:
        maturity = date(2029, 1, 15) if rating != "CCC" else date(2027, 1, 15)
        instrument = Bond.fixed_semiannual(
            pos_id,
            Money(10_000_000, "USD"),
            coupon,
            as_of,
            maturity,
            "USD-OIS",
        )
        positions.append(
            Position(pos_id, fund.id, pos_id, instrument, 1.0, PositionUnit.UNITS).with_tag("rating", rating)
        )

    portfolio = (
        PortfolioBuilder("OPTIMIZATION_DEMO")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(fund)
        .position(positions)
        .build()
    )

    # Optimize with a CCC exposure cap (by weight). Returns a small dict of results.
    _result = optimize_max_yield_with_ccc_limit(portfolio, market, ccc_limit=0.10)


if __name__ == "__main__":
    main()
