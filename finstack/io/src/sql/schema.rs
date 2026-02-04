//! Shared schema definitions using sea-query.

use sea_query::{ColumnDef, Iden, Table, TableCreateStatement};

use super::Backend;

#[derive(Iden)]
pub enum Instruments {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Portfolios {
    Table,
    Id,
    AsOf,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum MarketContexts {
    Table,
    Id,
    AsOf,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum Scenarios {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum StatementModels {
    Table,
    Id,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum MetricRegistries {
    Table,
    Namespace,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum SeriesMeta {
    Table,
    Namespace,
    Kind,
    SeriesId,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
pub enum SeriesPoints {
    Table,
    Namespace,
    Kind,
    SeriesId,
    Ts,
    Value,
    Payload,
    Meta,
    CreatedAt,
    UpdatedAt,
}

#[allow(dead_code)]
#[derive(Iden)]
#[iden = "finstack_schema_migrations"]
pub enum SchemaMigrations {
    Table,
    Version,
    AppliedAt,
}

fn created_at_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut col = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            col.string();
        }
        Backend::Postgres => {
            col.timestamp_with_time_zone();
        }
    }
    col.not_null();
    match backend {
        Backend::Sqlite => col.default("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => col.default("now()"),
    };
    col
}

fn updated_at_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut col = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            col.string();
        }
        Backend::Postgres => {
            col.timestamp_with_time_zone();
        }
    }
    col.not_null();
    match backend {
        Backend::Sqlite => col.default("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => col.default("now()"),
    };
    col
}

fn payload_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.binary();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def.not_null();
    def
}

fn meta_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def.not_null();
    match backend {
        Backend::Sqlite => def.default("'{}'"),
        Backend::Postgres => def.default("'{}'::jsonb"),
    };
    def
}

fn as_of_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.date();
        }
    }
    def.not_null();
    def
}

fn ts_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.timestamp_with_time_zone();
        }
    }
    def.not_null();
    def
}

fn json_col<T: Iden + 'static>(backend: Backend, col: T) -> ColumnDef {
    let mut def = ColumnDef::new(col);
    match backend {
        Backend::Sqlite => {
            def.string();
        }
        Backend::Postgres => {
            def.json_binary();
        }
    }
    def
}

pub fn instruments_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(Instruments::Table)
        .col(
            ColumnDef::new(Instruments::Id)
                .string()
                .not_null()
                .primary_key(),
        )
        .col(payload_col(backend, Instruments::Payload))
        .col(meta_col(backend, Instruments::Meta))
        .col(created_at_col(backend, Instruments::CreatedAt))
        .col(updated_at_col(backend, Instruments::UpdatedAt))
        .to_owned()
}

pub fn portfolios_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(Portfolios::Table)
        .col(ColumnDef::new(Portfolios::Id).string().not_null())
        .col(as_of_col(backend, Portfolios::AsOf))
        .col(payload_col(backend, Portfolios::Payload))
        .col(meta_col(backend, Portfolios::Meta))
        .col(created_at_col(backend, Portfolios::CreatedAt))
        .col(updated_at_col(backend, Portfolios::UpdatedAt))
        .primary_key(
            sea_query::Index::create()
                .col(Portfolios::Id)
                .col(Portfolios::AsOf),
        )
        .to_owned()
}

pub fn market_contexts_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(MarketContexts::Table)
        .col(ColumnDef::new(MarketContexts::Id).string().not_null())
        .col(as_of_col(backend, MarketContexts::AsOf))
        .col(payload_col(backend, MarketContexts::Payload))
        .col(meta_col(backend, MarketContexts::Meta))
        .col(created_at_col(backend, MarketContexts::CreatedAt))
        .col(updated_at_col(backend, MarketContexts::UpdatedAt))
        .primary_key(
            sea_query::Index::create()
                .col(MarketContexts::Id)
                .col(MarketContexts::AsOf),
        )
        .to_owned()
}

pub fn scenarios_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(Scenarios::Table)
        .col(
            ColumnDef::new(Scenarios::Id)
                .string()
                .not_null()
                .primary_key(),
        )
        .col(payload_col(backend, Scenarios::Payload))
        .col(meta_col(backend, Scenarios::Meta))
        .col(created_at_col(backend, Scenarios::CreatedAt))
        .col(updated_at_col(backend, Scenarios::UpdatedAt))
        .to_owned()
}

pub fn statement_models_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(StatementModels::Table)
        .col(
            ColumnDef::new(StatementModels::Id)
                .string()
                .not_null()
                .primary_key(),
        )
        .col(payload_col(backend, StatementModels::Payload))
        .col(meta_col(backend, StatementModels::Meta))
        .col(created_at_col(backend, StatementModels::CreatedAt))
        .col(updated_at_col(backend, StatementModels::UpdatedAt))
        .to_owned()
}

pub fn metric_registries_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(MetricRegistries::Table)
        .col(
            ColumnDef::new(MetricRegistries::Namespace)
                .string()
                .not_null()
                .primary_key(),
        )
        .col(payload_col(backend, MetricRegistries::Payload))
        .col(meta_col(backend, MetricRegistries::Meta))
        .col(created_at_col(backend, MetricRegistries::CreatedAt))
        .col(updated_at_col(backend, MetricRegistries::UpdatedAt))
        .to_owned()
}

pub fn series_meta_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(SeriesMeta::Table)
        .col(ColumnDef::new(SeriesMeta::Namespace).string().not_null())
        .col(ColumnDef::new(SeriesMeta::Kind).string().not_null())
        .col(ColumnDef::new(SeriesMeta::SeriesId).string().not_null())
        .col(json_col(backend, SeriesMeta::Meta))
        .col(created_at_col(backend, SeriesMeta::CreatedAt))
        .col(updated_at_col(backend, SeriesMeta::UpdatedAt))
        .primary_key(
            sea_query::Index::create()
                .col(SeriesMeta::Namespace)
                .col(SeriesMeta::Kind)
                .col(SeriesMeta::SeriesId),
        )
        .to_owned()
}

pub fn series_points_table(backend: Backend) -> TableCreateStatement {
    Table::create()
        .if_not_exists()
        .table(SeriesPoints::Table)
        .col(ColumnDef::new(SeriesPoints::Namespace).string().not_null())
        .col(ColumnDef::new(SeriesPoints::Kind).string().not_null())
        .col(ColumnDef::new(SeriesPoints::SeriesId).string().not_null())
        .col(ts_col(backend, SeriesPoints::Ts))
        .col(ColumnDef::new(SeriesPoints::Value).double())
        .col(json_col(backend, SeriesPoints::Payload))
        .col(json_col(backend, SeriesPoints::Meta))
        .col(created_at_col(backend, SeriesPoints::CreatedAt))
        .col(updated_at_col(backend, SeriesPoints::UpdatedAt))
        .primary_key(
            sea_query::Index::create()
                .col(SeriesPoints::Namespace)
                .col(SeriesPoints::Kind)
                .col(SeriesPoints::SeriesId)
                .col(SeriesPoints::Ts),
        )
        .to_owned()
}

#[allow(dead_code)]
pub fn schema_migrations_table(backend: Backend) -> TableCreateStatement {
    let applied_at = match backend {
        Backend::Sqlite => ColumnDef::new(SchemaMigrations::AppliedAt)
            .string()
            .not_null()
            .to_owned(),
        Backend::Postgres => ColumnDef::new(SchemaMigrations::AppliedAt)
            .timestamp_with_time_zone()
            .not_null()
            .to_owned(),
    };
    Table::create()
        .if_not_exists()
        .table(SchemaMigrations::Table)
        .col(
            ColumnDef::new(SchemaMigrations::Version)
                .big_integer()
                .not_null()
                .primary_key(),
        )
        .col(applied_at)
        .to_owned()
}
