//! The libraries and their scans (`docs/api.md`, 「Libraries」).

use std::path::Path as FilePath;

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Json, Response};
use sqlx::{Postgres, QueryBuilder, Row};

use super::{Answer, Shared, body, bounds, created, id_of, no_content, one_line, paged, query};
use crate::app::{remove_file, remove_folder};
use crate::caller::caller;
use crate::error::ApiError;
use crate::scan::{self, root_name, walk};
use crate::store::{self, LIBRARY, library_of, scan_of};
use crate::wire::{Deleted, Library, LibraryChange, NewLibrary, Page, PageQuery, RealmQuery, Scan, now_ms};

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<RealmQuery>, QueryRejection>,
) -> Answer<Json<Page<Library>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let (after, limit) = bounds(given.after, given.limit)?;
  let mut select = QueryBuilder::<Postgres>::new(format!("{LIBRARY} WHERE "));
  caller.push_visible(&mut select, "l", given.realm)?;
  select
    .push(" AND l.id > ")
    .push_bind(after)
    .push(" ORDER BY l.id LIMIT ")
    .push_bind(limit + 1);
  let rows = select.build().fetch_all(&app.pool).await?;
  let items = rows
    .iter()
    .map(|row| library_of(row).map(|(library, _)| library))
    .collect::<Result<_, _>>()?;
  Ok(Json(paged(items, limit)))
}

/// A library's path checked: an absolute folder the service reads, outside its file root.
fn folder_of(app: &crate::app::App, path: &str) -> Answer<std::path::PathBuf> {
  let refused = |why: &str| ApiError::of("not-a-folder", format!("{path} {why}."));
  let given = FilePath::new(path);
  if !given.is_absolute() {
    return Err(refused("is not an absolute path"));
  }
  let folder = std::fs::canonicalize(given).map_err(|_| refused("is no folder the service reads"))?;
  if !folder.is_dir() || std::fs::read_dir(&folder).is_err() {
    return Err(refused("is no folder the service reads"));
  }
  let root = std::fs::canonicalize(app.root()).unwrap_or_else(|_| app.root().to_path_buf());
  if folder.starts_with(&root) || root.starts_with(&folder) {
    return Err(refused("holds or stands inside the theater's file root"));
  }
  Ok(folder)
}

pub async fn create(State(app): Shared, headers: HeaderMap, bytes: Bytes) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  caller.may_administer("registers a library")?;
  let new: NewLibrary = body(&bytes)?;
  let owner = caller.owner_in(new.realm)?;
  let folder = folder_of(&app, &new.path)?;
  let name = match new.name.as_deref() {
    Some(name) => one_line(name, "name")?,
    None => {
      let walked = folder.clone();
      let found = tokio::task::spawn_blocking(move || walk(&walked))
        .await
        .map_err(|why| ApiError::failed(format!("the walk failed: {why}")))?
        .map_err(|why| {
          ApiError::of("not-a-folder", format!("{} could not be read: {why}", folder.display()))
        })?;
      let files: Vec<String> = found.into_iter().map(|file| file.relative).collect();
      let folder_name =
        folder.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
      root_name(&files, &folder_name)
    }
  };
  let row = sqlx::query(
    "INSERT INTO libraries (realm, owner, creator, name, path, created_at) VALUES ($1, $2, $3, $4, $5, $6) \
     ON CONFLICT (realm, owner, path) DO NOTHING RETURNING id",
  )
  .bind(new.realm.word())
  .bind(&owner)
  .bind(caller.creator())
  .bind(&name)
  .bind(folder.to_string_lossy().as_ref())
  .bind(now_ms())
  .fetch_optional(&app.pool)
  .await?
  .ok_or_else(|| ApiError::conflict("The realm already holds a library at that path."))?;
  let id: i64 = row.try_get("id")?;
  scan::start(&app, id).await?;
  let (library, standing) = store::library(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::failed("the library was not kept"))?;
  app.publish("library.created", &library, &standing);
  Ok(created(library))
}

pub async fn read(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Json<Library>> {
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "library")?;
  let (library, _) = caller.read(store::library(&app.pool, id).await?, "library")?;
  Ok(Json(library))
}

pub async fn change(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Library>> {
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "library")?;
  caller.read(store::library(&app.pool, id).await?, "library")?;
  caller.may_administer("changes a library")?;
  let change: LibraryChange = body(&bytes)?;
  if let Some(name) = &change.name {
    sqlx::query("UPDATE libraries SET name = $2 WHERE id = $1")
      .bind(id)
      .bind(one_line(name, "name")?)
      .execute(&app.pool)
      .await?;
  }
  let (library, standing) = store::library(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::not_found("No library stands there."))?;
  app.publish("library.changed", &library, &standing);
  Ok(Json(library))
}

pub async fn delete(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "library")?;
  let (_, standing) = caller.read(store::library(&app.pool, id).await?, "library")?;
  caller.may_administer("deletes a library")?;
  let derived = sqlx::query(
    "SELECT e.id AS episode, e.library_id IS NULL AS filed, s.id AS subtitle FROM episodes e \
     JOIN titles t ON t.id = e.title_id LEFT JOIN subtitles s ON s.episode_id = e.id WHERE t.library_id = $1",
  )
  .bind(id)
  .fetch_all(&app.pool)
  .await?;
  sqlx::query("DELETE FROM libraries WHERE id = $1").bind(id).execute(&app.pool).await?;
  for row in &derived {
    let episode: i64 = row.try_get("episode")?;
    remove_file(&app.poster_path(episode)).await;
    if row.try_get::<bool, _>("filed")? {
      remove_folder(&app.episode_folder(episode)).await;
    }
    if let Some(subtitle) = row.try_get::<Option<i64>, _>("subtitle")? {
      remove_folder(&app.subtitle_folder(subtitle)).await;
    }
  }
  app.publish("library.deleted", Deleted { id }, &standing);
  Ok(no_content())
}

pub async fn scan(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "library")?;
  caller.read(store::library(&app.pool, id).await?, "library")?;
  caller.may_administer("scans a library")?;
  let (scan, fresh) = scan::start(&app, id).await?;
  Ok(if fresh { created(scan) } else { Json(scan).into_response() })
}

pub async fn scans(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  given: Result<Query<PageQuery>, QueryRejection>,
) -> Answer<Json<Page<Scan>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "library")?;
  caller.read(store::library(&app.pool, id).await?, "library")?;
  let (_, limit) = bounds(None, given.limit)?;
  let before = given.after.unwrap_or(i64::MAX);
  let rows = sqlx::query("SELECT * FROM scans WHERE library_id = $1 AND id < $2 ORDER BY id DESC LIMIT $3")
    .bind(id)
    .bind(before)
    .bind(limit + 1)
    .fetch_all(&app.pool)
    .await?;
  let items = rows.iter().map(scan_of).collect::<Result<_, _>>()?;
  Ok(Json(paged(items, limit)))
}
