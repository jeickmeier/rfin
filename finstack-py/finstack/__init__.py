"""High-level Python API built on the finstack Rust core.

Import directly from :mod:`finstack` to work with currencies, configuration,
money arithmetic, and business-day calendars. The extension module underneath
provides rich docstrings and type hints so these re-exports stay discoverable
in IDEs.
"""

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

del _finstack, _name
