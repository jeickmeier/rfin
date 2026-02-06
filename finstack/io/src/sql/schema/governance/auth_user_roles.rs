//! AuthUserRoles table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum AuthUserRoles {
    #[allow(dead_code)]
    Table,
    UserId,
    RoleId,
    GroupId,
    CreatedAt,
}

impl TableDefinition for AuthUserRoles {
    const BASE_NAME: &'static str = "auth_user_roles";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        // group_id is NOT NULL with default "" to avoid NULL-in-PK issues.
        // SQL NULLs are not equal to each other, so a nullable PK column
        // would allow duplicate (user_id, role_id, NULL) rows.
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(ColumnDef::new(AuthUserRoles::UserId).string().not_null())
            .col(ColumnDef::new(AuthUserRoles::RoleId).string().not_null())
            .col(
                ColumnDef::new(AuthUserRoles::GroupId)
                    .string()
                    .not_null()
                    .default(""),
            )
            .col(created_at_col(backend, AuthUserRoles::CreatedAt))
            .primary_key(
                Index::create()
                    .col(AuthUserRoles::UserId)
                    .col(AuthUserRoles::RoleId)
                    .col(AuthUserRoles::GroupId)
                    .primary(),
            )
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_role = format!(
            "idx_{}auth_user_roles{}_role_id",
            naming.prefix(),
            naming.suffix()
        );
        let idx_user = format!(
            "idx_{}auth_user_roles{}_user_id",
            naming.prefix(),
            naming.suffix()
        );
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_user)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUserRoles::UserId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_role)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUserRoles::RoleId)
                .to_owned(),
        ]
    }
}
