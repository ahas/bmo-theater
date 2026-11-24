//! The schema's starting point (docs/design.md, 「The schema」): a blank database gets the baseline whole, an
//! install holding the record of the last release before 1.0.0 is taken as it stands, and one holding part of
//! it is refused.

use bmo_theater::db::{BASELINE, BEFORE, migrate};
use sqlx::PgPool;

use crate::support::{blank, dropped};

const SCHEMA: &str = include_str!("../src/baseline.sql");

// Every column and index of the theater's tables, the record's aside, in one line.
async fn shape(pool: &PgPool) -> String {
  sqlx::query_scalar(
    "SELECT COALESCE((SELECT string_agg(table_name || '.' || column_name || ' ' || data_type || ' ' || \
     is_nullable || ' ' || COALESCE(column_default, ''), ', ' ORDER BY table_name, ordinal_position) \
     FROM information_schema.columns WHERE table_schema = 'public' AND table_name <> 'theater_migrations'), '') \
     || ' / ' || COALESCE((SELECT string_agg(indexdef, ', ' ORDER BY indexname) FROM pg_indexes \
     WHERE schemaname = 'public' AND tablename <> 'theater_migrations'), '')",
  )
  .fetch_one(pool)
  .await
  .unwrap()
}

async fn recorded(pool: &PgPool) -> Vec<i64> {
  sqlx::query_scalar("SELECT number FROM theater_migrations ORDER BY number")
    .fetch_all(pool)
    .await
    .unwrap()
}

// A blank database holding the baseline's schema and a library, recorded as an install from before 1.0.0 that
// ran the first `through` migrations of its last release.
async fn installed(through: usize) -> (String, PgPool) {
  let (database, pool) = blank().await;
  sqlx::raw_sql(SCHEMA).execute(&pool).await.unwrap();
  sqlx::raw_sql("CREATE TABLE theater_migrations (number BIGINT PRIMARY KEY, applied_at BIGINT NOT NULL)")
    .execute(&pool)
    .await
    .unwrap();
  for number in &BEFORE[..through] {
    sqlx::query("INSERT INTO theater_migrations (number, applied_at) VALUES ($1, 1)")
      .bind(number)
      .execute(&pool)
      .await
      .unwrap();
  }
  sqlx::raw_sql(
    "INSERT INTO libraries (realm, owner, creator, name, path, created_at) \
     VALUES ('mine', 'account-a', 'account-a', 'a-library', '/library-a', 1)",
  )
  .execute(&pool)
  .await
  .unwrap();
  (database, pool)
}

#[tokio::test]
async fn a_blank_database_gets_the_baseline_whole() {
  let (migrated_name, migrated) = blank().await;
  migrate(&migrated).await.unwrap();
  let (laid_name, laid) = blank().await;
  sqlx::raw_sql(SCHEMA).execute(&laid).await.unwrap();

  assert_eq!(shape(&migrated).await, shape(&laid).await);
  assert_eq!(recorded(&migrated).await, [BASELINE]);
  dropped(&migrated_name, migrated).await;
  dropped(&laid_name, laid).await;
}

#[tokio::test]
async fn an_install_at_the_last_release_before_the_baseline_only_has_its_record_rewritten() {
  let (name, pool) = installed(BEFORE.len()).await;
  let before = shape(&pool).await;

  migrate(&pool).await.unwrap();
  assert_eq!(recorded(&pool).await, [BASELINE]);
  assert_eq!(shape(&pool).await, before);
  let libraries: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM libraries").fetch_one(&pool).await.unwrap();
  assert_eq!(libraries, 1);

  // A second start finds the baseline recorded and changes nothing.
  migrate(&pool).await.unwrap();
  assert_eq!(recorded(&pool).await, [BASELINE]);
  assert_eq!(shape(&pool).await, before);
  dropped(&name, pool).await;
}

#[tokio::test]
async fn an_install_short_of_the_last_release_before_the_baseline_is_refused() {
  let (name, pool) = installed(2).await;

  let refused = migrate(&pool).await.unwrap_err();
  assert!(refused.contains("through number 2"), "{refused}");
  assert_eq!(recorded(&pool).await, [1, 2]);
  dropped(&name, pool).await;
}
