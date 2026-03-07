#!/usr/bin/env python3
"""Portfolio optimization example using finstack.portfolio.

This example mirrors the Rust optimization example and integration test:

- Build a simple USD bond portfolio with rating tags (AAA/BBB/CCC)
- Maximize value-weighted average yield (YTM)
- Subject to a CCC exposure constraint (e.g. CCC <= 20% of portfolio)

Run with:
    uv run python finstack-py/examples/scripts/portfolio/portfolio_optimization_example.py
"""

from __future__ import annotations

from datetime import date

from finstack.core.market_data.context import MarketContext


def build_market_data(as_of: date) -> MarketContext:
    """Create a simple USD discount curve for optimization examples."""
    market = MarketContext()

    from finstack.core.market_data.term_structures import DiscountCurve

    usd_curve = DiscountCurve(
        "USD",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.99),
            (3.0, 0.96),
            (5.0, 0.93),
        ],
    )
    market.insert(usd_curve)

    return market


def build_bond_portfolio(as_of: date):
    """Build a small USD bond portfolio with rating tags."""
    from finstack.core.currency import Currency
    from finstack.core.money import Money
    from finstack.valuations.instruments import Bond

    from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit

    # 5-year horizon for all bonds
    maturity = date(as_of.year + 5, 1, 1)

    # All bonds priced off the same curve "USD" with explicit market clean prices.
    # Use par (100.0) for simplicity so coupon ordering drives YTM ordering.
    bond_aaa = (
        Bond.builder("BOND_AAA")
        .money(Money(1_000_000, "USD"))
        .issue(as_of)
        .maturity(maturity)
        .disc_id("USD")
        .coupon_rate(0.03)
        .quoted_clean_price(100.0)
        .build()
    )
    bond_bbb = (
        Bond.builder("BOND_BBB")
        .money(Money(1_000_000, "USD"))
        .issue(as_of)
        .maturity(maturity)
        .disc_id("USD")
        .coupon_rate(0.05)
        .quoted_clean_price(100.0)
        .build()
    )
    bond_ccc = (
        Bond.builder("BOND_CCC")
        .money(Money(1_000_000, "USD"))
        .issue(as_of)
        .maturity(maturity)
        .disc_id("USD")
        .coupon_rate(0.08)
        .quoted_clean_price(100.0)
        .build()
    )

    # Positions with rating tags
    pos_aaa = (
        Position(
            "POS_AAA",
            "FUND_A",
            "BOND_AAA",
            bond_aaa,
            1.0,
            PositionUnit.FACE_VALUE,
        )
        .with_tag("rating", "AAA")
        .with_tag("sector", "investment_grade")
    )

    pos_bbb = (
        Position(
            "POS_BBB",
            "FUND_A",
            "BOND_BBB",
            bond_bbb,
            1.0,
            PositionUnit.FACE_VALUE,
        )
        .with_tag("rating", "BBB")
        .with_tag("sector", "investment_grade")
    )

    pos_ccc = (
        Position(
            "POS_CCC",
            "FUND_A",
            "BOND_CCC",
            bond_ccc,
            1.0,
            PositionUnit.FACE_VALUE,
        )
        .with_tag("rating", "CCC")
        .with_tag("sector", "high_yield")
    )

    entity = Entity("FUND_A").with_name("Example Fund")

    portfolio = (
        PortfolioBuilder("BOND_FUND_OPT")
        .name("Credit Portfolio – Optimization Example")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(entity)
        .position([pos_aaa, pos_bbb, pos_ccc])
        .build()
    )

    portfolio.validate()
    return portfolio


def run_optimization_example() -> None:
    """Run the max-YTM / CCC-constrained optimization and print results."""
    from finstack.core.config import FinstackConfig

    from finstack.portfolio import optimize_max_yield_with_ccc_limit

    as_of = date(2025, 1, 1)
    market = build_market_data(as_of)
    portfolio = build_bond_portfolio(as_of)
    config = FinstackConfig()

    print("\n" + "=" * 80)
    print("Portfolio Optimization Example: Maximize YTM with CCC Limit")
    print("=" * 80)

    print(f"Portfolio ID: {portfolio.id}")
    print(f"As-of date : {portfolio.as_of}")
    print(f"Base ccy   : {portfolio.base_ccy}")
    print(f"Positions  : {len(portfolio.positions)}")

    # Optimize with CCC exposure capped at 20%
    result = optimize_max_yield_with_ccc_limit(
        portfolio,
        market,
        ccc_limit=0.20,
        strict_risk=False,
        config=config,
    )

    status = result["status"]
    objective = result["objective_value"]
    ccc_weight = result["ccc_weight"]
    optimal_weights = result["optimal_weights"]

    print("\nOptimization result:")
    print(f"  Label   : {result.get('label')}")
    print(f"  Status  : {status}")
    print(f"  YTM (objective value): {objective:.6f}")
    print(f"  CCC weight: {ccc_weight:.4f} ({ccc_weight * 100:.2f}%)")

    print("\nOptimal weights by position:")
    for pos_id, w in optimal_weights.items():
        print(f"  {pos_id}: {w:.4f}")


def main() -> None:
    """Entry point for the optimization example."""
    run_optimization_example()


if __name__ == "__main__":
    main()
