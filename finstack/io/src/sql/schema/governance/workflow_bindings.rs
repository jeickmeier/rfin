//! WorkflowBindings table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum WorkflowBindings {
    #[allow(dead_code)]
    Table,
    Id,
    ResourceType,
    VisibilityScope,
    VisibilityId,
    ChangeKind,
    BaseVerifiedSource,
    PolicyId,
    Priority,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for WorkflowBindings {
    const BASE_NAME: &'static str = "workflow_bindings";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(WorkflowBindings::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(WorkflowBindings::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(WorkflowBindings::VisibilityScope).string())
            .col(ColumnDef::new(WorkflowBindings::VisibilityId).string())
            .col(ColumnDef::new(WorkflowBindings::ChangeKind).string())
            .col(ColumnDef::new(WorkflowBindings::BaseVerifiedSource).string())
            .col(
                ColumnDef::new(WorkflowBindings::PolicyId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WorkflowBindings::Priority)
                    .integer()
                    .not_null()
                    .default(0),
            )
            .col(created_at_col(backend, WorkflowBindings::CreatedAt))
            .col(updated_at_col(backend, WorkflowBindings::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_resource = format!(
            "idx_{}workflow_bindings{}_resource",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .if_not_exists()
            .name(&idx_resource)
            .table(naming.alias(Self::BASE_NAME))
            .col(WorkflowBindings::ResourceType)
            .col(WorkflowBindings::VisibilityScope)
            .col(WorkflowBindings::VisibilityId)
            .to_owned()]
    }
}
