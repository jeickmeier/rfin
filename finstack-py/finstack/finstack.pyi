"""Type stubs for the compiled extension module (`finstack.finstack`).

The runtime package (`finstack/__init__.py`) imports the compiled PyO3
extension as `from . import finstack as _finstack`.

Pylance/pyright needs a `.pyi` for this module so attribute access like
`_finstack.core` is type-checkable.
"""

from __future__ import annotations

from . import core, io, portfolio, scenarios, statements, valuations
from .core.currency import Currency
from .core.market_data.term_structures import DiscountCurve
from .core.money import Money

__all__: list[str]
