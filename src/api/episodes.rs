//! The episodes: filing one, reading and changing it, its stream, its poster, the caller's position in it, and
//! its burns (`docs/api.md`, 「Titles」, 「Episodes」, 「Streams」, 「Positions and progress」, 「Burns」).

use std::path::Path as FilePath;

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, Request, State};
use axum::http::HeaderMap;
use axum::response::{Json, Response};
use sqlx::Row;

use super::uploads::{Handed, copy_in, local_file, number_field, receive, text_field};
use super::{Answer, Shared, body, bounds, created, id_of, no_content, one_line, paged, query, titles};
use crate::app::{App, remove_file, remove_folder};
use crate::burns::{self, file_into};
use crate::caller::{Caller, caller};
use crate::error::ApiError;
use crate::media;
use crate::names::{self, video_type};
use crate::progress::reaches_done;
use crate::range;
use crate::scan::write_read;
use crate::store::{self, Stored, burn_of};
use crate::wire::{
  Burn, Deleted, Episode, EpisodeChange, EpisodeKind, FiledByPath, NewBurn, Page, PageQuery, Position,
  PositionSet, now_ms,
};

/// An episode the caller reads.
pub async fn readable(app: &App, caller: &Caller, id: &str) -> Answer<Stored> {
  let id = id_of(id, "episode")?;
  match store::episode(&app.pool, id, caller.account.as_deref()).await? {
    Some(stored) if caller.reads(&stored.standing) => Ok(stored),
    _ => Err(ApiError::not_found("No episode stands there.")),
  }
}

/// What a filing names beside the file.
struct Filing {
  name: Option<String>,
  label: Option<String>,
  season: Option<i32>,
  number: Option<i32>,
}

pub async fn file(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  request: Request,
) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (title, standing) = titles::readable(&app, &caller, &id).await?;
  caller.may_change(&standing, "title")?;
  let (file, file_name, filing) = match receive(&app, request).await? {
    Handed::Upload {
      file,
      file_name,
      fields,
    } => {
      let filing = Filing {
        name: text_field(&fields, "name"),
        label: text_field(&fields, "label"),
        season: number_field(&fields, "season")?,
        number: number_field(&fields, "number")?,
      };
      (file, file_name, filing)
    }
    Handed::Json(bytes) => {
      caller.may_administer("files a video by its path")?;
      let given: FiledByPath = body(&bytes)?;
      let path = local_file(&given.path).await?;
      let file_name = super::uploads::file_name_of(&given.path);
      if video_type(names::extension(&file_name)).is_none() {
        return Err(not_a_video(&file_name));
      }
      let filing = Filing {
        name: given.name,
        label: given.label,
        season: given.season,
        number: given.number,
      };
      (copy_in(&app, &path).await?, file_name, filing)
    }
  };
  let media_type = video_type(names::extension(&file_name)).ok_or_else(|| not_a_video(&file_name))?;
  let probe = match media::probe(&app.settings.media, &file.path).await {
    Ok(probe) => probe,
    Err(why) if why.starts_with("ffprobe refused") => media::Probe::default(),
    Err(why) => return Err(ApiError::failed(why)),
  };
  if !probe.has_video() {
    return Err(not_a_video(&file_name));
  }
  let name = match filing.name.as_deref() {
    Some(name) => one_line(name, "name")?,
    None => names::clean(names::stem(&file_name)),
  };
  let name = if name.is_empty() { file_name.clone() } else { name };
  let now = now_ms();
  let row = sqlx::query(
    "INSERT INTO episodes (title_id, kind, season, number, name, label, path, file_name, media_type, byte_count, \
     modified_at, creator, created_at, changed_at) \
     VALUES ($1, 'clip', $2, $3, $4, $5, '', $6, $7, 0, 0, $8, $9, $9) RETURNING id",
  )
  .bind(title.id)
  .bind(filing.season)
  .bind(filing.number)
  .bind(&name)
  .bind(filing.label.unwrap_or_default())
  .bind(&file_name)
  .bind(media_type)
  .bind(caller.creator())
  .bind(now)
  .fetch_one(&app.pool)
  .await?;
  let id: i64 = row.try_get("id")?;
  let from = file.keep();
  let (size, modified) = match file_into(&app, id, &from, &file_name).await {
    Ok(filed) => filed,
    Err(why) => {
      remove_file(&from).await;
      sqlx::query("DELETE FROM episodes WHERE id = $1").bind(id).execute(&app.pool).await?;
      return Err(ApiError::failed(why));
    }
  };
  let path = app.episode_folder(id).join(&file_name);
  sqlx::query("UPDATE episodes SET path = $2, byte_count = $3, modified_at = $4 WHERE id = $1")
    .bind(id)
    .bind(path.to_string_lossy().as_ref())
    .bind(size)
    .bind(modified)
    .execute(&app.pool)
    .await?;
  let poster = media::poster(&app.settings.media, &path, probe.duration_ms, &app.poster_path(id)).await;
  write_read(&app, id, &probe, poster).await?;
  let stored = store::episode(&app.pool, id, caller.account.as_deref())
    .await?
    .ok_or_else(|| ApiError::failed("the episode was not kept"))?;
  let mut told = stored.episode.clone();
  told.position = None;
  app.publish("episode.created", &told, &stored.standing);
  Ok(created(stored.episode))
}

fn not_a_video(file_name: &str) -> ApiError {
  ApiError::of("not-a-video", format!("{file_name} is no video the theater keeps."))
}

pub async fn read(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Json<Episode>> {
  let caller = caller(&app.settings, &headers)?;
  Ok(Json(readable(&app, &caller, &id).await?.episode))
}

pub async fn change(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Episode>> {
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  caller.may_change(&stored.standing, "episode")?;
  let change: EpisodeChange = body(&bytes)?;
  let episode = stored.episode;
  let name = match &change.name {
    Some(name) => one_line(name, "name")?,
    None => episode.name,
  };
  sqlx::query(
    "UPDATE episodes SET name = $2, label = $3, season = $4, number = $5, changed_at = $6 WHERE id = $1",
  )
  .bind(episode.id)
  .bind(name)
  .bind(change.label.map(|label| label.trim().to_owned()).unwrap_or(episode.label))
  .bind(change.season.unwrap_or(episode.season))
  .bind(change.number.unwrap_or(episode.number))
  .bind(now_ms())
  .execute(&app.pool)
  .await?;
  let stored = store::episode(&app.pool, episode.id, caller.account.as_deref())
    .await?
    .ok_or_else(|| ApiError::not_found("No episode stands there."))?;
  let mut told = stored.episode.clone();
  told.position = None;
  app.publish("episode.changed", &told, &stored.standing);
  Ok(Json(stored.episode))
}

pub async fn delete(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  caller.may_change(&stored.standing, "episode")?;
  if stored.episode.kind == EpisodeKind::Episode || stored.library_id.is_some() {
    return Err(ApiError::conflict("A library's episode goes with its file; it is not deleted alone."));
  }
  let subtitles = store::episode_subtitles(&app.pool, stored.episode.id).await?;
  sqlx::query("DELETE FROM episodes WHERE id = $1")
    .bind(stored.episode.id)
    .execute(&app.pool)
    .await?;
  remove_folder(&app.episode_folder(stored.episode.id)).await;
  remove_file(&app.poster_path(stored.episode.id)).await;
  for subtitle in subtitles {
    remove_folder(&app.subtitle_folder(subtitle.id)).await;
  }
  app.publish(
    "episode.deleted",
    Deleted {
      id: stored.episode.id,
    },
    &stored.standing,
  );
  Ok(no_content())
}

/// An episode's file answered to a request, as 「Streams」 states.
pub async fn stream_of(app: &App, stored: &Stored, headers: &HeaderMap) -> Answer<Response> {
  match range::serve(FilePath::new(&stored.path), &stored.episode.media_type, headers).await? {
    Some(response) => Ok(response),
    None => {
      if stored.episode.available {
        sqlx::query("UPDATE episodes SET available = FALSE, changed_at = $2 WHERE id = $1")
          .bind(stored.episode.id)
          .bind(now_ms())
          .execute(&app.pool)
          .await?;
        if let Some(changed) = store::episode(&app.pool, stored.episode.id, None).await? {
          app.publish("episode.changed", &changed.episode, &changed.standing);
        }
      }
      Err(ApiError::not_found("The episode's file is no longer on disk."))
    }
  }
}

pub async fn stream(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  stream_of(&app, &stored, &headers).await
}

pub async fn poster(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  titles::poster_of(&app, stored.episode.id).await
}

pub async fn position(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Position>> {
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let stored = readable(&app, &caller, &id).await?;
  store::position(&app.pool, &account, stored.episode.id)
    .await?
    .map(Json)
    .ok_or_else(|| ApiError::not_found("The caller has no position in the episode."))
}

pub async fn set_position(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Json<Position>> {
  let caller = caller(&app.settings, &headers)?;
  let account = caller.account()?.to_owned();
  let stored = readable(&app, &caller, &id).await?;
  let set: PositionSet = body(&bytes)?;
  if !set.seconds.is_finite() || set.seconds < 0.0 {
    return Err(ApiError::bad_request("seconds is a number, 0 or more."));
  }
  let was = store::position(&app.pool, &account, stored.episode.id)
    .await?
    .is_some_and(|position| position.done);
  let done = match set.done {
    Some(done) => done,
    None => was || reaches_done(set.seconds, stored.episode.duration_ms),
  };
  sqlx::query(
    "INSERT INTO positions (account, episode_id, seconds, done, updated_at) VALUES ($1, $2, $3, $4, $5) \
     ON CONFLICT (account, episode_id) DO UPDATE SET seconds = $3, done = $4, updated_at = $5",
  )
  .bind(&account)
  .bind(stored.episode.id)
  .bind(set.seconds)
  .bind(done)
  .bind(now_ms())
  .execute(&app.pool)
  .await?;
  let position = store::position(&app.pool, &account, stored.episode.id)
    .await?
    .ok_or_else(|| ApiError::failed("the position was not kept"))?;
  app.publish_for("position.changed", &position, &account);
  Ok(Json(position))
}

pub async fn burn(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  bytes: Bytes,
) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  let new: NewBurn = body(&bytes)?;
  match store::subtitle(&app.pool, new.subtitle_id).await? {
    Some(subtitle) if subtitle.episode_id == stored.episode.id => {}
    _ => {
      return Err(ApiError::of(
        "unknown-subtitle",
        format!("The episode has no subtitle {}.", new.subtitle_id),
      ));
    }
  }
  Ok(created(burns::start(&app, stored.episode.id, new.subtitle_id, caller.creator()).await?))
}

pub async fn burns(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  given: Result<Query<PageQuery>, QueryRejection>,
) -> Answer<Json<Page<Burn>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let stored = readable(&app, &caller, &id).await?;
  let (_, limit) = bounds(None, given.limit)?;
  let rows = sqlx::query("SELECT * FROM burns WHERE episode_id = $1 AND id < $2 ORDER BY id DESC LIMIT $3")
    .bind(stored.episode.id)
    .bind(given.after.unwrap_or(i64::MAX))
    .bind(limit + 1)
    .fetch_all(&app.pool)
    .await?;
  let items = rows.iter().map(burn_of).collect::<Result<_, _>>()?;
  Ok(Json(paged(items, limit)))
}

pub async fn read_burn(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Burn>> {
  let caller = caller(&app.settings, &headers)?;
  let id = id_of(&id, "burn")?;
  let burn = store::burn(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::not_found("No burn stands there."))?;
  readable(&app, &caller, &burn.episode_id.to_string())
    .await
    .map_err(|_| ApiError::not_found("No burn stands there."))?;
  Ok(Json(burn))
}
