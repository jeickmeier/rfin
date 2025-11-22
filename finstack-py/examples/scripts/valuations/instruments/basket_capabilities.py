#!/usr/bin/env python3
"""Basket instrument example combining equity and bond constituents."""
import json
from datetime import date

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.scalars import MarketScalar
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import Basket
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()

    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.9970),
            (3.0, 0.9850),
            (5.0, 0.9650),
        ],
    )
    market.insert_discount(discount_curve)

    market.insert_price("ACME-SPOT", MarketScalar.price(Money(150.0, USD)))
    market.insert_price("MEGA-SPOT", MarketScalar.price(Money(95.0, USD)))
    market.insert_price("UST-10Y-PRICE", MarketScalar.price(Money(101.5, USD)))

    return market


def build_basket_definition() -> str:
    basket = {
        "id": "MULTI-ASSET-BASKET",
        "currency": "USD",
        "discount_curve_id": "USD-OIS",
        "expense_ratio": 0.0025,
        "constituents": [
            {
                "id": "ACME",
                "reference": {"price_id": "ACME-SPOT", "asset_type": "equity"},
                "weight": 0.5,
                "ticker": "ACME",
            },
            {
                "id": "MEGA",
                "reference": {"price_id": "MEGA-SPOT", "asset_type": "equity"},
                "weight": 0.3,
                "ticker": "MEGA",
            },
            {
                "id": "UST10Y",
                "reference": {"price_id": "UST-10Y-PRICE", "asset_type": "bond"},
                "weight": 0.2,
                "ticker": "UST10Y",
            },
        ],
        "pricing_config": {
            "days_in_year": 365.25,
            "fx_policy": "cashflow_date",
        },
    }
    return json.dumps(basket)


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = create_standard_registry()

    basket_json = build_basket_definition()
    basket = Basket.from_json(basket_json)

    result = registry.price_with_metrics(
        basket,
        "discounting",
        market,
        ["constituent_count", "expense_ratio"],
    )
    print("Basket PV:", round(result.value.amount, 2), result.value.currency)
    print("Constituent count:", result.measures.get("constituent_count"))
    print("Expense ratio:", result.measures.get("expense_ratio"))


if __name__ == "__main__":
    main()
