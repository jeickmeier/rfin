# Finstack IO — SQL Schema Reference

Extracted from `finstack-io` v0.4.x. This document captures the complete database
schema for personal reuse in a custom persistence layer.

## Table of Contents

- [Migration Versions](#migration-versions)
- [Tables](#tables)
  - [instruments](#instruments)
  - [market_contexts](#market_contexts)
  - [portfolios](#portfolios)
  - [scenarios](#scenarios)
  - [statement_models](#statement_models)
  - [metric_registries](#metric_registries)
  - [series_meta](#series_meta)
  - [series_points](#series_points)
  - [finstack_schema_migrations](#finstack_schema_migrations)
- [Indexes](#indexes)
- [Data Type Mappings](#data-type-mappings)
- [Serialization Format](#serialization-format)
- [Table Naming System](#table-naming-system)

---

## Migration Versions

| Version | Tables Introduced | Description |
|---------|-------------------|-------------|
| 1 | instruments, portfolios, market_contexts, scenarios, statement_models | Core JSON entity tables |
| 2 | metric_registries | Namespaced metric definitions |
| 3 | series_meta, series_points | Time-series storage |
| 4 | *(no new tables)* | Normalizes timestamps to 30-char nanosecond ISO 8601 for SQLite/Turso |

---

## Tables

### instruments

Single-keyed entity table for instrument definitions (bonds, deposits, swaps, etc.).

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "instruments" (
    "id"         TEXT    NOT NULL PRIMARY KEY,
    "payload"    BLOB    NOT NULL,
    "meta"       TEXT    NOT NULL DEFAULT '{}',
    "created_at" TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "instruments" (
    "id"         VARCHAR    NOT NULL PRIMARY KEY,
    "payload"    JSONB      NOT NULL,
    "meta"       JSONB      NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**Payload type:** `InstrumentJson` (from `finstack-valuations`)

---

### market_contexts

Composite-keyed snapshot table for market data keyed by `(id, as_of)`.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "market_contexts" (
    "id"         TEXT NOT NULL,
    "as_of"      TEXT NOT NULL,
    "payload"    BLOB NOT NULL,
    "meta"       TEXT NOT NULL DEFAULT '{}',
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY ("id", "as_of")
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "market_contexts" (
    "id"         VARCHAR     NOT NULL,
    "as_of"      DATE        NOT NULL,
    "payload"    JSONB       NOT NULL,
    "meta"       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("id", "as_of")
);
```

**Payload type:** `MarketContextState` (from `finstack-core`, serializable form of `MarketContext`)

---

### portfolios

Composite-keyed snapshot table for portfolio specifications keyed by `(id, as_of)`.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "portfolios" (
    "id"         TEXT NOT NULL,
    "as_of"      TEXT NOT NULL,
    "payload"    BLOB NOT NULL,
    "meta"       TEXT NOT NULL DEFAULT '{}',
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY ("id", "as_of")
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "portfolios" (
    "id"         VARCHAR     NOT NULL,
    "as_of"      DATE        NOT NULL,
    "payload"    JSONB       NOT NULL,
    "meta"       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("id", "as_of")
);
```

**Payload type:** `PortfolioSpec` (from `finstack-portfolio`)

---

### scenarios

Single-keyed entity table for scenario specifications.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "scenarios" (
    "id"         TEXT NOT NULL PRIMARY KEY,
    "payload"    BLOB NOT NULL,
    "meta"       TEXT NOT NULL DEFAULT '{}',
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "scenarios" (
    "id"         VARCHAR     NOT NULL PRIMARY KEY,
    "payload"    JSONB       NOT NULL,
    "meta"       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**Payload type:** `ScenarioSpec` (from `finstack-scenarios`)

---

### statement_models

Single-keyed entity table for financial statement model specifications.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "statement_models" (
    "id"         TEXT NOT NULL PRIMARY KEY,
    "payload"    BLOB NOT NULL,
    "meta"       TEXT NOT NULL DEFAULT '{}',
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "statement_models" (
    "id"         VARCHAR     NOT NULL PRIMARY KEY,
    "payload"    JSONB       NOT NULL,
    "meta"       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**Payload type:** `FinancialModelSpec` (from `finstack-statements`)

---

### metric_registries

Single-keyed entity table for namespaced metric definitions.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "metric_registries" (
    "namespace"  TEXT NOT NULL PRIMARY KEY,
    "payload"    BLOB NOT NULL,
    "meta"       TEXT NOT NULL DEFAULT '{}',
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "metric_registries" (
    "namespace"  VARCHAR     NOT NULL PRIMARY KEY,
    "payload"    JSONB       NOT NULL,
    "meta"       JSONB       NOT NULL DEFAULT '{}'::jsonb,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

**Payload type:** `MetricRegistry` (from `finstack-statements`)

---

### series_meta

Composite-keyed metadata table for time-series definitions.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "series_meta" (
    "namespace"  TEXT NOT NULL,
    "kind"       TEXT NOT NULL,
    "series_id"  TEXT NOT NULL,
    "meta"       TEXT,
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY ("namespace", "kind", "series_id")
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "series_meta" (
    "namespace"  VARCHAR     NOT NULL,
    "kind"       VARCHAR     NOT NULL,
    "series_id"  VARCHAR     NOT NULL,
    "meta"       JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("namespace", "kind", "series_id")
);
```

**`kind` values:** `quote`, `metric`, `result`, `pnl`, `risk`

---

### series_points

Composite-keyed time-series data point table.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "series_points" (
    "namespace"  TEXT NOT NULL,
    "kind"       TEXT NOT NULL,
    "series_id"  TEXT NOT NULL,
    "ts"         TEXT NOT NULL,
    "value"      REAL,
    "payload"    TEXT,
    "meta"       TEXT,
    "created_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    "updated_at" TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
    PRIMARY KEY ("namespace", "kind", "series_id", "ts")
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "series_points" (
    "namespace"  VARCHAR     NOT NULL,
    "kind"       VARCHAR     NOT NULL,
    "series_id"  VARCHAR     NOT NULL,
    "ts"         TIMESTAMPTZ NOT NULL,
    "value"      DOUBLE PRECISION,
    "payload"    JSONB,
    "meta"       JSONB,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY ("namespace", "kind", "series_id", "ts")
);
```

---

### finstack_schema_migrations

Internal migration tracking table.

**SQLite:**

```sql
CREATE TABLE IF NOT EXISTS "finstack_schema_migrations" (
    "version"    BIGINT NOT NULL PRIMARY KEY,
    "applied_at" TEXT   NOT NULL
);
```

**Postgres:**

```sql
CREATE TABLE IF NOT EXISTS "finstack_schema_migrations" (
    "version"    BIGINT      NOT NULL PRIMARY KEY,
    "applied_at" TIMESTAMPTZ NOT NULL
);
```

---

## Indexes

### Version 1

```sql
-- instruments: lookup by creation time
CREATE INDEX IF NOT EXISTS "idx_instruments_created_at"
    ON "instruments" ("created_at");
```

No additional indexes on `market_contexts`, `portfolios`, `scenarios`, or
`statement_models` — their composite primary keys support all common access
patterns (exact lookup, latest-on-or-before via ordered scan, range queries).

### Version 2

No indexes on `metric_registries` (single PK on `namespace` suffices).

### Version 3

```sql
-- series_points: time-range scans within a namespace
CREATE INDEX IF NOT EXISTS "idx_series_points_namespace_ts"
    ON "series_points" ("namespace", "ts");

-- series_points: global time-range queries across all series
CREATE INDEX IF NOT EXISTS "idx_series_points_ts"
    ON "series_points" ("ts");
```

---

## Data Type Mappings

| Concept | SQLite | Postgres |
|---------|--------|----------|
| String identifiers | `TEXT` | `VARCHAR` |
| JSON payload (required) | `BLOB` (serde_json bytes) | `JSONB` |
| JSON metadata (required, default `{}`) | `TEXT` (JSON string) | `JSONB` (default `'{}'::jsonb`) |
| JSON metadata (optional/nullable) | `TEXT` | `JSONB` |
| Date (`as_of`) | `TEXT` (ISO 8601 `YYYY-MM-DD`) | `DATE` |
| Timestamp (created/updated) | `TEXT` (default `strftime(...)`) | `TIMESTAMPTZ` (default `now()`) |
| Timestamp (time-series `ts`) | `TEXT` (fixed-width ISO 8601) | `TIMESTAMPTZ` |
| Numeric value | `REAL` | `DOUBLE PRECISION` |
| Integer (migration version) | `BIGINT` | `BIGINT` |

---

## Serialization Format

### Payload Encoding

- **SQLite/Turso:** `serde_json::to_vec(&obj)` -> `Vec<u8>` stored as `BLOB`
- **Postgres:** `serde_json::to_value(&obj)` -> `serde_json::Value` stored as `JSONB`

### Metadata Encoding

- Always JSON. `None` metadata is stored as `'{}'` (empty object).
- Metadata is stored for auditing but **not returned** by `get_*` methods.

### Date Keys

Format: `YYYY-MM-DD` (ISO 8601), zero-padded for lexicographic ordering.

```
format_date_key(date) -> "{year:04}-{month:02}-{day:02}"
```

### Timestamp Keys (Time-Series)

Fixed-width 30-character format for correct lexicographic ordering:

```
YYYY-MM-DDTHH:MM:SS.fffffffffZ
```

Always UTC, always 9 decimal places for nanoseconds. Examples:
- `2024-01-01T12:00:00.000000000Z`
- `2024-01-01T12:00:00.123456789Z`

Parsing accepts standard RFC 3339 for backwards compatibility.

---

## Table Naming System

All table names can be customized via prefix, suffix, or full override:

```
TableNaming::new()
    .with_prefix("ref_cln_")     // instruments -> ref_cln_instruments
    .with_suffix("_v2")          // instruments -> instruments_v2
    .with_override("instruments", "my_custom_table")  // takes precedence
```

Index names follow the pattern: `idx_{prefix}{table}{suffix}_{column(s)}`

When using custom naming, the `finstack_schema_migrations` table also uses
the naming convention, so each tenant/namespace gets its own migration state.
