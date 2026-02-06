//! WorkflowTransitions table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum WorkflowTransitions {
    #[allow(dead_code)]
    Table,
    Id,
    PolicyId,
    FromState,
    ToState,
    RequiredRoleId,
    RequiredGroupId,
    AllowOwner,
    AllowSystemActor,
    RequireVerifierNotOwner,
    RequireVerifierNotSubmitter,
    RequireDistinctFromLastActor,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for WorkflowTransitions {
    const BASE_NAME: &'static str = "workflow_transitions";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(WorkflowTransitions::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::PolicyId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::FromState)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::ToState)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(WorkflowTransitions::RequiredRoleId).string())
            .col(ColumnDef::new(WorkflowTransitions::RequiredGroupId).string())
            .col(
                ColumnDef::new(WorkflowTransitions::AllowOwner)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::AllowSystemActor)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::RequireVerifierNotOwner)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::RequireVerifierNotSubmitter)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                ColumnDef::new(WorkflowTransitions::RequireDistinctFromLastActor)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(created_at_col(backend, WorkflowTransitions::CreatedAt))
            .col(updated_at_col(backend, WorkflowTransitions::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_policy = format!(
            "idx_{}workflow_transitions{}_policy_state",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .if_not_exists()
            .name(&idx_policy)
            .table(naming.alias(Self::BASE_NAME))
            .col(WorkflowTransitions::PolicyId)
            .col(WorkflowTransitions::FromState)
            .col(WorkflowTransitions::ToState)
            .to_owned()]
    }
}
