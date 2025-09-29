#!/usr/bin/env python3
"""Structured credit examples: ABS, CLO, CMBS, and RMBS via JSON bindings."""
import json
from datetime import date
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import Abs, Clo, Cmbs, Rmbs
from finstack.valuations.pricer import create_standard_registry

CURRENCY = "USD"


def money(amount: float) -> dict:
    return {"amount": amount, "currency": CURRENCY}


def base_attributes() -> dict:
    return {"tags": [], "meta": {}}


def coverage_tests_template() -> dict:
    return {
        "test_definitions": {},
        "current_results": {
            "oc_ratios": {},
            "ic_ratios": {},
            "par_value_ratio": None,
            "custom_results": {},
            "breached_tests": [],
            "payment_diversion": {
                "amount_diverted": money(0.0),
                "diverted_from": [],
                "diverted_to": [],
                "reason": "",
            },
        },
        "historical_results": [],
    }


def waterfall_for_tranches(tranche_ids) -> dict:
    interest_steps = [
        {"TrusteeFees": {"amount": money(25_000.0)}},
    ]
    interest_steps.extend(
        {
            "TrancheInterest": {
                "tranche_id": tranche_id,
                "include_deferred": False,
            }
        }
        for tranche_id in tranche_ids
    )

    principal_steps = []
    for tranche_id in tranche_ids:
        principal_steps.append(
            {
                "TranchePrincipal": {
                    "tranche_id": tranche_id,
                    "payment_type": "Sequential",
                }
            }
        )

    return {
        "payment_mode": "ProRata",
        "interest_waterfall": interest_steps,
        "principal_waterfall": principal_steps,
        "excess_spread_waterfall": ["EquityDistribution"],
    }


def asset_entry(identifier: str, asset_type: dict, balance: float, rate: float, maturity: str, *,
                obligor: str) -> dict:
    return {
        "id": identifier,
        "asset_type": asset_type,
        "balance": money(balance),
        "rate": rate,
        "maturity": maturity,
        "credit_quality": "BBB",
        "industry": "diversified",
        "obligor_id": obligor,
        "is_defaulted": False,
        "recovery_amount": None,
        "purchase_price": None,
        "acquisition_date": "2024-01-02",
    }


def tranche_entry(identifier: str, attach: float, detach: float, seniority: str,
                  rate: float, balance: float, priority: int) -> dict:
    return {
        "id": identifier,
        "attachment_point": attach,
        "detachment_point": detach,
        "seniority": seniority,
        "rating": "BBB" if seniority != "equity" else "NR",
        "original_balance": money(balance),
        "current_balance": money(balance),
        "target_balance": None,
        "coupon": {"Fixed": {"rate": rate}},
        "oc_trigger": None,
        "ic_trigger": None,
        "other_triggers": [],
        "credit_enhancement": {
            "subordination": money(0.0),
            "overcollateralization": money(0.0),
            "reserve_account": money(0.0),
            "excess_spread": 0.0,
            "cash_trap_active": False,
        },
        "payment_frequency": {"Months": 3},
        "day_count": "Act360",
        "deferred_interest": money(0.0),
        "is_revolving": False,
        "can_reinvest": False,
        "legal_maturity": "2035-01-01",
        "expected_maturity": None,
        "payment_priority": priority,
        "attributes": base_attributes(),
    }


def eligibility_template() -> dict:
    return {
        "min_credit_rating": "BB",
        "max_maturity": "2035-12-31",
        "eligible_currencies": [CURRENCY],
        "excluded_industries": [],
        "min_spread_bp": 100.0,
        "max_asset_size": None,
        "min_asset_size": None,
    }


def concentration_limits_template() -> dict:
    return {
        "max_obligor_concentration": 5.0,
        "max_industry_concentration": 20.0,
        "max_ccc_assets": 7.5,
        "min_diversity_score": 20.0,
        "max_weighted_avg_life": 7.0,
        "max_single_asset_pct": 2.0,
    }


def pool_stats_template() -> dict:
    return {
        "weighted_avg_coupon": 0.065,
        "weighted_avg_spread": 0.02,
        "weighted_avg_life": 6.0,
        "weighted_avg_rating_factor": 5.0,
        "diversity_score": 25.0,
        "num_obligors": 20,
        "num_industries": 6,
        "cumulative_default_rate": 0.0,
        "recovery_rate": 0.4,
        "prepayment_rate": 0.05,
    }


def build_pool(deal_type: str, asset_kind: str) -> dict:
    if asset_kind == "loan":
        asset_variants = [
            {"Loan": {"loan_type": "first_lien", "industry": "technology"}},
            {"Loan": {"loan_type": "first_lien", "industry": "healthcare"}},
        ]
    elif asset_kind == "commercial_mortgage":
        asset_variants = [
            {"Mortgage": {"property_type": "commercial", "ltv": 0.65}},
            {"Mortgage": {"property_type": "commercial", "ltv": 0.60}},
        ]
    else:
        asset_variants = [
            {"Mortgage": {"property_type": "single_family", "ltv": 0.70}},
            {"Mortgage": {"property_type": "single_family", "ltv": 0.68}},
        ]

    assets = [
        asset_entry("ASSET1", asset_variants[0], 6_000_000.0, 0.072, "2032-01-01", obligor="OBL1"),
        asset_entry("ASSET2", asset_variants[1], 4_000_000.0, 0.068, "2031-06-01", obligor="OBL2"),
    ]

    return {
        "id": f"POOL-{deal_type.upper()}",
        "deal_type": deal_type,
        "assets": assets,
        "eligibility_criteria": eligibility_template(),
        "concentration_limits": concentration_limits_template(),
        "cumulative_defaults": money(0.0),
        "cumulative_recoveries": money(0.0),
        "cumulative_prepayments": money(0.0),
        "reinvestment_period": None,
        "collection_account": money(0.0),
        "reserve_account": money(500_000.0),
        "excess_spread_account": money(250_000.0),
        "stats": pool_stats_template(),
    }


def build_tranches(total_notional: float) -> dict:
    senior_balance = total_notional * 0.8
    equity_balance = total_notional - senior_balance

    senior = tranche_entry(
        "TRANCHE-A",
        attach=20.0,
        detach=100.0,
        seniority="senior",
        rate=0.045,
        balance=senior_balance,
        priority=1,
    )
    equity = tranche_entry(
        "TRANCHE-EQ",
        attach=0.0,
        detach=20.0,
        seniority="equity",
        rate=0.12,
        balance=equity_balance,
        priority=2,
    )

    return {
        "tranches": [equity, senior],
        "total_size": money(total_notional),
    }


def base_deal_payload(instrument_id: str, deal_type: str, asset_kind: str) -> dict:
    total_notional = 10_000_000.0
    tranches = build_tranches(total_notional)
    waterfall = waterfall_for_tranches([t["id"] for t in tranches["tranches"]])

    payload = {
        "id": instrument_id,
        "deal_type": deal_type,
        "pool": build_pool(deal_type, asset_kind),
        "tranches": tranches,
        "waterfall": waterfall,
        "coverage_tests": coverage_tests_template(),
        "closing_date": "2024-01-02",
        "first_payment_date": "2024-04-01",
        "reinvestment_end_date": None,
        "legal_maturity": "2035-01-01",
        "payment_frequency": {"Months": 3},
        "disc_id": "USD-OIS",
        "attributes": base_attributes(),
    }
    return payload


def build_abs_payload() -> dict:
    payload = base_deal_payload("ABS-SAMPLE", "abs", asset_kind="loan")
    payload.update({"servicer_id": "ABS-SERVICER", "trustee_id": "ABS-TRUST"})
    return payload


def build_clo_payload() -> dict:
    payload = base_deal_payload("CLO-SAMPLE", "clo", asset_kind="loan")
    payload.update({"manager_id": "CLO-MANAGER", "servicer_id": "CLO-SERVICER"})
    return payload


def build_cmbs_payload() -> dict:
    payload = base_deal_payload("CMBS-SAMPLE", "cmbs", asset_kind="commercial_mortgage")
    payload.update({"master_servicer_id": "CMBS-MASTER", "special_servicer_id": "CMBS-SPECIAL"})
    return payload


def build_rmbs_payload() -> dict:
    payload = base_deal_payload("RMBS-SAMPLE", "rmbs", asset_kind="residential_mortgage")
    payload.update({"servicer_id": "RMBS-SERVICER", "master_servicer_id": "RMBS-MASTER"})
    return payload


def build_market(as_of: date) -> MarketContext:
    market = MarketContext()
    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.9950),
            (3.0, 0.9800),
            (5.0, 0.9600),
        ],
    )
    market.insert_discount(discount_curve)
    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = create_standard_registry()

    deals = {
        "ABS": Abs.from_json(json.dumps(build_abs_payload())),
        "CLO": Clo.from_json(json.dumps(build_clo_payload())),
        "CMBS": Cmbs.from_json(json.dumps(build_cmbs_payload())),
        "RMBS": Rmbs.from_json(json.dumps(build_rmbs_payload())),
    }

    for name, instrument in deals.items():
        result = registry.price(instrument, "discounting", market)
        value = result.value
        print(f"{name} PV: {value.amount:,.2f} {value.currency}")


if __name__ == "__main__":
    main()
