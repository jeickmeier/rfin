#!/usr/bin/env python3
"""
FX Conversion Example for finstack

This example demonstrates how to use the FX conversion functionality
in finstack to convert monetary amounts between different currencies.

Key features demonstrated:
- Basic currency conversion using SimpleFxProvider
- Multi-currency portfolio valuation
- FX rate caching with FxMatrix
- Different FX conversion policies

Usage:
    python fx_conversion_example.py
"""

from finstack import Money, Currency, Date
from finstack.market_data import SimpleFxProvider, FxConversionPolicy, FxMatrix


def basic_fx_conversion():
    """Demonstrate basic FX conversion using SimpleFxProvider."""
    print("=" * 60)
    print("Basic FX Conversion Example")
    print("=" * 60)
    
    # Create an FX provider with some exchange rates
    fx_provider = SimpleFxProvider()
    
    # Set up exchange rates
    # USD to EUR: 1 USD = 0.85 EUR
    fx_provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
    # EUR to USD: 1 EUR = 1.18 USD  
    fx_provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
    # USD to GBP: 1 USD = 0.73 GBP
    fx_provider.set_rate(Currency("USD"), Currency("GBP"), 0.73)
    # GBP to USD: 1 GBP = 1.37 USD
    fx_provider.set_rate(Currency("GBP"), Currency("USD"), 1.37)
    # EUR to GBP: 1 EUR = 0.86 GBP
    fx_provider.set_rate(Currency("EUR"), Currency("GBP"), 0.86)
    # GBP to EUR: 1 GBP = 1.16 EUR
    fx_provider.set_rate(Currency("GBP"), Currency("EUR"), 1.16)
    
    # Create some money values
    usd_amount = Money(1000.0, Currency("USD"))
    print(f"Original amount: ${usd_amount.amount:,.2f} USD")
    print()
    
    # Convert USD to EUR
    date = Date(2025, 1, 15)
    eur_amount = usd_amount.convert(
        Currency("EUR"), 
        date, 
        fx_provider,
        FxConversionPolicy.CashflowDate
    )
    print(f"Converted to EUR: €{eur_amount.amount:,.2f} EUR")
    print(f"Exchange rate used: 1 USD = 0.85 EUR")
    print()
    
    # Convert USD to GBP
    gbp_amount = usd_amount.convert(
        Currency("GBP"),
        date,
        fx_provider,
        FxConversionPolicy.CashflowDate
    )
    print(f"Converted to GBP: £{gbp_amount.amount:,.2f} GBP")
    print(f"Exchange rate used: 1 USD = 0.73 GBP")
    print()
    
    # Convert EUR back to USD
    usd_back = eur_amount.convert(
        Currency("USD"),
        date,
        fx_provider,
        FxConversionPolicy.CashflowDate
    )
    print(f"Converting €{eur_amount.amount:,.2f} EUR back to USD: ${usd_back.amount:,.2f} USD")
    print(f"Exchange rate used: 1 EUR = 1.18 USD")
    print(f"Note: Due to bid-ask spread, we don't get exactly $1000 back")
    print()


def portfolio_fx_example():
    """Demonstrate FX conversion for a multi-currency portfolio."""
    print("=" * 60)
    print("Multi-Currency Portfolio Example")
    print("=" * 60)
    
    # Set up FX provider
    fx_provider = SimpleFxProvider()
    
    # Set up exchange rates (to USD as base)
    fx_provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
    fx_provider.set_rate(Currency("GBP"), Currency("USD"), 1.37)
    fx_provider.set_rate(Currency("JPY"), Currency("USD"), 0.0067)  # 1 JPY = 0.0067 USD
    fx_provider.set_rate(Currency("CHF"), Currency("USD"), 1.09)
    
    # Reverse rates
    fx_provider.set_rate(Currency("USD"), Currency("EUR"), 0.847)
    fx_provider.set_rate(Currency("USD"), Currency("GBP"), 0.730)
    fx_provider.set_rate(Currency("USD"), Currency("JPY"), 149.25)
    fx_provider.set_rate(Currency("USD"), Currency("CHF"), 0.917)
    
    # Create portfolio positions in different currencies
    positions = [
        Money(50000.0, Currency("USD")),
        Money(30000.0, Currency("EUR")),
        Money(25000.0, Currency("GBP")),
        Money(5000000.0, Currency("JPY")),
        Money(15000.0, Currency("CHF")),
    ]
    
    print("Portfolio positions:")
    for pos in positions:
        print(f"  {pos.currency.code}: {pos.amount:,.2f}")
    print()
    
    # Convert all to USD for total portfolio value
    date = Date(2025, 1, 15)
    total_usd = Money(0.0, Currency("USD"))
    
    print("Converting to USD:")
    for pos in positions:
        if pos.currency.code == "USD":
            converted = pos
        else:
            converted = pos.convert(
                Currency("USD"),
                date,
                fx_provider,
                FxConversionPolicy.CashflowDate
            )
        print(f"  {pos.currency.code} {pos.amount:,.2f} = USD {converted.amount:,.2f}")
        total_usd = total_usd + converted
    
    print()
    print(f"Total portfolio value: USD {total_usd.amount:,.2f}")


def fx_matrix_example():
    """Demonstrate using FxMatrix with caching."""
    print("=" * 60)
    print("FX Matrix with Caching Example")
    print("=" * 60)
    
    # Create provider with rates
    provider = SimpleFxProvider()
    provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
    provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
    
    # Create FxMatrix for caching
    fx_matrix = FxMatrix(provider)
    
    date = Date(2025, 1, 15)
    
    # Get rates multiple times (second call will use cache)
    print("Getting USD to EUR rate (first call - uncached):")
    rate1 = fx_matrix.get_rate(
        Currency("USD"), 
        Currency("EUR"),
        date,
        FxConversionPolicy.CashflowDate
    )
    print(f"  Rate: {rate1:.4f}")
    
    print("Getting USD to EUR rate (second call - cached):")
    rate2 = fx_matrix.get_rate(
        Currency("USD"),
        Currency("EUR"), 
        date,
        FxConversionPolicy.CashflowDate
    )
    print(f"  Rate: {rate2:.4f}")
    
    print()
    print("Note: FxMatrix provides caching and triangulation capabilities")
    print("for more efficient FX rate lookups in production systems.")


def different_policies_example():
    """Demonstrate different FX conversion policies."""
    print("=" * 60)
    print("FX Conversion Policies Example")
    print("=" * 60)
    
    fx_provider = SimpleFxProvider()
    fx_provider.set_rate(Currency("USD"), Currency("EUR"), 0.85)
    fx_provider.set_rate(Currency("EUR"), Currency("USD"), 1.18)
    
    usd_amount = Money(10000.0, Currency("USD"))
    date = Date(2025, 1, 15)
    
    print(f"Original amount: ${usd_amount.amount:,.2f} USD")
    print()
    
    # Different policies (in this simple example they all use the same rate)
    policies = [
        FxConversionPolicy.CashflowDate,
        FxConversionPolicy.PeriodEnd,
        FxConversionPolicy.PeriodAverage,
        FxConversionPolicy.Custom
    ]
    
    print("Converting with different policies:")
    print("(Note: SimpleFxProvider ignores policy for simplicity)")
    for policy in policies:
        eur_amount = usd_amount.convert(
            Currency("EUR"),
            date,
            fx_provider,
            policy
        )
        policy_name = str(policy).split('.')[-1]
        print(f"  {policy_name}: €{eur_amount.amount:,.2f} EUR")
    
    print()
    print("In production systems, different policies would use:")
    print("  - CashflowDate: Spot/forward rate on the cashflow date")
    print("  - PeriodEnd: Rate at the end of the reporting period")
    print("  - PeriodAverage: Average rate over the period")
    print("  - Custom: Custom strategy defined by the provider")


if __name__ == "__main__":
    # Run all examples
    basic_fx_conversion()
    print()
    portfolio_fx_example()
    print()
    fx_matrix_example()
    print()
    different_policies_example()
