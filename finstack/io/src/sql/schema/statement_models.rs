//! StatementModels table definition.

use sea_query::{ColumnDef, Iden, Table, TableCreateStatement};

use super::{created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum StatementModels {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for StatementModels {
    const BASE_NAME: &'static str = "statement_models";

    fn migration_version() -> i64 {
        1
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(StatementModels::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(payload_col(backend, StatementModels::Payload))
            .col(meta_col(backend, StatementModels::Meta))
            .col(created_at_col(backend, StatementModels::CreatedAt))
            .col(updated_at_col(backend, StatementModels::UpdatedAt))
            .to_owned()
    }
}
