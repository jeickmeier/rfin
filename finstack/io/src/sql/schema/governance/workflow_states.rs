//! WorkflowStates table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum WorkflowStates {
    #[allow(dead_code)]
    Table,
    PolicyId,
    StateKey,
    IsFinal,
    VerifiedSource,
    SystemOnly,
    Category,
    SortOrder,
}

impl TableDefinition for WorkflowStates {
    const BASE_NAME: &'static str = "workflow_states";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(_backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(WorkflowStates::PolicyId).string().not_null())
            .col(ColumnDef::new(WorkflowStates::StateKey).string().not_null())
            .col(ColumnDef::new(WorkflowStates::IsFinal).boolean().not_null())
            .col(ColumnDef::new(WorkflowStates::VerifiedSource).string())
            .col(
                ColumnDef::new(WorkflowStates::SystemOnly)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(ColumnDef::new(WorkflowStates::Category).string().not_null())
            .col(
                ColumnDef::new(WorkflowStates::SortOrder)
                    .integer()
                    .not_null()
                    .default(0),
            )
            .primary_key(
                Index::create()
                    .col(WorkflowStates::PolicyId)
                    .col(WorkflowStates::StateKey)
                    .primary(),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_policy = format!(
            "idx_{}workflow_states{}_policy",
            naming.prefix(),
            naming.suffix()
        );
        vec![Index::create()
            .if_not_exists()
            .name(&idx_policy)
            .table(naming.alias(Self::BASE_NAME))
            .col(WorkflowStates::PolicyId)
            .to_owned()]
    }
}
