import datetime as dt
import json

from finstack.core.currency import USD
from finstack.core.dates.daycount import DayCount
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar, ScalarTimeSeries
from finstack.core.market_data.term_structures import DiscountCurve

from finstack import Money


def build_market() -> MarketContext:
    market = MarketContext()
    market.insert_discount(
        DiscountCurve(
            "USD-OIS",
            dt.date(2025, 1, 2),
            [(0.0, 1.0), (1.0, 0.97), (5.0, 0.85)],
            day_count=DayCount.ACT_365F,
        )
    )
    market.insert_price("SPOT::ABC", MarketScalar.price(Money(42.0, USD)))
    market.insert_series(
        ScalarTimeSeries(
            "SERIES::ABC",
            [(dt.date(2025, 1, 2), 100.0), (dt.date(2025, 1, 3), 101.5)],
        )
    )
    market.map_collateral("USD-CSA", "USD-OIS")
    return market


def test_market_context_dict_roundtrip() -> None:
    market = build_market()

    payload = market.to_dict()

    assert payload["version"] >= 1
    assert payload["curves"][0]["type"] == "discount"
    assert payload["collateral"]["USD-CSA"] == "USD-OIS"

    restored = MarketContext.from_dict(payload)

    assert restored.get_discount("USD-OIS").id == "USD-OIS"
    assert restored.get_price("SPOT::ABC").value.amount == 42.0
    assert restored.get_series("SERIES::ABC").value_on(dt.date(2025, 1, 3)) == 101.5
    assert restored.get_collateral("USD-CSA").id == "USD-OIS"


def test_market_context_json_roundtrip() -> None:
    market = build_market()

    payload = market.to_json()
    restored = MarketContext.from_json(payload)

    assert restored.get_discount("USD-OIS").id == "USD-OIS"
    assert restored.get_price("SPOT::ABC").value.amount == 42.0

    decoded = json.loads(payload)
    assert decoded["version"] >= 1
    assert decoded["series"][0]["id"] == "SERIES::ABC"
