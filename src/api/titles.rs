//! The titles (`docs/api.md`, 「Titles」).

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Json, Response};
use sqlx::{Postgres, QueryBuilder, Row};

use super::{Answer, Shared, body, bounds, created, id_of, no_content, one_line, paged, query};
use crate::app::{App, remove_file, remove_folder};
use crate::caller::{Caller, Standing, caller};
use crate::error::ApiError;
use crate::progress::{on_main_line, title_progress};
use crate::range;
use crate::store::{self, TITLE, title_of};
use crate::wire::{
  Deleted, Episode, NewTitle, Page, Progress, Title, TitleChange, TitleKind, TitleOrder, TitlesQuery,
  now_ms,
};

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<TitlesQuery>, QueryRejection>,
) -> Answer<Json<Page<Title>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let (after, limit) = bounds(given.after, given.limit)?;
  let order = match (given.order.unwrap_or_default(), &caller.account) {
    (TitleOrder::Watched, None) => TitleOrder::Added,
    (order, _) => order,
  };
  let mut select = QueryBuilder::<Postgres>::new(match order {
    TitleOrder::Id => format!("{TITLE} AND "),
    _ => format!("WITH listed AS ({TITLE} AND "),
  });
  caller.push_visible(&mut select, "t", given.realm)?;
  if let Some(kind) = given.kind {
    select.push(" AND t.kind = ").push_bind(kind.word());
  }
  if let Some(library) = given.library_id {
    select.push(" AND t.library_id = ").push_bind(library);
  }
  if let Some(part) = given.q.as_deref().map(str::trim).filter(|part| !part.is_empty()) {
    let escaped = part.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
    select.push(" AND t.name ILIKE ").push_bind(format!("%{escaped}%"));
  }
  if order == TitleOrder::Id {
    select
      .push(" AND t.id > ")
      .push_bind(after)
      .push(" ORDER BY t.id LIMIT ")
      .push_bind(limit + 1);
  } else {
    ordered(&mut select, order, caller.account.as_deref(), given.after, limit);
  }
  let rows = select.build().fetch_all(&app.pool).await?;
  let items = rows.iter().map(|row| title_of(row).map(|(title, _)| title)).collect::<Result<_, _>>()?;
  Ok(Json(paged(items, limit)))
}

/// The rest of a list in an order other than by id, `select` holding the titles listed as `listed` so far: each
/// is given the key it is ordered by, and a page goes on after the key of the title `after` names, read from the
/// same titles; an `after` they do not hold ends the list (`docs/api.md`, 「Titles」).
fn ordered(
  select: &mut QueryBuilder<'_, Postgres>,
  order: TitleOrder,
  account: Option<&str>,
  after: Option<i64>,
  limit: i64,
) {
  select.push("), keyed AS (SELECT l.*, ");
  match order {
    TitleOrder::Name => {
      // A name's bytes in UTF-8 stand in the order of its characters' code points.
      select.push("convert_to(lower(l.name), 'UTF8') AS order_key");
    }
    TitleOrder::Watched => {
      // A title never watched keys below every moment, so it follows, by `addedAt`.
      select
        .push(
          "COALESCE((SELECT MAX(p.updated_at) FROM positions p JOIN episodes e ON e.id = p.episode_id \
           WHERE e.title_id = l.id AND p.account = ",
        )
        .push_bind(account.unwrap_or_default().to_owned())
        .push("), -1) AS order_key");
    }
    TitleOrder::Id | TitleOrder::Added => {
      select.push("l.added_at AS order_key");
    }
  }
  select.push(" FROM listed l) SELECT * FROM keyed k");
  let (columns, direction, compared): (&[&str], _, _) = match order {
    TitleOrder::Name => (&["order_key", "id"], "ASC", ">"),
    TitleOrder::Watched => (&["order_key", "added_at", "id"], "DESC", "<"),
    TitleOrder::Id | TitleOrder::Added => (&["order_key", "id"], "DESC", "<"),
  };
  let of = |alias: &str, suffix: &str| {
    columns
      .iter()
      .map(|column| format!("{alias}.{column}{suffix}"))
      .collect::<Vec<_>>()
      .join(", ")
  };
  if let Some(after) = after {
    select
      .push(format!(
        " WHERE ({}) {compared} (SELECT {} FROM keyed a WHERE a.id = ",
        of("k", ""),
        of("a", "")
      ))
      .push_bind(after)
      .push(")");
  }
  select
    .push(format!(" ORDER BY {} LIMIT ", of("k", &format!(" {direction}"))))
    .push_bind(limit + 1);
}

/// A title the caller reads, with where it stands.
pub async fn readable(app: &App, caller: &Caller, id: &str) -> Answer<(Title, Standing)> {
  let id = id_of(id, "title")?;
  caller.read(store::title(&app.pool, id).await?, "title")
}

/// A title made: its row, and its event.
pub async fn make(app: &App, caller: &Caller, new: NewTitle) -> Answer<Title> {
  let owner = caller.owner_in(new.realm)?;
  let name = one_line(&new.name, "name")?;
  let labels = serde_json::to_string(&new.labels.unwrap_or_default()).unwrap_or_else(|_| "{}".to_owned());
  let now = now_ms();
  let row = sqlx::query(
    "INSERT INTO titles (realm, owner, creator, name, kind, description, labels, created_at, changed_at) \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8) RETURNING id",
  )
  .bind(new.realm.word())
  .bind(owner)
  .bind(caller.creator())
  .bind(name)
  .bind(new.kind.unwrap_or(TitleKind::Clip).word())
  .bind(new.description.unwrap_or_default().trim())
  .bind(labels)
  .bind(now)
  .fetch_one(&app.pool)
  .await?;
  let id: i64 = row.try_get("id")?;
  let (title, standing) = store::title(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::failed("the title was not kept"))?;
  app.publish("title.created", &title, &standing);
  Ok(title)
}

pub async fn create(State(app): Shared, headers: HeaderMap, bytes: Bytes) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let new: NewTitle = body(&bytes)?;
  Ok(created(make(&app, &caller, new).await?))
}

pub async fn read(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Json<Title>> {
  let caller = caller(&app.settings, &headers)?;
  Ok(Json(readable(&app, &caller, &id).await?.0))
}

pub async fn change(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Title>> {
  let caller = caller(&app.settings, &headers)?;
  let (title, standing) = readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "title")?;
  let change: TitleChange = body(&bytes)?;
  let name = match &change.name {
    Some(name) => one_line(name, "name")?,
    None => title.name.clone(),
  };
  let labels =
    serde_json::to_string(&change.labels.unwrap_or(title.labels)).unwrap_or_else(|_| "{}".to_owned());
  sqlx::query(
    "UPDATE titles SET name = $2, kind = $3, description = $4, labels = $5, changed_at = $6 WHERE id = $1",
  )
  .bind(title.id)
  .bind(name)
  .bind(change.kind.unwrap_or(title.kind).word())
  .bind(change.description.map(|text| text.trim().to_owned()).unwrap_or(title.description))
  .bind(labels)
  .bind(now_ms())
  .execute(&app.pool)
  .await?;
  let (title, standing) = store::title(&app.pool, title.id)
    .await?
    .ok_or_else(|| ApiError::not_found("No title stands there."))?;
  app.publish("title.changed", &title, &standing);
  Ok(Json(title))
}

pub async fn delete(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (title, standing) = readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "title")?;
  let derived = sqlx::query(
    "SELECT e.id AS episode, e.library_id IS NULL AS filed, s.id AS subtitle FROM episodes e \
     LEFT JOIN subtitles s ON s.episode_id = e.id WHERE e.title_id = $1",
  )
  .bind(title.id)
  .fetch_all(&app.pool)
  .await?;
  if title.library_id.is_some() {
    let mut transaction = app.pool.begin().await?;
    sqlx::query("UPDATE titles SET retired = TRUE, changed_at = $2 WHERE id = $1")
      .bind(title.id)
      .bind(now_ms())
      .execute(&mut *transaction)
      .await?;
    // A retired title keeps its library's rows, so a scan leaves it retired; what was filed under it goes.
    sqlx::query("DELETE FROM episodes WHERE title_id = $1 AND library_id IS NULL")
      .bind(title.id)
      .execute(&mut *transaction)
      .await?;
    sqlx::query("DELETE FROM shares WHERE episode_id IN (SELECT id FROM episodes WHERE title_id = $1)")
      .bind(title.id)
      .execute(&mut *transaction)
      .await?;
    sqlx::query(
      "DELETE FROM playlist_items WHERE episode_id IN (SELECT id FROM episodes WHERE title_id = $1)",
    )
    .bind(title.id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
  } else {
    sqlx::query("DELETE FROM titles WHERE id = $1")
      .bind(title.id)
      .execute(&app.pool)
      .await?;
  }
  for row in &derived {
    let episode: i64 = row.try_get("episode")?;
    if row.try_get::<bool, _>("filed")? {
      remove_folder(&app.episode_folder(episode)).await;
      remove_file(&app.poster_path(episode)).await;
      if let Some(subtitle) = row.try_get::<Option<i64>, _>("subtitle")? {
        remove_folder(&app.subtitle_folder(subtitle)).await;
      }
    }
  }
  app.publish("title.deleted", Deleted { id: title.id }, &standing);
  Ok(no_content())
}

pub async fn poster(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (title, _) = readable(&app, &caller, &id).await?;
  let first = store::title_episodes(&app.pool, title.id, None)
    .await?
    .into_iter()
    .find(|stored| stored.episode.has_poster)
    .ok_or_else(|| ApiError::not_found("No episode of the title has a poster."))?;
  poster_of(&app, first.episode.id).await
}

/// An episode's poster frame answered, or `not-found`.
pub async fn poster_of(app: &App, episode: i64) -> Answer<Response> {
  let bytes = tokio::fs::read(app.poster_path(episode))
    .await
    .map_err(|_| ApiError::not_found("The episode has no poster."))?;
  Ok(range::bytes("image/jpeg", bytes))
}

pub async fn episodes(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Page<Episode>>> {
  let caller = caller(&app.settings, &headers)?;
  let (title, _) = readable(&app, &caller, &id).await?;
  let items = store::title_episodes(&app.pool, title.id, caller.account.as_deref()).await?;
  Ok(Json(Page {
    items: items.into_iter().map(|stored| stored.episode).collect(),
    more: false,
  }))
}

pub async fn progress(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Progress>> {
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let (title, _) = readable(&app, &caller, &id).await?;
  let stored = store::title_episodes(&app.pool, title.id, Some(&account)).await?;
  let main: Vec<bool> = stored
    .iter()
    .map(|stored| on_main_line(title.kind, stored.episode.season, stored.episode.number))
    .collect();
  let items: Vec<_> = stored
    .into_iter()
    .map(|stored| (stored.episode.id, stored.episode.duration_ms, stored.episode.position))
    .collect();
  Ok(Json(title_progress(&items, &main)))
}
