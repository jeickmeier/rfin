//! Migration SQL builders.
//!
//! This module automatically discovers tables and indexes from the schema module.
//! To add a new table:
//! 1. Create a new file in `schema/` (e.g., `schema/my_table.rs`)
//! 2. Implement the `TableDefinition` trait
//! 3. Add the table to `schema/mod.rs` in `tables_by_version_with_naming()`
//! 4. Add any indexes via `indexes_with_naming()` in your table module
//!
//! # Custom Table Naming
//!
//! Use [`migrations_for_with_naming`] to create migrations with custom table names:
//!
//! ```ignore
//! use finstack_io::sql::{schema::TableNaming, migrations, Backend};
//!
//! let naming = TableNaming::new().with_prefix("ref_cln_");
//! let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, &naming);
//! ```

use sea_query::{
    IndexCreateStatement, PostgresQueryBuilder, SchemaStatementBuilder, SqliteQueryBuilder,
};

use super::{
    schema::{self, TableNaming},
    Backend,
};

/// Latest schema version.
pub const LATEST_VERSION: i64 = 3;

fn build_sql(backend: Backend, stmt: impl SchemaStatementBuilder) -> String {
    match backend {
        Backend::Sqlite => stmt.to_string(SqliteQueryBuilder),
        Backend::Postgres => stmt.to_string(PostgresQueryBuilder),
    }
}

fn build_index_sql(backend: Backend, stmt: IndexCreateStatement) -> String {
    match backend {
        Backend::Sqlite => stmt.to_string(SqliteQueryBuilder),
        Backend::Postgres => stmt.to_string(PostgresQueryBuilder),
    }
}

/// Returns all migrations grouped by version with custom table naming.
///
/// This function automatically picks up tables and indexes from the schema module.
/// Tables come from `schema::tables_by_version_with_naming()` and indexes from
/// `schema::indexes_by_version_with_naming()`.
///
/// When adding new tables, just add them to the schema module - no changes needed here.
pub fn migrations_for_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Vec<(i64, Vec<String>)> {
    // Get all table definitions grouped by version
    let tables_by_version = schema::tables_by_version_with_naming(backend, naming);

    // Get all index definitions grouped by version
    let indexes_by_version = schema::indexes_by_version_with_naming(backend, naming);

    // Merge tables and indexes by version
    let mut migrations: Vec<(i64, Vec<String>)> = Vec::new();

    for (version, tables) in tables_by_version {
        let mut stmts: Vec<String> = tables.into_iter().map(|t| build_sql(backend, t)).collect();

        // Find indexes for this version and add them
        if let Some((_, indexes)) = indexes_by_version.iter().find(|(v, _)| *v == version) {
            for idx in indexes {
                stmts.push(build_index_sql(backend, idx.clone()));
            }
        }

        migrations.push((version, stmts));
    }

    migrations
}

/// Returns all migrations grouped by version with default naming.
///
/// This is a convenience function that uses [`TableNaming::default()`].
/// For custom table names, use [`migrations_for_with_naming`].
pub fn migrations_for(backend: Backend) -> Vec<(i64, Vec<String>)> {
    migrations_for_with_naming(backend, &TableNaming::default())
}

/// Returns the SQL to create the schema_migrations table with custom naming.
#[allow(dead_code)]
pub fn schema_migrations_table_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    build_sql(
        backend,
        schema::schema_migrations_table_with_naming(backend, naming),
    )
}

/// Returns the SQL to create the schema_migrations table.
#[allow(dead_code)]
pub fn schema_migrations_table_sql(backend: Backend) -> String {
    build_sql(backend, schema::schema_migrations_table(backend))
}
