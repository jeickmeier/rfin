#!/usr/bin/env python3
"""
Example of using RustFin Python bindings with uv.

To run this example:
1. Install uv if not already installed:
   curl -LsSf https://astral.sh/uv/install.sh | sh

2. Run the setup script:
   ./scripts/setup-python.sh

3. Activate the virtual environment:
   source .venv/bin/activate

4. Run this script:
   python examples/python_example.py

Or simply run with uv directly:
   uv run python examples/python_example.py
"""

import rfin
from rfin import Currency, Money

print(f"RustFin version: {rfin.__version__}")
print()


print("=== Currency Examples ===")

# Create common currencies once
USD = Currency("USD")
EUR = Currency("EUR")
GBP = Currency("GBP")
JPY = Currency("JPY")

# Display basic properties
print(f"   USD: code={USD.code}, numeric={USD.numeric_code}, decimals={USD.decimals}")
print(f"   EUR: code={EUR.code}, numeric={EUR.numeric_code}, decimals={EUR.decimals}")
print(f"   GBP: code={GBP.code}, numeric={GBP.numeric_code}, decimals={GBP.decimals}")

print()
print("2. Currency properties:")
currencies = [USD, EUR, GBP, JPY]
for curr in currencies:
    print(f"   {curr.code}: numeric={curr.numeric_code}, decimals={curr.decimals}")

print()
print("3. Currency comparison:")
usd1 = Currency("USD")
usd2 = USD
print(f"   USD variable matches new instance: {usd1 == usd2}")
print(f"   USD == EUR: {USD == EUR}")

print()
print("4. Case-insensitive parsing:")
try:
    eur_lower = Currency("eur")
    eur_upper = Currency("EUR")
    print(f"   'eur' == 'EUR': {eur_lower == eur_upper}")
except Exception as e:
    print(f"   Error: {e}")

print()
print("5. Error handling:")
try:
    invalid = Currency("INVALID")
except ValueError as e:
    print(f"   Invalid currency error: {e}")

print()
print("6. Different minor units:")
special_currencies = [
    ("JPY", "Japanese Yen (0 decimals)"),
    ("BHD", "Bahraini Dinar (3 decimals)"),
    ("USD", "US Dollar (2 decimals)"),
]

for code, description in special_currencies:
    try:
        curr = Currency(code)
        print(f"   {description}: {curr.decimals} decimals")
    except ValueError:
        print(f"   {code}: Not available")

print()
print("=== Money Examples ===")

# Create money amounts
print("1. Creating Money instances:")
amount_usd = Money(100.50, USD)
amount_eur = Money(85.75, EUR)
amount_gbp = Money(75.25, GBP)

print(f"   Amount in USD: {amount_usd}")
print(f"   Amount in EUR: {amount_eur}")
print(f"   Amount in GBP: {amount_gbp}")

print()
print("2. Using convenience constructors:")
usd_money = Money(250.0, USD)
eur_money = Money(200.0, EUR)

print(f"   USD money: {usd_money}")
print(f"   EUR money: {eur_money}")

print()
print("3. Money properties:")
print(
    f"   {amount_usd} -> amount: {amount_usd.amount}, currency: {amount_usd.currency}"
)
print(
    f"   {amount_eur} -> amount: {amount_eur.amount}, currency: {amount_eur.currency}"
)

print()
print("4. Money arithmetic (same currency):")
usd_100 = Money(100.0, USD)
usd_50 = Money(50.0, USD)

# Addition
total = usd_100 + usd_50
print(f"   {usd_100} + {usd_50} = {total}")

# Subtraction
difference = usd_100 - usd_50
print(f"   {usd_100} - {usd_50} = {difference}")

# Multiplication
doubled = usd_100 * 2
print(f"   {usd_100} * 2 = {doubled}")

# Division
half = usd_100 / 2
print(f"   {usd_100} / 2 = {half}")

# Right multiplication
also_doubled = 2 * usd_100
print(f"   2 * {usd_100} = {also_doubled}")

print()
print("5. Money comparison:")
usd_100_copy = Money(100.0, USD)
print(f"   Money(100, USD) == Money.usd(100): {usd_100 == usd_100_copy}")
print(f"   Money(100, USD) == Money(50, USD): {usd_100 == usd_50}")

print()
print("6. Error handling (different currencies):")
try:
    invalid_sum = amount_usd + amount_eur  # Should fail
    print(f"   Unexpected success: {invalid_sum}")
except ValueError as e:
    print(f"   ✓ Caught expected error: {e}")

try:
    invalid_diff = amount_usd - amount_eur  # Should fail
    print(f"   Unexpected success: {invalid_diff}")
except ValueError as e:
    print(f"   ✓ Caught expected error: {e}")

print()
print("7. Converting to parts:")
amount, currency = amount_usd.into_parts()
print(f"   {amount_usd} -> amount: {amount}, currency: {currency}")

print()
print("8. Complex calculations:")
# Portfolio calculation
portfolio_usd = [
    Money(1000.0, USD),  # Cash
    Money(2500.0, USD),  # Stocks
    Money(500.0, USD),  # Bonds
]

total_portfolio = Money(0.0, USD)
for holding in portfolio_usd:
    total_portfolio = total_portfolio + holding

print(f"   Portfolio holdings: {[str(h) for h in portfolio_usd]}")
print(f"   Total portfolio value: {total_portfolio}")

# Calculate percentage allocations
cash_pct = (portfolio_usd[0].amount / total_portfolio.amount) * 100
stocks_pct = (portfolio_usd[1].amount / total_portfolio.amount) * 100
bonds_pct = (portfolio_usd[2].amount / total_portfolio.amount) * 100

print(
    f"   Allocations: Cash {cash_pct:.1f}%, Stocks {stocks_pct:.1f}%, Bonds {bonds_pct:.1f}%"
)

print()
print("Currency and Money demo completed!")
