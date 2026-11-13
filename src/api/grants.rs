//! The grants (`docs/api.md`, 「Grants」): who an episode is shared with, read, made, changed and ended by
//! whoever changes the episode. What a grant opens is the episodes' and the subtitles' own watching reads.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::{Json, Response};

use super::{Answer, Shared, body, episodes, no_content};
use crate::app::App;
use crate::caller::{LONGEST_KEY, caller};
use crate::error::ApiError;
use crate::store::{self, Stored, grant_of};
use crate::wire::{Grant, GrantChange, NewGrants, Page, Realm, now_ms};

/// The episode whose grants a call reaches: read by the caller, and changed by it.
async fn granting(app: &App, headers: &HeaderMap, id: &str) -> Answer<(String, Stored)> {
  let caller = caller(&app.settings, headers)?;
  let stored = episodes::readable(app, &caller, id).await?;
  caller.may_change(&stored.standing, "episode")?;
  Ok((caller.creator(), stored))
}

fn no_grant() -> ApiError {
  ApiError::not_found("The episode is not granted to that account.")
}

/// The account keys given, trimmed, each 1 to 200 characters, at least one.
fn accounts(given: Vec<String>) -> Answer<Vec<String>> {
  let users: Vec<String> = given.into_iter().map(|user| user.trim().to_owned()).collect();
  let fits = |user: &String| !user.is_empty() && user.chars().count() <= LONGEST_KEY;
  if users.is_empty() || !users.iter().all(fits) {
    return Err(ApiError::bad_request("users is one or more account keys, each 1 to 200 characters."));
  }
  Ok(users)
}

async fn page(app: &App, episode: i64) -> Answer<Json<Page<Grant>>> {
  Ok(Json(Page {
    items: store::grants(&app.pool, episode).await?,
    more: false,
  }))
}

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Page<Grant>>> {
  let (_, stored) = granting(&app, &headers, &id).await?;
  page(&app, stored.episode.id).await
}

pub async fn grant(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Page<Grant>>> {
  let (grantor, stored) = granting(&app, &headers, &id).await?;
  let new: NewGrants = body(&bytes)?;
  let users = accounts(new.users)?;
  let owner = (stored.standing.realm == Realm::Mine).then_some(stored.standing.owner.as_str());
  let mut transaction = app.pool.begin().await?;
  for user in users.iter().filter(|user| Some(user.as_str()) != owner) {
    sqlx::query(
      "INSERT INTO grants (episode_id, account, permission, grantor, granted_at) VALUES ($1, $2, $3, $4, $5) \
       ON CONFLICT (episode_id, account) DO UPDATE SET permission = $3",
    )
    .bind(stored.episode.id)
    .bind(user)
    .bind(new.permission.word())
    .bind(&grantor)
    .bind(now_ms())
    .execute(&mut *transaction)
    .await?;
  }
  transaction.commit().await?;
  page(&app, stored.episode.id).await
}

pub async fn change(
  State(app): Shared,
  headers: HeaderMap,
  Path((id, user)): Path<(String, String)>,
  bytes: Bytes,
) -> Answer<Json<Grant>> {
  let (_, stored) = granting(&app, &headers, &id).await?;
  let set: GrantChange = body(&bytes)?;
  let row =
    sqlx::query("UPDATE grants SET permission = $3 WHERE episode_id = $1 AND account = $2 RETURNING *")
      .bind(stored.episode.id)
      .bind(&user)
      .bind(set.permission.word())
      .fetch_optional(&app.pool)
      .await?
      .ok_or_else(no_grant)?;
  Ok(Json(grant_of(&row)?))
}

pub async fn end(
  State(app): Shared,
  headers: HeaderMap,
  Path((id, user)): Path<(String, String)>,
) -> Answer<Response> {
  let (_, stored) = granting(&app, &headers, &id).await?;
  let ended = sqlx::query("DELETE FROM grants WHERE episode_id = $1 AND account = $2")
    .bind(stored.episode.id)
    .bind(&user)
    .execute(&app.pool)
    .await?;
  if ended.rows_affected() == 0 {
    return Err(no_grant());
  }
  Ok(no_content())
}
