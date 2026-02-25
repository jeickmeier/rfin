//! TursoStore struct and helper utilities.

use crate::{
    sql::schema::TableNaming,
    sql::{migrations, Backend},
    Error, Result,
};
use libsql::{Builder, Connection};
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub(crate) const SCHEMA_VERSION: i64 = migrations::LATEST_VERSION;

/// Configuration options for [`TursoStore`].
///
/// # Examples
///
/// ```rust
/// use finstack_io::{TursoConfig, TableNaming};
///
/// // Skip internal migrations (schema managed externally)
/// let config = TursoConfig::new()
///     .without_migrations();
///
/// // Custom table naming with migrations disabled
/// let config = TursoConfig::new()
///     .with_naming(TableNaming::new().with_prefix("ref_"))
///     .without_migrations();
/// ```
#[derive(Clone, Debug, Default)]
pub struct TursoConfig {
    /// Custom table naming conventions.
    pub naming: TableNaming,
    /// Whether to run built-in schema migrations on open.
    ///
    /// Defaults to `true`. Set to `false` when schema is managed externally
    /// (e.g., Liquibase, Flyway, or a custom migration framework).
    /// When disabled, you are responsible for ensuring the database schema
    /// matches the version expected by this crate.
    ///
    /// You can always run migrations manually via [`TursoStore::migrate`].
    pub auto_migrate: bool,
}

impl TursoConfig {
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

/// A Turso-backed store using async operations.
///
/// This store uses [libsql](https://docs.turso.tech/libsql), an in-process SQL
/// database engine compatible with SQLite. It provides native async operations
/// without needing a blocking runtime wrapper (unlike `tokio-rusqlite`).
///
/// # Advantages over SQLite
///
/// - **Native async I/O** — uses `io_uring` on Linux for efficient async reads/writes.
/// - **Native JSON support** — built-in JSON functions.
/// - **Optional encryption at rest** — (not yet exposed in this wrapper).
/// - **SQLite-compatible file format** — can read/write standard `.db` files,
///   so you can migrate between SQLite and Turso seamlessly.
///
/// # Cloneability
///
/// `TursoStore` implements `Clone` cheaply (via `Arc`). The underlying
/// connection is shared, not duplicated.
#[derive(Clone)]
pub struct TursoStore {
    pub(crate) path: PathBuf,
    pub(crate) conn: Arc<Connection>,
    pub(crate) naming: Arc<TableNaming>,
}

impl std::fmt::Debug for TursoStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TursoStore")
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

impl TursoStore {
    /// Open (or create) a Turso database at `path`, applying migrations.
    ///
    /// Parent directories are created automatically if they do not exist.
    /// Schema migrations run on first connect and are idempotent.
    ///
    /// # Errors
    ///
    /// Returns an error if the database file cannot be opened, parent
    /// directories cannot be created, or migrations fail.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::TursoStore;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = TursoStore::open("data/finstack.db").await?;
    /// // Store is ready — migrations ran automatically
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open(path: impl Into<PathBuf>) -> Result<Self> {
        Self::open_with_naming(path, TableNaming::default()).await
    }

    /// Open (or create) a Turso database at `path`, applying migrations with custom table naming.
    pub async fn open_with_naming(path: impl Into<PathBuf>, naming: TableNaming) -> Result<Self> {
        Self::open_with_config(path, TursoConfig::new().with_naming(naming)).await
    }

    /// Open (or create) a Turso database at `path` with full configuration.
    ///
    /// Use [`TursoConfig::without_migrations`] to skip the built-in schema
    /// migrations when your schema is managed by an external tool.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::{TursoStore, TursoConfig};
    /// # async fn example() -> finstack_io::Result<()> {
    /// // Schema managed by Liquibase — skip internal migrations
    /// let config = TursoConfig::new().without_migrations();
    /// let store = TursoStore::open_with_config("data/finstack.db", config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_with_config(path: impl Into<PathBuf>, config: TursoConfig) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let path_str = path.to_string_lossy().into_owned();
        let db = Builder::new_local(&path_str).build().await?;
        let conn = db.connect().map_err(Error::from)?;

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

    /// Open an in-memory Turso database (useful for testing).
    ///
    /// The database exists only for the lifetime of the returned handle.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use finstack_io::TursoStore;
    /// # async fn example() -> finstack_io::Result<()> {
    /// let store = TursoStore::open_in_memory().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_in_memory_with_naming(TableNaming::default()).await
    }

    /// Open an in-memory Turso database (useful for testing) with custom table naming.
    pub async fn open_in_memory_with_naming(naming: TableNaming) -> Result<Self> {
        Self::open_in_memory_with_config(TursoConfig::new().with_naming(naming)).await
    }

    /// Open an in-memory Turso database with full configuration.
    pub async fn open_in_memory_with_config(config: TursoConfig) -> Result<Self> {
        let db = Builder::new_local(":memory:").build().await?;
        let conn = db.connect().map_err(Error::from)?;

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

    /// Get a connection from the database.
    pub(crate) fn get_conn(&self) -> Result<Arc<Connection>> {
        Ok(Arc::clone(&self.conn))
    }

    /// Run schema migrations manually.
    ///
    /// This is called automatically on open unless [`TursoConfig::without_migrations`]
    /// is used. Safe to call multiple times — already-applied versions are skipped.
    pub async fn migrate(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Get current schema version using PRAGMA user_version
        let stmt = conn.prepare("PRAGMA user_version").await?;
        let mut rows = stmt.query(()).await?;
        let current: i64 = match rows.next().await? {
            Some(row) => row.get::<i64>(0).map_err(Error::from)?,
            None => 0i64,
        };
        drop(rows);
        drop(stmt);

        if current > SCHEMA_VERSION {
            return Err(Error::UnsupportedSchema {
                found: current,
                expected: SCHEMA_VERSION,
            });
        }

        if current == SCHEMA_VERSION {
            return Ok(());
        }

        let migrations = migrations::migrations_for_with_naming(Backend::Sqlite, self.naming());

        let tx = conn.transaction().await?;

        for (version, statements) in migrations {
            if version <= current {
                continue;
            }
            for sql in statements {
                tx.execute(&sql, ()).await?;
            }
        }

        tx.execute(&format!("PRAGMA user_version = {SCHEMA_VERSION}"), ())
            .await?;
        tx.commit().await?;

        Ok(())
    }
}

// Re-export common helpers with backend-specific names for compatibility
pub(crate) use crate::helpers::format_date_key as as_of_key;
pub(crate) use crate::helpers::format_timestamp_key as ts_key;
pub(crate) use crate::helpers::meta_json_optional_string as meta_json_str;
pub(crate) use crate::helpers::meta_json_string as meta_json;

/// Helper to get a string from a libsql row.
pub(crate) fn get_string(row: &libsql::Row, idx: i32) -> Result<String> {
    row.get::<String>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get string at column {idx}: {e}")))
}

/// Helper to get a blob from a libsql row.
pub(crate) fn get_blob(row: &libsql::Row, idx: i32) -> Result<Vec<u8>> {
    row.get::<Vec<u8>>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get blob at column {idx}: {e}")))
}

/// Helper to get an optional f64 from a libsql row.
pub(crate) fn get_optional_f64(row: &libsql::Row, idx: i32) -> Result<Option<f64>> {
    row.get::<Option<f64>>(idx)
        .map_err(|e| Error::Invariant(format!("Failed to get optional f64 at column {idx}: {e}")))
}

/// Helper to get an optional string from a libsql row.
pub(crate) fn get_optional_string(row: &libsql::Row, idx: i32) -> Result<Option<String>> {
    row.get::<Option<String>>(idx).map_err(|e| {
        Error::Invariant(format!(
            "Failed to get optional string at column {idx}: {e}"
        ))
    })
}
