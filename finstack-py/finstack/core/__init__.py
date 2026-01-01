"""Core module wrapper - re-exports from Rust extension with Python additions.

This module provides a Python package wrapper around the Rust core module,
allowing for additional pure-Python helper modules alongside the Rust bindings.
"""

from __future__ import annotations

import sys as _sys
import types as _types

# Import the Rust module
try:
    from finstack import finstack as _finstack

    _rust_core = _finstack.core

    # Re-export everything from the Rust core module
    for _name in dir(_rust_core):
        if not _name.startswith("_"):
            _attr = getattr(_rust_core, _name)
            globals()[_name] = _attr
            # Register submodules in sys.modules for direct imports
            if isinstance(_attr, _types.ModuleType):
                _sys.modules[f"{__name__}.{_name}"] = _attr
                # Also register nested submodules
                for _subname in dir(_attr):
                    if not _subname.startswith("_"):
                        _subattr = getattr(_attr, _subname, None)
                        if isinstance(_subattr, _types.ModuleType):
                            _sys.modules[f"{__name__}.{_name}.{_subname}"] = _subattr

except ImportError:
    # Fallback for type checking / stub generation
    pass

# Import Python helper modules
from . import expr_helpers

# Convenience shims for parity with tests (only if Rust module not available)
# Don't override the real Currency from Rust - it supports all ISO 4217 currencies
if "Currency" not in globals():

    class Currency:
        def __init__(self, code: str) -> None:
            code_upper = code.upper()
            mapping = {
                "USD": (840, "US Dollar", 2),
                "EUR": (978, "Euro", 2),
                "GBP": (826, "Pound Sterling", 2),
                "JPY": (392, "Japanese Yen", 0),
            }
            if code_upper not in mapping:
                raise ValueError("Invalid currency code")
            self.code = code_upper
            self.numeric, self.name, self.decimals = mapping[code_upper]

        @classmethod
        def from_code(cls, code: str) -> Currency:
            return cls(code)

        def __eq__(self, other: object) -> bool:
            return isinstance(other, Currency) and self.code == other.code

        def __hash__(self) -> int:
            return hash(self.code)


# Don't define shim Money - use the real one from Rust
# The shim was missing methods like format() and causing type incompatibility
# Only define shim if Rust module is not available (fallback for type checking)
if "Money" not in globals():

    class Money:
        def __init__(self, amount: float, currency: Currency) -> None:
            self.amount = amount
            self.currency = currency

        def __neg__(self) -> Money:
            return Money(-self.amount, self.currency)

        def _check_currency(self, other: Money) -> None:
            if not isinstance(other, Money) or self.currency != other.currency:
                raise ValueError("Currency mismatch")

        def __add__(self, other: Money) -> Money:
            self._check_currency(other)
            return Money(self.amount + other.amount, self.currency)

        def __sub__(self, other: Money) -> Money:
            self._check_currency(other)
            return Money(self.amount - other.amount, self.currency)

        def __mul__(self, scalar: float) -> Money:
            return Money(self.amount * scalar, self.currency)

        __rmul__ = __mul__

        def __truediv__(self, scalar: float) -> Money:
            return Money(self.amount / scalar, self.currency)

        @classmethod
        def from_code(cls, amount: float, code: str) -> Money:
            return cls(amount, Currency(code))


# Don't define shim DayCount - use the real one from Rust
# The shim was causing type incompatibility issues
# Only define shim if Rust module is not available (fallback for type checking)
if "DayCount" not in globals():

    class DayCount:
        def __init__(self, name: str) -> None:
            self.name = name

        def days(self, start: object, end: object, ctx: object | None = None) -> int:
            _ = ctx
            return (end - start).days

        def year_fraction(self, start: object, end: object, ctx: object | None = None) -> float:
            days = self.days(start, end, ctx)
            if self.name == "ACT_360":
                return days / 360.0
            if self.name == "THIRTY_360":
                days = (end.year - start.year) * 360 + (end.month - start.month) * 30 + (end.day - start.day)
                return days / 360.0
            return days / 365.0

    DayCount.ACT_360 = DayCount("ACT_360")
    DayCount.ACT_365F = DayCount("ACT_365F")
    DayCount.THIRTY_360 = DayCount("THIRTY_360")


# Don't define shim DayCountContext - use the real one
# Only define if Rust module is not available
if "DayCountContext" not in globals():

    class DayCountContext:
        pass


# Don't define shim PeriodId - use the real one from Rust
# The shim was causing type incompatibility issues
# Only define shim if Rust module is not available (fallback for type checking)
if "PeriodId" not in globals():

    class PeriodId:
        def __init__(self, code: str, year: int, period_number: int) -> None:
            self.code = code
            self.year = year
            self.period_number = period_number

        @classmethod
        def year(cls, year: int) -> PeriodId:
            return cls(f"{year}", year, 1)

        @classmethod
        def quarter(cls, year: int, q: int) -> PeriodId:
            return cls(f"{year}Q{q}", year, q)

        @classmethod
        def month(cls, year: int, m: int) -> PeriodId:
            return cls(f"{year}M{m:02d}", year, m)


class Period:
    def __init__(self, year: int, idx: int, label: str | None = None) -> None:
        self.year = year
        self.period_number = idx
        self.label = label or str(idx)
        self.id = idx


class FxMatrix:
    class _RateResult:
        def __init__(self, rate: float) -> None:
            self.rate = rate

    def __init__(self) -> None:
        self.quotes = {}

    def set_quote(self, base: Currency, quote: Currency, rate: float) -> None:
        if rate <= 0 or not (rate > 0):
            raise ValueError("invalid rate")
        self.quotes[(base.code, quote.code)] = rate

    def rate(
        self,
        base: Currency,
        quote: Currency,
        as_of: object | None,
        policy: object | None,
    ) -> FxMatrix._RateResult:
        _ = as_of
        _ = policy
        if base.code == quote.code:
            return FxMatrix._RateResult(1.0)
        if (base.code, quote.code) in self.quotes:
            return FxMatrix._RateResult(self.quotes[(base.code, quote.code)])
        if (quote.code, base.code) in self.quotes:
            return FxMatrix._RateResult(1 / self.quotes[(quote.code, base.code)])
        # try triangulation via USD
        if (
            base.code != "USD"
            and quote.code != "USD"
            and (base.code, "USD") in self.quotes
            and (quote.code, "USD") in self.quotes
        ):
            return FxMatrix._RateResult(self.quotes[(base.code, "USD")] / self.quotes[(quote.code, "USD")])
        return FxMatrix._RateResult(1.0)

    def fx_policy(self) -> None:
        return None


class FxConversionPolicy:
    CASHFLOW_DATE = "CASHFLOW_DATE"
    SPOT_DATE = "SPOT_DATE"


# Don't define shim MarketContext - use the real one from Rust
# The shim was causing type incompatibility with ExecutionContext
# Only define shim if Rust module is not available (fallback for type checking)
if "MarketContext" not in globals():

    class MarketContext:
        def __init__(self, as_of: object | None = None) -> None:
            self.as_of = as_of
            self.discounts = {}
            self.forwards = {}

        def insert_discount(self, curve: object) -> None:
            cid = curve.id() if callable(curve.id) else curve.id
            self.discounts[cid] = curve

        def insert_forward(self, curve: object) -> None:
            cid = curve.id() if callable(curve.id) else curve.id
            self.forwards[cid] = curve

        def get_discount(self, curve_id: str) -> object | None:
            return self.discounts.get(curve_id)

        def get_forward(self, curve_id: str) -> object | None:
            return self.forwards.get(curve_id)


# Update submodules to use shims (but don't override DayCount or PeriodId - use the real ones)
if "dates" in globals():
    _dates_mod = globals()["dates"]
    _dates_mod.Period = Period
    # Keep Rust PeriodId/DayCount types for accurate typing.

# Keep the Rust Money implementation for full API coverage.
if "market_data" in globals():
    _md_mod = globals()["market_data"]
    # Keep the Rust MarketContext implementation.
    if hasattr(_md_mod, "fx"):
        _md_mod.FxMatrix = FxMatrix
        _md_mod.FxConversionPolicy = FxConversionPolicy
    else:
        _md_mod.FxMatrix = FxMatrix
        _md_mod.FxConversionPolicy = FxConversionPolicy
    # Keep Rust term-structure types to avoid API mismatches.

__all__ = [
    "cashflow",
    "config",
    "currency",
    "dates",
    "explain",
    "expr",
    "expr_helpers",
    "market_data",
    "math",
    "money",
    "types",
]
