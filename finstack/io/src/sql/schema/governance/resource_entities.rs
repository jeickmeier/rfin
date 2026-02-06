//! ResourceEntities table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum ResourceEntities {
    #[allow(dead_code)]
    Table,
    ResourceType,
    ResourceId,
    OwnerUserId,
    VisibilityScope,
    VisibilityId,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for ResourceEntities {
    const BASE_NAME: &'static str = "resource_entities";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(ResourceEntities::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceEntities::ResourceId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceEntities::OwnerUserId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceEntities::VisibilityScope)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(ResourceEntities::VisibilityId).string())
            .col(created_at_col(backend, ResourceEntities::CreatedAt))
            .col(updated_at_col(backend, ResourceEntities::UpdatedAt))
            .primary_key(
                Index::create()
                    .col(ResourceEntities::ResourceType)
                    .col(ResourceEntities::ResourceId)
                    .primary(),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_owner = format!(
            "idx_{}resource_entities{}_owner_user_id",
            naming.prefix(),
            naming.suffix()
        );
        let idx_visibility = format!(
            "idx_{}resource_entities{}_visibility",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_owner)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceEntities::OwnerUserId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_visibility)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceEntities::VisibilityScope)
                .col(ResourceEntities::VisibilityId)
                .to_owned(),
        ]
    }
}
