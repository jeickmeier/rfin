//! ResourceChanges table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{
    created_at_col, meta_col, payload_col, updated_at_col, TableDefinition, TableNaming,
};
use crate::sql::Backend;

#[derive(Iden)]
pub enum ResourceChanges {
    #[allow(dead_code)]
    Table,
    ChangeId,
    ResourceType,
    ResourceId,
    ResourceKey2,
    ChangeKind,
    WorkflowPolicyId,
    WorkflowState,
    OwnerUserId,
    CreatedByKind,
    CreatedById,
    SubmittedAt,
    AppliedAt,
    BaseEtag,
    IngestionSource,
    IngestionRunId,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for ResourceChanges {
    const BASE_NAME: &'static str = "resource_changes";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        let mut table = Table::create();
        table
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(ResourceChanges::ChangeId)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(ResourceChanges::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceChanges::ResourceId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceChanges::ResourceKey2)
                    .string()
                    .not_null()
                    .default(""),
            )
            .col(
                ColumnDef::new(ResourceChanges::ChangeKind)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(ResourceChanges::WorkflowPolicyId).string())
            .col(
                ColumnDef::new(ResourceChanges::WorkflowState)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceChanges::OwnerUserId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceChanges::CreatedByKind)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(ResourceChanges::CreatedById)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(ResourceChanges::SubmittedAt).string())
            .col(ColumnDef::new(ResourceChanges::AppliedAt).string())
            .col(ColumnDef::new(ResourceChanges::BaseEtag).string())
            .col(ColumnDef::new(ResourceChanges::IngestionSource).string())
            .col(ColumnDef::new(ResourceChanges::IngestionRunId).string())
            .col(payload_col(backend, ResourceChanges::Payload))
            .col(meta_col(backend, ResourceChanges::Meta))
            .col(created_at_col(backend, ResourceChanges::CreatedAt))
            .col(updated_at_col(backend, ResourceChanges::UpdatedAt));
        table.to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_resource = format!(
            "idx_{}resource_changes{}_resource",
            naming.prefix(),
            naming.suffix()
        );
        let idx_owner_state = format!(
            "idx_{}resource_changes{}_owner_state",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_resource)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceChanges::ResourceType)
                .col(ResourceChanges::ResourceId)
                .col(ResourceChanges::ResourceKey2)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_owner_state)
                .table(naming.alias(Self::BASE_NAME))
                .col(ResourceChanges::OwnerUserId)
                .col(ResourceChanges::WorkflowState)
                .to_owned(),
        ]
    }
}
