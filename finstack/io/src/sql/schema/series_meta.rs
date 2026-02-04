//! SeriesMeta table definition.

use sea_query::{ColumnDef, Iden, Index, Table, TableCreateStatement};

use super::{created_at_col, json_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum SeriesMeta {
    Table,
    Namespace,
    Kind,
    SeriesId,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for SeriesMeta {
    const BASE_NAME: &'static str = "series_meta";

    fn migration_version() -> i64 {
        3
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(SeriesMeta::Namespace).string().not_null())
            .col(ColumnDef::new(SeriesMeta::Kind).string().not_null())
            .col(ColumnDef::new(SeriesMeta::SeriesId).string().not_null())
            .col(json_col(backend, SeriesMeta::Meta))
            .col(created_at_col(backend, SeriesMeta::CreatedAt))
            .col(updated_at_col(backend, SeriesMeta::UpdatedAt))
            .primary_key(
                Index::create()
                    .col(SeriesMeta::Namespace)
                    .col(SeriesMeta::Kind)
                    .col(SeriesMeta::SeriesId),
            )
            .to_owned()
    }
}
