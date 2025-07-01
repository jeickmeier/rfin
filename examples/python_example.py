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
import rfin.primitives as primitives

print(f"RustFin version: {rfin.__version__}")
print()

# Import the primitives module to use Currency
#primitives = rfin.primitives
Currency = primitives.Currency
USD = primitives.USD
EUR = primitives.EUR
GBP = primitives.GBP
JPY = primitives.JPY

print("=== Currency Examples ===")

# Create currencies using different methods
print("1. Creating currencies:")

# Method 1: From string
usd_from_str = Currency("USD")
print(f"   USD from string: {usd_from_str}")
print(f"   Numeric code: {usd_from_str.numeric_code}")
print(f"   Minor units: {usd_from_str.minor_units}")

# Method 2: Using class methods
eur_from_method = Currency.eur()
print(f"   EUR from method: {eur_from_method}")
print(f"   Numeric code: {eur_from_method.numeric_code}")
print(f"   Minor units: {eur_from_method.minor_units}")

# Method 3: Using predefined constants
print(f"   GBP constant: {GBP}")
print(f"   Numeric code: {GBP.numeric_code}")
print(f"   Minor units: {GBP.minor_units}")

print()
print("2. Currency properties:")
currencies = [USD, EUR, GBP, JPY]
for curr in currencies:
    print(f"   {curr.code}: numeric={curr.numeric_code}, minor_units={curr.minor_units}")

print()
print("3. Currency comparison:")
usd1 = Currency("USD")
usd2 = Currency.usd()
print(f"   USD from string == USD from method: {usd1 == usd2}")
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
    ("JPY", "Japanese Yen (0 minor units)"),
    ("BHD", "Bahraini Dinar (3 minor units)"),
    ("USD", "US Dollar (2 minor units)"),
]

for code, description in special_currencies:
    try:
        curr = Currency(code)
        print(f"   {description}: {curr.minor_units} minor units")
    except ValueError:
        print(f"   {code}: Not available")

# Future usage when Money is implemented:
# print()
# print("=== Money Examples (Future) ===")
# # Create money amounts
# amount_usd = Money(100.50, USD)
# amount_eur = Money(85.75, EUR)
# 
# print(f"Amount in USD: {amount_usd}")
# print(f"Amount in EUR: {amount_eur}")

print()
print("Currency demo completed!")