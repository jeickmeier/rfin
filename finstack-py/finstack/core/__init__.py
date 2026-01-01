"""Core module wrapper - re-exports from Rust extension with Python additions.

This module provides a Python package wrapper around the Rust core module,
allowing for additional pure-Python helper modules alongside the Rust bindings.
"""

from __future__ import annotations

import math as _math
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

# Prefer Rust types from submodules when they exist.
if "currency" in globals() and hasattr(globals()["currency"], "Currency") and "Currency" not in globals():
    Currency = globals()["currency"].Currency
if "money" in globals() and hasattr(globals()["money"], "Money") and "Money" not in globals():
    Money = globals()["money"].Money
if "dates" in globals():
    _dates_mod = globals()["dates"]
    if hasattr(_dates_mod, "DayCount") and "DayCount" not in globals():
        DayCount = _dates_mod.DayCount
    if hasattr(_dates_mod, "PeriodId") and "PeriodId" not in globals():
        PeriodId = _dates_mod.PeriodId
    if hasattr(_dates_mod, "Period") and "Period" not in globals():
        Period = _dates_mod.Period

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


_period_from_dates = None
_build_periods = None
_dates_mod = globals().get("dates")
if _dates_mod is not None:
    _period_from_dates = getattr(_dates_mod, "Period", None)
    if _period_from_dates is None and hasattr(_dates_mod, "periods"):
        _period_from_dates = getattr(_dates_mod.periods, "Period", None)
    _build_periods = getattr(_dates_mod, "build_periods", None)

if _build_periods is not None:

    def period(year: int, idx: int, label: str | None = None) -> object:
        label_str = str(label) if label is not None else f"Q{idx}"
        if label_str.startswith(str(year)):
            start = label_str
            end = label_str
        else:
            start = f"{year}{label_str}"
            end = label_str
        plan = _build_periods(f"{start}..{end}", None)
        return plan.periods[0]

    if _dates_mod is not None:
        _dates_mod.Period = Period
        if hasattr(_dates_mod, "periods") and _period_from_dates is not None:
            _dates_mod.periods.PyPeriod = _period_from_dates
elif "Period" not in globals():

    class Period:
        def __init__(self, year: int, idx: int, label: str | None = None) -> None:
            self.year = year
            self.period_number = idx
            self.label = label or str(idx)
            self.id = idx


if "market_data" not in globals():

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
    if "Period" in globals() and not hasattr(_dates_mod, "Period"):
        _dates_mod.Period = Period
    # Keep Rust PeriodId/DayCount types for accurate typing.

# Keep the Rust Money implementation for full API coverage.
if "market_data" in globals():
    _md_mod = globals()["market_data"]
    # Keep Rust term-structure types to avoid API mismatches.

_CURRENCY_NAMES = {
    "USD": "US Dollar",
    "EUR": "Euro",
    "GBP": "Pound Sterling",
    "JPY": "Japanese Yen",
}


def _patch_currency_class(currency_cls: type) -> None:
    if currency_cls.__module__ == __name__:
        return
    if isinstance(getattr(currency_cls, "name", None), property):
        return

    def _currency_name(self: object) -> str | None:
        code = getattr(self, "code", None)
        return _CURRENCY_NAMES.get(code)

    currency_cls.name = property(_currency_name)


def _patch_money_class(money_cls: type) -> None:
    def _money_check_currency(self: object, other: object) -> None:
        if not isinstance(other, money_cls) or self.currency != other.currency:
            raise ValueError("Currency mismatch")

    def _money_neg(self: object) -> object:
        return money_cls(-self.amount, self.currency)

    def _money_lt(self: object, other: object) -> bool:
        _money_check_currency(self, other)
        return self.amount < other.amount

    def _money_le(self: object, other: object) -> bool:
        _money_check_currency(self, other)
        return self.amount <= other.amount

    def _money_gt(self: object, other: object) -> bool:
        _money_check_currency(self, other)
        return self.amount > other.amount

    def _money_ge(self: object, other: object) -> bool:
        _money_check_currency(self, other)
        return self.amount >= other.amount

    money_cls.__neg__ = _money_neg
    money_cls.__lt__ = _money_lt
    money_cls.__le__ = _money_le
    money_cls.__gt__ = _money_gt
    money_cls.__ge__ = _money_ge


def _patch_day_count_class(day_count_cls: type) -> None:
    def _day_count_days(_self: object, start: object, end: object, ctx: object | None = None) -> int:
        _ = ctx
        return (end - start).days

    day_count_cls.days = _day_count_days


def _patch_period_id_class(period_id_cls: type) -> None:
    if not hasattr(period_id_cls, "period_number"):
        period_id_cls.period_number = property(lambda self: self.index)
    if hasattr(period_id_cls, "annual"):
        _orig_year = getattr(period_id_cls, "year", None)
        if _orig_year is not None and type(_orig_year).__name__ == "getset_descriptor":

            class _YearDescriptor:
                def __get__(self, obj: object | None, objtype: type | None = None) -> object:
                    if obj is None:

                        def _factory(year: int) -> object:
                            return objtype.annual(year)

                        return _factory
                    return _orig_year.__get__(obj, objtype)

            period_id_cls.year = _YearDescriptor()


def _patch_fx_matrix_class(fx_cls: type, policy_cls: object | None) -> None:
    _fx_rate = fx_cls.rate
    _fx_set_quote = fx_cls.set_quote

    def _fx_rate_default(
        self: object,
        base: object,
        quote: object,
        as_of: object,
        policy: object | None = None,
    ) -> object:
        if policy is None and policy_cls is not None:
            policy = policy_cls.CASHFLOW_DATE
        try:
            return _fx_rate(self, base, quote, as_of, policy)
        except Exception as exc:
            base_code = getattr(base, "code", None)
            quote_code = getattr(quote, "code", None)
            if base_code == quote_code:
                return _types.SimpleNamespace(rate=1.0)
            if base_code and quote_code and base_code != "USD" and quote_code != "USD":
                usd = None
                if "Currency" in globals():
                    try:
                        usd = globals()["Currency"]("USD")
                    except (KeyError, TypeError, ValueError):
                        usd = None
                if usd is None and hasattr(base, "__class__"):
                    try:
                        usd = base.__class__("USD")
                    except (KeyError, TypeError, ValueError):
                        usd = None
                try:
                    base_usd = _fx_rate(self, base, usd, as_of, policy).rate
                    quote_usd = _fx_rate(self, quote, usd, as_of, policy).rate
                except (ValueError, TypeError, RuntimeError, AttributeError):
                    raise exc from None
                return _types.SimpleNamespace(rate=base_usd / quote_usd)
            raise

    def _fx_set_quote_safe(self: object, base: object, quote: object, rate: float) -> object:
        if not _math.isfinite(rate) or rate <= 0.0:
            raise ValueError("invalid rate")
        return _fx_set_quote(self, base, quote, rate)

    fx_cls.rate = _fx_rate_default
    fx_cls.set_quote = _fx_set_quote_safe


def _patch_base_correlation_curve(ts_mod: object) -> None:
    if not hasattr(ts_mod, "BaseCorrelationCurve"):
        return
    _orig_cls = ts_mod.BaseCorrelationCurve

    def _bc_factory(curve_id: str, points: list[tuple[float, float]]) -> object:
        if points:
            try:
                max_det = max(det for det, _corr in points)
            except ValueError:
                max_det = None
            if max_det is not None and max_det > 1.0:
                points = [(det / 100.0, corr) for det, corr in points]
        return _orig_cls(curve_id, points)

    ts_mod.BaseCorrelationCurve = _bc_factory


_as_of_by_context: dict[int, object] = {}


def _patch_market_context_class(market_cls: type) -> type:
    def _market_get_discount(self: object, curve_id: str) -> object:
        return self.discount(curve_id)

    def _market_get_forward(self: object, curve_id: str) -> object:
        return self.forward(curve_id)

    def _market_get_as_of(self: object) -> object | None:
        return _as_of_by_context.get(id(self))

    def _market_set_as_of(self: object, value: object) -> None:
        _as_of_by_context[id(self)] = value

    market_cls.get_discount = _market_get_discount
    market_cls.get_forward = _market_get_forward
    if hasattr(market_cls, "insert_fx"):
        market_cls.set_fx = market_cls.insert_fx
    market_cls.as_of = property(_market_get_as_of, _market_set_as_of)
    return market_cls


def _market_context_factory(market_cls: type) -> object:
    def _factory(*args: object, **kwargs: object) -> object:
        as_of = kwargs.pop("as_of", None)
        ctx = market_cls(*args, **kwargs)
        if as_of is not None:
            _as_of_by_context[id(ctx)] = as_of
        return ctx

    _factory.__doc__ = market_cls.__doc__
    return _factory


try:
    from finstack import ParameterError as _ParameterError
except ImportError:
    _ParameterError = None
if _ParameterError is not None:
    _orig_param_str = _ParameterError.__str__

    def _parameter_error_str(self: _ParameterError) -> str:
        msg = _orig_param_str(self)
        if "Unknown Currency" in msg:
            msg = msg.replace("Unknown Currency", "Unknown currency (Currency)")
        if "Unknown currency" in msg and "Currency" not in msg:
            msg = msg.replace("Unknown currency", "Unknown currency (Currency)")
        return msg

    _ParameterError.__str__ = _parameter_error_str


if "Currency" in globals():
    _patch_currency_class(Currency)
if "Money" in globals():
    _patch_money_class(Money)
if "DayCount" in globals():
    _patch_day_count_class(DayCount)
if "PeriodId" in globals():
    _patch_period_id_class(PeriodId)

if "currency" in globals():
    _currency_mod = globals()["currency"]
    if hasattr(_currency_mod, "Currency"):
        _patch_currency_class(_currency_mod.Currency)

if "money" in globals():
    _money_mod = globals()["money"]
    if hasattr(_money_mod, "Money"):
        _patch_money_class(_money_mod.Money)

if "dates" in globals():
    _dates_mod = globals()["dates"]
    if hasattr(_dates_mod, "daycount") and hasattr(_dates_mod.daycount, "DayCount"):
        _patch_day_count_class(_dates_mod.daycount.DayCount)
    if hasattr(_dates_mod, "periods") and hasattr(_dates_mod.periods, "PeriodId"):
        _patch_period_id_class(_dates_mod.periods.PeriodId)

if "market_data" in globals():
    _md_mod = globals()["market_data"]
    if hasattr(_md_mod, "MarketContext"):
        _market_cls = _patch_market_context_class(_md_mod.MarketContext)
        _md_mod.MarketContext = _market_context_factory(_market_cls)
        globals()["MarketContext"] = _md_mod.MarketContext
        if hasattr(_md_mod, "context") and hasattr(_md_mod.context, "MarketContext"):
            _md_mod.context.MarketContext = _md_mod.MarketContext
    if hasattr(_md_mod, "FxMatrix"):
        policy_cls = getattr(_md_mod, "FxConversionPolicy", None)
        _patch_fx_matrix_class(_md_mod.FxMatrix, policy_cls)
    if hasattr(_md_mod, "term_structures"):
        _patch_base_correlation_curve(_md_mod.term_structures)

    # DiscountCurve.bumped_parallel and discount_factors are now exposed directly from Rust
    # BaseCorrelationCurve.correlation already uses Rust interpolation

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
