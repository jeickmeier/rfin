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

print(f"RustFin version: {rfin.__version__}")

# Future usage when implementations are ready:
# from rfin.dates import Date, Calendar
# from rfin.primitives import Money, Currency
#
# # Create a date
# date = Date(2024, 1, 1)
# 
# # Create money
# usd = Currency("USD")
# amount = Money(100.0, usd)
# 
# print(f"Date: {date}")
# print(f"Amount: {amount.value} {amount.currency.code}")