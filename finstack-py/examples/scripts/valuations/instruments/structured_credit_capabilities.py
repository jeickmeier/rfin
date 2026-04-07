#!/usr/bin/env python3
"""Structured credit examples: ABS, CLO, CMBS, and RMBS via JSON bindings.

Updated to use the unified StructuredCredit type and the current serde shapes.
"""

from datetime import date
import json

from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import StructuredCredit
from finstack.valuations.pricer import standard_registry

CURRENCY = "USD"


def money(amount: float) -> dict:
    return {"amount": amount, "currency": CURRENCY}


def base_attributes() -> dict:
    return {"tags": [], "meta": {}}


def coverage_tests_template() -> dict:
    return {"test_definitions": {}, "current_results": {}, "historical_results": []}


def waterfall_engine(tranche_ids) -> dict:
    # Minimal sequential engine: fees -> tranche interest -> tranche principal -> equity
    tiers: list[dict] = []

    # Fee tier
    tiers.append({
        "id": "trustee_fees",
        "priority": 1,
        "recipients": [
            {
                "id": "trustee_fee_payment",
                "recipient_type": {"ServiceProvider": "Trustee"},
                "calculation": {"FixedAmount": {"amount": money(25_000.0)}},
                "weight": None,
            }
        ],
        "payment_type": "Fee",
        "allocation_mode": "Sequential",
        "divertible": False,
    })

    prio = 2
    # Interest tiers
    for tid in tranche_ids:
        tiers.append({
            "id": f"{tid}_interest",
            "priority": prio,
            "recipients": [
                {
                    "id": f"{tid}_int_payment",
                    "recipient_type": {"Tranche": tid},
                    "calculation": {"TrancheInterest": {"tranche_id": tid}},
                    "weight": None,
                }
            ],
            "payment_type": "Interest",
            "allocation_mode": "Sequential",
            "divertible": False,
        })
        prio += 1

    # Principal tiers
    for tid in tranche_ids:
        tiers.append({
            "id": f"{tid}_principal",
            "priority": prio,
            "recipients": [
                {
                    "id": f"{tid}_prin_payment",
                    "recipient_type": {"Tranche": tid},
                    "calculation": {"TranchePrincipal": {"tranche_id": tid, "target_balance": None}},
                    "weight": None,
                }
            ],
            "payment_type": "Principal",
            "allocation_mode": "Sequential",
            "divertible": True,
        })
        prio += 1

    # Residual tier
    tiers.append({
        "id": "equity_distribution",
        "priority": prio,
        "recipients": [
            {
                "id": "equity_payment",
                "recipient_type": "Equity",
                "calculation": "ResidualCash",
                "weight": None,
            }
        ],
        "payment_type": "Residual",
        "allocation_mode": "Sequential",
        "divertible": False,
    })

    return {"tiers": tiers, "coverage_triggers": [], "base_currency": CURRENCY}


def asset_entry(identifier: str, asset_type: dict, balance: float, rate: float, maturity: str, *, obligor: str) -> dict:
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
        "day_count": "Act360",
    }


def tranche_entry(
    identifier: str, attach: float, detach: float, seniority: str, rate: float, balance: float, priority: int
) -> dict:
    return {
        "id": identifier,
        "attachment_point": attach,
        "detachment_point": detach,
        # Valid enum values: Senior, Mezzanine, Subordinated, Equity
        "seniority": seniority,
        "rating": "BBB" if seniority != "Equity" else "NR",
        "original_balance": money(balance),
        "current_balance": money(balance),
        "target_balance": None,
        "coupon": {"Fixed": {"rate": rate}},
        "oc_trigger": None,
        "ic_trigger": None,
        "credit_enhancement": {
            "subordination": money(0.0),
            "overcollateralization": money(0.0),
            "reserve_account": money(0.0),
            "excess_spread": 0.0,
            "cash_trap_active": False,
        },
        "payment_frequency": {"count": 3, "unit": "months"},
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
    return {}


def concentration_limits_template() -> dict:
    return {}


def pool_stats_template() -> dict:
    return {}


def build_pool(deal_type: str, asset_kind: str) -> dict:
    if asset_kind == "loan":
        asset_variants = [
            {"type": "FirstLienLoan", "industry": "technology"},
            {"type": "FirstLienLoan", "industry": "healthcare"},
        ]
    elif asset_kind == "commercial_mortgage":
        asset_variants = [
            {"type": "CommercialMortgage", "ltv": 0.65},
            {"type": "CommercialMortgage", "ltv": 0.60},
        ]
    else:
        asset_variants = [
            {"type": "SingleFamilyMortgage", "ltv": 0.70},
            {"type": "SingleFamilyMortgage", "ltv": 0.68},
        ]

    assets = [
        asset_entry("ASSET1", asset_variants[0], 6_000_000.0, 0.072, "2032-01-01", obligor="OBL1"),
        asset_entry("ASSET2", asset_variants[1], 4_000_000.0, 0.068, "2031-06-01", obligor="OBL2"),
    ]

    return {
        "id": f"POOL-{deal_type}",
        "deal_type": deal_type,
        "assets": assets,
        "cumulative_defaults": money(0.0),
        "cumulative_recoveries": money(0.0),
        "cumulative_prepayments": money(0.0),
        "reinvestment_period": None,
        "collection_account": money(0.0),
        "reserve_account": money(0.0),
        "excess_spread_account": money(0.0),
    }


def build_tranches(total_notional: float) -> dict:
    senior_balance = total_notional * 0.8
    equity_balance = total_notional - senior_balance

    senior = tranche_entry(
        "TRANCHE-A",
        attach=20.0,
        detach=100.0,
        seniority="Senior",
        rate=0.045,
        balance=senior_balance,
        priority=1,
    )
    equity = tranche_entry(
        "TRANCHE-EQ",
        attach=0.0,
        detach=20.0,
        seniority="Equity",
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

    payload = {
        "id": instrument_id,
        "deal_type": deal_type,
        "pool": build_pool(deal_type, asset_kind),
        "tranches": tranches,
        "closing_date": "2024-01-02",
        "first_payment_date": "2024-04-01",
        "reinvestment_end_date": None,
        "legal_maturity": "2035-01-01",
        "payment_frequency": {"count": 3, "unit": "months"},
        "payment_calendar_id": "nyse",
        "discount_curve_id": "USD-OIS",
        "attributes": base_attributes(),
        # Required defaults for behavioral models and market/credit context
        "prepayment_spec": {"type": "constant_cpr", "cpr": 0.15},
        "default_spec": {"type": "constant_cdr", "cdr": 0.02},
        "recovery_spec": {"type": "constant", "rate": 0.4, "recovery_lag": 12},
        "market_conditions": {
            "refi_rate": 0.04,
            "original_rate": None,
            "hpa": None,
            "unemployment": None,
            "seasonal_factor": 1.0,
            "custom_factors": {},
        },
        "credit_factors": {
            "credit_score": None,
            "dti": None,
            "ltv": None,
            "delinquency_days": 0,
            "unemployment_rate": None,
            "custom_factors": {},
        },
        "deal_metadata": {},
        "behavior_overrides": {},
        "default_assumptions": {
            "base_cdr_annual": 0.02,
            "base_recovery_rate": 0.40,
            "base_cpr_annual": 0.15,
            "psa_speed": None,
            "sda_speed": None,
            "abs_speed_monthly": None,
            "cpr_by_asset_type": {},
            "cdr_by_asset_type": {},
            "recovery_by_asset_type": {},
        },
    }
    return payload


def build_abs_payload() -> dict:
    payload = base_deal_payload("ABS-SAMPLE", "ABS", asset_kind="loan")
    return payload


def build_clo_payload() -> dict:
    payload = base_deal_payload("CLO-SAMPLE", "CLO", asset_kind="loan")
    return payload


def build_cmbs_payload() -> dict:
    payload = base_deal_payload("CMBS-SAMPLE", "CMBS", asset_kind="commercial_mortgage")
    return payload


def build_rmbs_payload() -> dict:
    payload = base_deal_payload("RMBS-SAMPLE", "RMBS", asset_kind="residential_mortgage")
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
    market.insert(discount_curve)
    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_market(as_of)
    registry = standard_registry()

    deals = {
        "ABS": StructuredCredit.from_json(json.dumps(build_abs_payload())),
        "CLO": StructuredCredit.from_json(json.dumps(build_clo_payload())),
        "CMBS": StructuredCredit.from_json(json.dumps(build_cmbs_payload())),
        "RMBS": StructuredCredit.from_json(json.dumps(build_rmbs_payload())),
    }

    for instrument in deals.values():
        registry.price(instrument, "discounting", market, as_of=as_of)


if __name__ == "__main__":
    main()
