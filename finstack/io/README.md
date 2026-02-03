# finstack-io

Persistent storage and import/export adapters for the Finstack workspace.

This crate is intended to be the “persistence boundary” for the rest of the
library: market data snapshots for historical lookbacks, instrument/portfolio
registries, statements models/results, and scenario definitions.

## Recommended architecture (simple + extensible)

### 1) Separate *domain types* from *persistence*

- Domain crates (`finstack-core`, `finstack-portfolio`, `finstack-valuations`,
  `finstack-statements`, `finstack-scenarios`) own the canonical types and
  business logic.
- `finstack-io` owns:
  - Storage backends (SQLite first; Postgres later)
  - Schemas/migrations
  - Codecs (JSON today; optional compression later)
  - High-level “loader” helpers (hydrate `Portfolio`, build `MarketContext`)

This keeps persistence concerns (SQL, filenames, codecs) out of pricing,
portfolio aggregation, and scenario engines.

### 2) Pick storage “tool for the job”

**Default (local / embedded / test)**: SQLite
- One-file DB, ACID, easy migrations, easy to ship.
- Good fit for:
  - MarketContext snapshots by `as_of` (daily curves/surfaces, FX matrices)
  - Instruments registry (JSON)
  - Portfolios by `as_of` (positions snapshots)
  - Scenario specs and statement model specs

**Scale-out (multi-user / central service)**: Postgres (future optional backend)
- Same logical schema, different connection + migrations.

**Very large time-series payloads (optional)**: Parquet on filesystem/object-store
- Store the blob (Parquet) separately and keep a pointer + checksum in SQL.
- Good fit for:
  - Tick/quote history
  - Large valuation result tables
  - Statement outputs exported as DataFrames

### 3) Store *versioned snapshots* for reproducibility

For calibration/backtesting and deterministic lookbacks, prefer persisting:
- **MarketContext snapshots** (`MarketContextState`) keyed by `(market_id, as_of)`
- **Portfolio snapshots** (`PortfolioSpec`) keyed by `(portfolio_id, as_of)`

This makes historical reruns reproducible: you can re-load “what we knew then”
without needing to replay external vendor data.

### 4) Keep schemas narrow and payloads typed

Use SQL tables for indexing + integrity, but store the “payload” as bytes:
- `payload` = JSON bytes (or compressed bytes) of a stable, serde type
- SQL columns capture query keys (`id`, `as_of`, optional `scope`, `kind`, etc.)

This avoids over-normalization early while staying easy to evolve.

## Data model (initial)

Suggested minimal tables (SQLite / Postgres):

- `market_contexts(id, as_of, payload, meta, created_at, updated_at)`
- `instruments(id, payload, meta, created_at, updated_at)`
- `portfolios(id, as_of, payload, meta, created_at, updated_at)`
- `scenarios(id, payload, meta, created_at, updated_at)`
- `statement_models(id, payload, meta, created_at, updated_at)`

Where:
- `payload` is the JSON snapshot of the domain type:
  - market: `finstack_core::market_data::context::MarketContextState`
  - instrument: `finstack_valuations::instruments::InstrumentJson`
  - portfolio: `finstack_portfolio::PortfolioSpec`
  - scenario: `finstack_scenarios::ScenarioSpec`
  - statements: `finstack_statements::FinancialModelSpec`
- `meta` is a small JSON object for provenance (vendor, run id, notes, tags).

## Loader helpers (what “easy setup” means)

The core ergonomic goal is to provide:

- `load_market_context(market_id, as_of) -> MarketContext`
- `load_portfolio(portfolio_id, as_of) -> Portfolio` (hydrated with instruments)
- `load_portfolio_with_market(...) -> (Portfolio, MarketContext)`

Hydration rule:
- Positions can either inline `instrument_spec` (self-contained portfolios), or
  store only `instrument_id` and resolve missing specs from the instruments
  registry.

## Additional considerations

- **As-of vs observed-at**: store both if you ingest real market feeds.
- **Provenance**: source/vendor, curve build config hash, and calibration trace
  (`finstack_core::explain::ExplanationTrace`) are often more valuable than the
  numbers themselves.
- **Schema versioning**: track a DB schema version and keep snapshot payloads
  versioned (domain crates already do this for `MarketContextState`).
- **Transactions**: portfolio + instruments + market snapshot updates should be
  atomic when used for official “runs”.
- **Determinism**: avoid “latest” reads by default; require an `as_of`.
- **Caching**: consider an in-memory cache layer for hot reads, but keep it
  behind the persistence API (don’t leak caches into domain code).

## Current crate surface

- `Store`: backend-agnostic CRUD for market contexts, instruments, portfolios,
  scenarios, and statement models.
- `LookbackStore`: optional range-query API (historical lookbacks).
- `SqliteStore`: default SQLite backend (feature `sqlite`, enabled by default).
