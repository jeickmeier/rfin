//! Portfolios table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use super::{
    as_of_col, created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming,
};
use crate::sql::Backend;

#[derive(Iden)]
pub enum Portfolios {
    Table,
    Id,
    AsOf,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for Portfolios {
    const BASE_NAME: &'static str = "portfolios";

    fn migration_version() -> i64 {
        1
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(Portfolios::Id).string().not_null())
            .col(as_of_col(backend, Portfolios::AsOf))
            .col(payload_col(backend, Portfolios::Payload))
            .col(meta_col(backend, Portfolios::Meta))
            .col(created_at_col(backend, Portfolios::CreatedAt))
            .col(updated_at_col(backend, Portfolios::UpdatedAt))
            .primary_key(Index::create().col(Portfolios::Id).col(Portfolios::AsOf))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!("idx_{}portfolios{}_as_of", naming.prefix(), naming.suffix());
        vec![Index::create()
            .if_not_exists()
            .name(&idx_name)
            .table(naming.alias(Self::BASE_NAME))
            .col(Portfolios::AsOf)
            .to_owned()]
    }
}
