#!/usr/bin/env python3
"""
Daily P&L Attribution Example

Demonstrates how to perform P&L attribution on a bond position to explain
daily MTM changes.
"""

import finstack
from datetime import date

# Note: This is a template example. Full implementation requires:
# 1. Loading actual market data snapshots at T₀ and T₁
# 2. Creating instruments with proper parameters
# 3. Running attribution

def main():
    print("=" * 60)
    print("Daily P&L Attribution Example")
    print("=" * 60)
    
    # Example structure (requires actual implementation):
    # 
    # # 1. Load market data for T-1 and T
    # market_yesterday = load_market("2025-01-15.json")
    # market_today = load_market("2025-01-16.json")
    # 
    # # 2. Create instrument
    # bond = finstack.Bond.fixed(
    #     id="US-BOND-001",
    #     notional=finstack.Money(1_000_000, "USD"),
    #     coupon=0.05,
    #     issue=date(2025, 1, 1),
    #     maturity=date(2030, 1, 1),
    #     discount_curve_id="USD-OIS"
    # )
    # 
    # # 3. Run attribution
    # attr = finstack.attribute_pnl(
    #     bond,
    #     market_yesterday,
    #     market_today,
    #     as_of_yesterday=date(2025, 1, 15),
    #     as_of_today=date(2025, 1, 16),
    #     method="parallel"
    # )
    # 
    # # 4. Display summary
    # print(f"Total P&L: {attr.total_pnl}")
    # print(f"Carry: {attr.carry} ({attr.carry/attr.total_pnl:.1%})")
    # print(f"Rates: {attr.rates_curves_pnl} ({attr.rates_curves_pnl/attr.total_pnl:.1%})")
    # print(f"Credit: {attr.credit_curves_pnl}")
    # print(f"FX: {attr.fx_pnl}")
    # print(f"Vol: {attr.vol_pnl}")
    # print(f"Residual: {attr.residual} ({attr.meta.residual_pct:.2}%)")
    # 
    # # 5. Explain tree
    # print("\nP&L Attribution Breakdown:")
    # print(attr.explain())
    # 
    # # 6. Export to CSV
    # csv_data = attr.to_csv()
    # with open("pnl_attribution.csv", "w") as f:
    #     f.write(csv_data)
    
    print("\nNote: This is a template. Implement market data loading and")
    print("      instrument creation to run actual attribution.")

if __name__ == "__main__":
    main()

