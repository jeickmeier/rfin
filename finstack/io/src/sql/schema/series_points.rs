//! SeriesPoints table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use super::{created_at_col, json_col, ts_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum SeriesPoints {
    Table,
    Namespace,
    Kind,
    SeriesId,
    Ts,
    Value,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for SeriesPoints {
    const BASE_NAME: &'static str = "series_points";

    fn migration_version() -> i64 {
        3
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(SeriesPoints::Namespace).string().not_null())
            .col(ColumnDef::new(SeriesPoints::Kind).string().not_null())
            .col(ColumnDef::new(SeriesPoints::SeriesId).string().not_null())
            .col(ts_col(backend, SeriesPoints::Ts))
            .col(ColumnDef::new(SeriesPoints::Value).double())
            .col(json_col(backend, SeriesPoints::Payload))
            .col(json_col(backend, SeriesPoints::Meta))
            .col(created_at_col(backend, SeriesPoints::CreatedAt))
            .col(updated_at_col(backend, SeriesPoints::UpdatedAt))
            .primary_key(
                Index::create()
                    .col(SeriesPoints::Namespace)
                    .col(SeriesPoints::Kind)
                    .col(SeriesPoints::SeriesId)
                    .col(SeriesPoints::Ts),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let prefix = naming.prefix();
        let suffix = naming.suffix();

        vec![
            // Index for time-range scans within a namespace (common query pattern)
            Index::create()
                .if_not_exists()
                .name(format!(
                    "idx_{prefix}{}{suffix}_namespace_ts",
                    Self::BASE_NAME
                ))
                .table(naming.alias(Self::BASE_NAME))
                .col(SeriesPoints::Namespace)
                .col(SeriesPoints::Ts)
                .to_owned(),
            // Index for global time-range queries across all series
            Index::create()
                .if_not_exists()
                .name(format!("idx_{prefix}{}{suffix}_ts", Self::BASE_NAME))
                .table(naming.alias(Self::BASE_NAME))
                .col(SeriesPoints::Ts)
                .to_owned(),
        ]
    }
}
