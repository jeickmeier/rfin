#!/usr/bin/env python3
"""Private markets fund example demonstrating waterfall-driven cashflows."""

from datetime import date
import json

from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import PrivateMarketsFund
from finstack.valuations.pricer import standard_registry


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()
    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.9960),
            (3.0, 0.9820),
            (7.0, 0.9500),
        ],
    )
    market.insert(discount_curve)
    return market


def build_fund_definition() -> str:
    fund = {
        "id": "PMF-CORE-EQ",
        "currency": "USD",
        "discount_curve_id": "USD-OIS",
        "waterfall_spec": {
            "style": "european",
            "catchup_mode": "full",
            "irr_basis": "Act365F",
            "tranches": [
                "return_of_capital",
                {"preferred_irr": {"irr": 0.08}},
                {"catch_up": {"gp_share": 0.2}},
                {
                    "promote_tier": {
                        "hurdle": {"irr": {"rate": 0.12}},
                        "lp_share": 0.7,
                        "gp_share": 0.3,
                    }
                },
            ],
        },
        "events": [
            {
                "date": "2024-01-02",
                "amount": {"amount": 2_000_000.0, "currency": "USD"},
                "kind": "contribution",
            },
            {
                "date": "2025-04-01",
                "amount": {"amount": 750_000.0, "currency": "USD"},
                "kind": "contribution",
            },
            {
                "date": "2026-09-30",
                "amount": {"amount": 600_000.0, "currency": "USD"},
                "kind": "proceeds",
                "deal_id": "EXIT-01",
            },
            {
                "date": "2027-06-30",
                "amount": {"amount": 1_250_000.0, "currency": "USD"},
                "kind": "distribution",
            },
            {
                "date": "2028-12-31",
                "amount": {"amount": 1_800_000.0, "currency": "USD"},
                "kind": "proceeds",
                "deal_id": "EXIT-02",
            },
        ],
    }
    return json.dumps({"type": "private_markets_fund", "spec": fund})


def main() -> None:
    as_of = date(2028, 12, 31)
    market = build_market(as_of)
    registry = standard_registry()

    fund = PrivateMarketsFund.from_json(build_fund_definition())
    registry.price_with_metrics(
        fund,
        "discounting",
        market,
        as_of,
        metrics=["lp_irr", "tvpi_lp"],
    )

    schedule = fund.cashflow_schedule(market, as_of)
    [(cf.date.isoformat(), round(cf.amount.amount, 2)) for cf in schedule.flows()[:3]]


if __name__ == "__main__":
    main()
