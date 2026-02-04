//! Instruments table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use super::{created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum Instruments {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for Instruments {
    const BASE_NAME: &'static str = "instruments";

    fn migration_version() -> i64 {
        1
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(Instruments::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(payload_col(backend, Instruments::Payload))
            .col(meta_col(backend, Instruments::Meta))
            .col(created_at_col(backend, Instruments::CreatedAt))
            .col(updated_at_col(backend, Instruments::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!(
            "idx_{}instruments{}_created_at",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .if_not_exists()
            .name(&idx_name)
            .table(naming.alias(Self::BASE_NAME))
            .col(Instruments::CreatedAt)
            .to_owned()]
    }
}
