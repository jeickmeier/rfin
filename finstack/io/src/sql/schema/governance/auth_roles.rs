//! AuthRoles table definition.

use sea_query::{ColumnDef, Iden, Index, IndexCreateStatement, Table, TableCreateStatement};

use crate::sql::schema::{created_at_col, updated_at_col, TableDefinition, TableNaming};
use crate::sql::Backend;

#[derive(Iden)]
pub enum AuthRoles {
    #[allow(dead_code)]
    Table,
    Id,
    Name,
    CreatedAt,
    UpdatedAt,
}

impl TableDefinition for AuthRoles {
    const BASE_NAME: &'static str = "auth_roles";

    fn migration_version() -> i64 {
        4
    }

    fn create_table_with_naming(backend: Backend, naming: &TableNaming) -> TableCreateStatement {
        Table::create()
            .if_not_exists()
            .table(naming.alias(Self::BASE_NAME))
            .col(
                ColumnDef::new(AuthRoles::Id)
                    .string()
                    .not_null()
                    .primary_key(),
            )
            .col(ColumnDef::new(AuthRoles::Name).string().not_null())
            .col(created_at_col(backend, AuthRoles::CreatedAt))
            .col(updated_at_col(backend, AuthRoles::UpdatedAt))
            .to_owned()
    }

    fn indexes_with_naming(_backend: Backend, naming: &TableNaming) -> Vec<IndexCreateStatement> {
        let idx_name = format!("idx_{}auth_roles{}_name", naming.prefix(), naming.suffix());
        vec![Index::create()
            .if_not_exists()
            .name(&idx_name)
            .table(naming.alias(Self::BASE_NAME))
            .col(AuthRoles::Name)
            .to_owned()]
    }
}
