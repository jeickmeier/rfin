#!/usr/bin/env python3
"""Demonstrate creating and valuing term loans with various features.

Term loans are corporate loans with fixed maturities, commonly used in
leveraged finance. This example shows:
- Basic fixed-rate term loan
- Term loan with PIK (payment-in-kind) interest
- Term loan serialization (to/from JSON)
"""

import json
from datetime import date

from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.valuations.instruments import TermLoan
from finstack.valuations.pricer import standard_registry


def build_market(as_of: date) -> MarketContext:
    """Create market context with discount curve."""
    market = MarketContext()

    # OIS discount curve
    disc = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.9950),
            (2.0, 0.9850),
            (3.0, 0.9700),
            (5.0, 0.9350),
            (7.0, 0.8950),
        ],
    )
    market.insert(disc)

    return market


def create_base_pricing_overrides():
    """Create standard pricing overrides structure."""
    return {
        "quoted_clean_price": None,
        "rho_bump_decimal": None,
        "vega_bump_decimal": None,
        "implied_volatility": None,
        "vol_surface_extrapolation": "error",
        "quoted_spread_bp": None,
        "upfront_payment": None,
        "ytm_bump_decimal": None,
        "theta_period": None,
        "mc_seed_scenario": None,
        "adaptive_bumps": False,
        "spot_bump_pct": None,
        "vol_bump_pct": None,
        "rate_bump_bp": None,
    }


def example_fixed_rate_term_loan(registry, market, as_of: date) -> None:
    """Create and price a basic fixed-rate term loan."""
    print("\n=== Fixed Rate Term Loan (6%) ===")

    # Define term loan via JSON specification
    term_loan_spec = {
        "id": "TL-FIXED-001",
        "currency": "USD",
        "notional_limit": {"amount": 50_000_000.0, "currency": "USD"},
        "issue": "2024-01-15",
        "maturity": "2029-01-15",
        "rate": {"Fixed": {"rate_bp": 600}},  # 6% fixed rate
        "pay_freq": {"count": 3, "unit": "months"},
        "day_count": "Act360",
        "bdc": "modified_following",
        "calendar_id": None,
        "stub": "None",
        "discount_curve_id": "USD-OIS",
        "credit_curve_id": None,
        "amortization": "None",
        "coupon_type": "Cash",
        "upfront_fee": None,
        "ddtl": {
            "commitment_limit": {"amount": 50_000_000.0, "currency": "USD"},
            "availability_start": "2024-01-15",
            "availability_end": "2024-01-15",
            "draws": [
                {"date": "2024-01-15", "amount": {"amount": 50_000_000.0, "currency": "USD"}}
            ],
            "commitment_step_downs": [],
            "usage_fee_bp": 0,
            "commitment_fee_bp": 0,
            "fee_base": "Undrawn",
            "oid_policy": None,
        },
        "covenants": None,
        "pricing_overrides": create_base_pricing_overrides(),
        "call_schedule": None,
        "attributes": {"meta": {"borrower": "ACME Corp"}, "tags": ["senior", "secured"]},
    }

    term_loan = TermLoan.from_json(json.dumps(term_loan_spec))

    print(f"  Instrument ID: {term_loan.instrument_id}")
    print(f"  Notional: {term_loan.notional_limit}")
    print(f"  Issue: {term_loan.issue}")
    print(f"  Maturity: {term_loan.maturity}")

    # Price the term loan
    result = registry.get_price(term_loan, "discounting", market, as_of=as_of)
    print(f"  PV: {result.value}")


def example_high_yield_term_loan(registry, market, as_of: date) -> None:
    """Create and price a higher-rate term loan."""
    print("\n=== High-Yield Term Loan (9%) ===")

    term_loan_spec = {
        "id": "TL-HY-001",
        "currency": "USD",
        "notional_limit": {"amount": 25_000_000.0, "currency": "USD"},
        "issue": "2024-01-15",
        "maturity": "2028-01-15",
        "rate": {"Fixed": {"rate_bp": 900}},  # 9% rate
        "pay_freq": {"count": 3, "unit": "months"},
        "day_count": "Act360",
        "bdc": "modified_following",
        "calendar_id": None,
        "stub": "None",
        "discount_curve_id": "USD-OIS",
        "credit_curve_id": None,
        "amortization": "None",
        "coupon_type": "Cash",
        "upfront_fee": None,
        "ddtl": {
            "commitment_limit": {"amount": 25_000_000.0, "currency": "USD"},
            "availability_start": "2024-01-15",
            "availability_end": "2024-01-15",
            "draws": [
                {"date": "2024-01-15", "amount": {"amount": 25_000_000.0, "currency": "USD"}}
            ],
            "commitment_step_downs": [],
            "usage_fee_bp": 0,
            "commitment_fee_bp": 0,
            "fee_base": "Undrawn",
            "oid_policy": None,
        },
        "covenants": None,
        "pricing_overrides": create_base_pricing_overrides(),
        "call_schedule": None,
        "attributes": {"meta": {"borrower": "LBO Target"}, "tags": ["high-yield", "mezzanine"]},
    }

    term_loan = TermLoan.from_json(json.dumps(term_loan_spec))

    print(f"  Instrument ID: {term_loan.instrument_id}")
    print(f"  Notional: {term_loan.notional_limit}")
    print(f"  Maturity: {term_loan.maturity}")

    result = registry.get_price(term_loan, "discounting", market, as_of=as_of)
    print(f"  PV: {result.value}")


def example_term_loan_with_pik(registry, market, as_of: date) -> None:
    """Create and price a term loan with PIK (payment-in-kind) interest."""
    print("\n=== Term Loan with PIK Toggle ===")

    term_loan_spec = {
        "id": "TL-PIK-001",
        "currency": "USD",
        "notional_limit": {"amount": 30_000_000.0, "currency": "USD"},
        "issue": "2024-01-15",
        "maturity": "2029-01-15",
        "rate": {"Fixed": {"rate_bp": 800}},  # 8% rate
        "pay_freq": {"count": 3, "unit": "months"},
        "day_count": "Act360",
        "bdc": "modified_following",
        "calendar_id": None,
        "stub": "None",
        "discount_curve_id": "USD-OIS",
        "credit_curve_id": None,
        "amortization": "None",
        "coupon_type": "PIK",  # Payment-in-kind: interest capitalizes
        "upfront_fee": None,
        "ddtl": {
            "commitment_limit": {"amount": 30_000_000.0, "currency": "USD"},
            "availability_start": "2024-01-15",
            "availability_end": "2024-01-15",
            "draws": [
                {"date": "2024-01-15", "amount": {"amount": 30_000_000.0, "currency": "USD"}}
            ],
            "commitment_step_downs": [],
            "usage_fee_bp": 0,
            "commitment_fee_bp": 0,
            "fee_base": "Undrawn",
            "oid_policy": None,
        },
        "covenants": None,
        "pricing_overrides": create_base_pricing_overrides(),
        "call_schedule": None,
        "attributes": {"meta": {"borrower": "GrowthCo"}, "tags": ["pik", "growth"]},
    }

    term_loan = TermLoan.from_json(json.dumps(term_loan_spec))

    print(f"  Instrument ID: {term_loan.instrument_id}")
    print(f"  Notional: {term_loan.notional_limit}")
    print(f"  Maturity: {term_loan.maturity}")
    print("  Coupon Type: PIK (payment-in-kind)")

    result = registry.get_price(term_loan, "discounting", market, as_of=as_of)
    print(f"  PV: {result.value}")


def example_term_loan_serialization(registry, market, as_of: date) -> None:
    """Demonstrate term loan serialization (to/from JSON)."""
    print("\n=== Term Loan Serialization ===")

    # Create a term loan
    term_loan_spec = {
        "id": "TL-SERIALIZE-001",
        "currency": "USD",
        "notional_limit": {"amount": 10_000_000.0, "currency": "USD"},
        "issue": "2024-01-15",
        "maturity": "2027-01-15",
        "rate": {"Fixed": {"rate_bp": 550}},
        "pay_freq": {"count": 3, "unit": "months"},
        "day_count": "Act360",
        "bdc": "modified_following",
        "calendar_id": None,
        "stub": "None",
        "discount_curve_id": "USD-OIS",
        "credit_curve_id": None,
        "amortization": "None",
        "coupon_type": "Cash",
        "upfront_fee": None,
        "ddtl": {
            "commitment_limit": {"amount": 10_000_000.0, "currency": "USD"},
            "availability_start": "2024-01-15",
            "availability_end": "2024-01-15",
            "draws": [
                {"date": "2024-01-15", "amount": {"amount": 10_000_000.0, "currency": "USD"}}
            ],
            "commitment_step_downs": [],
            "usage_fee_bp": 0,
            "commitment_fee_bp": 0,
            "fee_base": "Undrawn",
            "oid_policy": None,
        },
        "covenants": None,
        "pricing_overrides": create_base_pricing_overrides(),
        "call_schedule": None,
        "attributes": {"meta": {}, "tags": []},
    }

    # Create term loan from JSON
    original = TermLoan.from_json(json.dumps(term_loan_spec))
    print(f"  Original ID: {original.instrument_id}")

    # Serialize to JSON and back
    json_str = original.to_json()
    roundtrip = TermLoan.from_json(json_str)
    print(f"  Roundtrip ID: {roundtrip.instrument_id}")

    # Verify both price the same
    pv_original = registry.get_price(original, "discounting", market, as_of=as_of).value
    pv_roundtrip = registry.get_price(roundtrip, "discounting", market, as_of=as_of).value

    print(f"  Original PV:  {pv_original}")
    print(f"  Roundtrip PV: {pv_roundtrip}")
    print(f"  Match: {pv_original.amount == pv_roundtrip.amount}")


def main() -> None:
    """Run all term loan examples."""
    print("=" * 60)
    print("Term Loan Examples")
    print("=" * 60)

    as_of = date(2024, 1, 16)
    market = build_market(as_of)
    registry = standard_registry()

    # Run examples
    example_fixed_rate_term_loan(registry, market, as_of)
    example_high_yield_term_loan(registry, market, as_of)
    example_term_loan_with_pik(registry, market, as_of)
    example_term_loan_serialization(registry, market, as_of)

    print("\n" + "=" * 60)
    print("All term loan examples completed successfully!")
    print("=" * 60)


if __name__ == "__main__":
    main()
