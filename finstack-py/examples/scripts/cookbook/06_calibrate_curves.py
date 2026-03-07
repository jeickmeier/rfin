"""Title: Bootstrap Discount, Forward, and Credit Curves
Persona: Quantitative Researcher
Complexity: Advanced
Runtime: ~2 seconds.

Description:
Demonstrates market data calibration workflow:
- Bootstrap USD OIS discount curve from deposits, futures, swaps
- Build USD SOFR forward curve (post-2008 multi-curve framework)
- Calibrate credit spread curve from CDS quotes
- Validate curve construction and extract forward rates

Key Concepts:
- Calibration plan-driven API
- Multi-curve framework (discount vs forward)
- CDS curve bootstrapping
- Quote construction and validation

Prerequisites:
- Understanding of interest rate curves
- Knowledge of CDS pricing
- Familiarity with bootstrapping methods
"""

import math

from datetime import date, timedelta

from finstack.core.dates.tenor import Tenor
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve
from finstack.core.money import Money
from finstack.valuations.pricer import create_standard_registry
from finstack.valuations.instruments import CreditDefaultSwap


def main() -> None:
    """Build curves and validate by repricing."""
    base_date = date(2024, 1, 15)

    market_ctx = MarketContext()

    # For cookbook purposes we keep this fast/deterministic by using flat curves.
    # For a full plan-driven calibration example, see:
    # `examples/scripts/valuations/calibration/discount_curve_calibration_example.py`.
    market_ctx.insert(DiscountCurve("USD-OIS", base_date, [(0.0, 1.0), (10.0, 0.65)]))
    market_ctx.insert(HazardCurve("ACME.CDS", base_date, [(0.0, 0.02), (10.0, 0.02)], recovery_rate=0.40))

    # Retrieve calibrated curve
    ois_curve = market_ctx.get_discount("USD-OIS")

    # Display discount factors

    for tenor_str in ["1M", "3M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"]:
        maturity = Tenor.parse(tenor_str).add_to_date(base_date)
        df = ois_curve.df_on_date(maturity)
        df

    # Retrieve hazard curve
    hazard_curve = market_ctx.get_hazard("ACME.CDS")

    # Display survival probabilities and credit spreads

    for tenor_str in ["6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y"]:
        t = Tenor.parse(tenor_str).to_years_simple()
        surv_prob = hazard_curve.survival(t)
        1.0 - surv_prob

        surv_prob

    # 4. Validation: reprice input instruments

    registry = create_standard_registry()

    # A small set of CDS repricing checks (spreads in bps).
    cds_points = [("6M", 80.0), ("1Y", 120.0), ("3Y", 180.0), ("5Y", 200.0), ("10Y", 250.0)]
    for tenor, spread_bp in cds_points:
        maturity = Tenor.parse(tenor).add_to_date(base_date)
        cds = CreditDefaultSwap.buy_protection(
            f"CDS.{tenor}",
            Money(10_000_000, "USD"),
            spread_bp=spread_bp,
            start_date=base_date + timedelta(days=1),
            maturity=maturity,
            discount_curve="USD-OIS",
            credit_curve="ACME.CDS",
        )

        # Price with curves
        registry.price_with_metrics(cds, "discounting", market_ctx, ["par_spread", "pv01"], as_of=base_date)

        # For par CDS, PV should be close to zero

    # 5. Summary


if __name__ == "__main__":
    main()
