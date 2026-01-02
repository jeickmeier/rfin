"""Title: Portfolio Books (Hierarchy + Aggregation)
Persona: Portfolio Manager / Analyst
Complexity: Beginner
Runtime: ~2 seconds.

Description:
Demonstrates portfolio books (hierarchical organization):
- Build a small book tree (Americas > Credit)
- Assign positions to books
- Aggregate valuation totals by book (recursive rollup)

Notes:
- This script is intentionally thin: it only constructs inputs and calls Rust-backed APIs.
"""

from __future__ import annotations

from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxMatrix
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.money import Money
from finstack.portfolio import (
    Book,
    Entity,
    PortfolioBuilder,
    Position,
    PositionUnit,
    aggregate_by_book,
    value_portfolio,
)
from finstack.valuations.instruments import Deposit


def create_simple_market(as_of: date) -> MarketContext:
    market = MarketContext()
    market.insert_discount(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))
    fx = FxMatrix()
    fx.set_quote(Currency("USD"), Currency("USD"), 1.0)
    market.insert_fx(fx)
    return market


def main() -> None:
    as_of = date(2024, 1, 15)

    # Books: Americas > Credit
    americas = Book("americas", name="Americas")
    credit = Book("credit", name="Credit", parent_id="americas")

    fund = Entity("FUND-BOOKS").with_name("Books Demo Fund")

    dep_3m = Deposit(
        "USD.DEPOSIT.3M",
        Money(5_000_000, "USD"),
        as_of,
        date(2024, 4, 15),
        DayCount.ACT_360,
        "USD-OIS",
        quote_rate=0.05,
    )
    dep_6m = Deposit(
        "USD.DEPOSIT.6M",
        Money(3_000_000, "USD"),
        as_of,
        date(2024, 7, 15),
        DayCount.ACT_360,
        "USD-OIS",
        quote_rate=0.05,
    )

    pos_3m = Position("POS-DEP-3M", fund.id, dep_3m.instrument_id, dep_3m, 1.0, PositionUnit.UNITS).with_book(
        "credit"
    )
    pos_6m = Position("POS-DEP-6M", fund.id, dep_6m.instrument_id, dep_6m, 1.0, PositionUnit.UNITS).with_book(
        "americas"
    )

    portfolio = (
        PortfolioBuilder("BOOKS_DEMO")
        .name("Books Demo")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(fund)
        .position([pos_3m, pos_6m])
        .book([americas, credit])
        .build()
    )

    market = create_simple_market(as_of)
    valuation = value_portfolio(portfolio, market)
    by_book = aggregate_by_book(valuation, portfolio)

    print("Books in portfolio:")
    for book_id, book in portfolio.books.items():
        print(f"  - {book_id}: name={book.name!r}, parent_id={book.parent_id!r}, children={book.child_book_ids}")

    print("\nAggregated value by book (recursive rollup):")
    for book_id, total in by_book.items():
        print(f"  - {book_id}: {total}")


if __name__ == "__main__":
    main()
