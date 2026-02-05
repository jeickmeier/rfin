//! Shared statement builders using sea-query.
//!
//! SQL strings are cached using `OnceLock` to avoid repeated string allocations
//! and sea-query AST construction on every database operation.

use sea_query::{
    Expr, OnConflict, Order, PostgresQueryBuilder, Query, QueryStatementBuilder, SimpleExpr,
    SqliteQueryBuilder,
};
use std::borrow::Cow;
use std::sync::OnceLock;

use super::schema::TableNaming;
use super::{schema, Backend};

// ---------------------------------------------------------------------------
// SQL Statement Cache
// ---------------------------------------------------------------------------

/// Cached SQL statements for each backend.
/// Each statement is lazily initialized on first use.
struct SqlCache {
    sqlite: OnceLock<String>,
    postgres: OnceLock<String>,
}

impl SqlCache {
    const fn new() -> Self {
        Self {
            sqlite: OnceLock::new(),
            postgres: OnceLock::new(),
        }
    }

    fn get(&self, backend: Backend, builder: impl FnOnce(Backend) -> String) -> &str {
        match backend {
            Backend::Sqlite => self.sqlite.get_or_init(|| builder(Backend::Sqlite)),
            Backend::Postgres => self.postgres.get_or_init(|| builder(Backend::Postgres)),
        }
    }
}

// Static caches for each SQL statement type
static UPSERT_MARKET_CONTEXT: SqlCache = SqlCache::new();
static UPSERT_INSTRUMENT: SqlCache = SqlCache::new();
static UPSERT_PORTFOLIO: SqlCache = SqlCache::new();
static UPSERT_SCENARIO: SqlCache = SqlCache::new();
static UPSERT_STATEMENT_MODEL: SqlCache = SqlCache::new();
static UPSERT_METRIC_REGISTRY: SqlCache = SqlCache::new();
static SELECT_MARKET_CONTEXT: SqlCache = SqlCache::new();
static SELECT_INSTRUMENT: SqlCache = SqlCache::new();
static LIST_INSTRUMENTS: SqlCache = SqlCache::new();
static SELECT_PORTFOLIO: SqlCache = SqlCache::new();
static SELECT_SCENARIO: SqlCache = SqlCache::new();
static LIST_SCENARIOS: SqlCache = SqlCache::new();
static SELECT_STATEMENT_MODEL: SqlCache = SqlCache::new();
static LIST_STATEMENT_MODELS: SqlCache = SqlCache::new();
static SELECT_METRIC_REGISTRY: SqlCache = SqlCache::new();
static LIST_METRIC_REGISTRIES: SqlCache = SqlCache::new();
static DELETE_METRIC_REGISTRY: SqlCache = SqlCache::new();
static LIST_MARKET_CONTEXTS: SqlCache = SqlCache::new();
static LATEST_MARKET_CONTEXT: SqlCache = SqlCache::new();
static LIST_PORTFOLIOS: SqlCache = SqlCache::new();
static LATEST_PORTFOLIO: SqlCache = SqlCache::new();
static UPSERT_SERIES_META: SqlCache = SqlCache::new();
static SELECT_SERIES_META: SqlCache = SqlCache::new();
static LIST_SERIES: SqlCache = SqlCache::new();
static UPSERT_SERIES_POINT: SqlCache = SqlCache::new();
static SELECT_POINTS_RANGE: SqlCache = SqlCache::new();
static LATEST_POINT: SqlCache = SqlCache::new();

// ---------------------------------------------------------------------------
// Core SQL Building Functions
// ---------------------------------------------------------------------------

fn build_sql<T: QueryStatementBuilder + sea_query::QueryStatementWriter>(
    backend: Backend,
    stmt: T,
) -> String {
    // Use to_string() to inline literal values (like LIMIT 1) while keeping
    // explicit placeholders ($1/$2 or ?1/?2) from Expr::cust().
    match backend {
        Backend::Sqlite => stmt.to_string(SqliteQueryBuilder),
        Backend::Postgres => stmt.to_string(PostgresQueryBuilder),
    }
}

fn updated_at_expr(backend: Backend) -> SimpleExpr {
    match backend {
        Backend::Sqlite => Expr::cust("strftime('%Y-%m-%dT%H:%M:%fZ','now')"),
        Backend::Postgres => Expr::cust("now()"),
    }
}

/// Helper to generate numbered parameter placeholders.
/// Uses $N for Postgres, ?N for SQLite.
struct Placeholders {
    backend: Backend,
    next: usize,
}

impl Placeholders {
    fn new(backend: Backend) -> Self {
        Self { backend, next: 1 }
    }

    fn next(&mut self) -> SimpleExpr {
        let n = self.next;
        self.next += 1;
        match self.backend {
            Backend::Sqlite => Expr::cust(format!("?{}", n)),
            Backend::Postgres => Expr::cust(format!("${}", n)),
        }
    }
}

// ---------------------------------------------------------------------------
// Public SQL Statement Accessors (Cached)
// ---------------------------------------------------------------------------

/// Returns cached SQL for upserting a market context.
pub fn upsert_market_context_sql(backend: Backend) -> &'static str {
    UPSERT_MARKET_CONTEXT.get(backend, build_upsert_market_context_sql)
}

/// Returns SQL for upserting a market context using custom table naming.
pub fn upsert_market_context_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_market_context_sql(backend))
    } else {
        Cow::Owned(build_upsert_market_context_sql_with_naming(backend, naming))
    }
}

fn build_upsert_market_context_sql(backend: Backend) -> String {
    build_upsert_market_context_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_market_context_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("market_contexts"))
        .columns([
            schema::MarketContexts::Id,
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
            schema::MarketContexts::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next(), p.next()])
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

/// Returns cached SQL for upserting an instrument.
pub fn upsert_instrument_sql(backend: Backend) -> &'static str {
    UPSERT_INSTRUMENT.get(backend, build_upsert_instrument_sql)
}

/// Returns SQL for upserting an instrument using custom table naming.
pub fn upsert_instrument_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_instrument_sql(backend))
    } else {
        Cow::Owned(build_upsert_instrument_sql_with_naming(backend, naming))
    }
}

fn build_upsert_instrument_sql(backend: Backend) -> String {
    build_upsert_instrument_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_instrument_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("instruments"))
        .columns([
            schema::Instruments::Id,
            schema::Instruments::Payload,
            schema::Instruments::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next()])
        .on_conflict(
            OnConflict::column(schema::Instruments::Id)
                .update_columns([schema::Instruments::Payload, schema::Instruments::Meta])
                .value(schema::Instruments::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for upserting a portfolio.
pub fn upsert_portfolio_sql(backend: Backend) -> &'static str {
    UPSERT_PORTFOLIO.get(backend, build_upsert_portfolio_sql)
}

/// Returns SQL for upserting a portfolio using custom table naming.
pub fn upsert_portfolio_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_portfolio_sql(backend))
    } else {
        Cow::Owned(build_upsert_portfolio_sql_with_naming(backend, naming))
    }
}

fn build_upsert_portfolio_sql(backend: Backend) -> String {
    build_upsert_portfolio_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_portfolio_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("portfolios"))
        .columns([
            schema::Portfolios::Id,
            schema::Portfolios::AsOf,
            schema::Portfolios::Payload,
            schema::Portfolios::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next(), p.next()])
        .on_conflict(
            OnConflict::columns([schema::Portfolios::Id, schema::Portfolios::AsOf])
                .update_columns([schema::Portfolios::Payload, schema::Portfolios::Meta])
                .value(schema::Portfolios::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for upserting a scenario.
pub fn upsert_scenario_sql(backend: Backend) -> &'static str {
    UPSERT_SCENARIO.get(backend, build_upsert_scenario_sql)
}

/// Returns SQL for upserting a scenario using custom table naming.
pub fn upsert_scenario_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_scenario_sql(backend))
    } else {
        Cow::Owned(build_upsert_scenario_sql_with_naming(backend, naming))
    }
}

fn build_upsert_scenario_sql(backend: Backend) -> String {
    build_upsert_scenario_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_scenario_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("scenarios"))
        .columns([
            schema::Scenarios::Id,
            schema::Scenarios::Payload,
            schema::Scenarios::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next()])
        .on_conflict(
            OnConflict::column(schema::Scenarios::Id)
                .update_columns([schema::Scenarios::Payload, schema::Scenarios::Meta])
                .value(schema::Scenarios::UpdatedAt, updated_at_expr(backend))
                .to_owned(),
        )
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for upserting a statement model.
pub fn upsert_statement_model_sql(backend: Backend) -> &'static str {
    UPSERT_STATEMENT_MODEL.get(backend, build_upsert_statement_model_sql)
}

/// Returns SQL for upserting a statement model using custom table naming.
pub fn upsert_statement_model_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_statement_model_sql(backend))
    } else {
        Cow::Owned(build_upsert_statement_model_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_upsert_statement_model_sql(backend: Backend) -> String {
    build_upsert_statement_model_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_statement_model_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("statement_models"))
        .columns([
            schema::StatementModels::Id,
            schema::StatementModels::Payload,
            schema::StatementModels::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next()])
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

/// Returns cached SQL for upserting a metric registry.
pub fn upsert_metric_registry_sql(backend: Backend) -> &'static str {
    UPSERT_METRIC_REGISTRY.get(backend, build_upsert_metric_registry_sql)
}

/// Returns SQL for upserting a metric registry using custom table naming.
pub fn upsert_metric_registry_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_metric_registry_sql(backend))
    } else {
        Cow::Owned(build_upsert_metric_registry_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_upsert_metric_registry_sql(backend: Backend) -> String {
    build_upsert_metric_registry_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_metric_registry_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("metric_registries"))
        .columns([
            schema::MetricRegistries::Namespace,
            schema::MetricRegistries::Payload,
            schema::MetricRegistries::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next()])
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

/// Returns cached SQL for selecting a market context.
pub fn select_market_context_sql(backend: Backend) -> &'static str {
    SELECT_MARKET_CONTEXT.get(backend, build_select_market_context_sql)
}

/// Returns SQL for selecting a market context using custom table naming.
pub fn select_market_context_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_market_context_sql(backend))
    } else {
        Cow::Owned(build_select_market_context_sql_with_naming(backend, naming))
    }
}

fn build_select_market_context_sql(backend: Backend) -> String {
    build_select_market_context_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_market_context_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("market_contexts"))
        .column(schema::MarketContexts::Payload)
        .and_where(Expr::col(schema::MarketContexts::Id).eq(p.next()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for selecting an instrument.
pub fn select_instrument_sql(backend: Backend) -> &'static str {
    SELECT_INSTRUMENT.get(backend, build_select_instrument_sql)
}

/// Returns SQL for selecting an instrument using custom table naming.
pub fn select_instrument_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_instrument_sql(backend))
    } else {
        Cow::Owned(build_select_instrument_sql_with_naming(backend, naming))
    }
}

fn build_select_instrument_sql(backend: Backend) -> String {
    build_select_instrument_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_instrument_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("instruments"))
        .column(schema::Instruments::Payload)
        .and_where(Expr::col(schema::Instruments::Id).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Builds SQL for selecting instruments in a batch.
///
/// Note: This function is NOT cached because the number of placeholders varies
/// per call. See `MAX_BATCH_SIZE` for chunking large batches.
#[allow(dead_code)]
pub fn select_instruments_batch_sql(backend: Backend, count: usize) -> String {
    select_instruments_batch_sql_with_naming(backend, &TableNaming::default(), count)
}

/// Builds SQL for selecting instruments in a batch with custom table naming.
pub fn select_instruments_batch_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
    count: usize,
) -> String {
    let mut p = Placeholders::new(backend);
    let placeholders: Vec<SimpleExpr> = (0..count).map(|_| p.next()).collect();
    let query = Query::select()
        .from(naming.alias("instruments"))
        .columns([schema::Instruments::Id, schema::Instruments::Payload])
        .and_where(Expr::col(schema::Instruments::Id).is_in(placeholders))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing instruments.
pub fn list_instruments_sql(backend: Backend) -> &'static str {
    LIST_INSTRUMENTS.get(backend, build_list_instruments_sql)
}

/// Returns SQL for listing instruments using custom table naming.
pub fn list_instruments_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_instruments_sql(backend))
    } else {
        Cow::Owned(build_list_instruments_sql_with_naming(backend, naming))
    }
}

fn build_list_instruments_sql(backend: Backend) -> String {
    build_list_instruments_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_instruments_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let query = Query::select()
        .from(naming.alias("instruments"))
        .column(schema::Instruments::Id)
        .order_by(schema::Instruments::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for selecting a portfolio.
pub fn select_portfolio_sql(backend: Backend) -> &'static str {
    SELECT_PORTFOLIO.get(backend, build_select_portfolio_sql)
}

/// Returns SQL for selecting a portfolio using custom table naming.
pub fn select_portfolio_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_portfolio_sql(backend))
    } else {
        Cow::Owned(build_select_portfolio_sql_with_naming(backend, naming))
    }
}

fn build_select_portfolio_sql(backend: Backend) -> String {
    build_select_portfolio_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_portfolio_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("portfolios"))
        .column(schema::Portfolios::Payload)
        .and_where(Expr::col(schema::Portfolios::Id).eq(p.next()))
        .and_where(Expr::col(schema::Portfolios::AsOf).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for selecting a scenario.
pub fn select_scenario_sql(backend: Backend) -> &'static str {
    SELECT_SCENARIO.get(backend, build_select_scenario_sql)
}

/// Returns SQL for selecting a scenario using custom table naming.
pub fn select_scenario_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_scenario_sql(backend))
    } else {
        Cow::Owned(build_select_scenario_sql_with_naming(backend, naming))
    }
}

fn build_select_scenario_sql(backend: Backend) -> String {
    build_select_scenario_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_scenario_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("scenarios"))
        .column(schema::Scenarios::Payload)
        .and_where(Expr::col(schema::Scenarios::Id).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing scenarios.
pub fn list_scenarios_sql(backend: Backend) -> &'static str {
    LIST_SCENARIOS.get(backend, build_list_scenarios_sql)
}

/// Returns SQL for listing scenarios using custom table naming.
pub fn list_scenarios_sql_with_naming(backend: Backend, naming: &TableNaming) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_scenarios_sql(backend))
    } else {
        Cow::Owned(build_list_scenarios_sql_with_naming(backend, naming))
    }
}

fn build_list_scenarios_sql(backend: Backend) -> String {
    build_list_scenarios_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_scenarios_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let query = Query::select()
        .from(naming.alias("scenarios"))
        .column(schema::Scenarios::Id)
        .order_by(schema::Scenarios::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for selecting a statement model.
pub fn select_statement_model_sql(backend: Backend) -> &'static str {
    SELECT_STATEMENT_MODEL.get(backend, build_select_statement_model_sql)
}

/// Returns SQL for selecting a statement model using custom table naming.
pub fn select_statement_model_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_statement_model_sql(backend))
    } else {
        Cow::Owned(build_select_statement_model_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_select_statement_model_sql(backend: Backend) -> String {
    build_select_statement_model_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_statement_model_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("statement_models"))
        .column(schema::StatementModels::Payload)
        .and_where(Expr::col(schema::StatementModels::Id).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing statement models.
pub fn list_statement_models_sql(backend: Backend) -> &'static str {
    LIST_STATEMENT_MODELS.get(backend, build_list_statement_models_sql)
}

/// Returns SQL for listing statement models using custom table naming.
pub fn list_statement_models_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_statement_models_sql(backend))
    } else {
        Cow::Owned(build_list_statement_models_sql_with_naming(backend, naming))
    }
}

fn build_list_statement_models_sql(backend: Backend) -> String {
    build_list_statement_models_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_statement_models_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let query = Query::select()
        .from(naming.alias("statement_models"))
        .column(schema::StatementModels::Id)
        .order_by(schema::StatementModels::Id, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for selecting a metric registry.
pub fn select_metric_registry_sql(backend: Backend) -> &'static str {
    SELECT_METRIC_REGISTRY.get(backend, build_select_metric_registry_sql)
}

/// Returns SQL for selecting a metric registry using custom table naming.
pub fn select_metric_registry_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_metric_registry_sql(backend))
    } else {
        Cow::Owned(build_select_metric_registry_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_select_metric_registry_sql(backend: Backend) -> String {
    build_select_metric_registry_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_metric_registry_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("metric_registries"))
        .column(schema::MetricRegistries::Payload)
        .and_where(Expr::col(schema::MetricRegistries::Namespace).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing metric registries.
pub fn list_metric_registries_sql(backend: Backend) -> &'static str {
    LIST_METRIC_REGISTRIES.get(backend, build_list_metric_registries_sql)
}

/// Returns SQL for listing metric registries using custom table naming.
pub fn list_metric_registries_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_metric_registries_sql(backend))
    } else {
        Cow::Owned(build_list_metric_registries_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_list_metric_registries_sql(backend: Backend) -> String {
    build_list_metric_registries_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_metric_registries_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let query = Query::select()
        .from(naming.alias("metric_registries"))
        .column(schema::MetricRegistries::Namespace)
        .order_by(schema::MetricRegistries::Namespace, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for deleting a metric registry.
pub fn delete_metric_registry_sql(backend: Backend) -> &'static str {
    DELETE_METRIC_REGISTRY.get(backend, build_delete_metric_registry_sql)
}

/// Returns SQL for deleting a metric registry using custom table naming.
pub fn delete_metric_registry_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(delete_metric_registry_sql(backend))
    } else {
        Cow::Owned(build_delete_metric_registry_sql_with_naming(
            backend, naming,
        ))
    }
}

fn build_delete_metric_registry_sql(backend: Backend) -> String {
    build_delete_metric_registry_sql_with_naming(backend, &TableNaming::default())
}

fn build_delete_metric_registry_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::delete()
        .from_table(naming.alias("metric_registries"))
        .and_where(Expr::col(schema::MetricRegistries::Namespace).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing market contexts in a date range.
pub fn list_market_contexts_sql(backend: Backend) -> &'static str {
    LIST_MARKET_CONTEXTS.get(backend, build_list_market_contexts_sql)
}

/// Returns SQL for listing market contexts in a date range using custom table naming.
pub fn list_market_contexts_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_market_contexts_sql(backend))
    } else {
        Cow::Owned(build_list_market_contexts_sql_with_naming(backend, naming))
    }
}

fn build_list_market_contexts_sql(backend: Backend) -> String {
    build_list_market_contexts_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_market_contexts_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT as_of, payload FROM market_contexts \
                WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3 \
                ORDER BY as_of ASC"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("market_contexts"))
        .columns([
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
        ])
        .and_where(Expr::col(schema::MarketContexts::Id).eq(p.next()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).gte(p.next()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).lte(p.next()))
        .order_by(schema::MarketContexts::AsOf, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for getting the latest market context on or before a date.
pub fn latest_market_context_sql(backend: Backend) -> &'static str {
    LATEST_MARKET_CONTEXT.get(backend, build_latest_market_context_sql)
}

/// Returns SQL for getting the latest market context on or before a date using custom table naming.
pub fn latest_market_context_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(latest_market_context_sql(backend))
    } else {
        Cow::Owned(build_latest_market_context_sql_with_naming(backend, naming))
    }
}

fn build_latest_market_context_sql(backend: Backend) -> String {
    build_latest_market_context_sql_with_naming(backend, &TableNaming::default())
}

fn build_latest_market_context_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT as_of, payload FROM market_contexts \
                WHERE id = ?1 AND as_of <= ?2 \
                ORDER BY as_of DESC LIMIT 1"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("market_contexts"))
        .columns([
            schema::MarketContexts::AsOf,
            schema::MarketContexts::Payload,
        ])
        .and_where(Expr::col(schema::MarketContexts::Id).eq(p.next()))
        .and_where(Expr::col(schema::MarketContexts::AsOf).lte(p.next()))
        .order_by(schema::MarketContexts::AsOf, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing portfolios in a date range.
pub fn list_portfolios_sql(backend: Backend) -> &'static str {
    LIST_PORTFOLIOS.get(backend, build_list_portfolios_sql)
}

/// Returns SQL for listing portfolios in a date range using custom table naming.
pub fn list_portfolios_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_portfolios_sql(backend))
    } else {
        Cow::Owned(build_list_portfolios_sql_with_naming(backend, naming))
    }
}

fn build_list_portfolios_sql(backend: Backend) -> String {
    build_list_portfolios_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_portfolios_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT as_of, payload FROM portfolios \
                WHERE id = ?1 AND as_of BETWEEN ?2 AND ?3 \
                ORDER BY as_of ASC"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("portfolios"))
        .columns([schema::Portfolios::AsOf, schema::Portfolios::Payload])
        .and_where(Expr::col(schema::Portfolios::Id).eq(p.next()))
        .and_where(Expr::col(schema::Portfolios::AsOf).gte(p.next()))
        .and_where(Expr::col(schema::Portfolios::AsOf).lte(p.next()))
        .order_by(schema::Portfolios::AsOf, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for getting the latest portfolio on or before a date.
pub fn latest_portfolio_sql(backend: Backend) -> &'static str {
    LATEST_PORTFOLIO.get(backend, build_latest_portfolio_sql)
}

/// Returns SQL for getting the latest portfolio on or before a date using custom table naming.
pub fn latest_portfolio_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(latest_portfolio_sql(backend))
    } else {
        Cow::Owned(build_latest_portfolio_sql_with_naming(backend, naming))
    }
}

fn build_latest_portfolio_sql(backend: Backend) -> String {
    build_latest_portfolio_sql_with_naming(backend, &TableNaming::default())
}

fn build_latest_portfolio_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT as_of, payload FROM portfolios \
                WHERE id = ?1 AND as_of <= ?2 \
                ORDER BY as_of DESC LIMIT 1"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("portfolios"))
        .columns([schema::Portfolios::AsOf, schema::Portfolios::Payload])
        .and_where(Expr::col(schema::Portfolios::Id).eq(p.next()))
        .and_where(Expr::col(schema::Portfolios::AsOf).lte(p.next()))
        .order_by(schema::Portfolios::AsOf, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for upserting series metadata.
pub fn upsert_series_meta_sql(backend: Backend) -> &'static str {
    UPSERT_SERIES_META.get(backend, build_upsert_series_meta_sql)
}

/// Returns SQL for upserting series metadata using custom table naming.
pub fn upsert_series_meta_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_series_meta_sql(backend))
    } else {
        Cow::Owned(build_upsert_series_meta_sql_with_naming(backend, naming))
    }
}

fn build_upsert_series_meta_sql(backend: Backend) -> String {
    build_upsert_series_meta_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_series_meta_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("series_meta"))
        .columns([
            schema::SeriesMeta::Namespace,
            schema::SeriesMeta::Kind,
            schema::SeriesMeta::SeriesId,
            schema::SeriesMeta::Meta,
        ])
        .values_panic([p.next(), p.next(), p.next(), p.next()])
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

/// Returns cached SQL for selecting series metadata.
pub fn select_series_meta_sql(backend: Backend) -> &'static str {
    SELECT_SERIES_META.get(backend, build_select_series_meta_sql)
}

/// Returns SQL for selecting series metadata using custom table naming.
pub fn select_series_meta_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_series_meta_sql(backend))
    } else {
        Cow::Owned(build_select_series_meta_sql_with_naming(backend, naming))
    }
}

fn build_select_series_meta_sql(backend: Backend) -> String {
    build_select_series_meta_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_series_meta_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("series_meta"))
        .column(schema::SeriesMeta::Meta)
        .and_where(Expr::col(schema::SeriesMeta::Namespace).eq(p.next()))
        .and_where(Expr::col(schema::SeriesMeta::Kind).eq(p.next()))
        .and_where(Expr::col(schema::SeriesMeta::SeriesId).eq(p.next()))
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for listing series.
pub fn list_series_sql(backend: Backend) -> &'static str {
    LIST_SERIES.get(backend, build_list_series_sql)
}

/// Returns SQL for listing series using custom table naming.
pub fn list_series_sql_with_naming(backend: Backend, naming: &TableNaming) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(list_series_sql(backend))
    } else {
        Cow::Owned(build_list_series_sql_with_naming(backend, naming))
    }
}

fn build_list_series_sql(backend: Backend) -> String {
    build_list_series_sql_with_naming(backend, &TableNaming::default())
}

fn build_list_series_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("series_meta"))
        .column(schema::SeriesMeta::SeriesId)
        .and_where(Expr::col(schema::SeriesMeta::Namespace).eq(p.next()))
        .and_where(Expr::col(schema::SeriesMeta::Kind).eq(p.next()))
        .order_by(schema::SeriesMeta::SeriesId, Order::Asc)
        .to_owned();
    build_sql(backend, query)
}

/// Returns cached SQL for upserting a series point.
pub fn upsert_series_point_sql(backend: Backend) -> &'static str {
    UPSERT_SERIES_POINT.get(backend, build_upsert_series_point_sql)
}

/// Returns SQL for upserting a series point using custom table naming.
pub fn upsert_series_point_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(upsert_series_point_sql(backend))
    } else {
        Cow::Owned(build_upsert_series_point_sql_with_naming(backend, naming))
    }
}

fn build_upsert_series_point_sql(backend: Backend) -> String {
    build_upsert_series_point_sql_with_naming(backend, &TableNaming::default())
}

fn build_upsert_series_point_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    let mut p = Placeholders::new(backend);
    let query = Query::insert()
        .into_table(naming.alias("series_points"))
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
            p.next(),
            p.next(),
            p.next(),
            p.next(),
            p.next(),
            p.next(),
            p.next(),
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

/// Returns cached SQL for selecting points in a time range.
pub fn select_points_range_sql(backend: Backend) -> &'static str {
    SELECT_POINTS_RANGE.get(backend, build_select_points_range_sql)
}

/// Returns SQL for selecting points in a time range using custom table naming.
pub fn select_points_range_sql_with_naming(
    backend: Backend,
    naming: &TableNaming,
) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(select_points_range_sql(backend))
    } else {
        Cow::Owned(build_select_points_range_sql_with_naming(backend, naming))
    }
}

fn build_select_points_range_sql(backend: Backend) -> String {
    build_select_points_range_sql_with_naming(backend, &TableNaming::default())
}

fn build_select_points_range_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT ts, value, payload, meta FROM series_points \
                WHERE namespace = ?1 AND kind = ?2 AND series_id = ?3 \
                AND ts BETWEEN ?4 AND ?5 ORDER BY ts ASC"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("series_points"))
        .columns([
            schema::SeriesPoints::Ts,
            schema::SeriesPoints::Value,
            schema::SeriesPoints::Payload,
            schema::SeriesPoints::Meta,
        ])
        .and_where(Expr::col(schema::SeriesPoints::Namespace).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::Kind).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::SeriesId).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).gte(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).lte(p.next()))
        .order_by(schema::SeriesPoints::Ts, Order::Asc)
        .to_owned();

    build_sql(backend, query)
}

/// Returns cached SQL for getting the latest point on or before a timestamp.
pub fn latest_point_sql(backend: Backend) -> &'static str {
    LATEST_POINT.get(backend, build_latest_point_sql)
}

/// Returns SQL for getting the latest point on or before a timestamp using custom table naming.
pub fn latest_point_sql_with_naming(backend: Backend, naming: &TableNaming) -> Cow<'static, str> {
    if naming.is_identity() {
        Cow::Borrowed(latest_point_sql(backend))
    } else {
        Cow::Owned(build_latest_point_sql_with_naming(backend, naming))
    }
}

fn build_latest_point_sql(backend: Backend) -> String {
    build_latest_point_sql_with_naming(backend, &TableNaming::default())
}

fn build_latest_point_sql_with_naming(backend: Backend, naming: &TableNaming) -> String {
    if matches!(backend, Backend::Sqlite) && naming.is_identity() {
        return "SELECT ts, value, payload, meta FROM series_points \
                WHERE namespace = ?1 AND kind = ?2 AND series_id = ?3 \
                AND ts <= ?4 ORDER BY ts DESC LIMIT 1"
            .to_string();
    }
    let mut p = Placeholders::new(backend);
    let query = Query::select()
        .from(naming.alias("series_points"))
        .columns([
            schema::SeriesPoints::Ts,
            schema::SeriesPoints::Value,
            schema::SeriesPoints::Payload,
            schema::SeriesPoints::Meta,
        ])
        .and_where(Expr::col(schema::SeriesPoints::Namespace).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::Kind).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::SeriesId).eq(p.next()))
        .and_where(Expr::col(schema::SeriesPoints::Ts).lte(p.next()))
        .order_by(schema::SeriesPoints::Ts, Order::Desc)
        .limit(1)
        .to_owned();
    build_sql(backend, query)
}

#[cfg(test)]
mod test_generated_sql {
    use super::*;

    #[test]
    fn postgres_latest_market_context_has_correct_placeholders() {
        let sql = latest_market_context_sql(Backend::Postgres);
        // Should have exactly $1, $2 placeholders (not $3 for LIMIT)
        assert!(sql.contains("$1"), "Should contain $1 placeholder");
        assert!(sql.contains("$2"), "Should contain $2 placeholder");
        assert!(
            !sql.contains("$3"),
            "Should NOT contain $3 placeholder for LIMIT"
        );
        assert!(
            sql.contains("LIMIT 1"),
            "LIMIT should be literal, not parameterized"
        );
    }

    #[test]
    fn sqlite_latest_market_context_has_correct_placeholders() {
        let sql = latest_market_context_sql(Backend::Sqlite);
        // SQLite uses ?1, ?2 style
        assert!(sql.contains("?1"), "Should contain ?1 placeholder");
        assert!(sql.contains("?2"), "Should contain ?2 placeholder");
        assert!(!sql.contains("?3"), "Should NOT contain ?3 placeholder");
    }

    #[test]
    fn sql_cache_returns_same_reference() {
        // First call initializes the cache
        let sql1 = upsert_instrument_sql(Backend::Sqlite);
        // Second call should return the same reference
        let sql2 = upsert_instrument_sql(Backend::Sqlite);
        // Both should point to the same string in memory
        assert!(std::ptr::eq(sql1.as_ptr(), sql2.as_ptr()));
    }

    #[test]
    fn different_backends_have_different_cached_sql() {
        let sqlite_sql = upsert_instrument_sql(Backend::Sqlite);
        let postgres_sql = upsert_instrument_sql(Backend::Postgres);
        // Different backends should have different SQL
        assert_ne!(sqlite_sql, postgres_sql);
        // SQLite uses ?N, Postgres uses $N
        assert!(sqlite_sql.contains("?1"));
        assert!(postgres_sql.contains("$1"));
    }
}
