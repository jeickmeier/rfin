//! WorkflowPolicies table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum WorkflowPolicies {
    #[allow(dead_code)]
    Table,
    Id,
    ResourceType,
    Name,
    IsActive,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for WorkflowPolicies {
    const BASE_NAME: &'static str = "workflow_policies";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(WorkflowPolicies::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(WorkflowPolicies::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(WorkflowPolicies::Name).string())
            .col(
                ColumnDef::new(WorkflowPolicies::IsActive)
                    .boolean()
                    .not_null()
                    .default(true),
            )
            .col(created_at_col(backend, WorkflowPolicies::CreatedAt))
            .col(updated_at_col(backend, WorkflowPolicies::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_resource = format!(
            "idx_{}workflow_policies{}_resource_type",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .if_not_exists()
            .name(&idx_resource)
            .table(naming.alias(Self::BASE_NAME))
            .col(WorkflowPolicies::ResourceType)
            .to_owned()]
    }
}
