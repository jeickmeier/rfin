"""High-level Python API built on the finstack Rust core.

Import directly from :mod:`finstack` to work with currencies, configuration,
money arithmetic, business-day calendars, and market data primitives. The
compiled extension underneath provides rich docstrings and type hints so these
re-exports stay discoverable in IDEs.
"""

from . import core
from . import valuations
from . import statements
from . import scenarios
from . import portfolio

# Re-export all public symbols from the compiled extension
# The actual __all__ is determined at runtime from the compiled module
__all__: list[str]