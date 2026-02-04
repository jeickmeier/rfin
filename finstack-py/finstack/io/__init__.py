"""Persistence layer for Finstack domain objects.

This module provides a typed repository interface for storing and retrieving
market contexts, instruments, portfolios, scenarios, statement models, and
metric registries. Three backends are supported:

- **SqliteStore**: SQLite (default, embedded, transactional)
- **PostgresStore**: PostgreSQL (requires `postgres` feature)
- **TursoStore**: Turso (SQLite-compatible with native JSON support, requires `turso` feature)
"""

from __future__ import annotations

import os as _os
import sys as _sys
import types as _types
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from finstack.io import PostgresStore, SqliteStore, TursoStore

    StoreType = SqliteStore | PostgresStore | TursoStore
else:
    StoreType = object

from finstack import finstack as _finstack

_rust_io = _finstack.io

for _name in dir(_rust_io):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_io, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

__all__ = [name for name in globals() if not name.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]


def open_store_from_env() -> StoreType:
    """Open a store based on environment variables.

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
        SqliteStore, PostgresStore, or TursoStore: The opened store instance.

    Raises:
        ValueError: If required environment variables are missing or backend is unavailable.

    Examples:
        >>> import os
        >>> os.environ["FINSTACK_IO_BACKEND"] = "turso"
        >>> os.environ["FINSTACK_TURSO_PATH"] = "data/finstack.db"
        >>> store = open_store_from_env()
    """
    backend = _os.getenv("FINSTACK_IO_BACKEND", "sqlite").strip().lower()

    if backend in {"postgres", "postgresql"}:
        url = _os.getenv("FINSTACK_POSTGRES_URL")
        if not url:
            raise ValueError("FINSTACK_POSTGRES_URL is required for postgres backend")
        store_cls = globals().get("PostgresStore")
        if store_cls is None:
            raise ValueError("PostgresStore is not available in this build")
        return store_cls.connect(url)

    if backend == "turso":
        path = _os.getenv("FINSTACK_TURSO_PATH")
        if not path:
            raise ValueError("FINSTACK_TURSO_PATH is required for turso backend")
        store_cls = globals().get("TursoStore")
        if store_cls is None:
            raise ValueError("TursoStore is not available in this build")
        return store_cls.open(path)

    # Default to sqlite
    path = _os.getenv("FINSTACK_SQLITE_PATH")
    if not path:
        raise ValueError("FINSTACK_SQLITE_PATH is required for sqlite backend")
    store_cls = globals().get("SqliteStore")
    if store_cls is None:
        raise ValueError("SqliteStore is not available in this build")
    return store_cls.open(path)
