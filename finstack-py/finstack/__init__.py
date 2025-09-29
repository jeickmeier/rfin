"""High-level Python API built on the finstack Rust core.

Import directly from :mod:`finstack` to work with currencies, configuration,
money arithmetic, business-day calendars, and market data primitives. The
compiled extension underneath provides rich docstrings and type hints so these
re-exports stay discoverable in IDEs.
"""

from collections.abc import MutableMapping as _MutableMapping
import sys as _sys
import types as _types
from typing import Any as _Any

from . import finstack as _finstack

__all__ = tuple(
    getattr(
        _finstack,
        "__all__",
        [name for name in dir(_finstack) if not name.startswith("_")],
    )
)

for _name in __all__:
    globals()[_name] = getattr(_finstack, _name)

# Expose compiled core tree (no top-level aliases)
_core = _finstack.core
globals()["core"] = _core
_sys.modules[f"{__name__}.core"] = _core

def _walk_and_register(
    _parent_mod: _Any,
    _qualname: str,
    _modules: _MutableMapping[str, _Any] = _sys.modules,
    _module_type: type = _types.ModuleType,
) -> None:
    """Recursively register submodules under sys.modules for import to work.

    This avoids manual updates when new PyO3 submodules are added under core.
    """
    _seen: set[int] = set()

    def _recurse(_mod: object, _qname: str) -> None:
        if id(_mod) in _seen:
            return
        _seen.add(id(_mod))

        for _attr_name in dir(_mod):
            if _attr_name.startswith("_"):
                continue
            try:
                _attr = getattr(_mod, _attr_name)
            except AttributeError:
                continue
            if isinstance(_attr, _module_type):
                _fqname = f"{__name__}.{_qname}.{_attr_name}"
                _modules[_fqname] = _attr
                _recurse(_attr, f"{_qname}.{_attr_name}")

    _recurse(_parent_mod, _qualname)

_walk_and_register(_core, "core")

_valuations = _finstack.valuations
globals()["valuations"] = _valuations
_sys.modules[f"{__name__}.valuations"] = _valuations
_walk_and_register(_valuations, "valuations")

del _finstack, _sys, _name, _core, _valuations, _types, _walk_and_register, _Any, _MutableMapping
