"""Title: Comprehensive Risk Report (DV01/CS01/Greeks)
Persona: Risk Analyst
Complexity: Intermediate
Runtime: ~4 seconds.

Description:
Generates comprehensive risk report with:
- DV01 (interest rate sensitivity per $1M notional)
- CS01 (credit spread sensitivity per $1M notional)
- Options Greeks (Delta, Gamma, Vega, Theta, Rho)
- Risk aggregation by asset class and currency
- Risk ladder by tenor/maturity

Key Concepts:
- Metrics computation via price_with_metrics
- Risk aggregation across positions
- Greeks for options
- Risk bucketing by maturity

Prerequisites:
- Portfolio construction (Example 01)
- Understanding of risk metrics (DV01, Greeks)
"""

from datetime import date, timedelta

from finstack.core.currency import Currency
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.fx import FxMatrix
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.core.money import Money
from finstack.portfolio import Entity, PortfolioBuilder, Position, PositionUnit
from finstack.valuations.instruments import Bond, CreditDefaultSwap
from finstack.valuations.pricer import create_standard_registry


def create_market_data():
    """Create market with curves and vol surfaces."""
    market = MarketContext()
    as_of = date(2024, 1, 15)

    # Discount curve
    market.insert_discount(DiscountCurve("USD-OIS", as_of, [(0.0, 1.0), (10.0, 0.65)]))

    # Hazard curve for CDS
    market.insert_hazard(HazardCurve("CORP.CDS", as_of, [(0.0, 0.02), (10.0, 0.02)], recovery_rate=0.40))

    # FX
    fx = FxMatrix()
    fx.set_quote(Currency("EUR"), Currency("USD"), 1.0 / 0.92)
    market.insert_fx(fx)

    return market


def create_diversified_portfolio():
    """Create portfolio with diverse risk exposures."""
    as_of = date(2024, 1, 15)
    fund = Entity("FUND-001").with_name("Global Macro Fund")

    # 1. Short-term bond (low duration)
    bond_2y = (
        Bond.builder("BOND.2Y")
        .money(Money(10_000_000, "USD"))
        .coupon_rate(0.04)
        .frequency("semiannual")
        .issue(date(2024, 1, 15))
        .maturity(date(2026, 1, 15))
        .disc_id("USD-OIS")
        .build()
    )
    pos_2y = Position("POS-BOND-2Y", fund.id, "BOND.2Y", bond_2y, 1.0, PositionUnit.UNITS).with_tags(
        {"asset_class": "rates", "maturity_bucket": "0-2Y"}
    )

    # 2. Medium-term bond
    bond_5y = (
        Bond.builder("BOND.5Y")
        .money(Money(20_000_000, "USD"))
        .coupon_rate(0.045)
        .frequency("semiannual")
        .issue(date(2024, 1, 15))
        .maturity(date(2029, 1, 15))
        .disc_id("USD-OIS")
        .build()
    )
    pos_5y = Position("POS-BOND-5Y", fund.id, "BOND.5Y", bond_5y, 1.0, PositionUnit.UNITS).with_tags(
        {"asset_class": "rates", "maturity_bucket": "5-7Y"}
    )

    # 3. CDS (long protection)
    start = as_of + timedelta(days=1)
    cds = CreditDefaultSwap.buy_protection(
        "CDS.5Y",
        Money(10_000_000, "USD"),
        spread_bp=200.0,
        start_date=start,
        maturity=date(2029, 1, 15),
        discount_curve="USD-OIS",
        credit_curve="CORP.CDS",
    )
    pos_cds = Position("POS-CDS-5Y", fund.id, "CDS.5Y", cds, 1.0, PositionUnit.UNITS).with_tags(
        {"asset_class": "credit", "maturity_bucket": "5-7Y"}
    )

    portfolio = (
        PortfolioBuilder("RISK_REPORT")
        .base_ccy(Currency("USD"))
        .as_of(as_of)
        .entity(fund)
        .position([pos_2y, pos_5y, pos_cds])
        .build()
    )

    instruments_by_position_id = {
        "POS-BOND-2Y": bond_2y,
        "POS-BOND-5Y": bond_5y,
        "POS-CDS-5Y": cds,
    }

    return portfolio, instruments_by_position_id


def compute_risk_metrics(portfolio, instruments_by_position_id, market, *, as_of: date):
    """Compute risk metrics for each instrument."""
    registry = create_standard_registry()
    risk_data = []

    for pos_id, instrument in instruments_by_position_id.items():
        position = portfolio.get_position(pos_id)
        tags = position.tags if position is not None else {}

        # Define metrics based on instrument type
        if isinstance(instrument, Bond):
            metrics = ["dv01", "duration_mod", "convexity"]
        elif isinstance(instrument, CreditDefaultSwap):
            metrics = ["cs01", "dv01"]
        else:
            metrics = []

        if not metrics:
            continue

        # Price with metrics
        try:
            result = registry.price_with_metrics(instrument, "discounting", market, metrics, as_of=as_of)

            # Extract metrics
            metrics_dict = {
                "position_id": pos_id,
                "asset_class": tags.get("asset_class", "N/A"),
                "maturity_bucket": tags.get("maturity_bucket", "N/A"),
                "pv": result.value.amount,
            }

            for metric in metrics:
                value = result.measures.get(metric)
                if value is not None:
                    metrics_dict[metric] = value

            risk_data.append(metrics_dict)

        except Exception:
            pass

    return risk_data


def main() -> None:
    """Generate comprehensive risk report."""
    # 1. Create market and portfolio
    as_of = date(2024, 1, 15)
    market = create_market_data()
    portfolio, instruments_by_position_id = create_diversified_portfolio()

    # 2. Compute risk metrics
    risk_data = compute_risk_metrics(portfolio, instruments_by_position_id, market, as_of=as_of)

    # 3. Interest Rate Risk (DV01)

    total_dv01 = 0.0
    for pos in risk_data:
        if "dv01" in pos:
            pos["position_id"]
            pos["pv"]
            dv01 = pos.get("dv01", 0.0)
            pos.get("duration_mod", 0.0)
            pos.get("convexity", 0.0)

            total_dv01 += dv01

    # 4. Credit Risk (CS01)

    total_cs01 = 0.0
    for pos in risk_data:
        if "cs01" in pos:
            pos["position_id"]
            pos["pv"]
            cs01 = pos.get("cs01", 0.0)

            total_cs01 += cs01

    # 5. Equity Options Greeks

    total_delta = 0.0
    total_gamma = 0.0
    total_vega = 0.0
    total_theta = 0.0

    for pos in risk_data:
        if pos["asset_class"] == "equity":
            pos["position_id"]
            pos["pv"]
            delta = pos.get("delta", 0.0)
            gamma = pos.get("gamma", 0.0)
            vega = pos.get("vega", 0.0)
            theta = pos.get("theta", 0.0)

            total_delta += delta
            total_gamma += gamma
            total_vega += vega
            total_theta += theta

    # 6. Risk Ladder by Maturity Bucket

    # Aggregate DV01 by maturity bucket
    from collections import defaultdict

    dv01_by_bucket = defaultdict(float)
    pv_by_bucket = defaultdict(float)

    for pos in risk_data:
        if "dv01" in pos:
            bucket = pos["maturity_bucket"]
            dv01_by_bucket[bucket] += pos.get("dv01", 0.0)
            pv_by_bucket[bucket] += pos["pv"]

    for bucket in sorted(dv01_by_bucket.keys()):
        pv_by_bucket[bucket]
        dv01 = dv01_by_bucket[bucket]

    # 7. Summary Dashboard

    # 8. Export to DataFrame

    import polars as pl

    pl.DataFrame(risk_data)

    # Can export for further analysis
    # df.write_csv("risk_report.csv")
    # df.write_parquet("risk_report.parquet")


if __name__ == "__main__":
    main()
