//! MarketContexts table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use super::{
    as_of_col, created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming,
};
use crate::sql::Backend;

#[derive(Iden)]
pub enum MarketContexts {
    Table,
    Id,
    AsOf,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for MarketContexts {
    const BASE_NAME: &'static str = "market_contexts";

    fn migration_version() -> i64 {
        1
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(MarketContexts::Id).string().not_null())
            .col(as_of_col(backend, MarketContexts::AsOf))
            .col(payload_col(backend, MarketContexts::Payload))
            .col(meta_col(backend, MarketContexts::Meta))
            .col(created_at_col(backend, MarketContexts::CreatedAt))
            .col(updated_at_col(backend, MarketContexts::UpdatedAt))
            .primary_key(
                Index::create()
                    .col(MarketContexts::Id)
                    .col(MarketContexts::AsOf),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!(
            "idx_{}market_contexts{}_as_of",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .name(&idx_name)
            .table(naming.alias(Self::BASE_NAME))
            .col(MarketContexts::AsOf)
            .to_owned()]
    }
}
