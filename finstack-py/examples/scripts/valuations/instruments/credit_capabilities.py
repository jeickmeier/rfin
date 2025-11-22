#!/usr/bin/env python3
"""Examples covering credit derivatives: single-name CDS, CDS index, tranche, and options."""
from datetime import date, timedelta

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.surfaces import VolSurface
from finstack.core.market_data.term_structures import (
    BaseCorrelationCurve,
    CreditIndexData,
    DiscountCurve,
    HazardCurve,
)
from finstack.valuations.instruments import (
    CDSIndex,
    CdsOption,
    CdsTranche,
    CreditDefaultSwap,
)
from finstack.valuations.pricer import create_standard_registry


def build_credit_market(as_of: date) -> MarketContext:
    """Create discount, hazard, correlation, and volatility data for credit pricing."""
    market = MarketContext()

    discount_curve = DiscountCurve(
        "USD-OIS",
        as_of,
        [
            (0.0, 1.0),
            (0.5, 0.9980),
            (1.0, 0.9960),
            (3.0, 0.9850),
            (5.0, 0.9600),
        ],
    )
    market.insert_discount(discount_curve)

    single_name_curve = HazardCurve(
        "ACME-HZD",
        as_of,
        [
            (0.0, 0.0120),
            (3.0, 0.0180),
            (5.0, 0.0220),
        ],
        recovery_rate=0.40,
    )
    index_curve = HazardCurve(
        "CDX-IG-HZD",
        as_of,
        [
            (0.0, 0.0100),
            (3.0, 0.0160),
            (5.0, 0.0190),
            (7.0, 0.0210),
        ],
        recovery_rate=0.40,
    )
    market.insert_hazard(single_name_curve)
    market.insert_hazard(index_curve)

    base_corr = BaseCorrelationCurve(
        "CDX-IG-BC",
        [
            (0.03, 0.10),
            (0.06, 0.12),
            (0.10, 0.15),
            (0.30, 0.20),
            (0.70, 0.23),
            (1.00, 0.25),
        ],
    )

    index_data = CreditIndexData(
        125,
        0.40,
        index_curve,
        base_corr,
    )
    market.insert_credit_index("CDX.NA.IG", index_data)

    vol_surface = VolSurface(
        "CDS-VOL",
        expiries=[0.5, 1.0, 3.0, 5.0],
        strikes=[0.0100, 0.0200, 0.0400],
        grid=[
            [0.45, 0.40, 0.35],
            [0.42, 0.38, 0.33],
            [0.38, 0.35, 0.30],
            [0.35, 0.32, 0.28],
        ],
    )
    market.insert_surface(vol_surface)

    return market


def main() -> None:
    as_of = date(2024, 1, 2)
    market = build_credit_market(as_of)
    registry = create_standard_registry()

    start = as_of + timedelta(days=1)
    maturity = date(as_of.year + 5, as_of.month, as_of.day)

    cds = CreditDefaultSwap.buy_protection(
        "ACME-CDS",
        Money(10_000_000, USD),
        spread_bp=120.0,
        start_date=start,
        maturity=maturity,
        discount_curve="USD-OIS",
        credit_curve="ACME-HZD",
    )
    cds_result = registry.price_with_metrics(
        cds,
        "discounting",
        market,
        ["par_spread", "pv01"],
    )
    print("CDS PV:", round(cds_result.value.amount, 2), cds_result.value.currency)
    print("CDS par spread:", cds_result.measures.get("par_spread"))

    index = CDSIndex.create(
        "CDX-TRAD",
        index_name="CDX.NA.IG",
        series=42,
        version=1,
        notional=Money(25_000_000, USD),
        fixed_coupon_bp=100.0,
        start_date=start,
        maturity=maturity,
        discount_curve="USD-OIS",
        credit_curve="CDX-IG-HZD",
    )
    index_result = registry.price_with_metrics(
        index,
        "discounting",
        market,
        ["par_spread"],
    )
    print("CDS index PV:", round(index_result.value.amount, 2), index_result.value.currency)

    option = CdsOption.create(
        "ACME-CDSOPT",
        Money(5_000_000, USD),
        strike_spread_bp=150.0,
        expiry=date(2025, 1, 2),
        cds_maturity=maturity,
        discount_curve="USD-OIS",
        credit_curve="ACME-HZD",
        vol_surface="CDS-VOL",
        option_type="call",
    )
    option_result = registry.price_with_metrics(
        option,
        "discounting",
        market,
        ["vega"],
    )
    print("CDS option PV:", round(option_result.value.amount, 2), option_result.value.currency)

    tranche = CdsTranche.create(
        "CDX-MEZ-TRANCHE",
        index_name="CDX.NA.IG",
        series=42,
        attach_pct=3.0,
        detach_pct=7.0,
        notional=Money(10_000_000, USD),
        maturity=maturity,
        running_coupon_bp=500.0,
        discount_curve="USD-OIS",
        credit_index_curve="CDX.NA.IG",
        side="buy_protection",
        payments_per_year=4,
    )
    tranche_result = registry.price_with_metrics(
        tranche,
        "discounting",
        market,
        ["par_spread", "expected_loss"],
    )
    print("CDS tranche PV:", round(tranche_result.value.amount, 2), tranche_result.value.currency)


if __name__ == "__main__":
    main()
