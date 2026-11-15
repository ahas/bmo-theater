//! The theater's tables in PostgreSQL: the pool, and the migrations the binary carries, each applied once in
//! order under a lock, so two processes starting at once migrate one database once.

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{Connection, PgConnection, PgPool};

/// The lock every migrating process takes, so one migrates at a time.
const MIGRATION_LOCK: i64 = 0x0074_6865_6174_6572;

/// The migrations, by number, each a script of statements.
pub const MIGRATIONS: [(i64, &str); 3] = [
  (
    1,
    r#"
CREATE TABLE libraries (
  id BIGSERIAL PRIMARY KEY,
  realm TEXT NOT NULL,
  owner TEXT NOT NULL,
  creator TEXT NOT NULL,
  name TEXT NOT NULL,
  path TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  scanned_at BIGINT,
  UNIQUE (realm, owner, path)
);

CREATE TABLE scans (
  id BIGSERIAL PRIMARY KEY,
  library_id BIGINT NOT NULL REFERENCES libraries (id) ON DELETE CASCADE,
  state TEXT NOT NULL,
  found_count BIGINT NOT NULL DEFAULT 0,
  added_count BIGINT NOT NULL DEFAULT 0,
  changed_count BIGINT NOT NULL DEFAULT 0,
  missing_count BIGINT NOT NULL DEFAULT 0,
  error TEXT,
  started_at BIGINT NOT NULL,
  ended_at BIGINT
);
CREATE INDEX scans_library ON scans (library_id, id);
CREATE UNIQUE INDEX scans_running ON scans (library_id) WHERE state = 'running';

CREATE TABLE titles (
  id BIGSERIAL PRIMARY KEY,
  realm TEXT NOT NULL,
  owner TEXT NOT NULL,
  creator TEXT NOT NULL,
  name TEXT NOT NULL,
  kind TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  labels TEXT NOT NULL DEFAULT '{}',
  library_id BIGINT REFERENCES libraries (id) ON DELETE CASCADE,
  library_key TEXT,
  retired BOOLEAN NOT NULL DEFAULT FALSE,
  created_at BIGINT NOT NULL,
  changed_at BIGINT NOT NULL,
  UNIQUE (library_id, library_key)
);

CREATE TABLE episodes (
  id BIGSERIAL PRIMARY KEY,
  title_id BIGINT NOT NULL REFERENCES titles (id) ON DELETE CASCADE,
  kind TEXT NOT NULL,
  season INTEGER,
  number INTEGER,
  name TEXT NOT NULL,
  label TEXT NOT NULL DEFAULT '',
  path TEXT NOT NULL,
  library_id BIGINT REFERENCES libraries (id) ON DELETE CASCADE,
  library_key TEXT,
  file_name TEXT NOT NULL,
  media_type TEXT NOT NULL,
  byte_count BIGINT NOT NULL,
  modified_at BIGINT NOT NULL,
  duration_ms BIGINT,
  width INTEGER,
  height INTEGER,
  video_codec TEXT,
  audio_codec TEXT,
  available BOOLEAN NOT NULL DEFAULT TRUE,
  has_poster BOOLEAN NOT NULL DEFAULT FALSE,
  burned_episode BIGINT,
  burned_subtitle BIGINT,
  creator TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  changed_at BIGINT NOT NULL,
  UNIQUE (library_id, library_key)
);
CREATE INDEX episodes_title ON episodes (title_id);

CREATE TABLE subtitles (
  id BIGSERIAL PRIMARY KEY,
  episode_id BIGINT NOT NULL REFERENCES episodes (id) ON DELETE CASCADE,
  language TEXT NOT NULL,
  label TEXT NOT NULL DEFAULT '',
  format TEXT NOT NULL,
  file_name TEXT NOT NULL,
  cue_count BIGINT NOT NULL,
  creator TEXT NOT NULL,
  created_at BIGINT NOT NULL
);
CREATE INDEX subtitles_episode ON subtitles (episode_id);

CREATE TABLE playlists (
  id BIGSERIAL PRIMARY KEY,
  realm TEXT NOT NULL,
  owner TEXT NOT NULL,
  creator TEXT NOT NULL,
  name TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  changed_at BIGINT NOT NULL
);

CREATE TABLE playlist_items (
  playlist_id BIGINT NOT NULL REFERENCES playlists (id) ON DELETE CASCADE,
  ord INTEGER NOT NULL,
  episode_id BIGINT NOT NULL REFERENCES episodes (id) ON DELETE CASCADE,
  PRIMARY KEY (playlist_id, ord)
);
CREATE INDEX playlist_items_episode ON playlist_items (episode_id);

CREATE TABLE positions (
  account TEXT NOT NULL,
  episode_id BIGINT NOT NULL REFERENCES episodes (id) ON DELETE CASCADE,
  seconds DOUBLE PRECISION NOT NULL,
  done BOOLEAN NOT NULL,
  updated_at BIGINT NOT NULL,
  PRIMARY KEY (account, episode_id)
);

CREATE TABLE burns (
  id BIGSERIAL PRIMARY KEY,
  episode_id BIGINT NOT NULL REFERENCES episodes (id) ON DELETE CASCADE,
  subtitle_id BIGINT NOT NULL,
  state TEXT NOT NULL,
  clip_id BIGINT,
  error TEXT,
  creator TEXT NOT NULL,
  started_at BIGINT NOT NULL,
  ended_at BIGINT
);
CREATE INDEX burns_episode ON burns (episode_id, id);

CREATE TABLE shares (
  token TEXT PRIMARY KEY,
  episode_id BIGINT REFERENCES episodes (id) ON DELETE CASCADE,
  playlist_id BIGINT REFERENCES playlists (id) ON DELETE CASCADE,
  creator TEXT NOT NULL,
  created_at BIGINT NOT NULL,
  expires_at BIGINT,
  CHECK ((episode_id IS NULL) <> (playlist_id IS NULL))
);
CREATE UNIQUE INDEX shares_episode ON shares (episode_id) WHERE episode_id IS NOT NULL;
CREATE UNIQUE INDEX shares_playlist ON shares (playlist_id) WHERE playlist_id IS NOT NULL;
"#,
  ),
  (
    2,
    r#"
CREATE TABLE grants (
  episode_id BIGINT NOT NULL REFERENCES episodes (id) ON DELETE CASCADE,
  account TEXT NOT NULL,
  permission TEXT NOT NULL CHECK (permission IN ('read', 'edit')),
  grantor TEXT NOT NULL,
  granted_at BIGINT NOT NULL,
  PRIMARY KEY (episode_id, account)
);
"#,
  ),
  (
    3,
    r#"
ALTER TABLE episodes ADD COLUMN poster_width INTEGER, ADD COLUMN poster_height INTEGER;
ALTER TABLE positions ADD COLUMN set_aside BOOLEAN NOT NULL DEFAULT FALSE;
"#,
  ),
];

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
