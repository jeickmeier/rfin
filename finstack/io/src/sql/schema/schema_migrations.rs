//! SchemaMigrations table definition (internal migration tracking).

use sea_query::{ColumnDef, Iden, Table, TableCreateStatement};

use super::{TableDefinition, TableNaming};
use crate::sql::Backend;

#[allow(dead_code)]
#[derive(Iden)]
#[iden = "finstack_schema_migrations"]
pub enum SchemaMigrations {
    Table,
    Version,
    AppliedAt,
}

impl TableDefinition for SchemaMigrations {
    const BASE_NAME: &'static str = "finstack_schema_migrations";

    fn migration_version() -> i64 {
        0 // Internal table, not part of user migrations
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        let applied_at = match backend {
            Backend::Sqlite => ColumnDef::new(SchemaMigrations::AppliedAt)
                .string()
                .not_null()
                .to_owned(),
            Backend::Postgres => ColumnDef::new(SchemaMigrations::AppliedAt)
                .timestamp_with_time_zone()
                .not_null()
                .to_owned(),
        };
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(SchemaMigrations::Version)
                    .big_integer()
                    .not_null()
                    .primary_key(),
            )
            .col(applied_at)
            .to_owned()
    }
}
