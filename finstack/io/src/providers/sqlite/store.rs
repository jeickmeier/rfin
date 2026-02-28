//! SqliteStore struct and helper utilities.

use crate::{
    sql::schema::TableNaming,
    sql::{migrations, Backend},
    Error, Result,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio_rusqlite::Connection;

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// Configuration options for [`SqliteStore`].
///
/// # Examples
///
/// ```rust
/// use finstack_io::{SqliteConfig, TableNaming};
///
/// // Skip internal migrations (schema managed externally)
/// let config = SqliteConfig::new()
///     .without_migrations();
///
/// // Custom table naming with migrations disabled
/// let config = SqliteConfig::new()
///     .with_naming(TableNaming::new().with_prefix("ref_"))
///     .without_migrations();
/// ```
#[derive(Clone, Debug, Default)]
pub struct SqliteConfig {
    /// Custom table naming conventions.
    pub naming: TableNaming,
    /// Whether to run built-in schema migrations on open.
    ///
    /// Defaults to `true`. Set to `false` when schema is managed externally
    /// (e.g., Liquibase, Flyway, or a custom migration framework).
    /// When disabled, you are responsible for ensuring the database schema
    /// matches the version expected by this crate.
    ///
    /// You can always run migrations manually via [`SqliteStore::migrate`].
    pub auto_migrate: bool,
}

impl SqliteConfig {
    /// Creates a new configuration with default values (auto-migrate enabled).
    pub fn new() -> Self {
        Self {
            naming: TableNaming::default(),
            auto_migrate: true,
        }
    }

    /// Set custom table naming conventions.
    pub fn with_naming(mut self, naming: TableNaming) -> Self {
        self.naming = naming;
        self
    }

    /// Disable automatic schema migrations on open.
    ///
    /// When disabled, the store assumes the database schema already exists
    /// and matches the version expected by this crate. Use this when schema
    /// is managed by an external migration framework (Liquibase, Flyway, etc.).
    pub fn without_migrations(mut self) -> Self {
        self.auto_migrate = false;
        self
    }
}

/// A SQLite-backed store using async operations.
///
/// This store wraps a single `tokio-rusqlite` connection which is thread-safe
/// and provides async access via the `call()` method. The underlying connection
/// is managed on a dedicated thread, with synchronous rusqlite operations
/// dispatched to that thread.
///
/// For concurrent access, the connection serializes requests internally.
///
/// # Configuration
///
/// The following SQLite pragmas are applied on connection:
/// - **`journal_mode = WAL`** — Write-ahead logging for better read concurrency.
/// - **`foreign_keys = ON`** — Enforces foreign key constraints.
/// - **`synchronous = NORMAL`** — Balances durability and performance.
/// - **`busy_timeout = 5000`** — Waits up to 5 seconds on lock contention.
///
/// # Cloneability
///
/// `SqliteStore` implements `Clone` cheaply (via `Arc`). The underlying
/// connection is shared, not duplicated. Clone freely for use across tasks.
#[derive(Clone)]
pub struct SqliteStore {
    pub(crate) path: PathBuf,
    pub(crate) conn: Arc<Connection>,
    pub(crate) naming: Arc<TableNaming>,
}

impl std::fmt::Debug for SqliteStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SqliteStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl SqliteStore {
    /// Open (or create) a SQLite database at `path`, applying migrations.
    ///
    /// Parent directories are created automatically if they do not exist.
    /// Schema migrations run on first connect and are idempotent.
    ///
    /// # Errors
    ///
    /// Returns an error if the database file cannot be opened, parent
    /// directories cannot be created, or migrations fail (e.g., schema
    /// version is newer than this crate supports).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::SqliteStore;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open("data/finstack.db").await?;
    /// // Store is ready — migrations ran automatically
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self> {
        Self::open_with_naming(path, TableNaming::default()).await
    }

    /// Open (or create) a SQLite database at `path`, applying migrations with custom table naming.
    pub async fn open_with_naming(path: impl Into<PathBuf>, naming: TableNaming) -> Result<Self> {
        Self::open_with_config(path, SqliteConfig::new().with_naming(naming)).await
    }

    /// Open (or create) a SQLite database at `path` with full configuration.
    ///
    /// Use [`SqliteConfig::without_migrations`] to skip the built-in schema
    /// migrations when your schema is managed by an external tool.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{SqliteStore, SqliteConfig};
    /// # async fn example() -> finstack_io::Result<()> {
    /// // Schema managed by Liquibase — skip internal migrations
    /// let config = SqliteConfig::new().without_migrations();
    /// let store = SqliteStore::open_with_config("data/finstack.db", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_with_config(path: impl Into<PathBuf>, config: SqliteConfig) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path).await?;

        conn.call(|conn| -> std::result::Result<(), rusqlite::Error> {
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch(
                "PRAGMA foreign_keys = ON;\
                 PRAGMA journal_mode = WAL;\
                 PRAGMA synchronous = NORMAL;",
            )?;
            Ok(())
        })
        .await?;

        let store = Self {
            path,
            conn: Arc::new(conn),
            naming: Arc::new(config.naming),
        };

        if config.auto_migrate {
            store.migrate().await?;
        }

        Ok(store)
    }

    /// Open an in-memory SQLite database (useful for testing).
    ///
    /// The database exists only for the lifetime of the returned handle.
    /// Migrations run automatically.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::SqliteStore;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = SqliteStore::open_in_memory().await?;
    /// // Use in tests — data is discarded when `store` is dropped
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_naming(TableNaming::default()).await
    }

    /// Open an in-memory SQLite database (useful for testing) with custom table naming.
    pub async fn open_in_memory_with_naming(naming: TableNaming) -> Result<Self> {
        Self::open_in_memory_with_config(SqliteConfig::new().with_naming(naming)).await
    }

    /// Open an in-memory SQLite database with full configuration.
    pub async fn open_in_memory_with_config(config: SqliteConfig) -> Result<Self> {
        let conn = Connection::open_in_memory().await?;

        conn.call(|conn| -> std::result::Result<(), rusqlite::Error> {
            conn.busy_timeout(Duration::from_secs(5))?;
            conn.execute_batch("PRAGMA foreign_keys = ON;")?;
            Ok(())
        })
        .await?;

        let store = Self {
            path: PathBuf::from(":memory:"),
            conn: Arc::new(conn),
            naming: Arc::new(config.naming),
        };

        if config.auto_migrate {
            store.migrate().await?;
        }

        Ok(store)
    }

    /// Database path used by this store.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Table naming used by this store.
    pub fn naming(&self) -> &TableNaming {
        self.naming.as_ref()
    }

    /// Run schema migrations manually.
    ///
    /// This is called automatically on open unless [`SqliteConfig::without_migrations`]
    /// is used. Safe to call multiple times — already-applied versions are skipped.
    pub async fn migrate(&self) -> Result<()> {
        let schema_version = SCHEMA_VERSION;
        let current: i64 = self
            .conn
            .call(|conn| -> std::result::Result<i64, rusqlite::Error> {
                conn.pragma_query_value(None, "user_version", |row| row.get(0))
            })
            .await?;

        if current > schema_version {
            return Err(Error::UnsupportedSchema {
                found: current,
                expected: schema_version,
            });
        }

        if current == schema_version {
            return Ok(());
        }

        let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, self.naming());
        self.conn
            .call(move |conn| -> std::result::Result<(), rusqlite::Error> {
                let tx = conn.unchecked_transaction()?;
                for (version, statements) in migrations {
                    if version <= current {
                        continue;
                    }
                    for sql in statements {
                        tx.execute_batch(&sql)?;
                    }
                }
                tx.execute_batch(&format!("PRAGMA user_version = {schema_version}"))?;
                tx.commit()?;
                Ok(())
            })
            .await?;
        Ok(())
    }
}

// Re-export common helpers with backend-specific names for compatibility
pub(crate) use crate::helpers::format_date_key as as_of_key;
pub(crate) use crate::helpers::format_timestamp_key as ts_key;
pub(crate) use crate::helpers::meta_json_string as meta_json;

/// Helper to convert rusqlite optional query result
pub(crate) fn optional_row<T>(result: rusqlite::Result<T>) -> rusqlite::Result<Option<T>> {
    match result {
        Ok(v) => Ok(Some(v)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}
