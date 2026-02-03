# IO Examples

This folder contains examples demonstrating the `finstack.io` persistence module.

## Examples

### io_persistence_example.py

A comprehensive example showing how to use the SQLite-backed persistence store for:

1. **Market Data Persistence** - Save and load market contexts with discount curves
2. **Bulk Operations** - Efficiently store multiple records in single transactions
3. **Lookback Queries** - Query historical data by date ranges
4. **Portfolio Specifications** - Store and retrieve portfolio snapshots
5. **Portfolio Lookback** - Track portfolio changes over time
6. **Metric Registries** - Store reusable financial metric definitions
7. **Instruments** - Notes on storing instrument definitions
8. **Scenarios** - Store and retrieve scenario specifications

## Running the Examples

From the `finstack-py` directory:

```bash
# Run the persistence example
uv run python examples/scripts/io/io_persistence_example.py
```

## Key Concepts

- **SqliteStore** - The main persistence interface backed by SQLite
- **Lookback queries** - `list_*` and `latest_*_on_or_before` methods for historical data
- **Bulk operations** - `put_*_batch` methods for efficient multi-record inserts
- **PortfolioSpec** - Serializable portfolio specification for storage
- **MetricRegistry** - Reusable financial metric definitions

## See Also

- [finstack/io/README.md](../../../finstack/io/README.md) - Rust crate documentation
- [finstack/examples/io/](../../../finstack/examples/io/) - Rust examples
