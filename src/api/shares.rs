//! The share links and the share reads (`docs/api.md`, 「Share links」): a link made and ended by a caller who
//! reads its member, and what its token opens read with no owner key.

use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Json, Response};
use rand::Rng;
use rand::distr::Alphanumeric;

use super::subtitles::{subtitle_file, vtt_of};
use super::{Answer, Shared, body, created, episodes, id_of, no_content, query, titles};
use crate::app::App;
use crate::caller::{Caller, Standing, caller, credential};
use crate::console;
use crate::error::ApiError;
use crate::store::{self, Stored, share_of};
use crate::wire::{
  Episode, HeadQuery, NewShare, Page, Playlist, Share, ShareDeleted, ShareHead, SharesQuery, Subtitle,
  TitleKind, now_ms,
};

/// How many letters and digits a token holds.
const TOKEN_LENGTH: usize = 32;

fn fresh_token() -> String {
  rand::rng().sample_iter(&Alphanumeric).take(TOKEN_LENGTH).map(char::from).collect()
}

/// Where the member a link opens stands, where it still stands.
async fn member_standing(app: &App, share: &Share) -> Answer<Option<Standing>> {
  if let Some(episode) = share.episode_id {
    return Ok(store::episode(&app.pool, episode, None).await?.map(|stored| stored.standing));
  }
  match share.playlist_id {
    Some(playlist) => Ok(store::playlist(&app.pool, playlist).await?.map(|(_, standing)| standing)),
    None => Ok(None),
  }
}

/// A link removed and told.
async fn end(app: &App, share: &Share) -> Answer<()> {
  let standing = member_standing(app, share).await?;
  sqlx::query("DELETE FROM shares WHERE token = $1")
    .bind(&share.token)
    .execute(&app.pool)
    .await?;
  let told = ShareDeleted {
    token: share.token.clone(),
  };
  match standing {
    Some(standing) => app.publish("share.deleted", told, &standing),
    None => app.events.publish("share.deleted", told, None),
  }
  Ok(())
}

/// The live link a token names: `not-found` where it names none, `gone` where its time has passed.
async fn live(app: &App, token: &str) -> Answer<Share> {
  let share = store::share(&app.pool, token, &app.settings.share.address)
    .await?
    .ok_or_else(|| ApiError::not_found("No share link stands at that token."))?;
  if share.expires_at.is_some_and(|expires| expires <= now_ms()) {
    end(app, &share).await?;
    return Err(ApiError::gone("The share link's time has passed."));
  }
  Ok(share)
}

/// The episodes a link opens, in its playlist's order.
async fn opened(app: &App, share: &Share) -> Answer<(Vec<Stored>, Option<Playlist>)> {
  if let Some(episode) = share.episode_id {
    let found = store::episode(&app.pool, episode, None).await?;
    return Ok((found.into_iter().collect(), None));
  }
  let Some(playlist) = share.playlist_id else {
    return Ok((Vec::new(), None));
  };
  let Some((playlist, _)) = store::playlist(&app.pool, playlist).await? else {
    return Ok((Vec::new(), None));
  };
  Ok((store::episodes_named(&app.pool, &playlist.episode_ids, None).await?, Some(playlist)))
}

/// One episode a link opens, or `not-found`.
async fn opened_episode(app: &App, share: &Share, episode: &str) -> Answer<Stored> {
  let id = id_of(episode, "episode")?;
  opened(app, share)
    .await?
    .0
    .into_iter()
    .find(|stored| stored.episode.id == id)
    .ok_or_else(|| ApiError::not_found("The link opens no such episode."))
}

/// The link a share read names, checked against the credential alone.
async fn share_read(app: &App, headers: &HeaderMap, token: &str) -> Answer<Share> {
  credential(&app.settings, headers)?;
  live(app, token).await
}

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<SharesQuery>, QueryRejection>,
) -> Answer<Json<Page<Share>>> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let rows = sqlx::query(
    "SELECT * FROM shares WHERE ($1::BIGINT IS NULL OR episode_id = $1) AND ($2::BIGINT IS NULL OR playlist_id = $2) \
     ORDER BY created_at, token",
  )
  .bind(given.episode_id)
  .bind(given.playlist_id)
  .fetch_all(&app.pool)
  .await?;
  let mut items = Vec::new();
  for row in &rows {
    let share = share_of(row, &app.settings.share.address)?;
    if share.expires_at.is_some_and(|expires| expires <= now_ms()) {
      end(&app, &share).await?;
      continue;
    }
    if member_standing(&app, &share).await?.is_some_and(|standing| caller.reads(&standing)) {
      items.push(share);
    }
  }
  Ok(Json(Page { items, more: false }))
}

/// Where the member a new link names stands, read by the caller.
async fn named(app: &App, caller: &Caller, new: &NewShare) -> Answer<Standing> {
  match (new.episode_id, new.playlist_id) {
    (Some(episode), None) => Ok(episodes::readable(app, caller, &episode.to_string()).await?.standing),
    (None, Some(playlist)) => Ok(caller.read(store::playlist(&app.pool, playlist).await?, "playlist")?.1),
    _ => Err(ApiError::bad_request("A link names an episodeId or a playlistId, one of the two.")),
  }
}

pub async fn create(State(app): Shared, headers: HeaderMap, bytes: Bytes) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let new: NewShare = body(&bytes)?;
  let standing = named(&app, &caller, &new).await?;
  if new.expires_at.is_some_and(|expires| expires <= now_ms()) {
    return Err(ApiError::bad_request("expiresAt is not in the future."));
  }
  let standing_link = sqlx::query("SELECT * FROM shares WHERE episode_id = $1 OR playlist_id = $2")
    .bind(new.episode_id)
    .bind(new.playlist_id)
    .fetch_optional(&app.pool)
    .await?;
  if let Some(row) = standing_link {
    let share = share_of(&row, &app.settings.share.address)?;
    if share.expires_at.is_none_or(|expires| expires > now_ms()) {
      return Ok(Json(share).into_response());
    }
    end(&app, &share).await?;
  }
  let token = fresh_token();
  sqlx::query(
    "INSERT INTO shares (token, episode_id, playlist_id, creator, created_at, expires_at) \
     VALUES ($1, $2, $3, $4, $5, $6)",
  )
  .bind(&token)
  .bind(new.episode_id)
  .bind(new.playlist_id)
  .bind(caller.creator())
  .bind(now_ms())
  .bind(new.expires_at)
  .execute(&app.pool)
  .await?;
  let share = store::share(&app.pool, &token, &app.settings.share.address)
    .await?
    .ok_or_else(|| ApiError::failed("the link was not kept"))?;
  app.publish("share.created", &share, &standing);
  Ok(created(share))
}

pub async fn read(
  State(app): Shared,
  headers: HeaderMap,
  Path(token): Path<String>,
) -> Answer<Json<Share>> {
  Ok(Json(share_read(&app, &headers, &token).await?))
}

pub async fn delete(State(app): Shared, headers: HeaderMap, Path(token): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let share = store::share(&app.pool, &token, &app.settings.share.address)
    .await?
    .ok_or_else(|| ApiError::not_found("No share link stands at that token."))?;
  let standing = member_standing(&app, &share)
    .await?
    .filter(|standing| caller.reads(standing))
    .ok_or_else(|| ApiError::not_found("No share link stands at that token."))?;
  let own = Standing {
    creator: share.creator.clone(),
    ..standing
  };
  caller.may_change(&own, "share link")?;
  end(&app, &share).await?;
  Ok(no_content())
}

/// An episode as a share read answers it: with no position.
fn shared(stored: Stored) -> Episode {
  Episode {
    position: None,
    ..stored.episode
  }
}

pub async fn playlist(
  State(app): Shared,
  headers: HeaderMap,
  Path(token): Path<String>,
) -> Answer<Json<Playlist>> {
  let share = share_read(&app, &headers, &token).await?;
  opened(&app, &share)
    .await?
    .1
    .map(Json)
    .ok_or_else(|| ApiError::not_found("The link opens an episode, not a playlist."))
}

pub async fn episodes(
  State(app): Shared,
  headers: HeaderMap,
  Path(token): Path<String>,
) -> Answer<Json<Page<Episode>>> {
  let share = share_read(&app, &headers, &token).await?;
  let items = opened(&app, &share).await?.0.into_iter().map(shared).collect();
  Ok(Json(Page { items, more: false }))
}

pub async fn stream(
  State(app): Shared,
  headers: HeaderMap,
  Path((token, episode)): Path<(String, String)>,
) -> Answer<Response> {
  let share = share_read(&app, &headers, &token).await?;
  let stored = opened_episode(&app, &share, &episode).await?;
  episodes::stream_of(&app, &stored, &headers).await
}

pub async fn poster(
  State(app): Shared,
  headers: HeaderMap,
  Path((token, episode)): Path<(String, String)>,
) -> Answer<Response> {
  let share = share_read(&app, &headers, &token).await?;
  let stored = opened_episode(&app, &share, &episode).await?;
  titles::poster_of(&app, stored.episode.id).await
}

pub async fn subtitles(
  State(app): Shared,
  headers: HeaderMap,
  Path((token, episode)): Path<(String, String)>,
) -> Answer<Json<Page<Subtitle>>> {
  let share = share_read(&app, &headers, &token).await?;
  let stored = opened_episode(&app, &share, &episode).await?;
  Ok(Json(Page {
    items: store::episode_subtitles(&app.pool, stored.episode.id).await?,
    more: false,
  }))
}

pub async fn vtt(
  State(app): Shared,
  headers: HeaderMap,
  Path((token, segment)): Path<(String, String)>,
) -> Answer<Response> {
  let share = share_read(&app, &headers, &token).await?;
  let (id, vtt) = subtitle_file(&segment)?;
  if !vtt {
    return Err(ApiError::not_found("A subtitle is read here as its WebVTT, <id>.vtt."));
  }
  let subtitle = store::subtitle(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::not_found("No subtitle stands there."))?;
  opened_episode(&app, &share, &subtitle.episode_id.to_string()).await?;
  vtt_of(&app, subtitle.id).await
}

pub async fn head(
  State(app): Shared,
  headers: HeaderMap,
  Path(token): Path<String>,
  given: Result<Query<HeadQuery>, QueryRejection>,
) -> Answer<Json<ShareHead>> {
  let given = query(given)?;
  let share = share_read(&app, &headers, &token).await?;
  let url = share
    .url
    .clone()
    .ok_or_else(|| ApiError::not_found("No share address is set, so the link has no head."))?;
  let (episodes, playlist) = opened(&app, &share).await?;
  let first = episodes.first();
  let title = match first {
    Some(stored) => store::title(&app.pool, stored.episode.title_id).await?.map(|(title, _)| title),
    None => None,
  };
  let (name, kind, description) = match (&playlist, first, &title) {
    (Some(playlist), ..) => (playlist.name.clone(), "video.other", None),
    (None, Some(stored), Some(title)) => (
      format!("{} · {}", title.name, stored.episode.name),
      if title.kind == TitleKind::Show { "video.episode" } else { "video.movie" },
      Some(title.description.clone()).filter(|text| !text.is_empty()),
    ),
    (None, Some(stored), None) => (stored.episode.name.clone(), "video.movie", None),
    (None, None, _) => return Err(ApiError::not_found("The link opens nothing that stands.")),
  };
  let image = first
    .filter(|stored| stored.episode.has_poster)
    .map(|stored| format!("/shares/{}/episodes/{}/poster", share.token, stored.episode.id));
  let language = given.language.as_deref();
  let word = |key: &str, default: &str| {
    app
      .face
      .as_ref()
      .and_then(|face| console::word(face, language, key))
      .unwrap_or_else(|| default.to_owned())
  };
  Ok(Json(ShareHead {
    title: name,
    description,
    image,
    url,
    locale: word("theater.share.locale", "en_US"),
    site_name: word("theater.share.siteName", "Theater"),
    kind: kind.to_owned(),
  }))
}
