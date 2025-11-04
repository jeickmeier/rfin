"""
Credit‑Risky Revolving Credit Example (2‑factor MC)

Demonstrates pricing a revolving credit facility with stochastic utilization
and market‑anchored credit risk (hazard/spread) using the new mc_config.
"""

from datetime import date
from typing import Optional

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, HazardCurve, ForwardCurve
from finstack.valuations.instruments import RevolvingCredit
from finstack.valuations.pricer import create_standard_registry


def build_market(as_of: date) -> MarketContext:
    """Create minimal market inputs: discount curve + borrower hazard curve."""
    disc = DiscountCurve(
        "USD.OIS",
        as_of,
        [
            (0.0, 1.0),
            (1.0, 0.97),
            (3.0, 0.91),
        ],
    )

    # Constant hazard ~2% with 40% recovery (approx 120 bps spread)
    hazard = HazardCurve(
        "BORROWER-HZD",
        as_of,
        [
            (1.0, 0.02),
            (5.0, 0.02),
        ],
        recovery_rate=0.40,
    )

    # SOFR 1M forward curve (tenor = 1/12 years)
    sofr_1m = ForwardCurve(
        "USD.SOFR.1M",
        1.0 / 12.0,
        [
            (0.0, 0.0530),
            (1.0, 0.0550),
            (3.0, 0.0570),
        ],
        base_date=as_of,
    )

    market = MarketContext()
    market.insert_discount(disc)
    market.insert_forward(sofr_1m)
    market.insert_hazard(hazard)
    return market


def build_revolver(
    implied_vol: float,
    util_credit_corr: Optional[float] = 0.8,
    num_paths: int = 4000,
    seed: int = 42,
):
    """Create a credit‑risky revolver with market‑anchored credit process.

    implied_vol: CDS (index) option implied vol used to scale credit spread vol.
    util_credit_corr: correlation between utilization and credit (+0.8 typical).
    """
    return RevolvingCredit.builder(
        instrument_id=f"REVOLVER_CR_{implied_vol:.2f}",
        commitment_amount=Money(5_000_000, USD),
        drawn_amount=Money(1_500_000, USD),
        commitment_date=date(2025, 1, 1),
        maturity_date=date(2028, 1, 1),
        base_rate_spec={
            "type": "floating",
            "index_id": "USD.SOFR.1M",
            "margin_bp": 150.0,
            "reset_freq": "monthly",
        },
        payment_frequency="quarterly",
        fees={
            "commitment_fee_bp": 25.0,
            "usage_fee_bp": 50.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": 0.40,
                    "speed": 0.50,
                    "volatility": 0.20,
                },
                "num_paths": num_paths,
                "seed": seed,
                "mc_config": {
                    "recovery_rate": 0.40,
                    "credit_spread_process": {
                        "market_anchored": {
                            "hazard_curve_id": "BORROWER-HZD",
                            "kappa": 0.50,
                            "implied_vol": implied_vol,
                            "tenor_years": None,  # default to facility’s maturity horizon
                        }
                    },
                    # Either supply a full 3×3 correlation, or just util‑credit correlation
                    "util_credit_corr": util_credit_corr,
                },
            }
        },
        discount_curve="USD.OIS",
    )


def main():
    as_of = date(2025, 1, 1)
    market = build_market(as_of)
    registry = create_standard_registry()

    print("\n=== CREDIT‑RISKY REVOLVER (Market‑Anchored Credit) ===")
    base = build_revolver(implied_vol=0.25)
    pv = registry.price(base, "monte_carlo_gbm", market).value
    print(f"Base PV (implied vol 0.25): {pv}")

    print("\nVolatility sensitivity (CDS option vol → PV):")
    for vol in [0.0001, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30, 0.5]:
        inst = build_revolver(implied_vol=vol, num_paths=3000)
        val = registry.price(inst, "monte_carlo_gbm", market).value
        print(f"  vol={vol:0.2f} → {val}")

    print("\nCorrelation sensitivity (util‑credit ρ → PV):")
    for rho in [0.4, 0.6, 0.8, 0.9]:
        inst = build_revolver(implied_vol=0.25, util_credit_corr=rho, num_paths=3000)
        val = registry.price(inst, "monte_carlo_gbm", market).value
        print(f"  rho={rho:0.2f} → {val}")


if __name__ == "__main__":
    main()


