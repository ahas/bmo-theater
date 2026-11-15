//! A viewer's continue watching and next up (`docs/design.md`, 「Playing」; `docs/api.md`, 「Continue watching
//! and next up」).

use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{Json, Response};
use sqlx::{Postgres, QueryBuilder, Row};

use super::episodes::watchable;
use super::{Answer, Shared, bounds, no_content, paged, query};
use crate::caller::caller;
use crate::error::ApiError;
use crate::progress::{on_main_line, up_next};
use crate::store::{self, EPISODE, episode_of};
use crate::wire::{Episode, Page, RealmQuery, TitleKind, now_ms};

/// How long a show stays in next up after its last position: thirty days.
const NEXT_UP_SPAN_MS: i64 = 30 * 24 * 3600 * 1000;

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<RealmQuery>, QueryRejection>,
) -> Answer<Json<Page<Episode>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let (_, limit) = bounds(None, given.limit)?;
  let mut select = QueryBuilder::<Postgres>::new(EPISODE);
  select
    .push_bind(account.clone())
    .push(" WHERE p.seconds > 0 AND NOT p.done AND NOT p.set_aside AND ");
  caller.push_visible(&mut select, "t", given.realm)?;
  if let Some(after) = given.after {
    select
      .push(
        " AND (p.updated_at, e.id) < (SELECT a.updated_at, a.episode_id FROM positions a \
         WHERE a.account = ",
      )
      .push_bind(account)
      .push(" AND a.episode_id = ")
      .push_bind(after)
      .push(")");
  }
  select.push(" ORDER BY p.updated_at DESC, e.id DESC LIMIT ").push_bind(limit + 1);
  let rows = select.build().fetch_all(&app.pool).await?;
  let items = rows
    .iter()
    .map(|row| episode_of(row).map(|stored| stored.episode))
    .collect::<Result<_, _>>()?;
  Ok(Json(paged(items, limit)))
}

pub async fn set_aside(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let stored = watchable(&app, &caller, &id).await?;
  let set = sqlx::query("UPDATE positions SET set_aside = TRUE WHERE account = $1 AND episode_id = $2")
    .bind(&account)
    .bind(stored.episode.id)
    .execute(&app.pool)
    .await?;
  if set.rows_affected() == 0 {
    return Err(ApiError::not_found("The caller has no position in the episode."));
  }
  let position = store::position(&app.pool, &account, stored.episode.id)
    .await?
    .ok_or_else(|| ApiError::failed("the position was not kept"))?;
  app.publish_for("position.changed", &position, &account);
  Ok(no_content())
}

pub async fn next_up(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<RealmQuery>, QueryRejection>,
) -> Answer<Json<Page<Episode>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let (_, limit) = bounds(None, given.limit)?;
  let mut select = QueryBuilder::<Postgres>::new(
    "SELECT t.id FROM titles t JOIN episodes e ON e.title_id = t.id \
     JOIN positions p ON p.episode_id = e.id AND p.account = ",
  );
  select.push_bind(account.clone()).push(" WHERE NOT t.retired AND t.kind = 'show' AND ");
  caller.push_visible(&mut select, "t", given.realm)?;
  select
    .push(" GROUP BY t.id HAVING MAX(p.updated_at) >= ")
    .push_bind(now_ms() - NEXT_UP_SPAN_MS)
    .push(" ORDER BY MAX(p.updated_at) DESC, t.id DESC");
  let shows = select.build().fetch_all(&app.pool).await?;
  let mut offered = Vec::new();
  for show in &shows {
    let stored = store::title_episodes(&app.pool, show.try_get("id")?, Some(&account)).await?;
    let main: Vec<bool> = stored
      .iter()
      .map(|stored| on_main_line(TitleKind::Show, stored.episode.season, stored.episode.number))
      .collect();
    let items: Vec<_> = stored
      .iter()
      .map(|stored| (stored.episode.id, stored.episode.duration_ms, stored.episode.position.clone()))
      .collect();
    let Some(at) = up_next(&items, &main) else {
      continue;
    };
    let episode = &stored[at].episode;
    let not_started = episode
      .position
      .as_ref()
      .is_none_or(|position| position.seconds == 0.0 && !position.done && !position.set_aside);
    if not_started {
      offered.push(episode.clone());
    }
  }
  let from = match given.after {
    Some(after) => match offered.iter().position(|episode| episode.id == after) {
      Some(at) => at + 1,
      None => offered.len(),
    },
    None => 0,
  };
  Ok(Json(paged(offered.split_off(from), limit)))
}
