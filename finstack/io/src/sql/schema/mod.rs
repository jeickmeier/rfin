//! Shared schema definitions using sea-query.
//!
//! Each table is defined in its own module and implements [`TableDefinition`]
//! to enable automatic migration discovery.
//!
//! # Custom Table Naming
//!
//! Use [`TableNaming`] to customize table names for your deployment:
//!
//! ```ignore
//! use finstack_io::sql::schema::TableNaming;
//!
//! // Add a prefix to all tables: instruments -> ref_cln_instruments
//! let naming = TableNaming::new().with_prefix("ref_cln_");
//!
//! // Or use fully custom names
//! let naming = TableNaming::new()
//!     .with_override("instruments", "my_custom_instruments_table");
//! ```

mod instruments;
mod market_contexts;
mod metric_registries;
mod portfolios;
mod scenarios;
mod schema_migrations;
mod series_meta;
mod series_points;
mod statement_models;

use std::collections::HashMap;

use sea_query::{Alias, ColumnDef, Iden, IndexCreateStatement, TableCreateStatement};

use crate::sql::Backend;

// Re-export table enums for use in statements
pub use instruments::Instruments;
pub use market_contexts::MarketContexts;
pub use metric_registries::MetricRegistries;
pub use portfolios::Portfolios;
pub use scenarios::Scenarios;
pub use series_meta::SeriesMeta;
pub use series_points::SeriesPoints;
pub use statement_models::StatementModels;

// ---------------------------------------------------------------------------
// TableNaming - customize table names per deployment
// ---------------------------------------------------------------------------

/// Configuration for custom table naming conventions.
///
/// Supports prefixes, suffixes, and full custom name overrides.
///
/// # Examples
///
/// ```ignore
/// use finstack_io::sql::schema::TableNaming;
///
/// // Default naming (no changes)
/// let default = TableNaming::default();
/// assert_eq!(default.resolve("instruments"), "instruments");
///
/// // Prefix all tables
/// let prefixed = TableNaming::new().with_prefix("ref_cln_");
/// assert_eq!(prefixed.resolve("instruments"), "ref_cln_instruments");
///
/// // Suffix all tables
/// let suffixed = TableNaming::new().with_suffix("_v2");
/// assert_eq!(suffixed.resolve("instruments"), "instruments_v2");
///
/// // Both prefix and suffix
/// let both = TableNaming::new().with_prefix("app_").with_suffix("_tbl");
/// assert_eq!(both.resolve("instruments"), "app_instruments_tbl");
///
/// // Override specific tables
/// let custom = TableNaming::new()
///     .with_prefix("ref_")
///     .with_override("instruments", "my_instruments");
/// assert_eq!(custom.resolve("instruments"), "my_instruments");
/// assert_eq!(custom.resolve("portfolios"), "ref_portfolios");
/// ```
/// Note: These methods are public API for external consumers, but show as unused
/// within this crate since the crate itself uses default naming.
#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct TableNaming {
    /// Prefix to prepend to all table names (e.g., "ref_cln_")
    prefix: String,
    /// Suffix to append to all table names (e.g., "_v2")
    suffix: String,
    /// Explicit overrides for specific tables (base_name -> custom_name)
    overrides: HashMap<String, String>,
}

#[allow(dead_code)]
impl TableNaming {
    /// Creates a new TableNaming with default (unchanged) names.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a prefix for all table names.
    ///
    /// Example: `with_prefix("ref_cln_")` transforms `instruments` -> `ref_cln_instruments`
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Sets a suffix for all table names.
    ///
    /// Example: `with_suffix("_v2")` transforms `instruments` -> `instruments_v2`
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Overrides the name for a specific table.
    ///
    /// Overrides take precedence over prefix/suffix.
    pub fn with_override(
        mut self,
        base_name: impl Into<String>,
        custom_name: impl Into<String>,
    ) -> Self {
        self.overrides.insert(base_name.into(), custom_name.into());
        self
    }

    /// Resolves the actual table name for a base table name.
    pub fn resolve(&self, base_name: &str) -> String {
        if let Some(custom) = self.overrides.get(base_name) {
            custom.clone()
        } else {
            format!("{}{}{}", self.prefix, base_name, self.suffix)
        }
    }

    /// Returns a sea-query Alias for use in table definitions.
    pub fn alias(&self, base_name: &str) -> Alias {
        Alias::new(self.resolve(base_name))
    }

    /// Returns the prefix (for index naming, etc.)
    pub fn prefix(&self) -> &str {
        &self.prefix
    }

    /// Returns the suffix (for index naming, etc.)
    pub fn suffix(&self) -> &str {
        &self.suffix
    }
}

// ---------------------------------------------------------------------------
// TableDefinition trait - implement this for new tables
// ---------------------------------------------------------------------------

/// Trait for table definitions that enables automatic migration discovery.
///
/// Each table module implements this trait, allowing the migration system
/// to automatically pick up new tables when they are added.
pub trait TableDefinition {
    /// The base table name (e.g., "instruments").
    ///
    /// This is the canonical name before any naming customization is applied.
    const BASE_NAME: &'static str;

    /// The migration version this table was introduced in.
    ///
    /// This is informational and used for documentation/validation purposes.
    #[allow(dead_code)]
    fn migration_version() -> i64;

    /// Creates the table definition for the given backend with custom naming.
    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement;

    /// Creates the table definition with default naming.
    ///
    /// Convenience method that uses [`TableNaming::default()`].
    fn create_table(backend: Backend) -> TableCreateStatement {
        Self::create_table_with_naming(backend, &TableNaming::default())
    }

    /// Returns indexes for this table with custom naming.
    ///
    /// Override this method to define indexes for your table.
    /// The default implementation returns no indexes.
    fn indexes_with_naming(_backend: Backend, _naming: &TableNaming) -> Vec<IndexCreateStatement> {
        Vec::new()
    }

    /// Returns indexes for this table with default naming.
    #[allow(dead_code)]
    fn indexes(backend: Backend) -> Vec<IndexCreateStatement> {
        Self::indexes_with_naming(backend, &TableNaming::default())
    }
}

/// Returns all table definitions grouped by migration version with custom naming.
///
/// When adding a new table:
/// 1. Create a new module file in `schema/`
/// 2. Implement `TableDefinition` for your table
/// 3. Add the table to this function under the appropriate version
///
/// The migration system will automatically pick up the new table.
pub fn tables_by_version_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Vec<(i64, Vec<TableCreateStatement>)> {
    vec![
        // v1: core JSON tables
        (
            1,
            vec![
                instruments::Instruments::create_table_with_naming(backend, naming),
                portfolios::Portfolios::create_table_with_naming(backend, naming),
                market_contexts::MarketContexts::create_table_with_naming(backend, naming),
                scenarios::Scenarios::create_table_with_naming(backend, naming),
                statement_models::StatementModels::create_table_with_naming(backend, naming),
            ],
        ),
        // v2: metric registries
        (
            2,
            vec![metric_registries::MetricRegistries::create_table_with_naming(backend, naming)],
        ),
        // v3: time-series tables
        (
            3,
            vec![
                series_meta::SeriesMeta::create_table_with_naming(backend, naming),
                series_points::SeriesPoints::create_table_with_naming(backend, naming),
            ],
        ),
    ]
}

/// Returns all table definitions grouped by migration version with default naming.
///
/// Convenience function that uses [`TableNaming::default()`].
#[allow(dead_code)]
pub fn tables_by_version(backend: Backend) -> Vec<(i64, Vec<TableCreateStatement>)> {
    tables_by_version_with_naming(backend, &TableNaming::default())
}

/// Returns all index definitions grouped by migration version with custom naming.
///
/// Indexes are collected from each table's `indexes_with_naming()` implementation.
pub fn indexes_by_version_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Vec<(i64, Vec<IndexCreateStatement>)> {
    vec![
        // v1: indexes for core tables
        (
            1,
            [
                instruments::Instruments::indexes_with_naming(backend, naming),
                portfolios::Portfolios::indexes_with_naming(backend, naming),
                market_contexts::MarketContexts::indexes_with_naming(backend, naming),
                scenarios::Scenarios::indexes_with_naming(backend, naming),
                statement_models::StatementModels::indexes_with_naming(backend, naming),
            ]
            .concat(),
        ),
        // v2: indexes for metric registries
        (
            2,
            metric_registries::MetricRegistries::indexes_with_naming(backend, naming),
        ),
        // v3: indexes for time-series tables
        (
            3,
            [
                series_meta::SeriesMeta::indexes_with_naming(backend, naming),
                series_points::SeriesPoints::indexes_with_naming(backend, naming),
            ]
            .concat(),
        ),
    ]
}

/// Returns all index definitions grouped by migration version with default naming.
#[allow(dead_code)]
pub fn indexes_by_version(backend: Backend) -> Vec<(i64, Vec<IndexCreateStatement>)> {
    indexes_by_version_with_naming(backend, &TableNaming::default())
}

// ---------------------------------------------------------------------------
// Common column helpers
// ---------------------------------------------------------------------------

/// Creates a `created_at` timestamp column with backend-appropriate type and default.
pub fn created_at_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut col = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            col.string();
        }
        Backend::Postgres => {
            col.timestamp_with_time_zone();
        }
    }
    col.not_null();
    match backend {
        Backend::Sqlite => col.default("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => col.default("now()"),
    };
    col
}

/// Creates an `updated_at` timestamp column with backend-appropriate type and default.
pub fn updated_at_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut col = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            col.string();
        }
        Backend::Postgres => {
            col.timestamp_with_time_zone();
        }
    }
    col.not_null();
    match backend {
        Backend::Sqlite => col.default("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => col.default("now()"),
    };
    col
}

/// Creates a binary/JSONB payload column.
pub fn payload_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.binary();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def.not_null();
    def
}

/// Creates a JSON metadata column with empty object default.
pub fn meta_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def.not_null();
    match backend {
        Backend::Sqlite => def.default("'{}'"),
        Backend::Postgres => def.default("'{}'::jsonb"),
    };
    def
}

/// Creates an `as_of` date column.
pub fn as_of_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.date();
        }
    }
    def.not_null();
    def
}

/// Creates a timestamp column for time-series data.
pub fn ts_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.timestamp_with_time_zone();
        }
    }
    def.not_null();
    def
}

/// Creates an optional JSON column (nullable).
pub fn json_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def
}

// ---------------------------------------------------------------------------
// Schema migrations table (internal use)
// ---------------------------------------------------------------------------

/// Creates the schema_migrations table with custom naming.
#[allow(dead_code)]
pub fn schema_migrations_table_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> TableCreateStatement {
    schema_migrations::SchemaMigrations::create_table_with_naming(backend, naming)
}

/// Creates the schema_migrations table for tracking applied migrations.
#[allow(dead_code)]
pub fn schema_migrations_table(backend: Backend) -> TableCreateStatement {
    schema_migrations::SchemaMigrations::create_table(backend)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::Backend;
    use sea_query::SqliteQueryBuilder;

    #[test]
    fn table_naming_default_is_unchanged() {
        let naming = TableNaming::default();
        assert_eq!(naming.resolve("instruments"), "instruments");
        assert_eq!(naming.resolve("portfolios"), "portfolios");
    }

    #[test]
    fn table_naming_with_prefix() {
        let naming = TableNaming::new().with_prefix("ref_cln_");
        assert_eq!(naming.resolve("instruments"), "ref_cln_instruments");
        assert_eq!(naming.resolve("portfolios"), "ref_cln_portfolios");
    }

    #[test]
    fn table_naming_with_suffix() {
        let naming = TableNaming::new().with_suffix("_v2");
        assert_eq!(naming.resolve("instruments"), "instruments_v2");
        assert_eq!(naming.resolve("portfolios"), "portfolios_v2");
    }

    #[test]
    fn table_naming_with_prefix_and_suffix() {
        let naming = TableNaming::new().with_prefix("app_").with_suffix("_tbl");
        assert_eq!(naming.resolve("instruments"), "app_instruments_tbl");
    }

    #[test]
    fn table_naming_override_takes_precedence() {
        let naming = TableNaming::new()
            .with_prefix("ref_")
            .with_override("instruments", "my_custom_instruments");
        // Override takes precedence
        assert_eq!(naming.resolve("instruments"), "my_custom_instruments");
        // Other tables still get prefix
        assert_eq!(naming.resolve("portfolios"), "ref_portfolios");
    }

    #[test]
    fn table_naming_accessors() {
        let naming = TableNaming::new().with_prefix("pre_").with_suffix("_suf");
        assert_eq!(naming.prefix(), "pre_");
        assert_eq!(naming.suffix(), "_suf");
    }

    #[test]
    fn tables_by_version_with_default_naming() {
        let tables = tables_by_version(Backend::Sqlite);
        assert_eq!(tables.len(), 3); // v1, v2, v3

        // v1 has 5 tables
        assert_eq!(tables[0].0, 1);
        assert_eq!(tables[0].1.len(), 5);

        // v2 has 1 table
        assert_eq!(tables[1].0, 2);
        assert_eq!(tables[1].1.len(), 1);

        // v3 has 2 tables
        assert_eq!(tables[2].0, 3);
        assert_eq!(tables[2].1.len(), 2);
    }

    #[test]
    fn tables_by_version_with_custom_naming() {
        let naming = TableNaming::new().with_prefix("ref_cln_");
        let tables = tables_by_version_with_naming(Backend::Sqlite, &naming);

        // Check that the first table (instruments) has the prefixed name
        let sql = tables[0].1[0].to_string(SqliteQueryBuilder);
        assert!(
            sql.contains("ref_cln_instruments"),
            "Expected 'ref_cln_instruments' in SQL: {}",
            sql
        );
    }

    #[test]
    fn create_table_uses_custom_naming() {
        let naming = TableNaming::new().with_prefix("custom_");
        let stmt = Instruments::create_table_with_naming(Backend::Sqlite, &naming);
        let sql = stmt.to_string(SqliteQueryBuilder);
        assert!(
            sql.contains("custom_instruments"),
            "Expected 'custom_instruments' in SQL: {}",
            sql
        );
    }

    #[test]
    fn create_table_default_uses_standard_naming() {
        let stmt = Instruments::create_table(Backend::Sqlite);
        let sql = stmt.to_string(SqliteQueryBuilder);
        assert!(
            sql.contains("\"instruments\""),
            "Expected '\"instruments\"' in SQL: {}",
            sql
        );
    }

    #[test]
    fn instruments_has_created_at_index() {
        let indexes = Instruments::indexes(Backend::Sqlite);
        assert_eq!(indexes.len(), 1);

        let sql = indexes[0].to_string(SqliteQueryBuilder);
        assert!(
            sql.contains("idx_instruments_created_at"),
            "Expected 'idx_instruments_created_at' in SQL: {}",
            sql
        );
        assert!(
            sql.contains("\"created_at\""),
            "Expected 'created_at' column in SQL: {}",
            sql
        );
    }

    #[test]
    fn indexes_use_custom_naming() {
        let naming = TableNaming::new().with_prefix("ref_");
        let indexes = Instruments::indexes_with_naming(Backend::Sqlite, &naming);
        assert_eq!(indexes.len(), 1);

        let sql = indexes[0].to_string(SqliteQueryBuilder);
        assert!(
            sql.contains("idx_ref_instruments_created_at"),
            "Expected 'idx_ref_instruments_created_at' in SQL: {}",
            sql
        );
        assert!(
            sql.contains("\"ref_instruments\""),
            "Expected 'ref_instruments' table in SQL: {}",
            sql
        );
    }

    #[test]
    fn indexes_by_version_collects_all_indexes() {
        let indexes = indexes_by_version(Backend::Sqlite);
        assert_eq!(indexes.len(), 3); // v1, v2, v3

        // v1 has indexes for instruments, portfolios, market_contexts
        // (instruments: created_at, portfolios: as_of, market_contexts: as_of)
        assert_eq!(indexes[0].0, 1);
        assert_eq!(indexes[0].1.len(), 3);

        // v2 has no indexes (metric_registries)
        assert_eq!(indexes[1].0, 2);
        assert_eq!(indexes[1].1.len(), 0);

        // v3 has no indexes (series_meta, series_points)
        assert_eq!(indexes[2].0, 3);
        assert_eq!(indexes[2].1.len(), 0);
    }
}
