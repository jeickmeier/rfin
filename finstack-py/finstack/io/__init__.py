"""Persistence layer for Finstack domain objects.

This module provides a typed repository interface for storing and retrieving
market contexts, instruments, portfolios, scenarios, statement models, and
metric registries.

Three backends are supported:
- **SQLite**: Embedded, transactional, zero-config (default)
- **PostgreSQL**: Production-grade relational database (requires `postgres` feature)
- **Turso**: SQLite-compatible with native JSON support (requires `turso` feature)

Examples:
    >>> from finstack.io import Store
    >>> # Create stores with specific backends
    >>> store = Store.open_sqlite("finstack.db")
    >>> store = Store.open_turso("finstack.db")
    >>> store = Store.connect_postgres("postgresql://...")
    >>> # Or auto-detect from environment
    >>> store = Store.from_env()
"""

from __future__ import annotations

import sys as _sys
import types as _types
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from finstack.io import Store

from finstack import finstack as _finstack

_rust_io = _finstack.io

for _name in dir(_rust_io):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_io, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr


def open_store_from_env() -> Store:
    """Open a store based on environment variables.

    This is a convenience wrapper for `Store.from_env()`.

    Environment Variables:
        FINSTACK_IO_BACKEND: Backend to use ("sqlite", "postgres", or "turso").
            Defaults to "sqlite".
        FINSTACK_SQLITE_PATH: Path to SQLite database file.
            Required when FINSTACK_IO_BACKEND="sqlite" (or not set).
        FINSTACK_POSTGRES_URL: Postgres connection URL.
            Required when FINSTACK_IO_BACKEND="postgres".
        FINSTACK_TURSO_PATH: Path to Turso database file.
            Required when FINSTACK_IO_BACKEND="turso".

    Returns:
        Store: The opened store instance.

    Raises:
        IoError: If the store cannot be opened.
        ValueError: If required environment variables are missing.

    Examples:
        >>> import os
        >>> os.environ["FINSTACK_IO_BACKEND"] = "turso"
        >>> os.environ["FINSTACK_TURSO_PATH"] = "data/finstack.db"
        >>> store = open_store_from_env()
    """
    store_cls = globals().get("Store")
    if store_cls is None:
        raise RuntimeError("Store class is not available")
    return store_cls.from_env()


__all__ = [name for name in globals() if not name.startswith("_") and name not in ("TYPE_CHECKING",)]  # pyright: ignore[reportUnsupportedDunderAll]
