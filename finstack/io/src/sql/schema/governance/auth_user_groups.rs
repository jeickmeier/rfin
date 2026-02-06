//! AuthUserGroups table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum AuthUserGroups {
    #[allow(dead_code)]
    Table,
    UserId,
    GroupId,
    CreatedAt,
}

impl TableDefinition for AuthUserGroups {
    const BASE_NAME: &'static str = "auth_user_groups";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(AuthUserGroups::UserId).string().not_null())
            .col(ColumnDef::new(AuthUserGroups::GroupId).string().not_null())
            .col(created_at_col(backend, AuthUserGroups::CreatedAt))
            .primary_key(
                Index::create()
                    .col(AuthUserGroups::UserId)
                    .col(AuthUserGroups::GroupId)
                    .primary(),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_group = format!(
            "idx_{}auth_user_groups{}_group_id",
            naming.prefix(),
            naming.suffix()
        );
        let idx_user = format!(
            "idx_{}auth_user_groups{}_user_id",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_user)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUserGroups::UserId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_group)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUserGroups::GroupId)
                .to_owned(),
        ]
    }
}
