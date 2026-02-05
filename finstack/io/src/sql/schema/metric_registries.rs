//! MetricRegistries table definition.

use sea_query::{ColumnDef, Iden, Table, TableCreateStatement};

use super::{created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum MetricRegistries {
    #[allow(dead_code)]
    Table,
    Namespace,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for MetricRegistries {
    const BASE_NAME: &'static str = "metric_registries";

    fn migration_version() -> i64 {
        2
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(MetricRegistries::Namespace)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(payload_col(backend, MetricRegistries::Payload))
            .col(meta_col(backend, MetricRegistries::Meta))
            .col(created_at_col(backend, MetricRegistries::CreatedAt))
            .col(updated_at_col(backend, MetricRegistries::UpdatedAt))
            .to_owned()
    }
}
