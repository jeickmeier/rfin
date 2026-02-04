//! Shared statement builders using sea-query.

use sea_query::{
    Expr, OnConflict, Order, PostgresQueryBuilder, Query, QueryStatementBuilder, SimpleExpr,
    SqliteQueryBuilder, Value,
};

use super::{schema, Backend};

fn build_sql<T: QueryStatementBuilder>(backend: Backend, stmt: T) -> String {
    match backend {
        Backend::Sqlite => stmt.build_any(&SqliteQueryBuilder).0,
        Backend::Postgres => stmt.build_any(&PostgresQueryBuilder).0,
    }
}

fn updated_at_expr(backend: Backend) -> SimpleExpr {
    match backend {
        Backend::Sqlite => Expr::cust("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => Expr::cust("now()"),
    }
}

fn dummy_value() -> SimpleExpr {
    Expr::val(Value::Int(Some(0))).into()
}

pub fn upsert_market_context_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::MarketContexts::Table)
        .columns([
            schema::MarketContexts::Id,
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
            schema::MarketContexts::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::columns([schema::MarketContexts::Id, schema::MarketContexts::AsOf])
                .update_columns([
                    schema::MarketContexts::Payload,
                    schema::MarketContexts::Meta,
                ])
                .value(schema::MarketContexts::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_instrument_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::Instruments::Table)
        .columns([
            schema::Instruments::Id,
            schema::Instruments::Payload,
            schema::Instruments::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::column(schema::Instruments::Id)
                .update_columns([schema::Instruments::Payload, schema::Instruments::Meta])
                .value(schema::Instruments::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_portfolio_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::Portfolios::Table)
        .columns([
            schema::Portfolios::Id,
            schema::Portfolios::AsOf,
            schema::Portfolios::Payload,
            schema::Portfolios::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::columns([schema::Portfolios::Id, schema::Portfolios::AsOf])
                .update_columns([schema::Portfolios::Payload, schema::Portfolios::Meta])
                .value(schema::Portfolios::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_scenario_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::Scenarios::Table)
        .columns([
            schema::Scenarios::Id,
            schema::Scenarios::Payload,
            schema::Scenarios::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::column(schema::Scenarios::Id)
                .update_columns([schema::Scenarios::Payload, schema::Scenarios::Meta])
                .value(schema::Scenarios::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_statement_model_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::StatementModels::Table)
        .columns([
            schema::StatementModels::Id,
            schema::StatementModels::Payload,
            schema::StatementModels::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::column(schema::StatementModels::Id)
                .update_columns([
                    schema::StatementModels::Payload,
                    schema::StatementModels::Meta,
                ])
                .value(schema::StatementModels::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_metric_registry_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::MetricRegistries::Table)
        .columns([
            schema::MetricRegistries::Namespace,
            schema::MetricRegistries::Payload,
            schema::MetricRegistries::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::column(schema::MetricRegistries::Namespace)
                .update_columns([
                    schema::MetricRegistries::Payload,
                    schema::MetricRegistries::Meta,
                ])
                .value(
                    schema::MetricRegistries::UpdatedAt,
                    updated_at_expr(backend),
                )
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn select_market_context_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::MarketContexts::Table)
        .column(schema::MarketContexts::Payload)
        .and_where(Expr::col(schema::MarketContexts::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn select_instrument_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::Instruments::Table)
        .column(schema::Instruments::Payload)
        .and_where(Expr::col(schema::Instruments::Id).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn select_instruments_batch_sql(backend: Backend, count: usize) -> String {
    let placeholders: Vec<SimpleExpr> = (0..count).map(|_| dummy_value()).collect();
    let query = Query::select()
        .from(schema::Instruments::Table)
        .columns([schema::Instruments::Id, schema::Instruments::Payload])
        .and_where(Expr::col(schema::Instruments::Id).is_in(placeholders))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_instruments_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::Instruments::Table)
        .column(schema::Instruments::Id)
        .order_by(schema::Instruments::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn select_portfolio_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::Portfolios::Table)
        .column(schema::Portfolios::Payload)
        .and_where(Expr::col(schema::Portfolios::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::Portfolios::AsOf).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn select_scenario_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::Scenarios::Table)
        .column(schema::Scenarios::Payload)
        .and_where(Expr::col(schema::Scenarios::Id).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_scenarios_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::Scenarios::Table)
        .column(schema::Scenarios::Id)
        .order_by(schema::Scenarios::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn select_statement_model_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::StatementModels::Table)
        .column(schema::StatementModels::Payload)
        .and_where(Expr::col(schema::StatementModels::Id).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_statement_models_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::StatementModels::Table)
        .column(schema::StatementModels::Id)
        .order_by(schema::StatementModels::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn select_metric_registry_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::MetricRegistries::Table)
        .column(schema::MetricRegistries::Payload)
        .and_where(Expr::col(schema::MetricRegistries::Namespace).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_metric_registries_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::MetricRegistries::Table)
        .column(schema::MetricRegistries::Namespace)
        .order_by(schema::MetricRegistries::Namespace, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn delete_metric_registry_sql(backend: Backend) -> String {
    let query = Query::delete()
        .from_table(schema::MetricRegistries::Table)
        .and_where(Expr::col(schema::MetricRegistries::Namespace).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_market_contexts_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT as_of, payload FROM market_contexts \
                WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3 \
                ORDER BY as_of ASC"
            .to_string();
    }
    let query = Query::select()
        .from(schema::MarketContexts::Table)
        .columns([
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
        ])
        .and_where(Expr::col(schema::MarketContexts::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).gte(dummy_value()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).lte(dummy_value()))
        .order_by(schema::MarketContexts::AsOf, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn latest_market_context_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT as_of, payload FROM market_contexts \
                WHERE id = ?1 AND as_of <= ?2 \
                ORDER BY as_of DESC LIMIT 1"
            .to_string();
    }
    let query = Query::select()
        .from(schema::MarketContexts::Table)
        .columns([
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
        ])
        .and_where(Expr::col(schema::MarketContexts::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).lte(dummy_value()))
        .order_by(schema::MarketContexts::AsOf, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}

pub fn list_portfolios_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT as_of, payload FROM portfolios \
                WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3 \
                ORDER BY as_of ASC"
            .to_string();
    }
    let query = Query::select()
        .from(schema::Portfolios::Table)
        .columns([schema::Portfolios::AsOf, schema::Portfolios::Payload])
        .and_where(Expr::col(schema::Portfolios::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::Portfolios::AsOf).gte(dummy_value()))
        .and_where(Expr::col(schema::Portfolios::AsOf).lte(dummy_value()))
        .order_by(schema::Portfolios::AsOf, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn latest_portfolio_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT as_of, payload FROM portfolios \
                WHERE id = ?1 AND as_of <= ?2 \
                ORDER BY as_of DESC LIMIT 1"
            .to_string();
    }
    let query = Query::select()
        .from(schema::Portfolios::Table)
        .columns([schema::Portfolios::AsOf, schema::Portfolios::Payload])
        .and_where(Expr::col(schema::Portfolios::Id).eq(dummy_value()))
        .and_where(Expr::col(schema::Portfolios::AsOf).lte(dummy_value()))
        .order_by(schema::Portfolios::AsOf, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_series_meta_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::SeriesMeta::Table)
        .columns([
            schema::SeriesMeta::Namespace,
            schema::SeriesMeta::Kind,
            schema::SeriesMeta::SeriesId,
            schema::SeriesMeta::Meta,
        ])
        .values_panic([dummy_value(), dummy_value(), dummy_value(), dummy_value()])
        .on_conflict(
            OnConflict::columns([
                schema::SeriesMeta::Namespace,
                schema::SeriesMeta::Kind,
                schema::SeriesMeta::SeriesId,
            ])
            .update_columns([schema::SeriesMeta::Meta])
            .value(schema::SeriesMeta::UpdatedAt, updated_at_expr(backend))
            .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn select_series_meta_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::SeriesMeta::Table)
        .column(schema::SeriesMeta::Meta)
        .and_where(Expr::col(schema::SeriesMeta::Namespace).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesMeta::Kind).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesMeta::SeriesId).eq(dummy_value()))
        .to_owned();
    build_sql(backend, query)
}

pub fn list_series_sql(backend: Backend) -> String {
    let query = Query::select()
        .from(schema::SeriesMeta::Table)
        .column(schema::SeriesMeta::SeriesId)
        .and_where(Expr::col(schema::SeriesMeta::Namespace).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesMeta::Kind).eq(dummy_value()))
        .order_by(schema::SeriesMeta::SeriesId, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

pub fn upsert_series_point_sql(backend: Backend) -> String {
    let query = Query::insert()
        .into_table(schema::SeriesPoints::Table)
        .columns([
            schema::SeriesPoints::Namespace,
            schema::SeriesPoints::Kind,
            schema::SeriesPoints::SeriesId,
            schema::SeriesPoints::Ts,
            schema::SeriesPoints::Value,
            schema::SeriesPoints::Payload,
            schema::SeriesPoints::Meta,
        ])
        .values_panic([
            dummy_value(),
            dummy_value(),
            dummy_value(),
            dummy_value(),
            dummy_value(),
            dummy_value(),
            dummy_value(),
        ])
        .on_conflict(
            OnConflict::columns([
                schema::SeriesPoints::Namespace,
                schema::SeriesPoints::Kind,
                schema::SeriesPoints::SeriesId,
                schema::SeriesPoints::Ts,
            ])
            .update_columns([
                schema::SeriesPoints::Value,
                schema::SeriesPoints::Payload,
                schema::SeriesPoints::Meta,
            ])
            .value(schema::SeriesPoints::UpdatedAt, updated_at_expr(backend))
            .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

pub fn select_points_range_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT ts, value, payload, meta FROM series_points \
                WHERE namespace = ?1 AND kind = ?2 AND series_id = ?3 \
                AND ts BETWEEN ?4 AND ?5 ORDER BY ts ASC"
            .to_string();
    }
    let query = Query::select()
        .from(schema::SeriesPoints::Table)
        .columns([
            schema::SeriesPoints::Ts,
            schema::SeriesPoints::Value,
            schema::SeriesPoints::Payload,
            schema::SeriesPoints::Meta,
        ])
        .and_where(Expr::col(schema::SeriesPoints::Namespace).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::Kind).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::SeriesId).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).gte(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).lte(dummy_value()))
        .order_by(schema::SeriesPoints::Ts, Order::Asc)
        .to_owned();

    build_sql(backend, query)
}

pub fn latest_point_sql(backend: Backend) -> String {
    if matches!(backend, Backend::Sqlite) {
        return "SELECT ts, value, payload, meta FROM series_points \
                WHERE namespace = ?1 AND kind = ?2 AND series_id = ?3 \
                AND ts <= ?4 ORDER BY ts DESC LIMIT 1"
            .to_string();
    }
    let query = Query::select()
        .from(schema::SeriesPoints::Table)
        .columns([
            schema::SeriesPoints::Ts,
            schema::SeriesPoints::Value,
            schema::SeriesPoints::Payload,
            schema::SeriesPoints::Meta,
        ])
        .and_where(Expr::col(schema::SeriesPoints::Namespace).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::Kind).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::SeriesId).eq(dummy_value()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).lte(dummy_value()))
        .order_by(schema::SeriesPoints::Ts, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}
