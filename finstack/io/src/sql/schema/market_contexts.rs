//! MarketContexts table definition.

use sea_query::{ColumnDef, Iden, Index, Table, TableCreateStatement};

use super::{
    as_of_col, created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming,
};
use crate::sql::Backend;

#[derive(Iden)]
pub enum MarketContexts {
    #[allow(dead_code)]
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

    // No additional indexes by default:
    // the primary key (id, as_of) supports the common access patterns:
    // - exact snapshot lookup (id, as_of)
    // - latest on/before (id with ordered as_of)
    // - range scans (id with as_of BETWEEN)
}
