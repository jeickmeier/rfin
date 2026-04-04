"""Type stubs for the compiled extension module (`finstack.finstack`).

The runtime package (`finstack/__init__.py`) imports the compiled PyO3
extension as `from . import finstack as _finstack`.

Pylance/pyright needs a `.pyi` for this module so attribute access like
`_finstack.core` is type-checkable.
"""

from __future__ import annotations

from typing import Any

from . import analytics, correlation, core, portfolio, scenarios, statements, valuations
from .core.currency import Currency
from .core.market_data.term_structures import DiscountCurve
from .core.money import Money

analytics: Any
correlation: Any
core: Any
portfolio: Any
scenarios: Any
statements: Any
valuations: Any

__all__: list[str]
