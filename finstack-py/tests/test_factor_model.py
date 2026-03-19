from __future__ import annotations

from datetime import date

from finstack.core.currency import Currency
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.core.money import Money
from finstack.portfolio.factor_model import (
    AttributeFilter,
    DependencyFilter,
    FactorConstraint,
    FactorCovarianceMatrix,
    FactorDefinition,
    FactorModelBuilder,
    FactorModelConfig,
    FactorNode,
    HierarchicalConfig,
    MappingRule,
    MarketMapping,
    MatchingConfig,
    PositionChange,
)
from finstack.valuations.instruments import Deposit
import pytest

from finstack import FinstackError, ParameterError, ValidationError
from finstack.portfolio import Entity, Portfolio, PortfolioBuilder, Position, PositionUnit

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


def build_portfolio() -> Portfolio:
    entity = Entity.dummy()
    position = build_position(entity.id, "POS-DEP-1", "DEP-USD", 1.0)
    return (
        PortfolioBuilder("PORT-1")
        .name("Factor Model Test Portfolio")
        .base_ccy(Currency("USD"))
        .as_of(AS_OF)
        .entity(entity)
        .position(position)
        .build()
    )


def build_position(entity_id: str, position_id: str, instrument_id: str, quantity: float) -> Position:
    deposit = (
        Deposit
        .builder(instrument_id)
        .money(Money(1_000_000, Currency("USD")))
        .start(AS_OF)
        .maturity(date(2024, 7, 2))
        .day_count(DayCount.ACT_360)
        .disc_id("USD-OIS")
        .quote_rate(0.05)
        .build()
    )
    return Position(
        position_id,
        entity_id,
        instrument_id,
        deposit,
        quantity,
        PositionUnit.UNITS,
    )


def build_config() -> FactorModelConfig:
    return FactorModelConfig(
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
        matching=MatchingConfig.mapping_table([
            MappingRule(
                dependency_filter=DependencyFilter(
                    dependency_type="Discount",
                    curve_type="Discount",
                ),
                attribute_filter=AttributeFilter(),
                factor_id="USD-Rates",
            )
        ]),
        pricing_mode="DeltaBased",
        risk_measure="Variance",
        unmatched_policy="Residual",
    )


def test_factor_config_construction_and_json_roundtrip() -> None:
    config = build_config()

    assert len(config.factors) == 1
    assert config.factors[0].id == "USD-Rates"
    assert config.factors[0].factor_type == "Rates"

    payload = config.to_json()
    roundtrip = FactorModelConfig.from_json(payload)
    assert roundtrip.factors[0].id == "USD-Rates"


def test_covariance_matrix_rejects_invalid_input() -> None:
    with pytest.raises(ValidationError, match="positive semi-definite"):
        FactorCovarianceMatrix(
            factor_ids=["Rates", "Credit"],
            matrix=[[1.0, 3.0], [3.0, 1.0]],
        )


def test_matching_config_supports_mapping_cascade_and_hierarchical() -> None:
    mapping = MatchingConfig.mapping_table([
        MappingRule(
            dependency_filter=DependencyFilter(dependency_type="Discount"),
            attribute_filter=AttributeFilter(),
            factor_id="USD-Rates",
        )
    ])
    cascade = MatchingConfig.cascade([mapping])
    hierarchical = MatchingConfig.hierarchical(
        HierarchicalConfig(
            dependency_filter=DependencyFilter(dependency_type="Credit"),
            root=FactorNode(
                factor_id="NA-Credit",
                filter=AttributeFilter(meta=[("region", "NA")]),
                children=[],
            ),
        )
    )

    assert "mapping_table" in mapping.to_json()
    assert "cascade" in cascade.to_json()
    assert "hierarchical" in hierarchical.to_json()


def test_factor_model_analysis_and_assignment_flow() -> None:
    portfolio = build_portfolio()
    market = build_market()
    model = FactorModelBuilder().config(build_config()).build()

    assignments = model.assign_factors(portfolio)
    assert len(assignments.assignments) == 1
    assert assignments.assignments[0].position_id == "POS-DEP-1"
    assert assignments.unmatched == []

    sensitivities = model.compute_sensitivities(portfolio, market, AS_OF)
    assert sensitivities.n_positions() == 1
    assert sensitivities.n_factors() == 1
    assert sensitivities.position_ids() == ["POS-DEP-1"]
    assert sensitivities.factor_ids() == ["USD-Rates"]

    decomposition = model.analyze(portfolio, market, AS_OF)
    assert decomposition.total_risk >= 0.0
    assert decomposition.measure == "Variance"
    assert len(decomposition.factor_contributions) == 1
    assert decomposition.factor_contributions[0].factor_id == "USD-Rates"


def test_what_if_resize_remove_stress_and_unsupported_paths() -> None:
    portfolio = build_portfolio()
    market = build_market()
    model = FactorModelBuilder().config(build_config()).build()
    base = model.analyze(portfolio, market, AS_OF)
    sensitivities = model.compute_sensitivities(portfolio, market, AS_OF)
    engine = model.what_if(base, sensitivities, portfolio, market, AS_OF)

    resized = engine.position_what_if([PositionChange.resize("POS-DEP-1", 2.0)])
    assert resized.after.total_risk >= resized.before.total_risk
    assert resized.delta[0].factor_id == "USD-Rates"

    removed = engine.position_what_if([PositionChange.remove("POS-DEP-1")])
    assert removed.after.total_risk == pytest.approx(0.0)

    stressed = engine.factor_stress([("USD-Rates", 1.0)])
    assert len(stressed.position_pnl) == 1
    assert stressed.stressed_decomposition.total_risk >= 0.0

    new_position = build_position("X", "POS-NEW", "DEP-USD-NEW", 1.0)
    with pytest.raises(ParameterError, match="PositionChange::Add is not supported yet"):
        engine.position_what_if([PositionChange.add(new_position)])

    with pytest.raises(FinstackError, match="Factor-constrained optimization is not supported yet"):
        engine.optimize([FactorConstraint.factor_neutral("USD-Rates")])
