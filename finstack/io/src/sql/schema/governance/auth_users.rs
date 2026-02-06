//! AuthUsers table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum AuthUsers {
    #[allow(dead_code)]
    Table,
    Id,
    ExternalId,
    Name,
    Email,
    EmailVerified,
    Image,
    Status,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for AuthUsers {
    const BASE_NAME: &'static str = "auth_users";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(AuthUsers::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(AuthUsers::ExternalId).string())
            .col(ColumnDef::new(AuthUsers::Name).string())
            .col(ColumnDef::new(AuthUsers::Email).string().not_null())
            .col(
                ColumnDef::new(AuthUsers::EmailVerified)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(ColumnDef::new(AuthUsers::Image).string())
            .col(ColumnDef::new(AuthUsers::Status).string().not_null())
            .col(created_at_col(backend, AuthUsers::CreatedAt))
            .col(updated_at_col(backend, AuthUsers::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!(
            "idx_{}auth_users{}_external_id",
            naming.prefix(),
            naming.suffix()
        );
        let idx_email = format!("idx_{}auth_users{}_email", naming.prefix(), naming.suffix());
        vec![
            Index::create()
                .if_not_exists()
                .name(&idx_name)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUsers::ExternalId)
                .to_owned(),
            Index::create()
                .if_not_exists()
                .name(&idx_email)
                .table(naming.alias(Self::BASE_NAME))
                .col(AuthUsers::Email)
                .to_owned(),
        ]
    }
}
