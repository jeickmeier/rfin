//! Shared SQL schema and statement builders.

pub mod migrations;
pub mod schema;
pub mod statements;

/// SQL backend dialect selector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Backend {
    /// SQLite dialect.
    Sqlite,
    /// Postgres dialect.
    #[allow(dead_code)]
    Postgres,
}
