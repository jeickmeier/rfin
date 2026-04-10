"""High-level Python API built on the finstack Rust core.

Import directly from :mod:`finstack` to work with currencies, configuration,
money arithmetic, business-day calendars, and market data primitives. The
compiled extension underneath provides rich docstrings and type hints so these
re-exports stay discoverable in IDEs.
"""

from pathlib import Path as _Path

from . import finstack as _finstack
from ._binding_exports import register_subpackage as _register_subpackage

_name = None
_submodule_name = None

__all__ = tuple(  # pyright: ignore[reportUnsupportedDunderAll]
    getattr(
        _finstack,
        "__all__",
        [name for name in dir(_finstack) if not name.startswith("_")],
    )
)

for _name in __all__:
    if hasattr(_finstack, _name):
        globals()[_name] = getattr(_finstack, _name)


_pkg_path = _Path(__file__).parent
for _submodule_name in (
    "core",
    "scenarios",
    "valuations",
    "statements",
    "portfolio",
    "correlation",
    "analytics",
):
    globals()[_submodule_name] = _register_subpackage(
        getattr(_finstack, _submodule_name),
        root_package=__name__,
        qualname=_submodule_name,
        pkg_dir=_pkg_path / _submodule_name,
    )

del (
    _finstack,
    _name,
    _submodule_name,
    _register_subpackage,
    _Path,
    _pkg_path,
)
