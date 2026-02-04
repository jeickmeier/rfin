//! Migration SQL builders.

use sea_query::{
    Index, IndexCreateStatement, PostgresQueryBuilder, SchemaStatementBuilder, SqliteQueryBuilder,
};

use super::{schema, Backend};

/// Latest schema version.
pub const LATEST_VERSION: i64 = 3;

fn build_sql(backend: Backend, stmt: impl SchemaStatementBuilder) -> String {
    match backend {
        Backend::Sqlite => stmt.to_string(SqliteQueryBuilder),
        Backend::Postgres => stmt.to_string(PostgresQueryBuilder),
    }
}

fn build_index_sql(backend: Backend, stmt: IndexCreateStatement) -> String {
    match backend {
        Backend::Sqlite => stmt.to_string(SqliteQueryBuilder),
        Backend::Postgres => stmt.to_string(PostgresQueryBuilder),
    }
}

pub fn migrations_for(backend: Backend) -> Vec<(i64, Vec<String>)> {
    let mut migrations = Vec::new();

    // v1: core JSON tables
    let mut v1 = vec![
        build_sql(backend, schema::instruments_table(backend)),
        build_sql(backend, schema::portfolios_table(backend)),
        build_sql(backend, schema::market_contexts_table(backend)),
        build_sql(backend, schema::scenarios_table(backend)),
        build_sql(backend, schema::statement_models_table(backend)),
    ];

    let portfolios_as_of = Index::create()
        .name("idx_portfolios_as_of")
        .table(schema::Portfolios::Table)
        .col(schema::Portfolios::AsOf)
        .to_owned();
    v1.push(build_index_sql(backend, portfolios_as_of));

    let market_contexts_as_of = Index::create()
        .name("idx_market_contexts_as_of")
        .table(schema::MarketContexts::Table)
        .col(schema::MarketContexts::AsOf)
        .to_owned();
    v1.push(build_index_sql(backend, market_contexts_as_of));

    migrations.push((1, v1));

    // v2: metric registries
    let v2 = vec![build_sql(backend, schema::metric_registries_table(backend))];
    migrations.push((2, v2));

    // v3: time-series tables
    let v3 = vec![
        build_sql(backend, schema::series_meta_table(backend)),
        build_sql(backend, schema::series_points_table(backend)),
    ];
    migrations.push((3, v3));

    migrations
}

#[allow(dead_code)]
pub fn schema_migrations_table_sql(backend: Backend) -> String {
    build_sql(backend, schema::schema_migrations_table(backend))
}
