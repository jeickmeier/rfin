#!/usr/bin/env python3
"""Factor-model portfolio example for finstack.portfolio.factor_model.

Run with:
    uv run python finstack-py/examples/scripts/portfolio/factor_model_example.py
"""

from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit
from finstack.portfolio.factor_model import (
    AttributeFilter,
    DependencyFilter,
    FactorCovarianceMatrix,
    FactorDefinition,
    FactorModelBuilder,
    FactorModelConfig,
    MappingRule,
    MarketMapping,
    MatchingConfig,
    PositionChange,
)
from finstack.valuations.instruments import Deposit


AS_OF = date(2024, 1, 2)


def build_market() -> MarketContext:
    market = MarketContext()
    market.insert(
        DiscountCurve(
            "USD-OIS",
            AS_OF,
            [
                (0.0, 1.0),
                (0.5, 0.9975),
                (1.0, 0.9950),
                (3.0, 0.9750),
                (5.0, 0.9500),
            ],
        )
    )
    return market


def build_portfolio():
    entity = Entity.dummy()
    deposit = (
        Deposit.builder("DEP-USD")
        .money(Money(1_000_000, Currency("USD")))
        .start(AS_OF)
        .maturity(date(2024, 7, 2))
        .day_count(DayCount.ACT_360)
        .disc_id("USD-OIS")
        .quote_rate(0.05)
        .build()
    )
    position = Position(
        "POS-DEP-1",
        entity.id,
        deposit.instrument_id,
        deposit,
        1.0,
        PositionUnit.UNITS,
    )
    return (
        PortfolioBuilder("PORT-1")
        .name("Factor Model Example")
        .base_ccy(Currency("USD"))
        .as_of(AS_OF)
        .entity(entity)
        .position(position)
        .build()
    )


def build_model():
    config = FactorModelConfig(
        factors=[
            FactorDefinition(
                id="USD-Rates",
                factor_type="Rates",
                market_mapping=MarketMapping.curve_parallel(["USD-OIS"], units="bp"),
                description="USD discount curve factor",
            )
        ],
        covariance=FactorCovarianceMatrix(
            factor_ids=["USD-Rates"],
            matrix=[[0.04]],
        ),
        matching=MatchingConfig.mapping_table(
            [
                MappingRule(
                    dependency_filter=DependencyFilter(
                        dependency_type="Discount",
                        curve_type="Discount",
                    ),
                    attribute_filter=AttributeFilter(),
                    factor_id="USD-Rates",
                )
            ]
        ),
        pricing_mode="DeltaBased",
        risk_measure="Variance",
        unmatched_policy="Residual",
    )
    return FactorModelBuilder().config(config).build()


def main() -> None:
    market = build_market()
    portfolio = build_portfolio()
    model = build_model()

    assignments = model.assign_factors(portfolio)
    sensitivities = model.compute_sensitivities(portfolio, market, AS_OF)
    decomposition = model.analyze(portfolio, market, AS_OF)
    engine = model.what_if(decomposition, sensitivities, portfolio, market, AS_OF)
    resized = engine.position_what_if([PositionChange.resize("POS-DEP-1", 2.0)])

    print("Assignments:", len(assignments.assignments))
    print("Factors:", [factor.id for factor in model.factors()])
    print("Total risk:", decomposition.total_risk)
    print("Resized total risk:", resized.after.total_risk)


if __name__ == "__main__":
    main()
