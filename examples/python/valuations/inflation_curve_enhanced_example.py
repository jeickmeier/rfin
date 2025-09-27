#!/usr/bin/env python3
"""Example demonstrating enhanced inflation curve calibration with lag and seasonality."""

import finstack as fs
import numpy as np
from datetime import date
from dateutil.relativedelta import relativedelta


def create_inflation_curve_with_seasonality():
    """Create an inflation curve with lag and seasonality adjustments."""
    
    # Base configuration
    base_date = date(2025, 1, 1)
    base_cpi = 290.0
    currency = fs.Currency.USD
    
    # Create a simple discount curve for valuation
    discount_curve = fs.DiscountCurve.builder("USD-OIS") \
        .base_date(base_date) \
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80), (10.0, 0.65)]) \
        .build()
    
    # Create inflation swap quotes (par rates)
    quotes = [
        fs.InflationQuote.inflation_swap(
            maturity=base_date + relativedelta(years=1),
            rate=0.025,  # 2.5% inflation expectation
            index="US-CPI-U"
        ),
        fs.InflationQuote.inflation_swap(
            maturity=base_date + relativedelta(years=2),
            rate=0.023,  # 2.3% inflation expectation
            index="US-CPI-U"
        ),
        fs.InflationQuote.inflation_swap(
            maturity=base_date + relativedelta(years=5),
            rate=0.024,  # 2.4% inflation expectation
            index="US-CPI-U"
        ),
        fs.InflationQuote.inflation_swap(
            maturity=base_date + relativedelta(years=10),
            rate=0.025,  # 2.5% inflation expectation
            index="US-CPI-U"
        ),
    ]
    
    # Create market context
    market_context = fs.MarketContext() \
        .insert_discount(discount_curve)
    
    # Define monthly seasonality factors (higher inflation in summer)
    seasonality_factors = [
        0.98,  # January - lower inflation
        0.98,  # February
        0.99,  # March
        1.00,  # April
        1.01,  # May
        1.02,  # June - higher inflation
        1.02,  # July - higher inflation
        1.02,  # August - higher inflation
        1.01,  # September
        1.00,  # October
        0.99,  # November
        0.98,  # December - lower inflation
    ]
    
    # Create calibrator with enhanced features
    calibrator = fs.InflationCurveCalibrator(
        curve_id="US-CPI-U",
        base_date=base_date,
        currency=currency,
        base_cpi=base_cpi,
        discount_id="USD-OIS"
    )
    
    # Configure lag and seasonality
    calibrator = calibrator \
        .with_inflation_lag(fs.InflationLag.months(3)) \
        .with_seasonality_adjustments(seasonality_factors) \
        .with_inflation_interpolation(fs.InflationInterpolation.Linear) \
        .with_solve_interp(fs.InterpStyle.LogLinear)
    
    # Calibrate the curve
    inflation_curve, report = calibrator.calibrate(quotes, market_context)
    
    # Display results
    print("=" * 80)
    print("Enhanced Inflation Curve Calibration Results")
    print("=" * 80)
    
    print(f"\nCalibration successful: {report.success}")
    print(f"Number of knots: {report.knot_count}")
    
    print("\nCalibration Metadata:")
    for key, value in report.metadata.items():
        print(f"  {key}: {value}")
    
    print("\nResiduals (pricing errors):")
    for instrument, residual in report.residuals.items():
        print(f"  {instrument}: {residual:.6f}")
    
    # Test the calibrated curve
    print("\nForward CPI levels:")
    for t in [0.25, 0.5, 1.0, 2.0, 5.0, 10.0]:
        cpi = inflation_curve.cpi(t)
        print(f"  T={t:4.2f}y: CPI = {cpi:.2f}")
    
    # Calculate implied inflation rates
    print("\nImplied inflation rates:")
    periods = [(0.0, 1.0), (1.0, 2.0), (2.0, 5.0), (5.0, 10.0)]
    for t1, t2 in periods:
        rate = inflation_curve.inflation_rate(t1, t2)
        print(f"  {t1:.0f}y-{t2:.0f}y: {rate*100:.2f}%")
    
    return inflation_curve, report


def compare_with_and_without_seasonality():
    """Compare calibration results with and without seasonality."""
    
    base_date = date(2025, 1, 1)
    base_cpi = 290.0
    currency = fs.Currency.USD
    
    # Create market data
    discount_curve = fs.DiscountCurve.builder("USD-OIS") \
        .base_date(base_date) \
        .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)]) \
        .build()
    
    quotes = [
        fs.InflationQuote.inflation_swap(
            maturity=base_date + relativedelta(years=y),
            rate=0.025,
            index="US-CPI-U"
        ) for y in [1, 2, 3, 5]
    ]
    
    market_context = fs.MarketContext().insert_discount(discount_curve)
    
    # Calibrate without seasonality
    calibrator_plain = fs.InflationCurveCalibrator(
        "US-CPI-U", base_date, currency, base_cpi, "USD-OIS"
    )
    curve_plain, _ = calibrator_plain.calibrate(quotes, market_context)
    
    # Calibrate with seasonality
    seasonality = [0.98, 0.98, 0.99, 1.00, 1.01, 1.02,
                   1.02, 1.02, 1.01, 1.00, 0.99, 0.98]
    
    calibrator_seasonal = fs.InflationCurveCalibrator(
        "US-CPI-U", base_date, currency, base_cpi, "USD-OIS"
    ).with_seasonality_adjustments(seasonality)
    
    curve_seasonal, _ = calibrator_seasonal.calibrate(quotes, market_context)
    
    # Compare results
    print("\n" + "=" * 80)
    print("Comparison: With vs Without Seasonality")
    print("=" * 80)
    
    print("\nForward CPI levels:")
    print(f"{'Time':>6} | {'Plain':>10} | {'Seasonal':>10} | {'Diff (%)':>10}")
    print("-" * 45)
    
    for t in [0.5, 1.0, 2.0, 3.0, 5.0]:
        cpi_plain = curve_plain.cpi(t)
        cpi_seasonal = curve_seasonal.cpi(t)
        diff_pct = (cpi_seasonal / cpi_plain - 1) * 100
        print(f"{t:6.1f} | {cpi_plain:10.2f} | {cpi_seasonal:10.2f} | {diff_pct:10.4f}")


if __name__ == "__main__":
    # Run the enhanced calibration example
    curve, report = create_inflation_curve_with_seasonality()
    
    # Run the comparison
    compare_with_and_without_seasonality()
    
    print("\n✓ Enhanced inflation curve calibration example completed successfully!")
