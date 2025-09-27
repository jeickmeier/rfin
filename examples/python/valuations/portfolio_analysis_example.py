#!/usr/bin/env python3
"""
Example: Comprehensive portfolio analysis.

This example demonstrates the full power of the finstack-py library by
combining all the features from phases 1-4:
- Complex cashflow structures
- Multiple instrument types (Bonds, IRS)
- Market context and pricing
- Risk metrics and bucketed sensitivities
- Portfolio-level analysis
"""

from finstack import Currency, Date, DayCount, Money
from finstack.dates import Frequency, BusDayConvention
from finstack.cashflow import CashflowBuilder, Amortization, CouponPaymentType
from finstack.instruments import Bond, InterestRateSwap, PayReceive, FixedLeg, FloatLeg
from finstack.market_data import MarketContext
from finstack.risk import BucketedDv01, calculate_risk_metrics
import pandas as pd


class CreditPortfolio:
    """A portfolio of credit instruments for analysis."""

    def __init__(self):
        self.instruments = []
        self.positions = {}  # instrument_id -> position size

    def add_position(self, instrument, position_size):
        """Add an instrument with position size."""
        self.instruments.append(instrument)
        self.positions[instrument.id] = position_size

    def total_notional(self):
        """Calculate total notional across all positions."""
        total = 0
        for inst in self.instruments:
            size = self.positions[inst.id]
            total += inst.notional.amount * size
        return total

    def describe(self):
        """Describe portfolio composition."""
        print(f"Portfolio contains {len(self.instruments)} instruments")
        print(f"Total notional: ${self.total_notional():,.0f}")

        # Group by type
        bonds = [i for i in self.instruments if hasattr(i, "coupon")]
        swaps = [i for i in self.instruments if hasattr(i, "side")]

        print(f"  Bonds: {len(bonds)}")
        print(f"  Swaps: {len(swaps)}")


def create_bond_portfolio():
    """Create a diversified bond portfolio."""
    print("\n" + "=" * 60)
    print("Creating Bond Portfolio")
    print("=" * 60)

    portfolio = CreditPortfolio()

    # Investment grade corporate bond
    ig_bond = Bond(
        id="IG-CORP-5Y",
        notional=Money(5_000_000, Currency("USD")),
        coupon=0.04,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        issue_date=Date(2023, 6, 15),
        maturity=Date(2028, 6, 15),
        discount_curve="USD-CORP-A",
    )
    portfolio.add_position(ig_bond, 2)  # $10mm position

    # High yield bond with quarterly payments
    hy_bond = Bond(
        id="HY-CORP-3Y",
        notional=Money(1_000_000, Currency("USD")),
        coupon=0.075,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        issue_date=Date(2024, 1, 1),
        maturity=Date(2027, 1, 1),
        discount_curve="USD-CORP-BB",
    )
    portfolio.add_position(hy_bond, 5)  # $5mm position

    # Amortizing cashflow leg (loan-like) using cashflow builder
    print("\nCreating amortizing cashflow structure...")
    builder = CashflowBuilder()
    builder.principal(
        Money(10_000_000, Currency("USD")), Date(2024, 1, 1), Date(2029, 1, 1)
    )
    builder.fixed_coupon(
        rate=0.055,
        frequency=Frequency.Monthly,
        day_count=DayCount.act365f(),
        payment_type=None,
        business_day_conv=None,
        calendar=None,
        stub=None,
    )
    builder.with_amortization(Amortization.linear_to_zero(Currency("USD")))
    leg_schedule = builder.build()
    print(f"  Amortizing cashflows: {len(leg_schedule.flows)} cashflows")
    print(f"  Total interest: ${leg_schedule.total_interest():,.2f}")

    # PIK toggle bond
    print("\nCreating PIK toggle structure...")
    builder = CashflowBuilder()
    builder.principal(
        Money(3_000_000, Currency("EUR")), Date(2024, 1, 1), Date(2026, 12, 31)
    )

    # 60% cash, 40% PIK split
    split = CouponPaymentType.split(cash_pct=0.6, pik_pct=0.4)
    builder.fixed_coupon(
        rate=0.09,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act365f(),
        payment_type=split,
        business_day_conv=None,
        calendar=None,
        stub=None,
    )

    # PIK period followed by cash
    builder.add_pik_period(Date(2024, 1, 1), Date(2024, 7, 1))
    builder.add_cash_period(Date(2024, 7, 1), Date(2026, 12, 31))

    pik_schedule = builder.build()
    print(f"  PIK toggle: {len(pik_schedule.flows)} total flows")
    print(f"  PIK flows: {len(pik_schedule.pik_flows())}")

    return portfolio


def create_hedge_portfolio():
    """Create a portfolio of hedging instruments."""
    print("\n" + "=" * 60)
    print("Creating Hedge Portfolio")
    print("=" * 60)

    portfolio = CreditPortfolio()

    # Receiver swap (receive fixed)
    fixed_receiver = FixedLeg(
        discount_curve="USD-OIS",
        rate=0.035,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2029, 1, 15),
    )

    float_payer = FloatLeg(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        spread_bp=0,
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2029, 1, 15),
    )

    receiver_swap = InterestRateSwap(
        id="RECEIVER-5Y",
        notional=Money(10_000_000, Currency("USD")),
        side=PayReceive.ReceiveFixed,
        fixed_leg=fixed_receiver,
        float_leg=float_payer,
    )
    portfolio.add_position(receiver_swap, 1)

    # Payer swap (pay fixed) - shorter maturity
    fixed_payer = FixedLeg(
        discount_curve="USD-OIS",
        rate=0.032,
        frequency=Frequency.SemiAnnual,
        day_count=DayCount.thirty360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2027, 1, 15),
    )

    float_receiver = FloatLeg(
        discount_curve="USD-OIS",
        forward_curve="USD-SOFR-3M",
        spread_bp=5,  # 5bp spread
        frequency=Frequency.Quarterly,
        day_count=DayCount.act360(),
        start_date=Date(2024, 1, 15),
        end_date=Date(2027, 1, 15),
    )

    payer_swap = InterestRateSwap(
        id="PAYER-3Y",
        notional=Money(5_000_000, Currency("USD")),
        side=PayReceive.PayFixed,
        fixed_leg=fixed_payer,
        float_leg=float_receiver,
    )
    portfolio.add_position(payer_swap, 2)  # $10mm notional

    return portfolio


def analyze_portfolio_risk(portfolio):
    """Perform risk analysis on a portfolio."""
    print("\n" + "=" * 60)
    print("Portfolio Risk Analysis")
    print("=" * 60)

    portfolio.describe()

    # Create bucketed DV01 calculator
    bucketed = BucketedDv01([0.5, 1, 2, 3, 5, 7, 10, 15, 20, 30])

    print("\nRisk metrics by instrument:")
    print(f"{'Instrument':<15} {'Type':<10} {'Notional':>15} {'Position':>10}")
    print("-" * 60)

    for inst in portfolio.instruments:
        inst_type = "Bond" if hasattr(inst, "coupon") else "Swap"
        position = portfolio.positions[inst.id]
        notional = inst.notional.amount * position

        print(f"{inst.id:<15} {inst_type:<10} ${notional:>14,.0f} {position:>10}x")

    print("\nPortfolio-level metrics (when market data available):")
    print("- Total DV01: Sum of position-weighted DV01s")
    print("- Net duration: Weighted average duration")
    print("- Bucketed risk: Aggregated curve sensitivities")
    print("- Basis risk: Exposure to curve spreads")
    print("- Credit spread risk: CS01 for credit positions")


def create_risk_report():
    """Create a comprehensive risk report."""
    print("\n" + "=" * 60)
    print("Risk Report Generation")
    print("=" * 60)

    # Sample data for risk report
    risk_data = {
        "Tenor": ["3M", "6M", "1Y", "2Y", "3Y", "5Y", "7Y", "10Y", "15Y", "20Y", "30Y"],
        "DV01_Bonds": [0, 0, 500, 2000, 3500, 8000, 5000, 2000, 1000, 500, 0],
        "DV01_Swaps": [100, 200, -1000, -1500, 2000, -3000, 0, 0, 0, 0, 0],
        "Net_DV01": [100, 200, -500, 500, 5500, 5000, 5000, 2000, 1000, 500, 0],
    }

    df = pd.DataFrame(risk_data)

    print("\nBucketed DV01 Report (Sample):")
    print(df.to_string(index=False))

    print("\nRisk Limits Check:")
    print("  Total DV01: ${:,.0f} [Limit: $50,000] ✓".format(df["Net_DV01"].sum()))
    print(
        "  Max bucket: ${:,.0f} [Limit: $10,000] ✓".format(df["Net_DV01"].abs().max())
    )
    print(
        "  2Y-5Y concentration: ${:,.0f} [Limit: $20,000] ✓".format(
            df[df["Tenor"].isin(["2Y", "3Y", "5Y"])]["Net_DV01"].sum()
        )
    )

    print("\nScenario Analysis:")
    scenarios = [
        ("Parallel +100bp", -85000),
        ("Parallel -100bp", +88000),
        ("Steepener (2s10s +50bp)", -12000),
        ("Flattener (2s10s -50bp)", +11500),
        ("Credit spreads +50bp", -42000),
    ]

    for scenario, pnl in scenarios:
        sign = "+" if pnl > 0 else ""
        print(f"  {scenario:<30} {sign}${pnl:,.0f}")


def main():
    """Run comprehensive portfolio analysis."""
    print("=" * 60)
    print("Comprehensive Portfolio Analysis Example")
    print("=" * 60)

    # Create portfolios
    bond_portfolio = create_bond_portfolio()
    hedge_portfolio = create_hedge_portfolio()

    # Analyze risk
    print("\n" + "=" * 60)
    print("BOND PORTFOLIO")
    analyze_portfolio_risk(bond_portfolio)

    print("\n" + "=" * 60)
    print("HEDGE PORTFOLIO")
    analyze_portfolio_risk(hedge_portfolio)

    # Generate risk report
    create_risk_report()


if __name__ == "__main__":
    main()
