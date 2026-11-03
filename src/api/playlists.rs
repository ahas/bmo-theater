//! The playlists and a viewer's progress through one (`docs/api.md`, 「Playlists」, 「Positions and progress」).

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Json, Response};
use sqlx::{Postgres, QueryBuilder, Row};

use super::{Answer, Shared, body, bounds, created, id_of, no_content, one_line, paged, query};
use crate::app::App;
use crate::caller::{Caller, Standing, caller};
use crate::error::ApiError;
use crate::progress::progress as progress_of;
use crate::store::{self, playlist_from, write_items};
use crate::wire::{
  Deleted, NewPlaylist, Page, Playlist, PlaylistChange, PlaylistItems, Progress, Realm, RealmQuery, now_ms,
};

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<RealmQuery>, QueryRejection>,
) -> Answer<Json<Page<Playlist>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let (after, limit) = bounds(given.after, given.limit)?;
  let mut select = QueryBuilder::<Postgres>::new("SELECT p.* FROM playlists p WHERE ");
  caller.push_visible(&mut select, "p", given.realm)?;
  select
    .push(" AND p.id > ")
    .push_bind(after)
    .push(" ORDER BY p.id LIMIT ")
    .push_bind(limit + 1);
  let rows = select.build().fetch_all(&app.pool).await?;
  let mut items = Vec::new();
  for row in &rows {
    items.push(playlist_from(&app.pool, row).await?.0);
  }
  Ok(Json(paged(items, limit)))
}

/// The episodes named checked for a playlist standing so: each one the caller reads, and one every reader of the
/// playlist reads.
async fn checked(app: &App, caller: &Caller, standing: &Standing, ids: &[i64]) -> Answer<()> {
  let found = store::episodes_named(&app.pool, ids, None).await?;
  for id in ids {
    let Some(stored) = found.iter().find(|stored| stored.episode.id == *id) else {
      return Err(ApiError::of("unknown-episode", format!("No episode {id} stands.")));
    };
    if !caller.reads(&stored.standing) {
      return Err(ApiError::of("unknown-episode", format!("No episode {id} stands.")));
    }
    let shared = match standing.realm {
      Realm::Organization => stored.standing.realm == Realm::Organization,
      Realm::Mine => {
        stored.standing.realm == Realm::Organization || stored.standing.owner == standing.owner
      }
    };
    if !shared {
      return Err(ApiError::of(
        "not-in-realm",
        format!("Episode {id} is not read by every reader of the playlist."),
      ));
    }
  }
  Ok(())
}

async fn readable(app: &App, caller: &Caller, id: &str) -> Answer<(Playlist, Standing)> {
  let id = id_of(id, "playlist")?;
  caller.read(store::playlist(&app.pool, id).await?, "playlist")
}

async fn changed(app: &App, id: i64, kind: &str) -> Answer<Playlist> {
  let (playlist, standing) = store::playlist(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::not_found("No playlist stands there."))?;
  app.publish(kind, &playlist, &standing);
  Ok(playlist)
}

pub async fn create(State(app): Shared, headers: HeaderMap, bytes: Bytes) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let new: NewPlaylist = body(&bytes)?;
  let standing = Standing {
    realm: new.realm,
    owner: caller.owner_in(new.realm)?,
    creator: caller.creator(),
  };
  let name = one_line(&new.name, "name")?;
  let episodes = new.episode_ids.unwrap_or_default();
  checked(&app, &caller, &standing, &episodes).await?;
  let mut transaction = app.pool.begin().await?;
  let now = now_ms();
  let row = sqlx::query(
    "INSERT INTO playlists (realm, owner, creator, name, created_at, changed_at) VALUES ($1, $2, $3, $4, $5, $5) \
     RETURNING id",
  )
  .bind(new.realm.word())
  .bind(&standing.owner)
  .bind(&standing.creator)
  .bind(name)
  .bind(now)
  .fetch_one(&mut *transaction)
  .await?;
  let id: i64 = row.try_get("id")?;
  write_items(&mut transaction, id, &episodes).await?;
  transaction.commit().await?;
  Ok(created(changed(&app, id, "playlist.created").await?))
}

pub async fn read(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Playlist>> {
  let caller = caller(&app.settings, &headers)?;
  Ok(Json(readable(&app, &caller, &id).await?.0))
}

pub async fn change(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Playlist>> {
  let caller = caller(&app.settings, &headers)?;
  let (playlist, standing) = readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "playlist")?;
  let change: PlaylistChange = body(&bytes)?;
  let name = match &change.name {
    Some(name) => one_line(name, "name")?,
    None => playlist.name.clone(),
  };
  if let Some(episodes) = &change.episode_ids {
    checked(&app, &caller, &standing, episodes).await?;
  }
  let mut transaction = app.pool.begin().await?;
  sqlx::query("UPDATE playlists SET name = $2, changed_at = $3 WHERE id = $1")
    .bind(playlist.id)
    .bind(name)
    .bind(now_ms())
    .execute(&mut *transaction)
    .await?;
  if let Some(episodes) = &change.episode_ids {
    write_items(&mut transaction, playlist.id, episodes).await?;
  }
  transaction.commit().await?;
  Ok(Json(changed(&app, playlist.id, "playlist.changed").await?))
}

pub async fn delete(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (playlist, standing) = readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "playlist")?;
  sqlx::query("DELETE FROM playlists WHERE id = $1")
    .bind(playlist.id)
    .execute(&app.pool)
    .await?;
  app.publish("playlist.deleted", Deleted { id: playlist.id }, &standing);
  Ok(no_content())
}

pub async fn add(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (playlist, standing) = readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "playlist")?;
  let items: PlaylistItems = body(&bytes)?;
  if items.episode_ids.is_empty() {
    return Err(ApiError::bad_request("episodeIds names no episode."));
  }
  checked(&app, &caller, &standing, &items.episode_ids).await?;
  let mut order = playlist.episode_ids.clone();
  let at = items.at.unwrap_or(order.len()).min(order.len());
  order.splice(at..at, items.episode_ids.iter().copied());
  let mut transaction = app.pool.begin().await?;
  write_items(&mut transaction, playlist.id, &order).await?;
  sqlx::query("UPDATE playlists SET changed_at = $2 WHERE id = $1")
    .bind(playlist.id)
    .bind(now_ms())
    .execute(&mut *transaction)
    .await?;
  transaction.commit().await?;
  Ok(created(changed(&app, playlist.id, "playlist.changed").await?))
}

pub async fn progress(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Progress>> {
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let (playlist, _) = readable(&app, &caller, &id).await?;
  let episodes = store::episodes_named(&app.pool, &playlist.episode_ids, Some(&account)).await?;
  let items: Vec<_> = episodes
    .into_iter()
    .map(|stored| (stored.episode.id, stored.episode.duration_ms, stored.episode.position))
    .collect();
  Ok(Json(progress_of(&items)))
}
