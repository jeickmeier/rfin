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
- Calibration plan-driven API (v2)
- Multi-curve framework (discount vs forward)
- CDS curve bootstrapping
- Quote construction and validation

Prerequisites:
- Understanding of interest rate curves
- Knowledge of CDS pricing
- Familiarity with bootstrapping methods
"""

from finstack import (
    CreditQuote,
    Date,
    RatesQuote,
    execute_calibration_v2,
)


def create_usd_ois_quotes():
    """Create USD OIS quotes for discount curve."""
    base_date = Date(2024, 1, 15)

    quotes = [
        # Deposits (ON - 1W)
        RatesQuote.deposit(tenor="ON", rate=0.0525, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.deposit(tenor="1W", rate=0.0530, base_date=base_date, curve_id="USD.OIS"),
        # OIS swaps (1M - 30Y)
        RatesQuote.ois_swap(tenor="1M", rate=0.0535, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="3M", rate=0.0540, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="6M", rate=0.0545, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="1Y", rate=0.0475, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="2Y", rate=0.0455, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="3Y", rate=0.0445, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="5Y", rate=0.0435, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="7Y", rate=0.0430, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="10Y", rate=0.0425, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="15Y", rate=0.0420, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="20Y", rate=0.0415, base_date=base_date, curve_id="USD.OIS"),
        RatesQuote.ois_swap(tenor="30Y", rate=0.0410, base_date=base_date, curve_id="USD.OIS"),
    ]

    return quotes


def create_usd_sofr_quotes():
    """Create USD SOFR quotes for forward curve."""
    base_date = Date(2024, 1, 15)

    quotes = [
        # SOFR futures (short end)
        RatesQuote.futures(tenor="1M", rate=0.0540, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.futures(tenor="3M", rate=0.0545, base_date=base_date, curve_id="USD.SOFR"),
        # SOFR swaps (1Y - 30Y)
        RatesQuote.swap(tenor="1Y", rate=0.0480, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="2Y", rate=0.0460, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="3Y", rate=0.0450, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="5Y", rate=0.0440, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="7Y", rate=0.0435, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="10Y", rate=0.0430, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="15Y", rate=0.0425, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="20Y", rate=0.0420, base_date=base_date, curve_id="USD.SOFR"),
        RatesQuote.swap(tenor="30Y", rate=0.0415, base_date=base_date, curve_id="USD.SOFR"),
    ]

    return quotes


def create_cds_quotes():
    """Create CDS quotes for credit curve."""
    base_date = Date(2024, 1, 15)

    quotes = [
        # CDS par spreads at standard tenors
        CreditQuote.cds_par_spread(
            tenor="6M",
            spread=0.0080,  # 80bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="1Y",
            spread=0.0120,  # 120bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="2Y",
            spread=0.0150,  # 150bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="3Y",
            spread=0.0180,  # 180bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="5Y",
            spread=0.0200,  # 200bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="7Y",
            spread=0.0220,  # 220bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
        CreditQuote.cds_par_spread(
            tenor="10Y",
            spread=0.0250,  # 250bps
            base_date=base_date,
            curve_id="ACME.CDS",
        ),
    ]

    return quotes


def main() -> None:
    """Calibrate discount, forward, and credit curves."""
    base_date = Date(2024, 1, 15)

    # 1. Calibrate USD OIS discount curve

    ois_quotes = create_usd_ois_quotes()

    # Create calibration plan
    plan = {
        "base_date": base_date.to_dict(),
        "steps": [
            {
                "kind": "discount",
                "id": "USD.OIS",
                "quotes": [q.to_dict() for q in ois_quotes],
                "interpolation": "log_linear",
                "day_count": "Act360",
            }
        ],
        "config": {"solver": {"kind": "brent"}, "validation_mode": "strict"},
    }

    # Execute calibration
    market_ctx, _report = execute_calibration_v2(plan)

    # Retrieve calibrated curve
    ois_curve = market_ctx.get_discount("USD.OIS")

    # Display discount factors

    for tenor_str in ["1M", "3M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"]:
        date = base_date.add_tenor(tenor_str)
        df = ois_curve.df_on_date(date)
        yf = ois_curve.day_count().year_fraction(base_date, date)
        -math.log(df) / yf if yf > 0 else 0.0

    # 2. Calibrate USD SOFR forward curve

    sofr_quotes = create_usd_sofr_quotes()

    # Create calibration plan (needs discount curve dependency)
    {
        "base_date": base_date.to_dict(),
        "steps": [
            {
                "kind": "forward",
                "id": "USD.SOFR",
                "quotes": [q.to_dict() for q in sofr_quotes],
                "discount_curve_id": "USD.OIS",  # Dependency
                "interpolation": "log_linear",
                "day_count": "Act360",
            }
        ],
        "config": {"solver": {"kind": "brent"}, "validation_mode": "strict"},
    }

    # Note: Need to merge with existing market context
    # For simplicity, we'll show the structure

    # 3. Calibrate credit spread curve

    cds_quotes = create_cds_quotes()

    # Create calibration plan for hazard curve
    plan_credit = {
        "base_date": base_date.to_dict(),
        "steps": [
            {
                "kind": "hazard",
                "id": "ACME.CDS",
                "quotes": [q.to_dict() for q in cds_quotes],
                "discount_curve_id": "USD.OIS",  # For discounting CDS cashflows
                "recovery_rate": 0.40,
                "interpolation": "log_linear",
            }
        ],
        "config": {"solver": {"kind": "brent"}, "validation_mode": "strict"},
    }

    # Execute credit calibration
    market_ctx_credit, _report_credit = execute_calibration_v2(plan_credit)

    # Retrieve hazard curve
    hazard_curve = market_ctx_credit.get_hazard("ACME.CDS")

    # Display survival probabilities and credit spreads

    for tenor_str in ["6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y"]:
        date = base_date.add_tenor(tenor_str)
        surv_prob = hazard_curve.survival_probability(base_date, date)
        1.0 - surv_prob

        # Approximate hazard rate from survival probability
        yf = hazard_curve.day_count().year_fraction(base_date, date)
        -math.log(surv_prob) / yf if yf > 0 else 0.0

    # 4. Validation: reprice input instruments

    from finstack import CreditDefaultSwap, create_standard_registry

    registry = create_standard_registry()

    for quote in cds_quotes[:5]:  # First 5 for brevity
        # Create CDS at market spread
        maturity = base_date.add_tenor(quote.tenor)
        cds = CreditDefaultSwap(
            id=f"CDS.{quote.tenor}",
            notional=Money.from_code(10_000_000, "USD"),
            spread=quote.spread,
            issue_date=base_date,
            maturity_date=maturity,
            is_protection_buyer=True,
            hazard_curve_id="ACME.CDS",
            discount_curve_id="USD.OIS",
        )

        # Price with calibrated curves
        registry.price_cds(cds, "discounting", market_ctx_credit)

        # For par CDS, PV should be close to zero

    # 5. Summary


# Import math for calculations
import math

from finstack import Money

if __name__ == "__main__":
    main()
