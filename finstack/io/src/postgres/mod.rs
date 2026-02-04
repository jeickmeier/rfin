//! Postgres backend for `finstack-io`.

mod bulk_store;
mod core_store;
mod lookback_store;
mod store;
mod timeseries_store;

pub use store::PostgresStore;
