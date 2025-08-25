"""Example: Build and value a fixed-rate cash-flow leg using finstack.

Run with an active virtualenv that has the finstack extension installed, e.g.:

    python -m maturin develop --manifest-path finstack-py/Cargo.toml
    python examples/python/cashflow_leg_example.py
"""

from finstack import Currency, Date, DayCount
from finstack.cashflow import FixedRateLeg
from finstack.dates import Frequency

# Parameters
notional = 1_000_000.0
ccy = Currency("USD")
rate = 0.04  # 4 %
start = Date(2025, 1, 15)
end = Date(2027, 1, 15)
frequency = Frequency.SemiAnnual

dc = DayCount.act365f()

# Construct leg
leg = FixedRateLeg(notional, ccy, rate, start, end, frequency, dc)

for cf in leg.flows():
    print(cf)

print("Number of flows:", leg.num_flows)
print("NPV (no discount):", leg.npv())
print("Accrued (2025-09-30):", leg.accrued(Date(2025, 9, 30)))
