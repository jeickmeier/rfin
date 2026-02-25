# finstack-io

Persistent storage and import/export adapters for the Finstack workspace.

This crate provides a **stable persistence boundary** for domain crates:

- Market data snapshots (`MarketContext`) for historical lookbacks
- Instrument registries (`InstrumentJson`)
- Portfolio snapshots (`PortfolioSpec`)
- Scenario definitions (`ScenarioSpec`)
- Statement models (`FinancialModelSpec`)
- Metric registries (`MetricRegistry`)
- Time-series data (quotes, metrics, results)

## Architecture

```text
┌──────────────────────────────────────────────────────────────────────┐
│                            Application                                │
├──────────────────────────────────────────────────────────────────────┤
│  Store trait    │  BulkStore    │  LookbackStore  │  TimeSeriesStore  │
├──────────────────────────────────────────────────────────────────────┤
│                    sql/statements.rs (sea-query)                      │
├──────────────────────────────────────────────────────────────────────┤
│  sql/migrations.rs  │  sql/schema/*.rs (TableDefinition trait)        │
├───────────────────────┬───────────────────────┬──────────────────────┤
│      SqliteStore      │     PostgresStore     │      TursoStore      │
│  (default, embedded)  │  (optional, scale-out)│ (optional, embedded) │
└───────────────────────┴───────────────────────┴──────────────────────┘
```

### Design Principles

1. **Separate domain types from persistence** - Domain crates own types and logic;
   `finstack-io` owns storage, schemas, and codecs.

2. **Backend-agnostic traits** - The `Store` trait abstracts over SQLite and Postgres,
   making backends swappable.

3. **Versioned snapshots** - Data is keyed by `(id, as_of)` for reproducible lookbacks.

4. **Typed payloads** - SQL tables provide indexing; payloads are stored as JSON blobs
   of stable serde types.

5. **Auto-discovered migrations** - New tables implement `TableDefinition` and are
   automatically picked up by the migration system.

## Quick Start

### SQLite (Default)

```rust
use finstack_io::{SqliteStore, Store};
use time::macros::date;

// Open (or create) a database file
let store = SqliteStore::open("finstack.db").await?;

// Store and retrieve data
store.put_instrument("DEPO-001", &instrument, None).await?;
let loaded = store.get_instrument("DEPO-001").await?;
```

### Postgres

Enable the `postgres` feature in `Cargo.toml`:

```toml
finstack-io = { version = "0.4", features = ["postgres"] }
```

```rust
use finstack_io::{PostgresStore, Store};

let store = PostgresStore::connect("postgres://user:pass@localhost/finstack").await?;
store.put_instrument("DEPO-001", &instrument, None).await?;
```

### Turso

Turso is an in-process SQL database engine compatible with SQLite, written in Rust.
It offers native JSON support, optional encryption at rest, and modern async I/O.

Enable the `turso` feature in `Cargo.toml`:

```toml
finstack-io = { version = "0.4", features = ["turso"] }
```

```rust
use finstack_io::{TursoStore, Store};

let store = TursoStore::open("finstack.db").await?;
store.put_instrument("DEPO-001", &instrument, None).await?;
```

Turso can read/write standard SQLite database files, so you can migrate between
SQLite and Turso backends seamlessly.

### Environment-Based Configuration

```rust
use finstack_io::{open_store_from_env, StoreHandle};

// Reads configuration from environment variables
let store: StoreHandle = open_store_from_env().await?;
```

Environment variables:
- `FINSTACK_IO_BACKEND`: `sqlite` (default), `postgres`, or `turso`
- `FINSTACK_SQLITE_PATH`: Path for SQLite database (required when backend is `sqlite`)
- `FINSTACK_POSTGRES_URL`: Connection URL for Postgres (required when backend is `postgres`)
- `FINSTACK_TURSO_PATH`: Path for Turso database (required when backend is `turso`)

## Database Setup

### SQLite

No setup required. The database file is created automatically:

```rust
let store = SqliteStore::open("path/to/finstack.db").await?;
// Migrations run automatically on first connect
```

SQLite configuration:
- **Busy timeout**: 5 seconds (handles concurrent access)
- **Journal mode**: WAL (write-ahead logging for better concurrency)

### Postgres

1. Create a database:

```bash
# Using Docker
docker run -d --name finstack-pg \
    -e POSTGRES_PASSWORD=secret \
    -e POSTGRES_DB=finstack \
    -p 5432:5432 \
    postgres:15

# Or using psql
createdb finstack
```

2. Connect and run migrations:

```rust
let store = PostgresStore::connect("postgres://user:secret@localhost/finstack").await?;
// Migrations run automatically on connect
```

Postgres configuration:
- **Statement timeout**: 5 seconds (per-statement limit, configurable)
- **Connection pooling**: built-in via `deadpool-postgres` (configurable pool size)

### Turso

No setup required. The database file is created automatically:

```rust
let store = TursoStore::open("path/to/finstack.db").await?;
// Migrations run automatically on first connect
```

Turso configuration:
- SQLite-compatible file format (can read/write standard `.db` files)
- Async I/O with io_uring on Linux
- Optional encryption at rest (not yet exposed in this wrapper)

### Custom Table Names

For deployments requiring custom naming conventions:

```rust
use finstack_io::sql::schema::TableNaming;
use finstack_io::sql::migrations;

// Add prefix to all tables: instruments -> ref_cln_instruments
let naming = TableNaming::new().with_prefix("ref_cln_");

// Or override specific tables
let naming = TableNaming::new()
    .with_prefix("app_")
    .with_override("instruments", "custom_instruments_table");

// Generate migrations with custom naming
let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, &naming);
```

## Schema & Migrations

### Schema Version

The current schema version is tracked in `sql/migrations.rs`:

```rust
pub const LATEST_VERSION: i64 = 3;
```

Migrations are tracked in the `finstack_schema_migrations` table.

### Tables

| Table | Primary Key | Description |
|-------|-------------|-------------|
| `instruments` | `id` | Instrument definitions (bonds, deposits, swaps, etc.) |
| `market_contexts` | `(id, as_of)` | Market data snapshots (curves, surfaces, FX) |
| `portfolios` | `(id, as_of)` | Portfolio position snapshots |
| `scenarios` | `id` | Scenario specifications |
| `statement_models` | `id` | Financial statement model specs |
| `metric_registries` | `namespace` | Metric definition registries |
| `series_meta` | `(namespace, series_id, kind)` | Time-series metadata |
| `series_points` | `(namespace, series_id, kind, ts)` | Time-series data points |

### Adding a New Table

1. **Create the table module** in `src/sql/schema/`:

```rust
// src/sql/schema/my_table.rs
use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};
use super::{created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum MyTable {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for MyTable {
    // Base name used for custom naming (prefix/suffix applied to this)
    const BASE_NAME: &'static str = "my_table";

    // Migration version when this table was introduced
    fn migration_version() -> i64 {
        4  // Next version after current LATEST_VERSION
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(MyTable::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(payload_col(backend, MyTable::Payload))
            .col(meta_col(backend, MyTable::Meta))
            .col(created_at_col(backend, MyTable::CreatedAt))
            .col(updated_at_col(backend, MyTable::UpdatedAt))
            .to_owned()
    }

    // Optional: Add indexes for this table
    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!("idx_{}my_table{}_created_at", naming.prefix(), naming.suffix());
        vec![Index::create()
            .name(&idx_name)
            .table(naming.alias(Self::BASE_NAME))
            .col(MyTable::CreatedAt)
            .to_owned()]
    }
}
```

2. **Register the module** in `src/sql/schema/mod.rs`:

```rust
mod my_table;
pub use my_table::MyTable;
```

3. **Add to migration discovery** in `tables_by_version_with_naming()`:

```rust
// In schema/mod.rs, add to the appropriate version
(4, vec![MyTable::create_table_with_naming(backend, naming)]),
```

4. **Update LATEST_VERSION** in `src/sql/migrations.rs`:

```rust
pub const LATEST_VERSION: i64 = 4;
```

5. **Add Store trait methods** (optional) - see next section.

### Adding New Statements (Queries)

SQL statements are defined in `src/sql/statements.rs` using `sea-query`.

1. **Add the query builder**:

```rust
// src/sql/statements.rs
pub fn upsert_my_table_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::MyTable::Table)
        .columns([
            schema::MyTable::Id,
            schema::MyTable::Payload,
            schema::MyTable::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::column(schema::MyTable::Id)
                .update_columns([schema::MyTable::Payload, schema::MyTable::Meta])
                .value(schema::MyTable::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn select_my_table_sql(backend: Backend) -> String {
    let query = Query::select()
        .columns([schema::MyTable::Id, schema::MyTable::Payload])
        .from(schema::MyTable::Table)
        .and_where(Expr::col(schema::MyTable::Id).eq("?"))
        .to_owned();
    build_sql(backend, query)
}
```

2. **Add Store trait methods** in `src/store.rs`:

```rust
pub trait Store {
    // ... existing methods ...

    fn put_my_entity(&self, id: &str, entity: &MyEntity, meta: Option<&serde_json::Value>) -> Result<()>;

    #[must_use]
    fn get_my_entity(&self, id: &str) -> Result<Option<MyEntity>>;
}
```

3. **Implement for each backend** (`src/sqlite/core_store.rs`, `src/postgres/core_store.rs`):

```rust
impl Store for SqliteStore {
    fn put_my_entity(&self, id: &str, entity: &MyEntity, meta: Option<&serde_json::Value>) -> Result<()> {
        self.with_conn(|conn| {
            let sql = statements::upsert_my_table_sql(Backend::Sqlite);
            let payload = serde_json::to_string(entity)?;
            let meta_json = meta.cloned().unwrap_or_else(|| serde_json::json!({}));
            conn.execute(&sql, params![id, payload, meta_json.to_string()])?;
            Ok(())
        })
    }

    fn get_my_entity(&self, id: &str) -> Result<Option<MyEntity>> {
        self.with_conn(|conn| {
            let sql = statements::select_my_table_sql(Backend::Sqlite);
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query(params![id])?;
            match rows.next()? {
                Some(row) => {
                    let payload: String = row.get(1)?;
                    Ok(Some(serde_json::from_str(&payload)?))
                }
                None => Ok(None),
            }
        })
    }
}
```

## API Reference

### Core Traits

| Trait | Purpose |
|-------|---------|
| `Store` | Basic CRUD for all entity types |
| `BulkStore` | Batch operations (transactional) |
| `LookbackStore` | Range queries by `as_of` date |
| `TimeSeriesStore` | Time-series data operations |

### Store Methods

```rust
// Market contexts (keyed by id + as_of)
store.put_market_context(market_id, as_of, &context, meta).await?;
store.get_market_context(market_id, as_of).await?;
store.load_market_context(market_id, as_of).await?;  // Returns error if not found

// Instruments
store.put_instrument(id, &instrument, meta).await?;
store.get_instrument(id).await?;
store.list_instruments().await?;  // Returns all instrument IDs

// Portfolios (keyed by id + as_of)
store.put_portfolio_spec(portfolio_id, as_of, &spec, meta).await?;
store.get_portfolio_spec(portfolio_id, as_of).await?;
store.load_portfolio(portfolio_id, as_of).await?;  // Hydrates with instruments

// Convenience: load portfolio + market together
let (portfolio, market) = store.load_portfolio_with_market(
    portfolio_id, market_id, as_of
).await?;

// Scenarios
store.put_scenario(id, &spec, meta).await?;
store.get_scenario(id).await?;

// Statement models
store.put_statement_model(id, &spec, meta).await?;
store.get_statement_model(id).await?;

// Metric registries
store.put_metric_registry(namespace, &registry, meta).await?;
store.get_metric_registry(namespace).await?;
store.list_metric_registries().await?;
store.delete_metric_registry(namespace).await?;
```

### Bulk Operations

```rust
// Batch insert (transactional)
store.put_instruments_batch(&[(id1, instr1), (id2, instr2)]).await?;
store.put_market_contexts_batch(&[(id, as_of, ctx, meta), ...]).await?;
```

### Lookback Queries

```rust
// Get latest snapshot on or before a date
store.latest_market_context_on_or_before(market_id, as_of).await?;
store.latest_portfolio_on_or_before(portfolio_id, as_of).await?;

// List all snapshots in a date range
store.list_market_contexts(market_id, start_date, end_date).await?;
store.list_portfolios(portfolio_id, start_date, end_date).await?;
```

### Time-Series

```rust
let key = SeriesKey::new("namespace", "series_id", SeriesKind::Quote);

// Store metadata
store.put_series_meta(&key, Some(&serde_json::json!({"source": "bloomberg"}))).await?;

// Store points
store.put_points_batch(&key, &[
    TimeSeriesPoint { ts, value: Some(100.0), payload: None, meta: None },
]).await?;

// Query range
let points = store.get_points_range(&key, start_ts, end_ts, Some(limit)).await?;

// Get latest point
let latest = store.latest_point_on_or_before(&key, as_of_ts).await?;
```

## Testing

```bash
# Run all tests (SQLite only)
cargo test -p finstack-io

# Run with Postgres (requires running Postgres instance)
POSTGRES_URL="postgres://user:pass@localhost/finstack_test" \
    cargo test -p finstack-io --features postgres
```

## Error Handling

The crate uses strict error handling:

- `#![deny(clippy::unwrap_used)]` - No panics from unwrap
- `#![deny(clippy::expect_used)]` - No panics from expect
- `#![deny(clippy::panic)]` - No explicit panics

All operations return `Result<T, Error>` with typed error variants:

```rust
pub enum Error {
    Sqlite(rusqlite::Error),
    Postgres(postgres::Error),
    Turso(turso::Error),
    SerdeJson(serde_json::Error),
    NotFound { entity, id },
    UnsupportedSchema { found, expected },
    InvalidSeriesKind(String),
    Invariant(String),
    // ...
}
```

## Features

| Feature | Default | Description |
|---------|---------|-------------|
| `sqlite` | Yes | SQLite backend via `rusqlite` |
| `postgres` | No | Postgres backend via `postgres` crate |
| `turso` | No | Turso backend via `turso` crate (SQLite-compatible) |
