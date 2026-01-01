"""Title: Multi-Asset Portfolio Construction
Persona: Portfolio Manager
Complexity: Beginner
Runtime: ~2 seconds.

Description:
Demonstrates building a multi-asset portfolio with:
- Multiple entities (companies, funds)
- Diverse instruments (bonds, deposits, swaps)
- Position tags for grouping and analysis

Key Concepts:
- Portfolio construction with PortfolioBuilder
- Entity and position organization
- Tag-based aggregation for analysis

Prerequisites:
- Basic understanding of portfolio management
- Familiarity with fixed income instruments
"""

from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxMatrix
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit, aggregate_by_attribute, value_portfolio
from finstack.valuations.instruments import Bond, Deposit


def create_market_data(as_of: date) -> MarketContext:
    """Create market context with curves and FX rates."""
    market = MarketContext()

    # USD OIS discount curve (flat for simplicity)
    market.insert_discount(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))

    # EUR OIS discount curve
    market.insert_discount(DiscountCurve("EUR-OIS", as_of, [(0.0, 1.0), (10.0, 0.72)]))

    # GBP OIS discount curve
    market.insert_discount(DiscountCurve("GBP-OIS", as_of, [(0.0, 1.0), (10.0, 0.68)]))

    # FX rates (vs USD)
    fx = FxMatrix()
    fx.set_quote(Currency("EUR"), Currency("USD"), 1.0 / 0.92)
    fx.set_quote(Currency("GBP"), Currency("USD"), 1.0 / 0.79)
    market.insert_fx(fx)

    return market


def create_positions(as_of: date, entity_id: str) -> list[Position]:
    """Create diverse instrument portfolio."""
    positions: list[Position] = []

    # 1. Corporate bonds
    # Investment grade bond
    ig_bond = Bond.fixed_semiannual(
        "ACME.5Y.IG",
        Money(5_000_000, "USD"),
        0.045,
        date(2024, 1, 15),
        date(2029, 1, 15),
        "USD-OIS",
    )
    positions.append(
        Position("POS-IG", entity_id, "ACME.5Y.IG", ig_bond, 1.0, PositionUnit.UNITS).with_tags(
            {
                "asset_class": "Fixed Income",
                "rating": "BBB",
                "sector": "Technology",
                "region": "Americas",
            }
        )
    )

    # High yield bond
    hy_bond = Bond.fixed_semiannual(
        "SPECTRE.3Y.HY",
        Money(2_000_000, "USD"),
        0.085,
        date(2024, 1, 15),
        date(2027, 1, 15),
        "USD-OIS",
    )
    positions.append(
        Position("POS-HY", entity_id, "SPECTRE.3Y.HY", hy_bond, 1.0, PositionUnit.UNITS).with_tags(
            {
                "asset_class": "Fixed Income",
                "rating": "BB",
                "sector": "Energy",
                "region": "Americas",
            }
        )
    )

    # EUR corporate bond
    eur_bond = Bond.fixed_semiannual(
        "EUROTECH.7Y",
        Money(3_000_000, "EUR"),
        0.035,
        date(2024, 1, 15),
        date(2031, 1, 15),
        "EUR-OIS",
    )
    positions.append(
        Position("POS-EUR-BOND", entity_id, "EUROTECH.7Y", eur_bond, 1.0, PositionUnit.UNITS).with_tags(
            {
                "asset_class": "Fixed Income",
                "rating": "A",
                "sector": "Technology",
                "region": "Europe",
            }
        )
    )

    # 2. Deposits
    usd_deposit = Deposit(
        "USD.DEPOSIT.3M",
        Money(10_000_000, "USD"),
        as_of,
        date(2024, 4, 15),
        DayCount.ACT_360,
        "USD-OIS",
        quote_rate=0.0525,
    )
    positions.append(
        Position("POS-DEPOSIT", entity_id, "USD.DEPOSIT.3M", usd_deposit, 1.0, PositionUnit.UNITS).with_tags(
            {
                "asset_class": "Cash",
                "rating": "AAA",
                "sector": "Cash Management",
                "region": "Americas",
            }
        )
    )

    return positions


def main() -> None:
    """Build and value multi-asset portfolio."""
    as_of = date(2024, 1, 15)

    # 1. Create entities
    main_fund = (
        Entity("FUND-001")
        .with_name("Global Multi-Asset Fund")
        .with_tags({"fund_type": "multi-asset", "strategy": "core", "aum": "500M"})
    )

    # 2. Create positions (tagged)
    positions = create_positions(as_of, main_fund.id)

    # 3. Build portfolio
    portfolio = (
        PortfolioBuilder("MULTI_ASSET_FUND")
        .name("Global Multi-Asset Fund")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(main_fund)
        .position(positions)
        .build()
    )

    # 4. Create market data
    market = create_market_data(as_of)

    # 5. Value portfolio
    valuation = value_portfolio(portfolio, market)

    # 6. Tag-based aggregation
    aggregate_by_attribute(valuation, portfolio, "asset_class")
    aggregate_by_attribute(valuation, portfolio, "rating")
    aggregate_by_attribute(valuation, portfolio, "region")

    # Can write to CSV, Parquet, etc.
    # df.write_csv("portfolio_valuation.csv")
    # df.write_parquet("portfolio_valuation.parquet")


if __name__ == "__main__":
    main()
