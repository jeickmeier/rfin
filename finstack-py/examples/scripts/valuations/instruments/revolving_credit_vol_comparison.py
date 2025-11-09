#!/usr/bin/env python3
# ruff: noqa: T201
"""Revolving Credit Volatility Comparison Example.

Compares revolving credit pricing across different modeling approaches:
1. Deterministic pricer (constant utilization)
2. Stochastic pricer with 0% volatility (should match deterministic)
3. Stochastic pricer with varying volatility levels

This demonstrates:
- How volatility affects facility valuation
- Convergence of stochastic pricer to deterministic at 0% vol
- Impact of mean-reverting utilization dynamics
- Monte Carlo variance reduction techniques
"""

from datetime import date
import math

import pandas as pd

from finstack import Money
from finstack.core.currency import USD
from finstack.core.market_data import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve, ForwardCurve
from finstack.valuations.instruments import RevolvingCredit


def create_market_data() -> tuple[MarketContext, date]:
    """Create market data with discount and forward curves."""
    val_date = date(2025, 1, 1)

    # Discount curve (SOFR-based, ~4% rate)
    rate = 0.04
    disc_curve = DiscountCurve(
        "USD-OIS",
        val_date,
        [
            (0.0, 1.0),
            (0.25, math.exp(-rate * 0.25)),
            (0.5, math.exp(-rate * 0.5)),
            (1.0, math.exp(-rate * 1.0)),
            (2.0, math.exp(-rate * 2.0)),
            (3.0, math.exp(-rate * 3.0)),
            (5.0, math.exp(-rate * 5.0)),
        ],
    )

    # Forward curve for floating rate (SOFR 3M, ~4.5% forward rates)
    forward_curve = ForwardCurve(
        "USD-SOFR-3M",
        0.25,  # 3-month tenor
        [
            (0.0, 0.0450),
            (1.0, 0.0475),
            (2.0, 0.0500),
            (3.0, 0.0525),
            (5.0, 0.0550),
        ],
        base_date=val_date,
    )

    market = MarketContext()
    market.insert_discount(disc_curve)
    market.insert_forward(forward_curve)

    return market, val_date


def create_deterministic_facility(
    instrument_id: str,
    commitment: Money,
    drawn: Money,
    commitment_date: date,
    maturity_date: date,
    margin_bp: float,
    commitment_fee_bp: float,
) -> RevolvingCredit:
    """Create a deterministic revolving credit facility."""
    return RevolvingCredit.builder(
        instrument_id=instrument_id,
        commitment_amount=commitment,
        drawn_amount=drawn,
        commitment_date=commitment_date,
        maturity_date=maturity_date,
        base_rate_spec={
            "type": "floating",
            "index_id": "USD-SOFR-3M",
            "margin_bp": margin_bp,
            "reset_freq": "quarterly",
        },
        payment_frequency="quarterly",
        fees={
            "upfront_fee": None,
            "commitment_fee_bp": commitment_fee_bp,
            "usage_fee_bp": 0.0,
            "facility_fee_bp": 0.0,
        },
        draw_repay_spec={"deterministic": []},  # No draws/repays
        discount_curve="USD-OIS",
    )


def create_stochastic_facility(
    instrument_id: str,
    commitment: Money,
    drawn: Money,
    commitment_date: date,
    maturity_date: date,
    margin_bp: float,
    commitment_fee_bp: float,
    volatility: float,
    num_paths: int = 10000,
    seed: int = 42,
    antithetic: bool = True,
) -> RevolvingCredit:
    """Create a stochastic revolving credit facility with mean-reverting utilization."""
    initial_util = drawn.amount / commitment.amount

    return RevolvingCredit.builder(
        instrument_id=instrument_id,
        commitment_amount=commitment,
        drawn_amount=drawn,
        commitment_date=commitment_date,
        maturity_date=maturity_date,
        base_rate_spec={
            "type": "floating",
            "index_id": "USD-SOFR-3M",
            "margin_bp": margin_bp,
            "reset_freq": "quarterly",
        },
        payment_frequency="quarterly",
        fees={
            "upfront_fee": None,
            "commitment_fee_bp": commitment_fee_bp,
            "usage_fee_bp": 0.0,
            "facility_fee_bp": 0.0,
        },
        draw_repay_spec={
            "stochastic": {
                "utilization_process": {
                    "type": "mean_reverting",
                    "target_rate": initial_util,  # Mean-revert to current utilization
                    "speed": 0.5,  # Moderate mean reversion speed
                    "volatility": volatility,
                },
                "num_paths": num_paths,
                "seed": seed,
                "antithetic": antithetic,
                "use_sobol_qmc": False,
                "default_model": None,
            }
        },
        discount_curve="USD-OIS",
    )


def main() -> pd.DataFrame:
    """Main example comparing deterministic and stochastic pricing."""
    print("=" * 80)
    print("REVOLVING CREDIT VOLATILITY COMPARISON")
    print("=" * 80)

    # Create market data
    market, val_date = create_market_data()
    maturity_date = date(2030, 1, 1)  # 5 year term

    # Facility parameters
    commitment_amount = Money(10_000_000.0, USD)
    drawn_amount = Money(3_000_000.0, USD)  # 30% utilization
    margin_bp = 150.0
    commitment_fee_bp = 50.0

    print("\nFacility Configuration:")
    print(f"  Commitment: {commitment_amount}")
    print(f"  Drawn: {drawn_amount} ({drawn_amount.amount / commitment_amount.amount * 100:.1f}%)")
    print(f"  Undrawn: {commitment_amount.checked_sub(drawn_amount)}")
    print(f"  Term: {val_date} to {maturity_date} (5 years)")
    print(f"  Rate: Floating (SOFR 3M) + {margin_bp}bp")
    print(f"  Commitment Fee: {commitment_fee_bp}bp on undrawn")

    # =========================================================================
    # Example 1: Deterministic Pricing
    # =========================================================================
    print("\n" + "=" * 80)
    print("EXAMPLE 1: Deterministic Pricing (Constant Utilization)")
    print("=" * 80)

    det_facility = create_deterministic_facility(
        instrument_id="RC_DET",
        commitment=commitment_amount,
        drawn=drawn_amount,
        commitment_date=val_date,
        maturity_date=maturity_date,
        margin_bp=margin_bp,
        commitment_fee_bp=commitment_fee_bp,
    )

    det_npv = det_facility.npv(market, val_date)
    print(f"\n✓ Deterministic NPV: ${det_npv.amount:,.2f}")
    print(f"  (Assumes constant {drawn_amount.amount / commitment_amount.amount:.1%} utilization)")

    # =========================================================================
    # Example 2: Stochastic Pricing with Near-Zero Volatility
    # =========================================================================
    print("\n" + "=" * 80)
    print("EXAMPLE 2: Stochastic Pricing with Near-Zero Volatility")
    print("=" * 80)
    print("(Should converge to deterministic value)")

    stoch_0vol = create_stochastic_facility(
        instrument_id="RC_STOCH_0VOL",
        commitment=commitment_amount,
        drawn=drawn_amount,
        commitment_date=val_date,
        maturity_date=maturity_date,
        margin_bp=margin_bp,
        commitment_fee_bp=commitment_fee_bp,
        volatility=0.0001,  # Near-zero volatility (0.01%)
        num_paths=5000,
        antithetic=True,
    )

    stoch_0vol_npv = stoch_0vol.npv(market, val_date)
    diff_0vol = stoch_0vol_npv.amount - det_npv.amount
    pct_diff_0vol = (diff_0vol / det_npv.amount) * 100 if det_npv.amount != 0 else 0

    print(f"\n✓ Stochastic NPV (0.01% vol): ${stoch_0vol_npv.amount:,.2f}")
    print(f"  Difference from deterministic: ${diff_0vol:,.2f} ({pct_diff_0vol:+.4f}%)")

    if abs(pct_diff_0vol) < 1.0:  # Within 1%
        print("  ✓ Good convergence (within 1%)")
    else:
        print("  ⚠ Convergence outside 1% tolerance")

    # =========================================================================
    # Example 3: Volatility Grid Comparison
    # =========================================================================
    print("\n" + "=" * 80)
    print("EXAMPLE 3: Volatility Grid Comparison")
    print("=" * 80)

    # Define volatility grid (annualized utilization volatilities)
    vol_grid = [0.0001, 0.05, 0.10, 0.15, 0.20, 0.30, 0.50]
    num_paths = 10000

    print("\nConfiguration:")
    print(f"  Volatility levels: {[f'{v:.0%}' for v in vol_grid]}")
    print(f"  Paths per scenario: {num_paths:,}")
    print("  Variance reduction: Antithetic enabled")

    results = []

    print("\nPricing across volatility grid...")
    for vol in vol_grid:
        print(f"  Computing vol={vol:.0%}...", end="", flush=True)

        stoch_facility = create_stochastic_facility(
            instrument_id=f"RC_VOL_{int(vol*100)}",
            commitment=commitment_amount,
            drawn=drawn_amount,
            commitment_date=val_date,
            maturity_date=maturity_date,
            margin_bp=margin_bp,
            commitment_fee_bp=commitment_fee_bp,
            volatility=vol,
            num_paths=num_paths,
            antithetic=True,
        )

        npv = stoch_facility.npv(market, val_date)
        diff_from_det = npv.amount - det_npv.amount
        pct_diff = (diff_from_det / det_npv.amount) * 100 if det_npv.amount != 0 else 0

        results.append({
            "Volatility": vol,
            "Volatility_pct": f"{vol:.0%}",
            "NPV": npv.amount,
            "Diff_from_Det": diff_from_det,
            "Pct_Diff": pct_diff,
        })

        print(f" NPV=${npv.amount:,.0f}")

    # Create DataFrame for analysis
    df = pd.DataFrame(results)

    print("\n" + "-" * 80)
    print("Pricing Results Summary:")
    print("-" * 80)
    print(f"{'Volatility':<12} {'NPV ($)':<16} {'Diff from Det ($)':<20} {'Diff (%)':<12}")
    print("-" * 80)
    for _, row in df.iterrows():
        print(f"{row['Volatility_pct']:<12} ${row['NPV']:>14,.2f} ${row['Diff_from_Det']:>18,.2f} {row['Pct_Diff']:>10.4f}%")
    print("-" * 80)
    print(f"{'Deterministic':<12} ${det_npv.amount:>14,.2f} {'(baseline)':<20}")
    print("-" * 80)

    # =========================================================================
    # Analysis and Insights
    # =========================================================================
    print("\n" + "=" * 80)
    print("ANALYSIS & INSIGHTS")
    print("=" * 80)

    # Find max/min NPV
    max_row = df.loc[df["NPV"].idxmax()]
    min_row = df.loc[df["NPV"].idxmin()]

    print("\nNPV Range:")
    print(f"  Maximum: ${max_row['NPV']:,.2f} at {max_row['Volatility_pct']} vol")
    print(f"  Minimum: ${min_row['NPV']:,.2f} at {min_row['Volatility_pct']} vol")
    print(f"  Range: ${max_row['NPV'] - min_row['NPV']:,.2f}")

    # Volatility impact
    high_vol_row = df[df["Volatility"] == 0.5].iloc[0] if 0.5 in df["Volatility"].values else None
    if high_vol_row is not None:
        print("\nHigh Volatility Impact (50% vol vs deterministic):")
        print(f"  NPV change: ${high_vol_row['Diff_from_Det']:+,.2f} ({high_vol_row['Pct_Diff']:+.2f}%)")
        print("  Interpretation: ", end="")
        if high_vol_row["Diff_from_Det"] > 0:
            print("Higher volatility increases facility value")
            print("    → Optionality value from flexible draws/repays")
        else:
            print("Higher volatility decreases facility value")
            print("    → Increased exposure to adverse utilization paths")

    # Near-zero vol convergence
    zero_vol_df = df[df["Volatility"] < 0.001]  # Use tolerance for floating point comparison
    if not zero_vol_df.empty:
        zero_vol_row = zero_vol_df.iloc[0]
        print("\nConvergence Check (near-zero vol):")
        print(f"  Stochastic (0.01% vol): ${zero_vol_row['NPV']:,.2f}")
        print(f"  Deterministic: ${det_npv.amount:,.2f}")
        print(f"  Difference: ${zero_vol_row['Diff_from_Det']:,.2f} ({zero_vol_row['Pct_Diff']:+.4f}%)")
        if abs(zero_vol_row["Pct_Diff"]) < 1.0:
            print("  ✓ Excellent convergence (within 1%)")
        elif abs(zero_vol_row["Pct_Diff"]) < 2.5:
            print("  ✓ Good convergence (within 2.5%)")
        else:
            print("  ⚠ Convergence could be improved (increase num_paths)")

    # Volatility impact visualization
    print("\n" + "-" * 80)
    print("Volatility Impact Visualization:")
    print("-" * 80)
    max_npv_val = df["NPV"].max()
    min_npv_val = df["NPV"].min()
    npv_range = max_npv_val - min_npv_val

    for _, row in df.iterrows():
        vol_pct = row["Volatility_pct"]
        npv = row["NPV"]
        # Calculate bar length (0-50 characters)
        bar_len = int((npv - min_npv_val) / npv_range * 50) if npv_range > 0 else 25
        bar = "█" * bar_len
        print(f"  {vol_pct:>4} │{bar:<50}│ ${npv:>12,.0f}")
    print("-" * 80)

    # Model recommendations
    print("\n" + "-" * 80)
    print("Modeling Recommendations:")
    print("-" * 80)
    print("• Use deterministic pricing when:")
    print("  - Utilization is highly predictable (e.g., seasonal patterns)")
    print("  - Fast valuation is required (risk systems, P&L)")
    print("  - Conservative/simplified assumptions are acceptable")
    print("\n• Use stochastic pricing when:")
    print("  - Utilization is uncertain or mean-reverting")
    print("  - Need to capture optionality value")
    print("  - Pricing facilities with usage-dependent features")
    print("  - Stress testing under various utilization scenarios")
    print("\n• Calibration tips:")
    print("  - Set target_rate to historical average utilization")
    print("  - Estimate volatility from historical utilization data")
    print("  - Use higher mean reversion speed for stable borrowers")
    print("  - Increase num_paths if convergence is poor (try 20k-50k)")

    print("\n" + "=" * 80)
    print("Example completed successfully!")
    print("=" * 80)

    return df


if __name__ == "__main__":
    results_df = main()
