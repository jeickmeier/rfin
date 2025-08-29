#!/usr/bin/env python3
"""Example demonstrating financial policies in finstack."""

from finstack import (
    Date, Money, Currency, USD,
    GridMarginPolicy, IndexFallbackPolicy, DSCRSweepPolicy
)
from datetime import date

def main():
    print("=" * 60)
    print("Financial Policy Examples")
    print("=" * 60)
    
    # 1. Grid Margin Policy
    print("\n1. Grid Margin Policy")
    print("-" * 30)
    
    grid_policy = GridMarginPolicy()
    
    # Set base spreads for different ratings (in bps)
    grid_policy.set_base_spread("AAA", 25)  # 25 bps
    grid_policy.set_base_spread("AA", 50)   # 50 bps
    grid_policy.set_base_spread("A", 75)    # 75 bps
    grid_policy.set_base_spread("BBB", 150) # 150 bps
    grid_policy.set_base_spread("BB", 300)  # 300 bps
    
    # Set sector adjustments (in bps)
    grid_policy.set_sector_adjustment("Financial", -10)  # 10 bps tighter
    grid_policy.set_sector_adjustment("Corporate", 0)    # No adjustment
    grid_policy.set_sector_adjustment("Energy", 25)      # 25 bps wider
    grid_policy.set_sector_adjustment("Tech", 15)        # 15 bps wider
    
    # Add maturity buckets (years, adjustment in bps)
    grid_policy.add_maturity_bucket(1.0, 0)    # No adjustment for < 1Y
    grid_policy.add_maturity_bucket(3.0, 10)   # +10 bps for 1-3Y
    grid_policy.add_maturity_bucket(5.0, 25)   # +25 bps for 3-5Y
    grid_policy.add_maturity_bucket(7.0, 40)   # +40 bps for 5-7Y
    grid_policy.add_maturity_bucket(10.0, 60)  # +60 bps for 7-10Y
    
    # Calculate spreads for different scenarios
    scenarios = [
        ("AAA", "Financial", 2.5),
        ("AA", "Corporate", 4.0),
        ("BBB", "Energy", 5.5),
        ("BB", "Tech", 8.0),
    ]
    
    for rating, sector, maturity in scenarios:
        spread = grid_policy.calculate_spread(rating, sector, maturity)
        print(f"  {rating} {sector} @ {maturity}Y: {spread:.0f} bps")
    
    # 2. Index Fallback Policy
    print("\n2. Index Fallback Policy")
    print("-" * 30)
    
    fallback_policy = IndexFallbackPolicy()
    
    # Set up fallback chains
    fallback_policy.set_fallback_chain("LIBOR_3M", ["SOFR_3M", "FED_FUNDS"])
    fallback_policy.set_fallback_chain("LIBOR_6M", ["SOFR_6M", "FED_FUNDS"])
    fallback_policy.set_fallback_chain("EURIBOR_3M", ["ESTER_3M", "ECB_RATE"])
    
    # Set fallback spread adjustments (in bps)
    fallback_policy.set_fallback_spread("LIBOR_3M", 26)   # 26 bps spread
    fallback_policy.set_fallback_spread("LIBOR_6M", 42)   # 42 bps spread
    fallback_policy.set_fallback_spread("EURIBOR_3M", 8)  # 8 bps spread
    
    # Set static rates for some indices
    fallback_policy.set_static_rate("FED_FUNDS", 0.0533)  # 5.33%
    fallback_policy.set_static_rate("ECB_RATE", 0.0400)   # 4.00%
    
    # Available rates (simulating market data)
    available_rates = {
        "SOFR_3M": 0.0525,    # 5.25%
        "SOFR_6M": 0.0535,    # 5.35%
        "ESTER_3M": 0.0385,   # 3.85%
    }
    
    # Get rates with fallback
    indices = ["LIBOR_3M", "LIBOR_6M", "EURIBOR_3M", "SOFR_3M", "FED_FUNDS"]
    
    for index in indices:
        try:
            rate = fallback_policy.get_rate(index, available_rates)
            print(f"  {index}: {rate*100:.2f}%")
        except:
            print(f"  {index}: No rate available")
    
    # 3. DSCR Sweep Policy
    print("\n3. DSCR Sweep Policy")
    print("-" * 30)
    
    sweep_policy = DSCRSweepPolicy()
    
    # Configure sweep tiers (DSCR threshold, sweep percentage)
    sweep_policy.add_sweep_tier(1.0, 1.0)   # 100% sweep below 1.0x DSCR
    sweep_policy.add_sweep_tier(1.25, 0.75) # 75% sweep at 1.25x DSCR
    sweep_policy.add_sweep_tier(1.5, 0.50)  # 50% sweep at 1.5x DSCR
    sweep_policy.add_sweep_tier(1.75, 0.25) # 25% sweep at 1.75x DSCR
    sweep_policy.add_sweep_tier(2.0, 0.0)   # No sweep above 2.0x DSCR
    
    # Set policy parameters
    sweep_policy.set_min_dscr(1.0)      # Minimum DSCR threshold
    sweep_policy.set_max_sweep(1.0)     # Maximum sweep percentage (100%)
    
    # Calculate sweep amounts for different DSCR levels
    test_dscrs = [0.9, 1.1, 1.3, 1.6, 1.9, 2.2]
    excess_cash = Money(1_000_000, USD)
    minimum_cash = Money(100_000, USD)  # Reserve requirement
    
    print(f"  Excess cash: {excess_cash}")
    print(f"  Minimum cash reserve: {minimum_cash}")
    print()
    
    for dscr in test_dscrs:
        sweep_pct = sweep_policy.calculate_sweep(dscr)
        sweep_amount = sweep_policy.calculate_sweep_amount(
            dscr, 
            excess_cash,
            minimum_cash
        )
        print(f"  DSCR {dscr:.1f}x: {sweep_pct*100:.0f}% sweep = {sweep_amount}")
    
    print()
    print("Example completed successfully!")

if __name__ == "__main__":
    main()
