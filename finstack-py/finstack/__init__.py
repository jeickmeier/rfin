"""Public re-exports from the compiled extension module.

This avoids wildcard imports (which break static analysis) while still exposing
the same surface as the native extension by reflecting its `__all__` or, if
absent, all non-private attributes.
"""

from . import finstack as _finstack

__all__ = getattr(
    _finstack,
    "__all__",
    [name for name in dir(_finstack) if not name.startswith("_")],
)

for _name in __all__:
    globals()[_name] = getattr(_finstack, _name)

del _finstack, _name
