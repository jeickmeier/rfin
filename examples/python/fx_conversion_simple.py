#!/usr/bin/env python3
"""
Simple FX Conversion Example

A minimal example showing how to convert money between currencies.
"""

from finstack import Money, Currency, Date
from finstack.market_data import SimpleFxProvider, FxConversionPolicy


def main():
    # Step 1: Create an FX provider and set exchange rates
    fx_provider = SimpleFxProvider()
    fx_provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)  # 1 USD = 0.85 EUR
    fx_provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)  # 1 EUR = 1.18 USD
    
    # Step 2: Create a money value
    usd_100 = Money(100.0, Currency("USD"))
    print(f"Original: {usd_100}")
    
    # Step 3: Convert to another currency
    date = Date(2025, 1, 15)  # Date for the conversion
    eur_amount = usd_100.convert(
        Currency("EUR"),
        date,
        fx_provider,
        FxConversionPolicy.CashflowDate
    )
    print(f"Converted: {eur_amount}")
    print(f"Exchange rate: 1 USD = 0.85 EUR")
    

if __name__ == "__main__":
    main()
