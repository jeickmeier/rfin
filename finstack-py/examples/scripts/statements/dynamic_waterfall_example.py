"""Example script demonstrating dynamic capital structure features.

This script shows how to:
1. Build a financial model with a term loan and waterfall mechanics
2. Configure an Excess Cash Flow (ECF) sweep
3. Configure a PIK toggle based on liquidity
4. Evaluate the model and inspect the dynamic results
"""

from datetime import date
from finstack.core.dates.periods import PeriodId
from finstack.core.market_data.context import MarketContext
from finstack.core.market_data.term_structures import DiscountCurve
from finstack.statements.builder import ModelBuilder
from finstack.statements.types import AmountOrScalar, EcfSweepSpec, PaymentPriority, PikToggleSpec, WaterfallSpec

def run_example():
    print("Building Financial Model with Dynamic Capital Structure...")
    
    # 1. Create builder and define periods
    builder = ModelBuilder.new("LBO Model")
    builder.periods("2025Q1..2026Q4", None)
    
    # 2. Add operating model nodes (Revenue, EBITDA, Capex, Taxes)
    # Scenario: High EBITDA in 2025Q2 triggers a sweep
    builder.value(
        "revenue",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(2_000_000.0)), # High revenue
            (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2026, 1), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2026, 2), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2026, 3), AmountOrScalar.scalar(1_000_000.0)),
            (PeriodId.quarter(2026, 4), AmountOrScalar.scalar(1_000_000.0)),
        ]
    )
    
    builder.compute("ebitda", "revenue * 0.4") # 40% EBITDA margin
    builder.compute("capex", "revenue * 0.05") # 5% Capex
    builder.compute("taxes", "ebitda * 0.25")  # 25% Tax rate
    
    # Add liquidity metric for PIK toggle
    # Scenario: Low liquidity in 2025Q1 triggers PIK
    builder.value(
        "liquidity",
        [
            (PeriodId.quarter(2025, 1), AmountOrScalar.scalar(50_000.0)), # Triggers PIK (< 100k)
            (PeriodId.quarter(2025, 2), AmountOrScalar.scalar(500_000.0)),
            (PeriodId.quarter(2025, 3), AmountOrScalar.scalar(200_000.0)),
            (PeriodId.quarter(2025, 4), AmountOrScalar.scalar(250_000.0)),
            (PeriodId.quarter(2026, 1), AmountOrScalar.scalar(250_000.0)),
            (PeriodId.quarter(2026, 2), AmountOrScalar.scalar(275_000.0)),
            (PeriodId.quarter(2026, 3), AmountOrScalar.scalar(300_000.0)),
            (PeriodId.quarter(2026, 4), AmountOrScalar.scalar(325_000.0)),
        ]
    )
    
    # 3. Add Capital Structure
    # $10M Term Loan at 8% interest
    term_loan_spec = {
        "id": "TL-A",
        "currency": "USD",
        "notional_limit": {"amount": "10000000", "currency": "USD"},
        "issue": "2024-12-31",
        "maturity": "2030-01-01",
        "rate": {"Fixed": {"rate_bp": 800}},  # 8%
        "pay_freq": {"Months": 3},
        "day_count": "Act360",
        "bdc": "modified_following",
        "calendar_id": None,
        "stub": "None",
        "discount_curve_id": "USD-OIS",
        "credit_curve_id": None,
        "amortization": "None",
        "coupon_type": "Cash",  # Default to cash, PIK toggle overrides
        "upfront_fee": None,
        "ddtl": {
            "commitment_limit": {"amount": "10000000", "currency": "USD"},
            "availability_start": "2024-12-31",
            "availability_end": "2026-01-01",
            "draws": [
                {
                    "date": "2024-12-31",
                    "amount": {"amount": "10000000", "currency": "USD"},
                }
            ],
            "commitment_step_downs": [],
            "usage_fee_bp": 0,
            "commitment_fee_bp": 0,
            "fee_base": "Undrawn",
            "oid_policy": None,
        },
        "covenants": None,
        "pricing_overrides": {
            "quoted_clean_price": None,
            "rho_bump_decimal": None,
            "vega_bump_decimal": None,
            "implied_volatility": None,
            "quoted_spread_bp": None,
            "upfront_payment": None,
            "ytm_bump_decimal": None,
            "theta_period": None,
            "mc_seed_scenario": None,
            "adaptive_bumps": False,
            "spot_bump_pct": None,
            "vol_bump_pct": None,
            "rate_bump_bp": None,
        },
        "call_schedule": None,
        "attributes": {"tags": [], "meta": {}},
    }

    builder.add_custom_debt("TL-A", term_loan_spec)
    
    # 4. Configure Waterfall
    waterfall = WaterfallSpec(
        priority_of_payments=[
            PaymentPriority.Fees,
            PaymentPriority.Interest,
            PaymentPriority.Amortization,
            PaymentPriority.Sweep,
            PaymentPriority.Equity
        ],
        # Sweep 50% of ECF (EBITDA - Taxes - Capex)
        ecf_sweep=EcfSweepSpec(
            ebitda_node="ebitda",
            sweep_percentage=0.5,
            taxes_node="taxes",
            capex_node="capex",
            target_instrument_id="TL-A"
        ),
        # Toggle to PIK if liquidity < 100k
        pik_toggle=PikToggleSpec(
            liquidity_metric="liquidity",
            threshold=100_000.0,
            target_instrument_ids=["TL-A"]
        )
    )
    
    builder.waterfall(waterfall)
    
    # Add CS references to output for inspection
    builder.compute("interest_expense", "cs.interest_expense.total")
    builder.compute("interest_cash", "cs.interest_expense_cash.total")
    builder.compute("interest_pik", "cs.interest_expense_pik.total")
    builder.compute("principal_payment", "cs.principal_payment.total")
    builder.compute("debt_balance", "cs.debt_balance.total")
    
    # 5. Build and Evaluate
    model = builder.build()
    
    from finstack.statements.evaluator import Evaluator
    evaluator = Evaluator.new()
    
    # Minimal market context for capital structure pricing (flat USD discount curve)
    market_ctx = MarketContext()
    usd_ois_curve = DiscountCurve(
        "USD-OIS",
        date(2025, 1, 1),
        [
            (0.0, 1.0),
            (1.0, 0.97),
            (5.0, 0.85),
        ],
    )
    market_ctx.insert_discount(usd_ois_curve)

    results = evaluator.evaluate_with_market_context(model, market_ctx, date(2025, 1, 1))
    
    print("\nResults Summary:")
    separator = "-" * 108
    print(separator)
    print(f"{'Period':<10} | {'EBITDA':>12} | {'Liquidity':>12} | {'Interest':>12} | {'PIK':>12} | {'Principal':>12} | {'Balance':>12}")
    print(separator)
    
    for period in model.periods:
        pid = period.id
        ebitda = results.get("ebitda", pid)
        liquidity = results.get_or("liquidity", pid, 0.0)
        interest = results.get("interest_expense", pid)
        interest_pik = results.get_or("interest_pik", pid, 0.0)
        principal = results.get("principal_payment", pid)
        balance = results.get("debt_balance", pid)
        
        print(
            f"{str(pid):<10} | "
            f"{ebitda:>12,.0f} | "
            f"{liquidity:>12,.0f} | "
            f"{interest:>12,.0f} | "
            f"{interest_pik:>12,.0f} | "
            f"{principal:>12,.0f} | "
            f"{balance:>12,.0f}"
        )
        
    print(separator)
    
    # Verification
    q1_pik_interest = results.get_or("interest_pik", PeriodId.quarter(2025, 1), 0.0)
    
    print("\nAnalysis:")
    if q1_pik_interest > 0:
        print(f"✅ Q1 PIK Toggle Active: ${q1_pik_interest:,.0f} accrued as PIK interest")
    else:
        print("❌ Q1 PIK Toggle Failed")
        
    q2_principal = results.get("principal_payment", PeriodId.quarter(2025, 2))
    if q2_principal > 0:
        print(f"✅ Q2 ECF Sweep Active: Sweep payment of ${q2_principal:.0f} applied")
    else:
        print("❌ Q2 ECF Sweep Failed")

if __name__ == "__main__":
    run_example()

