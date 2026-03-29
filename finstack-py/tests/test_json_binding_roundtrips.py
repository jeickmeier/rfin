"""Direct JSON helper roundtrip tests for Python bindings."""

from __future__ import annotations

from datetime import date
import json

from finstack.core.currency import USD
from finstack.core.dates.periods import build_periods
from finstack.core.money import Money
from tests.fixtures.strategies import create_flat_market_context, create_test_bond

from finstack.portfolio import (
    Entity,
    NettingSetId,
    NettingSetMargin,
    PortfolioBuilder,
    PortfolioMarginResult,
    Position,
    PositionUnit,
    aggregate_cashflows,
    cashflows_to_base_by_period,
)
from finstack.scenarios import ApplicationReport, RollForwardReport


def _build_simple_portfolio() -> object:
    builder = PortfolioBuilder("JSON-ROUNDTRIP-PORT")
    builder.base_ccy(USD)
    builder.as_of(date(2024, 1, 1))

    entity = Entity("FUND-JSON").with_name("JSON Test Fund")
    builder.entity(entity)

    bond = create_test_bond(bond_id="JSON-BOND")
    position = Position(
        "POS-JSON-1",
        entity.id,
        bond.instrument_id,
        bond,
        1.0,
        PositionUnit.UNITS,
    )
    builder.position(position)
    return builder.build()


def test_portfolio_cashflows_json_roundtrip() -> None:
    market = create_flat_market_context()
    portfolio = _build_simple_portfolio()

    cashflows = aggregate_cashflows(portfolio, market)
    restored = cashflows.from_json(cashflows.to_json())

    assert restored.by_date.keys() == cashflows.by_date.keys()
    assert restored.by_position.keys() == cashflows.by_position.keys()
    assert len(restored.warnings) == len(cashflows.warnings)


def test_portfolio_cashflow_buckets_json_roundtrip() -> None:
    market = create_flat_market_context()
    portfolio = _build_simple_portfolio()
    cashflows = aggregate_cashflows(portfolio, market)
    periods = build_periods("2024Q1..2029Q4", None).periods

    buckets = cashflows_to_base_by_period(cashflows, market, USD, periods)
    restored = buckets.from_json(buckets.to_json())

    assert restored.by_period.keys() == buckets.by_period.keys()


def test_netting_set_margin_json_roundtrip() -> None:
    margin = NettingSetMargin(
        NettingSetId.bilateral("CP1", "CSA1"),
        date(2024, 6, 15),
        Money(5_000_000.0, USD),
        Money(1_000_000.0, USD),
        10,
        "Simm",
    )

    restored = NettingSetMargin.from_json(margin.to_json())

    assert restored.netting_set_id == str(NettingSetId.bilateral("CP1", "CSA1"))
    assert restored.position_count == margin.position_count
    assert restored.total_margin.amount == margin.total_margin.amount


def test_portfolio_margin_result_json_roundtrip() -> None:
    margin = NettingSetMargin(
        NettingSetId.cleared("LCH"),
        date(2024, 6, 15),
        Money(900_000.0, USD),
        Money(100_000.0, USD),
        5,
        "ClearingHouse",
    )
    margin_payload = json.loads(margin.to_json())
    payload = {
        "as_of": "2024-06-15",
        "base_currency": "USD",
        "total_initial_margin": {"amount": "900000", "currency": "USD"},
        "total_variation_margin": {"amount": "100000", "currency": "USD"},
        "total_margin": {"amount": "1000000", "currency": "USD"},
        "netting_sets": [margin_payload],
        "total_positions": 5,
        "positions_without_margin": 1,
        "degraded_positions": [{"position_id": "POS_9", "message": "missing VM source"}],
    }

    result = PortfolioMarginResult.from_json(json.dumps(payload))
    restored = PortfolioMarginResult.from_json(result.to_json())

    assert restored.total_positions == 5
    assert restored.positions_without_margin == 1
    assert restored.netting_set_count() == 1


def test_application_report_json_roundtrip() -> None:
    report = ApplicationReport.from_json(
        json.dumps({
            "operations_applied": 3,
            "warnings": ["rounded discount factor"],
            "rounding_context": "book-close-v1",
        })
    )
    restored = ApplicationReport.from_json(report.to_json())

    assert restored.operations_applied == 3
    assert restored.warnings == ["rounded discount factor"]
    assert restored.rounding_context == "book-close-v1"


def test_roll_forward_report_json_roundtrip() -> None:
    report = RollForwardReport.example()
    restored = RollForwardReport.from_json(report.to_json())

    assert restored.days == report.days
    assert restored.total_carry["USD"] == report.total_carry["USD"]
