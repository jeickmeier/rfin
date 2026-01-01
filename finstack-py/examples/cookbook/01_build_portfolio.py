"""
Title: Multi-Asset Portfolio Construction
Persona: Portfolio Manager
Complexity: Beginner
Runtime: ~2 seconds

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
    PortfolioBuilder,
    Entity,
    Book,
    BookId,
    Bond,
    Deposit,
    InterestRateSwap,
    Money,
    Date,
    MarketContext,
    DiscountCurve,
    FxMatrix,
    create_standard_registry,
)


def create_market_data():
    """Create market context with curves and FX rates."""
    market = MarketContext()
    market.set_as_of(Date(2024, 1, 15))
    
    # USD OIS discount curve (flat for simplicity)
    usd_curve = DiscountCurve.flat(
        id="USD.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.0475,
        day_count="Act360"
    )
    market.insert_discount(usd_curve)
    
    # EUR OIS discount curve
    eur_curve = DiscountCurve.flat(
        id="EUR.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.0325,
        day_count="Act360"
    )
    market.insert_discount(eur_curve)
    
    # GBP OIS discount curve
    gbp_curve = DiscountCurve.flat(
        id="GBP.OIS",
        base_date=Date(2024, 1, 15),
        rate=0.0455,
        day_count="Act360"
    )
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
        discount_curve_id="USD.OIS"
    )
    instruments.append(("ACME.5Y.IG", ig_bond, {
        "asset_class": "Fixed Income",
        "rating": "BBB",
        "sector": "Technology",
        "region": "Americas"
    }))
    
    # High yield bond
    hy_bond = Bond.fixed_quarterly(
        id="SPECTRE.3Y.HY",
        notional=Money.from_code(2_000_000, "USD"),
        coupon_rate=0.085,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2027, 1, 15),
        discount_curve_id="USD.OIS"
    )
    instruments.append(("SPECTRE.3Y.HY", hy_bond, {
        "asset_class": "Fixed Income",
        "rating": "BB",
        "sector": "Energy",
        "region": "Americas"
    }))
    
    # EUR corporate bond
    eur_bond = Bond.fixed_annual(
        id="EUROTECH.7Y",
        notional=Money.from_code(3_000_000, "EUR"),
        coupon_rate=0.035,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2031, 1, 15),
        discount_curve_id="EUR.OIS"
    )
    instruments.append(("EUROTECH.7Y", eur_bond, {
        "asset_class": "Fixed Income",
        "rating": "A",
        "sector": "Technology",
        "region": "Europe"
    }))
    
    # 2. Deposits
    usd_deposit = Deposit(
        id="USD.DEPOSIT.3M",
        notional=Money.from_code(10_000_000, "USD"),
        rate=0.0525,
        start_date=Date(2024, 1, 15),
        maturity_date=Date(2024, 4, 15),
        day_count="Act360",
        discount_curve_id="USD.OIS"
    )
    instruments.append(("USD.DEPOSIT.3M", usd_deposit, {
        "asset_class": "Cash",
        "rating": "AAA",
        "sector": "Cash Management",
        "region": "Americas"
    }))
    
    # 3. Interest rate swap (receiver - receiving fixed, paying float)
    irs = InterestRateSwap.fixed_vs_float(
        id="USD.IRS.5Y.RECEIVER",
        notional=Money.from_code(15_000_000, "USD"),
        fixed_rate=0.045,
        issue_date=Date(2024, 1, 15),
        maturity_date=Date(2029, 1, 15),
        pay_fixed=False,  # Receiver swap
        discount_curve_id="USD.OIS",
        forward_curve_id="USD.OIS"
    )
    instruments.append(("USD.IRS.5Y.RECEIVER", irs, {
        "asset_class": "Derivatives",
        "rating": "N/A",
        "sector": "Interest Rate",
        "region": "Americas"
    }))
    
    return instruments


def main():
    """Build and value multi-asset portfolio."""
    print("="*80)
    print("COOKBOOK EXAMPLE 01: Multi-Asset Portfolio Construction")
    print("="*80)
    print()
    
    # 1. Create entities
    print("1. Creating entities...")
    main_fund = Entity(
        id="FUND-001",
        name="Global Multi-Asset Fund",
        tags={
            "fund_type": "multi-asset",
            "strategy": "core",
            "aum": "500M"
        }
    )
    
    sub_fund = Entity(
        id="FUND-002",
        name="Credit Opportunities Sub-Fund",
        tags={
            "fund_type": "credit",
            "strategy": "opportunistic",
            "aum": "100M"
        }
    )
    print(f"  ✓ Created entity: {main_fund.name}")
    print(f"  ✓ Created entity: {sub_fund.name}")
    print()
    
    # 2. Create book hierarchy
    print("2. Creating book hierarchy...")
    # Americas > Credit > IG/HY structure
    americas_book = Book(
        id=BookId("AMERICAS"),
        name="Americas Portfolio"
    )
    
    credit_book = Book(
        id=BookId("CREDIT"),
        name="Credit Portfolio",
        parent_id=BookId("AMERICAS")
    )
    
    ig_book = Book(
        id=BookId("IG"),
        name="Investment Grade",
        parent_id=BookId("CREDIT")
    )
    
    hy_book = Book(
        id=BookId("HY"),
        name="High Yield",
        parent_id=BookId("CREDIT")
    )
    
    cash_book = Book(
        id=BookId("CASH"),
        name="Cash Management",
        parent_id=BookId("AMERICAS")
    )
    
    derivatives_book = Book(
        id=BookId("DERIVATIVES"),
        name="Derivatives",
        parent_id=BookId("AMERICAS")
    )
    
    print("  ✓ Book hierarchy:")
    print("    AMERICAS/")
    print("      ├── CREDIT/")
    print("      │   ├── IG/")
    print("      │   └── HY/")
    print("      ├── CASH/")
    print("      └── DERIVATIVES/")
    print()
    
    # 3. Create instruments
    print("3. Creating instruments...")
    instruments = create_instruments()
    print(f"  ✓ Created {len(instruments)} instruments")
    print()
    
    # 4. Build portfolio
    print("4. Building portfolio...")
    builder = PortfolioBuilder()
    builder.base_currency("USD")
    builder.as_of(Date(2024, 1, 15))
    
    # Add entities
    builder.entity(main_fund)
    builder.entity(sub_fund)
    
    # Add books
    builder.books([
        americas_book,
        credit_book,
        ig_book,
        hy_book,
        cash_book,
        derivatives_book
    ])
    
    # Add positions with book assignments
    book_assignments = {
        "ACME.5Y.IG": BookId("IG"),
        "SPECTRE.3Y.HY": BookId("HY"),
        "EUROTECH.7Y": BookId("IG"),
        "USD.DEPOSIT.3M": BookId("CASH"),
        "USD.IRS.5Y.RECEIVER": BookId("DERIVATIVES")
    }
    
    for pos_id, instrument, tags in instruments:
        builder.position(
            id=pos_id,
            instrument=instrument,
            entity_id=main_fund.id,
            quantity=1.0,  # Notional already in instrument
            tags=tags
        )
        
        # Assign to book
        if pos_id in book_assignments:
            builder.add_position_to_book(pos_id, book_assignments[pos_id])
    
    portfolio = builder.build()
    print(f"  ✓ Portfolio built with {len(portfolio.positions())} positions")
    print(f"  ✓ {len(portfolio.entities())} entities")
    print(f"  ✓ {len(portfolio.books())} books")
    print()
    
    # 5. Create market data
    print("5. Creating market data...")
    market = create_market_data()
    print("  ✓ Market data ready")
    print()
    
    # 6. Value portfolio
    print("6. Valuing portfolio...")
    from finstack import value_portfolio
    
    result = value_portfolio(portfolio, market, None)
    print(f"  ✓ Portfolio valued")
    print()
    
    # 7. Display results
    print("7. Portfolio Summary")
    print("="*80)
    print(f"Total Value (USD): ${result.total.amount:,.2f}")
    print()
    
    print("Position Breakdown:")
    print("-"*80)
    print(f"{'Position ID':<25} {'Value (Native)':<20} {'Value (USD)':<20}")
    print("-"*80)
    for pos_val in result.position_values:
        native_val = pos_val.native_value
        base_val = pos_val.base_value
        print(f"{pos_val.position_id:<25} "
              f"{native_val.amount:>12,.2f} {native_val.currency.code:<6} "
              f"{base_val.amount:>12,.2f} {base_val.currency.code:<6}")
    print("-"*80)
    print()
    
    print("Entity Breakdown:")
    print("-"*80)
    for entity_id, value in result.entity_totals.items():
        entity = next(e for e in portfolio.entities() if e.id == entity_id)
        print(f"{entity.name:<40} ${value.amount:>15,.2f}")
    print("-"*80)
    print()
    
    # 8. Book aggregation
    print("8. Book Aggregation")
    print("="*80)
    from finstack import aggregate_by_book
    
    book_totals = aggregate_by_book(result, market.fx_matrix(), Date(2024, 1, 15))
    print("Book-level totals (includes child books):")
    print("-"*80)
    for book_id, value in book_totals.items():
        book = next(b for b in portfolio.books() if b.id == book_id)
        indent = "  " * (0 if not book.parent_id else (1 if book.parent_id.id == "AMERICAS" else 2))
        print(f"{indent}{book.name:<40} ${value.amount:>15,.2f}")
    print("-"*80)
    print()
    
    # 9. Tag-based aggregation
    print("9. Tag-Based Aggregation")
    print("="*80)
    from finstack import aggregate_by_attribute
    
    # By asset class
    print("By Asset Class:")
    print("-"*80)
    asset_class_totals = aggregate_by_attribute(
        result,
        portfolio.positions(),
        "asset_class",
        "USD"
    )
    for asset_class, value in asset_class_totals.items():
        print(f"{asset_class:<40} ${value.amount:>15,.2f}")
    print()
    
    # By rating
    print("By Rating:")
    print("-"*80)
    rating_totals = aggregate_by_attribute(
        result,
        portfolio.positions(),
        "rating",
        "USD"
    )
    for rating, value in rating_totals.items():
        print(f"{rating:<40} ${value.amount:>15,.2f}")
    print()
    
    # By region
    print("By Region:")
    print("-"*80)
    region_totals = aggregate_by_attribute(
        result,
        portfolio.positions(),
        "region",
        "USD"
    )
    for region, value in region_totals.items():
        print(f"{region:<40} ${value.amount:>15,.2f}")
    print("-"*80)
    print()
    
    # 10. Export to DataFrame
    print("10. Export to DataFrame")
    print("="*80)
    df = result.to_polars()
    print(f"DataFrame shape: {df.shape}")
    print(f"Columns: {', '.join(df.columns)}")
    print()
    print("Sample rows:")
    print(df.head(3))
    print()
    
    # Can write to CSV, Parquet, etc.
    # df.write_csv("portfolio_valuation.csv")
    # df.write_parquet("portfolio_valuation.parquet")
    
    print("="*80)
    print("EXAMPLE COMPLETE")
    print("="*80)
    print()
    print("Key Takeaways:")
    print("- Portfolio supports multi-entity, multi-book structure")
    print("- Positions can be tagged for flexible grouping")
    print("- Book hierarchy enables multi-level aggregation")
    print("- FX conversion handled automatically")
    print("- DataFrame export for further analysis")


if __name__ == "__main__":
    main()
