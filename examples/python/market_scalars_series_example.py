#!/usr/bin/env python3
"""
Example: Using MarketScalar and ScalarTimeSeries in CurveSet.

Demonstrates how to:
- Create and insert a MarketScalar (spot price) into CurveSet
- Create a generic ScalarTimeSeries for an economic metric
- Retrieve and use them in analysis
"""

from finstack import Date, Currency
from finstack.money import Money
from finstack.market_data import (
    CurveSet,
    MarketScalar,
    ScalarTimeSeries,
    SeriesInterpolation,
)


def main() -> None:
    curves = CurveSet()

    # 1) Add a scalar (spot price)
    aapl_spot = MarketScalar.unitless(
        195.25
    )  # could also be MarketScalar.price(Money(...))
    curves["AAPL-SPOT"] = aapl_spot

    # 2) Add a scalar time series (e.g., unemployment rate)
    d0 = Date(2025, 1, 1)
    d1 = Date(2025, 2, 1)
    d2 = Date(2025, 3, 1)
    unemployment = ScalarTimeSeries(
        "US-UNEMP",
        [(d0, 3.8), (d1, 3.9), (d2, 4.0)],
        interpolation=SeriesInterpolation.STEP,
    )
    curves["US-UNEMP"] = unemployment

    # 3) Retrieve and use
    scalar = curves.market_scalar("AAPL-SPOT")
    print("AAPL spot scalar:", scalar)

    ts = curves.scalar_time_series("US-UNEMP")
    mid = Date(2025, 1, 15)
    print("Unemployment at", mid, "=", ts.value_on(mid))

    # Price example with currency
    btc_price = MarketScalar.price(Money(60000.0, Currency("USD")))
    curves["BTC-USD-SPOT"] = btc_price
    print("BTC spot:", curves.market_scalar("BTC-USD-SPOT"))


if __name__ == "__main__":
    main()
