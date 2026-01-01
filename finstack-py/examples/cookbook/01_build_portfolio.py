"""Title: Multi-Asset Portfolio Construction
Persona: Portfolio Manager
Complexity: Beginner
Runtime: ~2 seconds.

Description:
Demonstrates building a multi-asset portfolio with:
- Multiple entities (companies, funds)
- Book hierarchy (Americas > Credit > IG/HY)
- Diverse instruments (bonds, deposits, swaps)
- Position tags for grouping and analysis

Key Concepts:
- Portfolio construction with PortfolioBuilder
- Entity and book organization
- Position tagging for analysis
- Valuation with FX conversion

Prerequisites:
- Basic understanding of portfolio management
- Familiarity with fixed income instruments
"""

from finstack import (
    Bond,
    Book,
    BookId,
    Date,
    Deposit,
    DiscountCurve,
    Entity,
    FxMatrix,
    InterestRateSwap,
    MarketContext,
    Money,
    PortfolioBuilder,
)


def create_market_data():
    """Create market context with curves and FX rates."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))

    # USD OIS discount curve (flat for simplicity)
    usd_curve = DiscountCurve.flat(id="USD.OIS", base_date=Date(2024, 1, 15), rate=0.0475, day_count="Act360")
    market.insert_discount(usd_curve)

    # EUR OIS discount curve
    eur_curve = DiscountCurve.flat(id="EUR.OIS", base_date=Date(2024, 1, 15), rate=0.0325, day_count="Act360")
    market.insert_discount(eur_curve)

    # GBP OIS discount curve
    gbp_curve = DiscountCurve.flat(id="GBP.OIS", base_date=Date(2024, 1, 15), rate=0.0455, day_count="Act360")
    market.insert_discount(gbp_curve)

    # FX rates (vs USD)
    fx = FxMatrix()
    fx.set_spot("USD", "EUR", 0.92)
    fx.set_spot("USD", "GBP", 0.79)
    market.set_fx_matrix(fx)

    return market


def create_instruments():
    """Create diverse instrument portfolio."""
    instruments = []

    # 1. Corporate bonds
    # Investment grade bond
    ig_bond = Bond.fixed_semiannual(
        id="ACME.5Y.IG",
        notional=Money.from_code(5_000_000, "USD"),
        coupon_rate=0.045,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),
        discount_curve_id="USD.OIS",
    )
    instruments.append((
        "ACME.5Y.IG",
        ig_bond,
        {"asset_class": "Fixed Income", "rating": "BBB", "sector": "Technology", "region": "Americas"},
    ))

    # High yield bond
    hy_bond = Bond.fixed_quarterly(
        id="SPECTRE.3Y.HY",
        notional=Money.from_code(2_000_000, "USD"),
        coupon_rate=0.085,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2027, 1, 15),
        discount_curve_id="USD.OIS",
    )
    instruments.append((
        "SPECTRE.3Y.HY",
        hy_bond,
        {"asset_class": "Fixed Income", "rating": "BB", "sector": "Energy", "region": "Americas"},
    ))

    # EUR corporate bond
    eur_bond = Bond.fixed_annual(
        id="EUROTECH.7Y",
        notional=Money.from_code(3_000_000, "EUR"),
        coupon_rate=0.035,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2031, 1, 15),
        discount_curve_id="EUR.OIS",
    )
    instruments.append((
        "EUROTECH.7Y",
        eur_bond,
        {"asset_class": "Fixed Income", "rating": "A", "sector": "Technology", "region": "Europe"},
    ))

    # 2. Deposits
    usd_deposit = Deposit(
        id="USD.DEPOSIT.3M",
        notional=Money.from_code(10_000_000, "USD"),
        rate=0.0525,
        start_date=Date(2024, 1, 15),
        maturity_date=Date(2024, 4, 15),
        day_count="Act360",
        discount_curve_id="USD.OIS",
    )
    instruments.append((
        "USD.DEPOSIT.3M",
        usd_deposit,
        {"asset_class": "Cash", "rating": "AAA", "sector": "Cash Management", "region": "Americas"},
    ))

    # 3. Interest rate swap (receiver - receiving fixed, paying float)
    irs = InterestRateSwap.fixed_vs_float(
        id="USD.IRS.5Y.RECEIVER",
        notional=Money.from_code(15_000_000, "USD"),
        fixed_rate=0.045,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),
        pay_fixed=False,  # Receiver swap
        discount_curve_id="USD.OIS",
        forward_curve_id="USD.OIS",
    )
    instruments.append((
        "USD.IRS.5Y.RECEIVER",
        irs,
        {"asset_class": "Derivatives", "rating": "N/A", "sector": "Interest Rate", "region": "Americas"},
    ))

    return instruments


def main() -> None:
    """Build and value multi-asset portfolio."""
    # 1. Create entities
    main_fund = Entity(
        id="FUND-001",
        name="Global Multi-Asset Fund",
        tags={"fund_type": "multi-asset", "strategy": "core", "aum": "500M"},
    )

    sub_fund = Entity(
        id="FUND-002",
        name="Credit Opportunities Sub-Fund",
        tags={"fund_type": "credit", "strategy": "opportunistic", "aum": "100M"},
    )

    # 2. Create book hierarchy
    # Americas > Credit > IG/HY structure
    americas_book = Book(id=BookId("AMERICAS"), name="Americas Portfolio")

    credit_book = Book(id=BookId("CREDIT"), name="Credit Portfolio", parent_id=BookId("AMERICAS"))

    ig_book = Book(id=BookId("IG"), name="Investment Grade", parent_id=BookId("CREDIT"))

    hy_book = Book(id=BookId("HY"), name="High Yield", parent_id=BookId("CREDIT"))

    cash_book = Book(id=BookId("CASH"), name="Cash Management", parent_id=BookId("AMERICAS"))

    derivatives_book = Book(id=BookId("DERIVATIVES"), name="Derivatives", parent_id=BookId("AMERICAS"))

    # 3. Create instruments
    instruments = create_instruments()

    # 4. Build portfolio
    builder = PortfolioBuilder()
    builder.base_currency("USD")
    builder.as_of(Date(2024, 1, 15))

    # Add entities
    builder.entity(main_fund)
    builder.entity(sub_fund)

    # Add books
    builder.books([americas_book, credit_book, ig_book, hy_book, cash_book, derivatives_book])

    # Add positions with book assignments
    book_assignments = {
        "ACME.5Y.IG": BookId("IG"),
        "SPECTRE.3Y.HY": BookId("HY"),
        "EUROTECH.7Y": BookId("IG"),
        "USD.DEPOSIT.3M": BookId("CASH"),
        "USD.IRS.5Y.RECEIVER": BookId("DERIVATIVES"),
    }

    for pos_id, instrument, tags in instruments:
        builder.position(
            id=pos_id,
            instrument=instrument,
            entity_id=main_fund.id,
            quantity=1.0,  # Notional already in instrument
            tags=tags,
        )

        # Assign to book
        if pos_id in book_assignments:
            builder.add_position_to_book(pos_id, book_assignments[pos_id])

    portfolio = builder.build()

    # 5. Create market data
    market = create_market_data()

    # 6. Value portfolio
    from finstack import value_portfolio

    result = value_portfolio(portfolio, market, None)

    # 7. Display results

    for _pos_val in result.position_values:
        pass

    for entity_id in result.entity_totals:
        next(e for e in portfolio.entities() if e.id == entity_id)

    # 8. Book aggregation
    from finstack import aggregate_by_book

    book_totals = aggregate_by_book(result, market.fx_matrix(), Date(2024, 1, 15))
    for book_id in book_totals:
        book = next(b for b in portfolio.books() if b.id == book_id)
        "  " * (0 if not book.parent_id else (1 if book.parent_id.id == "AMERICAS" else 2))

    # 9. Tag-based aggregation
    from finstack import aggregate_by_attribute

    # By asset class
    asset_class_totals = aggregate_by_attribute(result, portfolio.positions(), "asset_class", "USD")
    for _asset_class, _value in asset_class_totals.items():
        pass

    # By rating
    rating_totals = aggregate_by_attribute(result, portfolio.positions(), "rating", "USD")
    for _rating, _value in rating_totals.items():
        pass

    # By region
    region_totals = aggregate_by_attribute(result, portfolio.positions(), "region", "USD")
    for _region, _value in region_totals.items():
        pass

    # 10. Export to DataFrame
    result.to_polars()

    # Can write to CSV, Parquet, etc.
    # df.write_csv("portfolio_valuation.csv")
    # df.write_parquet("portfolio_valuation.parquet")


if __name__ == "__main__":
    main()
