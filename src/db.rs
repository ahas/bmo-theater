//! The theater's tables in PostgreSQL: the pool, and the migrations the binary carries, each applied once in
//! order under a lock, so two processes starting at once migrate one database once. An install from before
//! 1.0.0 at its last release is taken as it stands.

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

/// The lock every migrating process takes, so one migrates at a time.
const MIGRATION_LOCK: i64 = 0x0074_6865_6174_6572;

/// The migrations, by number, each a script of statements: the 1.0.0 baseline first (docs/design.md, 「The
/// schema」).
pub const MIGRATIONS: [(i64, &str); 1] = [(BASELINE, include_str!("baseline.sql"))];

/// The baseline's number: the schema whole.
pub const BASELINE: i64 = 100;

/// The numbers an install from before 1.0.0 holds once every migration of its last release ran. Its schema is
/// the baseline's.
pub const BEFORE: [i64; 3] = [1, 2, 3];

/// A pool over the database the address names.
pub async fn connect(url: &str) -> Result<PgPool, String> {
  if url.trim().is_empty() {
    return Err("no database is named: set [database] url".to_owned());
  }
  let options = PgConnectOptions::from_str(url).map_err(|why| format!("the database's address: {why}"))?;
  PgPoolOptions::new()
    .max_connections(8)
    .connect_with(options)
    .await
    .map_err(|why| format!("the database could not be reached: {why}"))
}

/// The database brought to the schema the binary carries.
pub async fn migrate(pool: &PgPool) -> Result<(), String> {
  let fault = |why: sqlx::Error| format!("the database could not be migrated: {why}");
  let mut connection = pool.acquire().await.map_err(fault)?;
  sqlx::query("SELECT pg_advisory_lock($1)")
    .bind(MIGRATION_LOCK)
    .execute(&mut *connection)
    .await
    .map_err(fault)?;
  let migrated = apply(&mut connection).await;
  let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
    .bind(MIGRATION_LOCK)
    .execute(&mut *connection)
    .await;
  migrated.map_err(fault)
}

/// Every migration not yet applied, applied in order, each in a transaction of its own.
async fn apply(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
  sqlx::query(
    "CREATE TABLE IF NOT EXISTS theater_migrations (number BIGINT PRIMARY KEY, applied_at BIGINT NOT NULL)",
  )
  .execute(&mut *connection)
  .await?;
  adopt(connection).await?;
  for (number, script) in MIGRATIONS {
    let applied: Option<i64> =
      sqlx::query_scalar("SELECT number FROM theater_migrations WHERE number = $1")
        .bind(number)
        .fetch_optional(&mut *connection)
        .await?;
    if applied.is_some() {
      continue;
    }
    let mut transaction = connection.begin().await?;
    sqlx::raw_sql(script).execute(&mut *transaction).await?;
    sqlx::query("INSERT INTO theater_migrations (number, applied_at) VALUES ($1, $2)")
      .bind(number)
      .bind(crate::wire::now_ms())
      .execute(&mut *transaction)
      .await?;
    transaction.commit().await?;
  }
  Ok(())
}

/// A record naming every migration of the last release before 1.0.0, and nothing else, rewritten to name the
/// baseline alone, touching no table and no row; one naming only part of them refused.
async fn adopt(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
  let held: Vec<i64> = sqlx::query_scalar("SELECT number FROM theater_migrations ORDER BY number")
    .fetch_all(&mut *connection)
    .await?;
  if held.is_empty() || !held.iter().all(|number| BEFORE.contains(number)) {
    return Ok(());
  }
  if held.len() != BEFORE.len() {
    let reached = held.last().copied().unwrap_or_default();
    return Err(sqlx::Error::Protocol(format!(
      "the database holds the migrations of a release before 1.0.0 only through number {reached}; that \
       release's last migration runs first"
    )));
  }
  let mut transaction = connection.begin().await?;
  sqlx::query("DELETE FROM theater_migrations").execute(&mut *transaction).await?;
  sqlx::query("INSERT INTO theater_migrations (number, applied_at) VALUES ($1, $2)")
    .bind(BASELINE)
    .bind(crate::wire::now_ms())
    .execute(&mut *transaction)
    .await?;
  transaction.commit().await
}
