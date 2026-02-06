//! WorkflowEvents table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum WorkflowEvents {
    #[allow(dead_code)]
    Table,
    Id,
    ChangeId,
    ResourceType,
    ResourceId,
    ResourceKey2,
    FromState,
    ToState,
    ActorKind,
    ActorId,
    AtTs,
    Note,
}

impl TableDefinition for WorkflowEvents {
    const BASE_NAME: &'static str = "workflow_events";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(WorkflowEvents::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(WorkflowEvents::ChangeId).string().not_null())
            .col(
                ColumnDef::new(WorkflowEvents::ResourceType)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WorkflowEvents::ResourceId)
                    .string()
                    .not_null(),
            )
            .col(
                ColumnDef::new(WorkflowEvents::ResourceKey2)
                    .string()
                    .not_null()
                    .default(""),
            )
            .col(
                ColumnDef::new(WorkflowEvents::FromState)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(WorkflowEvents::ToState).string().not_null())
            .col(
                ColumnDef::new(WorkflowEvents::ActorKind)
                    .string()
                    .not_null(),
            )
            .col(ColumnDef::new(WorkflowEvents::ActorId).string().not_null())
            .col(created_at_col(backend, WorkflowEvents::AtTs))
            .col(ColumnDef::new(WorkflowEvents::Note).string())
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_change = format!(
            "idx_{}workflow_events{}_change",
            naming.prefix(),
            naming.suffix()
        );
        let idx_resource = format!(
            "idx_{}workflow_events{}_resource",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_change)
                .table(naming.alias(Self::BASE_NAME))
                .col(WorkflowEvents::ChangeId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_resource)
                .table(naming.alias(Self::BASE_NAME))
                .col(WorkflowEvents::ResourceType)
                .col(WorkflowEvents::ResourceId)
                .to_owned(),
        ]
    }
}
