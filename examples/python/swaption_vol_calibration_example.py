#!/usr/bin/env python3
"""
Example demonstrating swaption volatility calibration using the new SwaptionVolCalibrator.

This example shows:
1. Creating swaption volatility quotes
2. Building discount curves required for forward swap rate calculation
3. Calibrating a swaption volatility surface
4. Accessing calibrated volatilities
"""

import finstack as fs
from datetime import date
from decimal import Decimal

def main():
    # Set up base date
    base_date = date(2025, 1, 1)
    
    # Create a simple discount curve first (required for forward rate calculation)
    disc_quotes = [
        fs.RatesQuote.deposit(
            maturity=date(2025, 4, 1),
            rate=0.04,
            day_count=fs.DayCount.Act360
        ),
        fs.RatesQuote.swap(
            maturity=date(2026, 1, 1),
            rate=0.042,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-SOFR"
        ),
        fs.RatesQuote.swap(
            maturity=date(2027, 1, 1),
            rate=0.043,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-SOFR"
        ),
        fs.RatesQuote.swap(
            maturity=date(2030, 1, 1),
            rate=0.045,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-SOFR"
        ),
        fs.RatesQuote.swap(
            maturity=date(2035, 1, 1),
            rate=0.046,
            fixed_freq=fs.Frequency.semi_annual(),
            float_freq=fs.Frequency.quarterly(),
            fixed_dc=fs.DayCount.Thirty360,
            float_dc=fs.DayCount.Act360,
            index="3M-SOFR"
        ),
    ]
    
    # Create swaption volatility quotes
    # Format: expiry, tenor (end date), strike, volatility, quote_type
    swaption_quotes = [
        # 1Y x 1Y swaptions
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2027, 1, 1),  # 1Y swap
            strike=0.040,  # 4% strike
            vol=0.20,  # 20% lognormal vol
            quote_type="ATM-100"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2027, 1, 1),
            strike=0.042,
            vol=0.22,
            quote_type="ATM"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2027, 1, 1),
            strike=0.044,
            vol=0.24,
            quote_type="ATM+100"
        ),
        
        # 1Y x 5Y swaptions
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2031, 1, 1),  # 5Y swap
            strike=0.042,
            vol=0.18,
            quote_type="ATM-100"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2031, 1, 1),
            strike=0.044,
            vol=0.20,
            quote_type="ATM"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2026, 1, 1),
            tenor=date(2031, 1, 1),
            strike=0.046,
            vol=0.22,
            quote_type="ATM+100"
        ),
        
        # 2Y x 1Y swaptions
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2028, 1, 1),
            strike=0.041,
            vol=0.19,
            quote_type="ATM-100"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2028, 1, 1),
            strike=0.043,
            vol=0.21,
            quote_type="ATM"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2028, 1, 1),
            strike=0.045,
            vol=0.23,
            quote_type="ATM+100"
        ),
        
        # 2Y x 5Y swaptions
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2032, 1, 1),
            strike=0.043,
            vol=0.17,
            quote_type="ATM-100"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2032, 1, 1),
            strike=0.045,
            vol=0.19,
            quote_type="ATM"
        ),
        fs.VolQuote.swaption_vol(
            expiry=date(2027, 1, 1),
            tenor=date(2032, 1, 1),
            strike=0.047,
            vol=0.21,
            quote_type="ATM+100"
        ),
    ]
    
    # Combine all quotes
    all_quotes = [fs.MarketQuote.rates(q) for q in disc_quotes]
    all_quotes.extend([fs.MarketQuote.vol(q) for q in swaption_quotes])
    
    # Create calibration object
    calibration = fs.SimpleCalibration(base_date, fs.Currency.USD)
    
    # Calibrate market data
    market_context, report = calibration.calibrate(all_quotes)
    
    # Print calibration report
    print("Calibration Report:")
    print(f"Success: {report.success}")
    print(f"Iterations: {report.iterations}")
    print(f"Max Residual: {report.max_residual:.6f}")
    print(f"RMSE: {report.rmse:.6f}")
    
    # Access the calibrated swaption volatility surface
    if market_context.surfaces.get("SWAPTION-VOL"):
        vol_surface = market_context.surfaces["SWAPTION-VOL"]
        
        print("\nCalibrated Swaption Volatility Surface:")
        print("=========================================")
        
        # Test some points on the surface
        # Note: In the surface, "expiry" is the first dimension and "tenor" is the second
        test_points = [
            (1.0, 1.0),  # 1Y expiry, 1Y tenor
            (1.0, 5.0),  # 1Y expiry, 5Y tenor
            (2.0, 1.0),  # 2Y expiry, 1Y tenor
            (2.0, 5.0),  # 2Y expiry, 5Y tenor
            (1.5, 3.0),  # 1.5Y expiry, 3Y tenor (interpolated)
        ]
        
        for expiry_years, tenor_years in test_points:
            # The surface uses tenor as the "strike" dimension
            vol = vol_surface.value(expiry_years, tenor_years)
            print(f"{expiry_years}Y x {tenor_years}Y: {vol:.4f} ({vol*100:.2f}%)")
    else:
        print("\nWarning: Swaption volatility surface not found in calibrated market context")
    
    # Now we can use this surface to price swaptions
    print("\n\nExample Swaption Pricing:")
    print("=========================")
    
    # Create a payer swaption
    swaption = fs.Swaption.new_payer(
        id="1Y5Y_PAYER",
        notional=fs.Money(10_000_000, fs.Currency.USD),
        strike_rate=0.045,  # 4.5% strike
        expiry=date(2026, 1, 1),  # 1Y expiry
        swap_start=date(2026, 1, 1),
        swap_end=date(2031, 1, 1),  # 5Y tenor
        disc_id="USD-SOFR",
        forward_id="USD-SOFR",
        vol_id="SWAPTION-VOL"
    )
    
    # Price the swaption
    try:
        price = swaption.value(market_context, base_date)
        print(f"Swaption price: {price}")
    except Exception as e:
        print(f"Error pricing swaption: {e}")

if __name__ == "__main__":
    main()
