#!/usr/bin/env python3
"""Private markets fund example demonstrating waterfall-driven cashflows."""
import json
from datetime import date

from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import PrivateMarketsFund
from finstack.valuations.pricer import create_standard_registry


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
    market.insert_discount(discount_curve)
    return market


def build_fund_definition() -> str:
    fund = {
        "id": "PMF-CORE-EQ",
        "currency": "USD",
        "discount_curve_id": "USD-OIS",
        "spec": {
            "style": "european",
            "catchup_mode": "full",
            "irr_basis": "act_365f",
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
    return json.dumps(fund)


def main() -> None:
    as_of = date(2028, 12, 31)
    market = build_market(as_of)
    registry = create_standard_registry()

    fund = PrivateMarketsFund.from_json(build_fund_definition())
    result = registry.price_with_metrics(
        fund,
        "discounting",
        market,
        ["lp_irr", "tvpi_lp"],
    )
    print("Private markets fund PV:", round(result.value.amount, 2), result.value.currency)
    print("LP IRR:", result.measures.get("lp_irr"))
    print("TVPI:", result.measures.get("tvpi_lp"))

    ledger = fund.lp_cashflows()
    preview = [(dt.isoformat(), round(cf.amount, 2)) for dt, cf in ledger[:3]]
    print("LP cashflows preview:", preview)


if __name__ == "__main__":
    main()
