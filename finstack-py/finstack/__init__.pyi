"""Finstack: Deterministic Financial Computation Library for Python.

Finstack is a deterministic, cross-platform financial computation engine with a
Rust core and first-class Python bindings. It emphasizes accounting-grade
correctness (Decimal numerics), currency-safety, stable wire formats, and
predictable performance for statements, valuations, scenarios, and portfolio
analysis.

Key Features
------------
- **Determinism**: Decimal by default; serial and parallel runs produce
  identical results.
- **Currency-safety**: No implicit cross-currency math; explicit FX policies
  stamped in results.
- **Stable schemas**: Strict type names for long-lived pipelines and golden tests.
- **Performance**: Vectorized and parallel execution without changing Decimal
  results.
- **Parity**: Ergonomic, parity-checked APIs for Python and WASM.

Major Domains
-------------
- **core**: Primitives (types, money/fx, time, expression engine, market data)
- **valuations**: Instrument cashflows, pricing, risk (DV01/CS01/Greeks),
  period aggregation
- **statements**: Declarative financial statement modeling as directed graphs,
  period-by-period evaluation
- **scenarios**: Deterministic scenario capability for stress testing and
  what-if analysis
- **portfolio**: Entity-based position tracking, multi-instrument valuation,
  cross-currency aggregation

Examples
--------
Basic usage with currencies and money:

    >>> from finstack import Currency, Money
    >>> usd = Currency("USD")
    >>> amount = Money(100.50, usd)
    >>> print(amount)
    USD 100.50

Building market data and pricing instruments:

    >>> from finstack.core.market_data.term_structures import DiscountCurve

    >>> from finstack.core.market_data.context import MarketContext
    >>> from datetime import date
    >>> curve = DiscountCurve.from_rates("USD", date(2024, 1, 1), [(0.5, 0.99), (1.0, 0.98)])
    >>> ctx = MarketContext()
    >>> ctx.insert_discount(curve)
    >>> # Use ctx for instrument pricing...

See Also
--------
- :mod:`finstack.core`: Core primitives and market data
- :mod:`finstack.valuations`: Instrument pricing and risk
- :mod:`finstack.statements`: Financial statement modeling
- :mod:`finstack.scenarios`: Scenario analysis
- :mod:`finstack.portfolio`: Portfolio aggregation
"""

from . import core
from . import valuations
from . import statements
from . import scenarios
from . import portfolio

# Re-export all public symbols from the compiled extension
# The actual __all__ is determined at runtime from the compiled module
__all__: list[str]
