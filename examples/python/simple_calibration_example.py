#!/usr/bin/env python3
"""Simple market calibration example.

Demonstrates the new simplified calibration API that replaces the
over-engineered orchestrator and dependency DAG approach.
"""

from finstack import (
    Date, Currency, DayCount, Frequency,
    SimpleCalibration, InstrumentQuote, MarketContext
)
from datetime import date

def main():
    # Base setup
    base_date = Date.from_ymd(2025, 1, 1)
    base_currency = Currency.USD
    
    # Create simple calibration builder
    calibration = SimpleCalibration(base_date, base_currency)
    
    # Market quotes - mix of different instrument types
    quotes = [
        # Discount curve instruments
        InstrumentQuote.deposit(
            maturity=Date.from_ymd(2025, 2, 1),
            rate=0.045,
            day_count=DayCount.Act360
        ),
        InstrumentQuote.swap(
            maturity=Date.from_ymd(2026, 1, 1),
            rate=0.047,
            fixed_freq=Frequency.SemiAnnual,
            float_freq=Frequency.Quarterly,
            fixed_dc=DayCount.Thirty360,
            float_dc=DayCount.Act360,
            index="USD-SOFR-3M"
        ),
        InstrumentQuote.swap(
            maturity=Date.from_ymd(2027, 1, 1),
            rate=0.048,
            fixed_freq=Frequency.SemiAnnual,
            float_freq=Frequency.Quarterly,
            fixed_dc=DayCount.Thirty360,
            float_dc=DayCount.Act360,
            index="USD-SOFR-3M"
        ),
        
        # Credit curve instruments
        InstrumentQuote.cds(
            entity="AAPL",
            maturity=Date.from_ymd(2027, 1, 1),
            spread_bp=50.0,
            recovery_rate=0.4,
            currency=Currency.USD
        ),
        InstrumentQuote.cds(
            entity="AAPL",
            maturity=Date.from_ymd(2030, 1, 1),
            spread_bp=75.0,
            recovery_rate=0.4,
            currency=Currency.USD
        ),
        
        # Inflation instruments
        InstrumentQuote.inflation_swap(
            maturity=Date.from_ymd(2027, 1, 1),
            rate=0.025,
            index="US-CPI-U"
        ),
        InstrumentQuote.inflation_swap(
            maturity=Date.from_ymd(2030, 1, 1),
            rate=0.028,
            index="US-CPI-U"
        ),
    ]
    
    # Calibrate everything in one simple call
    # No DAG, no complex dependency resolution, just straightforward calibration
    market_context, report = calibration.calibrate(quotes)
    
    print(f"Calibration {'succeeded' if report.success else 'failed'}")
    print(f"Total iterations: {report.iterations}")
    print(f"Convergence reason: {report.convergence_reason}")
    
    # Check what we calibrated
    print("\nCalibrated market data:")
    
    # Discount curve (new API via Python wrapper if available)
    try:
        disc_curve = market_context.get_discount_curve("USD-OIS")
        print(f"✓ USD-OIS discount curve (retrieved)")
    except Exception:
        print("✗ USD-OIS discount curve not found")
    
    # Hazard curve (new API naming)
    try:
        hazard_curve = market_context.get_hazard_curve("AAPL-Senior")
        print(f"✓ AAPL hazard curve (retrieved)")
    except Exception:
        print("✗ AAPL hazard curve not found")
    
    # Inflation curve (new API naming)
    try:
        infl_curve = market_context.get_inflation_curve("US-CPI-U")
        print(f"✓ US-CPI-U inflation curve (retrieved)")
    except Exception:
        print("✗ US-CPI-U inflation curve not found")
    
    print("\n" + "="*50)
    print("Simple calibration completed!")
    print("Much cleaner than the DAG-based approach!")
    print("="*50)

if __name__ == "__main__":
    main()
