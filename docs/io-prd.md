# Finstack IO — Product Requirements Document

Extracted from `finstack-io` v0.4.x. This document captures the full design intent
of the persistence layer for reimplementation in a custom database layer.

## Table of Contents

- [Purpose](#purpose)
- [Architecture Overview](#architecture-overview)
- [Trait Architecture](#trait-architecture)
  - [Store](#store)
  - [BulkStore](#bulkstore)
  - [LookbackStore](#lookbackstore)
  - [TimeSeriesStore](#timeseriesstore)
- [Domain Entities](#domain-entities)
- [Query Patterns](#query-patterns)
- [Backend Requirements](#backend-requirements)
  - [SQLite](#sqlite)
  - [Postgres](#postgres)
  - [Turso](#turso)
- [Configuration](#configuration)
- [Error Taxonomy](#error-taxonomy)
- [Python API Surface](#python-api-surface)
- [Migration System](#migration-system)
- [Batch Processing](#batch-processing)

---

## Purpose

A persistence layer for finstack domain objects providing:

- **CRUD** for instruments, market contexts, portfolios, scenarios, statement models, and metric registries
- **Time-indexed lookback** queries for market contexts and portfolios
- **Time-series** storage for quotes, metrics, results, PnL, and risk data
- **Bulk transactional** writes for batch ingestion
- **Backend-agnostic** API over SQLite, Postgres, and Turso

The store treats `put_*` operations as **idempotent upserts** (ON CONFLICT ... UPDATE).

---

## Architecture Overview

```
+--------------------------------------------------------------------+
|                           Application                              |
+--------------------------------------------------------------------+
|  Store trait  |  BulkStore  |  LookbackStore  |  TimeSeriesStore   |
+--------------------------------------------------------------------+
|               SQL statement builders (sea-query)                   |
+-------------------+-------------------+----------------------------+
|   SqliteStore     |  PostgresStore    |       TursoStore           |
| (default, embed)  | (scale-out, pool) |  (embedded, async)        |
+-------------------+-------------------+----------------------------+
```

---

## Trait Architecture

### Store

Core CRUD interface. All methods are async and `Send + Sync`.
All built-in backends implement `Clone` cheaply via internal `Arc`.

**Market Contexts** (keyed by `market_id` + `as_of` date):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_market_context` | `(market_id, as_of, context, meta?) -> ()` | Upsert snapshot |
| `get_market_context` | `(market_id, as_of) -> Option<MarketContext>` | Exact lookup, None if missing |
| `load_market_context` | `(market_id, as_of) -> MarketContext` | Like get, but returns NotFound error |

**Instruments** (keyed by `instrument_id`):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_instrument` | `(id, instrument, meta?) -> ()` | Upsert |
| `get_instrument` | `(id) -> Option<InstrumentJson>` | Exact lookup |
| `get_instruments_batch` | `(ids[]) -> HashMap<id, InstrumentJson>` | Batch fetch, missing IDs silently omitted |
| `list_instruments` | `() -> Vec<String>` | All stored IDs, ordered ascending |

**Portfolios** (keyed by `portfolio_id` + `as_of` date):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_portfolio_spec` | `(id, as_of, spec, meta?) -> ()` | Upsert snapshot |
| `get_portfolio_spec` | `(id, as_of) -> Option<PortfolioSpec>` | Exact lookup |
| `load_portfolio_spec` | `(id, as_of) -> PortfolioSpec` | NotFound error variant |
| `load_portfolio` | `(id, as_of) -> Portfolio` | Hydrates instruments from store |
| `load_portfolio_with_market` | `(portfolio_id, market_id, as_of) -> (Portfolio, MarketContext)` | Convenience combo |

**Portfolio hydration logic** (`load_portfolio`):
1. Load portfolio spec
2. Collect unique `instrument_id` values from positions where `instrument_spec` is `None`
3. Batch-fetch all missing instruments via `get_instruments_batch`
4. Assign fetched instrument specs to positions
5. Return `Portfolio::from_spec(spec)`

Note: Hydration is NOT transactionally isolated (multiple reads without a wrapping transaction).

**Scenarios** (keyed by `scenario_id`):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_scenario` | `(id, spec, meta?) -> ()` | Upsert |
| `get_scenario` | `(id) -> Option<ScenarioSpec>` | Exact lookup |
| `list_scenarios` | `() -> Vec<String>` | All IDs, ascending |

**Statement Models** (keyed by `model_id`):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_statement_model` | `(id, spec, meta?) -> ()` | Upsert |
| `get_statement_model` | `(id) -> Option<FinancialModelSpec>` | Exact lookup |
| `list_statement_models` | `() -> Vec<String>` | All IDs, ascending |

**Metric Registries** (keyed by `namespace`):

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_metric_registry` | `(namespace, registry, meta?) -> ()` | Upsert |
| `get_metric_registry` | `(namespace) -> Option<MetricRegistry>` | Exact lookup |
| `load_metric_registry` | `(namespace) -> MetricRegistry` | NotFound error variant |
| `list_metric_registries` | `() -> Vec<String>` | All namespaces, ascending |
| `delete_metric_registry` | `(namespace) -> bool` | Returns true if deleted |

**Metadata handling:** All `put_*` methods accept optional `meta: Option<&serde_json::Value>` for
provenance tracking (source, version, tags). Metadata is persisted alongside the payload but is
NOT returned by `get_*` methods.

---

### BulkStore

Extension of `Store` for batch operations within a single transaction (all-or-nothing).

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_instruments_batch` | `[(id, instrument, meta?)]` | Transactional batch upsert |
| `put_market_contexts_batch` | `[(market_id, as_of, context, meta?)]` | Transactional batch upsert |
| `put_portfolios_batch` | `[(portfolio_id, as_of, spec, meta?)]` | Transactional batch upsert |

---

### LookbackStore

Range queries over time-indexed snapshots.

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `list_market_contexts` | `(market_id, start, end) -> Vec<Snapshot>` | Range `[start, end]`, ordered by `as_of` asc |
| `latest_market_context_on_or_before` | `(market_id, as_of) -> Option<Snapshot>` | Latest where `as_of <= target` |
| `list_portfolios` | `(portfolio_id, start, end) -> Vec<Snapshot>` | Range `[start, end]`, ordered by `as_of` asc |
| `latest_portfolio_on_or_before` | `(portfolio_id, as_of) -> Option<Snapshot>` | Latest where `as_of <= target` |

**Snapshot types:**

- `MarketContextSnapshot { as_of: Date, context: MarketContext }`
- `PortfolioSnapshot { as_of: Date, spec: PortfolioSpec }`

---

### TimeSeriesStore

Storage and retrieval of time-series data points.

**Series identification:** `SeriesKey { namespace, series_id, kind }`

**Series kinds:** `Quote`, `Metric`, `Result`, `Pnl`, `Risk`
(stored as lowercase strings: `quote`, `metric`, `result`, `pnl`, `risk`)

**Data point:** `TimeSeriesPoint { ts: OffsetDateTime, value: Option<f64>, payload: Option<JSON>, meta: Option<JSON> }`

| Method | Signature | Semantics |
|--------|-----------|-----------|
| `put_series_meta` | `(key, meta?) -> ()` | Upsert series metadata |
| `get_series_meta` | `(key) -> Option<JSON>` | Load series metadata |
| `list_series` | `(namespace, kind) -> Vec<String>` | All series_ids for namespace+kind |
| `put_points_batch` | `(key, points[]) -> ()` | Transactional upsert of points |
| `get_points_range` | `(key, start, end, limit?) -> Vec<Point>` | Range `[start, end]`, ts ascending, optional limit |
| `latest_point_on_or_before` | `(key, ts) -> Option<Point>` | Latest point where `ts <= target` |

---

## Domain Entities

| Table | Rust Type | Source Crate | Serde |
|-------|-----------|-------------|-------|
| instruments | `InstrumentJson` | finstack-valuations | Serialize + manual Deserialize |
| market_contexts | `MarketContextState` | finstack-core | Serialize, Deserialize (serializable form of `MarketContext`) |
| portfolios | `PortfolioSpec` | finstack-portfolio | Serialize, Deserialize |
| scenarios | `ScenarioSpec` | finstack-scenarios | Serialize, Deserialize |
| statement_models | `FinancialModelSpec` | finstack-statements | Serialize, Deserialize |
| metric_registries | `MetricRegistry` | finstack-statements | Serialize, Deserialize |
| series_points | `TimeSeriesPoint` | finstack-io (local) | ts + optional value/payload/meta |

---

## Query Patterns

### Upsert (all entity tables)

```sql
-- SQLite
INSERT INTO "instruments" ("id", "payload", "meta", "created_at", "updated_at")
VALUES (?1, ?2, ?3, strftime('%Y-%m-%dT%H:%M:%fZ','now'), strftime('%Y-%m-%dT%H:%M:%fZ','now'))
ON CONFLICT ("id") DO UPDATE SET
    "payload" = excluded."payload",
    "meta" = excluded."meta",
    "updated_at" = excluded."updated_at";

-- Postgres
INSERT INTO "instruments" ("id", "payload", "meta", "created_at", "updated_at")
VALUES ($1, $2, $3, now(), now())
ON CONFLICT ("id") DO UPDATE SET
    "payload" = EXCLUDED."payload",
    "meta" = EXCLUDED."meta",
    "updated_at" = EXCLUDED."updated_at";
```

### Batch Fetch (instruments)

```sql
-- SQLite: chunked into MAX_BATCH_SIZE (500) groups
SELECT "id", "payload" FROM "instruments" WHERE "id" IN (?1, ?2, ..., ?N);

-- Postgres: uses ANY
SELECT "id", "payload" FROM "instruments" WHERE "id" = ANY($1);
```

### Lookback Range

```sql
SELECT "as_of", "payload" FROM "market_contexts"
WHERE "id" = ?1 AND "as_of" BETWEEN ?2 AND ?3
ORDER BY "as_of" ASC;
```

### Latest On-or-Before

```sql
SELECT "as_of", "payload" FROM "market_contexts"
WHERE "id" = ?1 AND "as_of" <= ?2
ORDER BY "as_of" DESC
LIMIT 1;
```

### Time-Series Range

```sql
SELECT "ts", "value", "payload", "meta" FROM "series_points"
WHERE "namespace" = ?1 AND "kind" = ?2 AND "series_id" = ?3
  AND "ts" BETWEEN ?4 AND ?5
ORDER BY "ts" ASC
LIMIT ?6;  -- optional
```

### Time-Series Latest

```sql
SELECT "ts", "value", "payload", "meta" FROM "series_points"
WHERE "namespace" = ?1 AND "kind" = ?2 AND "series_id" = ?3
  AND "ts" <= ?4
ORDER BY "ts" DESC
LIMIT 1;
```

### List (all entity tables)

```sql
SELECT "id" FROM "instruments" ORDER BY "id" ASC;
SELECT "namespace" FROM "metric_registries" ORDER BY "namespace" ASC;
SELECT "series_id" FROM "series_meta"
    WHERE "namespace" = ?1 AND "kind" = ?2
    ORDER BY "series_id" ASC;
```

### Delete (metric registries only)

```sql
DELETE FROM "metric_registries" WHERE "namespace" = ?1;
-- Returns rows_affected > 0 as boolean result
```

---

## Backend Requirements

### SQLite

- **Connection:** Single `tokio-rusqlite::Connection` wrapped in `Arc`
- **Async model:** `conn.call(|conn| { ... })` bridges sync rusqlite into tokio
- **Payload encoding:** `serde_json::to_vec()` -> `Vec<u8>` (BLOB)
- **Date encoding:** ISO 8601 strings for lexicographic ordering
- **Transactions:** Individual ops are implicit; bulk ops use `conn.unchecked_transaction()`
- **Migrations:** `PRAGMA user_version` tracks applied version; runs if current < LATEST_VERSION
- **Config:** `SqliteConfig::new()`, `.without_migrations()`
- **Opening:** `SqliteStore::open(path)`, `open_with_config(path, config)`, `open_in_memory()`

### Postgres

- **Connection:** `deadpool-postgres` pool (default pool size: 16)
- **Async model:** Native async via `tokio-postgres`
- **Payload encoding:** `serde_json::to_value()` -> `serde_json::Value` (JSONB)
- **Date encoding:** Native `chrono::NaiveDate` for DATE, `DateTime<Utc>` for TIMESTAMPTZ
- **Transactions:** Individual ops are separate connections; bulk ops use `conn.transaction()`
- **Bulk inserts:** Uses `UNNEST`-based batch upserts for efficiency
- **Migrations:** `pg_advisory_xact_lock` for cross-process safety
- **Config:** `PostgresConfig::new()`, `.without_migrations()`, `.with_pool_size(n)`, `.with_statement_timeout(duration)`
- **Opening:** `PostgresStore::connect(url)`, `connect_with_config(url, config)`

### Turso

- **Connection:** `libsql::Builder` + `Connection` in `Arc`
- **Async model:** Native async, no blocking runtime wrapper
- **Payload encoding:** Same as SQLite (BLOB)
- **Date encoding:** Same as SQLite (ISO 8601 strings)
- **Transactions:** `conn.transaction().await`, serialize before opening tx
- **Schema:** Uses SQLite-compatible schema (`Backend::Sqlite`)
- **Config:** `TursoConfig::new()`, `.without_migrations()`
- **Opening:** `TursoStore::open(path)`, `open_with_config(path, config)`

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `FINSTACK_IO_BACKEND` | `sqlite` | Backend: `sqlite`, `postgres`/`postgresql`, or `turso` |
| `FINSTACK_SQLITE_PATH` | *(required for sqlite)* | Path to SQLite database file |
| `FINSTACK_POSTGRES_URL` | *(required for postgres)* | Postgres connection URL |
| `FINSTACK_TURSO_PATH` | *(required for turso)* | Path to Turso database file |
| `FINSTACK_AUTO_MIGRATE` | `true` | Set to `false`/`0`/`no` to disable auto-migration |

### Config Types

```
IoBackend: Sqlite | Postgres | Turso

FinstackIoConfig {
    backend: IoBackend,
    sqlite_path: Option<PathBuf>,
    postgres_url: Option<String>,
    turso_path: Option<PathBuf>,
    auto_migrate: bool,
}

StoreHandle: Sqlite(SqliteStore) | Postgres(PostgresStore) | Turso(TursoStore)
```

`open_store_from_env()` reads config from environment and returns a `StoreHandle`.

---

## Error Taxonomy

| Category | Variants | Description |
|----------|----------|-------------|
| **Backend** | `Sqlite`, `SqliteAsync`, `Postgres`, `PostgresPool`, `PostgresConfig`, `PostgresBuild`, `PostgresCreatePool`, `Turso` | Driver errors, feature-gated, Arc-wrapped for Clone |
| **Serialization** | `SerdeJson` | JSON (de)serialization failures |
| **I/O** | `Io` | Filesystem errors |
| **Domain** | `Core`, `Portfolio`, `Statements`, `Scenarios` | Errors from finstack domain crates during hydration |
| **Application** | `NotFound { entity, id }` | Requested entity missing |
| | `UnsupportedSchema { found, expected }` | Schema version mismatch |
| | `Invariant(String)` | Internal invariant violated |
| | `InvalidSeriesKind(String)` | Unrecognized series kind string |

Convenience constructors: `Error::not_found(entity, id)`, `Error::invalid_series_kind(value)`

---

## Python API Surface

### Store Factory

```python
store = Store.open_sqlite("path/to/db.sqlite")
store = Store.connect_postgres("postgresql://user:pass@host/db")
store = Store.open_turso("path/to/db.turso")
store = Store.from_env()
store.backend  # -> "sqlite" | "postgres" | "turso"
```

### Store Methods

All Store, BulkStore, LookbackStore, and TimeSeriesStore methods are exposed
with Python-friendly argument names. Key differences from Rust:

- Dates are Python `datetime.date` objects
- Timestamps are Python `datetime.datetime` objects
- Market contexts use `PyMarketContext` wrapper
- Instruments accept/return `dict` (JSON-compatible)
- Portfolio specs accept/return `dict` or `PyPortfolioSpec`
- Time-series points are tuples: `(ts, value?, payload?, meta?)`

### Python Types

```python
class MarketContextSnapshot:
    as_of: date
    context: MarketContext

class PortfolioSpec:
    id: str
    name: str
    base_ccy: str
    as_of: date
    position_count: int
    entity_count: int
    def to_dict() -> dict
    @staticmethod
    def from_dict(data: dict) -> PortfolioSpec

class PortfolioSnapshot:
    as_of: date
    spec: PortfolioSpec
```

### Python Exceptions

```python
class IoError(Exception): ...        # Base for all IO errors
class NotFoundError(IoError): ...    # Entity not found
class SchemaVersionError(IoError): ...  # Schema mismatch
```

---

## Migration System

- **Latest version:** 4
- **Auto-migration:** Enabled by default on `open()`/`connect()`, disable via config or env var
- **Version tracking:**
  - SQLite: `PRAGMA user_version` (for single-table tracking) or `finstack_schema_migrations` table
  - Postgres: `finstack_schema_migrations` table with `pg_advisory_xact_lock` for safety
  - Turso: Same as SQLite
- **Idempotent:** All `CREATE TABLE IF NOT EXISTS`, `CREATE INDEX IF NOT EXISTS`
- **Manual trigger:** Each store exposes a public `migrate()` method

### Migration History

| Version | Changes |
|---------|---------|
| 1 | Create instruments, portfolios, market_contexts, scenarios, statement_models |
| 2 | Create metric_registries |
| 3 | Create series_meta, series_points with indexes |
| 4 | Normalize time-series timestamps (27-char microsecond -> 30-char nanosecond ISO 8601) |

---

## Batch Processing

- `MAX_BATCH_SIZE = 500`
- `get_instruments_batch` automatically chunks large requests into groups of 500
- Results are merged from all chunks
- Prevents query plan cache pollution (Postgres) and excessive query complexity
- Bulk write methods (`put_*_batch`) execute within a single transaction
