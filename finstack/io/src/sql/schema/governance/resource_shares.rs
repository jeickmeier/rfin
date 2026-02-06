//! ResourceShares table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum ResourceShares {
    #[allow(dead_code)]
    Table,
    Id,
    ResourceType,
    ResourceId,
    ShareType,
    ShareId,
    ShareScopeId,
    Permission,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for ResourceShares {
    const BASE_NAME: &'static str = "resource_shares";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(ResourceShares::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(ResourceShares::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceShares::ResourceId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceShares::ShareType)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(ResourceShares::ShareId).string().not_null())
            .col(ColumnDef::new(ResourceShares::ShareScopeId).string())
            .col(
                ColumnDef::new(ResourceShares::Permission)
                    .string()
                    .not_null(),
            )
            .col(created_at_col(backend, ResourceShares::CreatedAt))
            .col(updated_at_col(backend, ResourceShares::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_resource = format!(
            "idx_{}resource_shares{}_resource",
            naming.prefix(),
            naming.suffix()
        );
        let idx_share = format!(
            "idx_{}resource_shares{}_share",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_resource)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceShares::ResourceType)
                .col(ResourceShares::ResourceId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_share)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceShares::ShareType)
                .col(ResourceShares::ShareId)
                .col(ResourceShares::ShareScopeId)
                .to_owned(),
        ]
    }
}
