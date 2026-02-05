//! Scenarios table definition.

use sea_query::{ColumnDef, Iden, Table, TableCreateStatement};

use super::{created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum Scenarios {
    #[allow(dead_code)]
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for Scenarios {
    const BASE_NAME: &'static str = "scenarios";

    fn migration_version() -> i64 {
        1
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(Scenarios::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(payload_col(backend, Scenarios::Payload))
            .col(meta_col(backend, Scenarios::Meta))
            .col(created_at_col(backend, Scenarios::CreatedAt))
            .col(updated_at_col(backend, Scenarios::UpdatedAt))
            .to_owned()
    }
}
