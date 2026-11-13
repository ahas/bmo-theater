//! The subtitles (`docs/api.md`, 「Subtitles」): added as an upload or by a path, kept as the original and its
//! WebVTT, and read as either.

use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::{IntoResponse, Json, Response};
use sqlx::Row;

use super::uploads::{Handed, local_file, receive, text_field};
use super::{Answer, Shared, body, created, episodes, id_of, no_content};
use crate::app::{App, remove_folder};
use crate::burns::original_name;
use crate::caller::{Caller, Standing, caller};
use crate::error::ApiError;
use crate::names;
use crate::store;
use crate::subtitles::{convert, decode};
use crate::wire::{Deleted, Page, Subtitle, SubtitleByPath, SubtitleFormat, now_ms};

pub async fn list(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
) -> Answer<Json<Page<Subtitle>>> {
  let caller = caller(&app.settings, &headers)?;
  let stored = episodes::watchable(&app, &caller, &id).await?;
  Ok(Json(Page {
    items: store::episode_subtitles(&app.pool, stored.episode.id).await?,
    more: false,
  }))
}

/// A language tag as given, `und` where none is.
fn language_of(given: Option<String>) -> Answer<String> {
  let language = given.map(|language| language.trim().to_owned()).filter(|language| !language.is_empty());
  let language = language.unwrap_or_else(|| "und".to_owned());
  if language.len() > 35
    || !language.chars().all(|character| character.is_ascii_alphanumeric() || character == '-')
  {
    return Err(ApiError::bad_request("language is a language tag: letters, digits and hyphens."));
  }
  Ok(language)
}

pub async fn add(
  State(app): Shared,
  headers: HeaderMap,
  Path(id): Path<String>,
  request: Request,
) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let stored = episodes::readable(&app, &caller, &id).await?;
  let (bytes, file_name, language, label, format) = match receive(&app, request).await? {
    Handed::Upload {
      file,
      file_name,
      fields,
    } => {
      let format = match text_field(&fields, "format") {
        Some(word) => Some(SubtitleFormat::of(&word).ok_or_else(|| unknown_format(&word))?),
        None => None,
      };
      let bytes = tokio::fs::read(&file.path).await?;
      (bytes, file_name, text_field(&fields, "language"), text_field(&fields, "label"), format)
    }
    Handed::Json(given) => {
      caller.may_administer("adds a subtitle by its path")?;
      let given: SubtitleByPath = body(&given)?;
      let path = local_file(&given.path).await?;
      (
        tokio::fs::read(&path).await?,
        super::uploads::file_name_of(&given.path),
        given.language,
        given.label,
        given.format,
      )
    }
  };
  let format = match format {
    Some(format) => format,
    None => {
      let extension = names::extension(&file_name);
      SubtitleFormat::of(extension).ok_or_else(|| unknown_format(extension))?
    }
  };
  let language = language_of(language)?;
  let converted = convert(format, &decode(&bytes))
    .map_err(|why| ApiError::of("no-cues", format!("{file_name}: {why}.")))?;
  let row = sqlx::query(
    "INSERT INTO subtitles (episode_id, language, label, format, file_name, cue_count, creator, created_at) \
     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING id",
  )
  .bind(stored.episode.id)
  .bind(&language)
  .bind(label.unwrap_or_default())
  .bind(format.word())
  .bind(&file_name)
  .bind(converted.cues as i64)
  .bind(caller.creator())
  .bind(now_ms())
  .fetch_one(&app.pool)
  .await?;
  let id: i64 = row.try_get("id")?;
  let folder = app.subtitle_folder(id);
  let written = async {
    tokio::fs::create_dir_all(&folder).await?;
    tokio::fs::write(folder.join(original_name(format)), &bytes).await?;
    tokio::fs::write(app.vtt_path(id), converted.vtt.as_bytes()).await
  }
  .await;
  if let Err(why) = written {
    sqlx::query("DELETE FROM subtitles WHERE id = $1").bind(id).execute(&app.pool).await?;
    remove_folder(&folder).await;
    return Err(why.into());
  }
  let subtitle = store::subtitle(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::failed("the subtitle was not kept"))?;
  app.publish("subtitle.created", &subtitle, &stored.standing);
  Ok(created(subtitle))
}

fn unknown_format(word: &str) -> ApiError {
  ApiError::of("unknown-format", format!("{word} is none of srt, ass, smi and vtt."))
}

fn no_subtitle() -> ApiError {
  ApiError::not_found("No subtitle stands there.")
}

/// A subtitle, wherever it stands.
async fn subtitle(app: &App, id: i64) -> Answer<Subtitle> {
  store::subtitle(&app.pool, id).await?.ok_or_else(no_subtitle)
}

/// A subtitle the caller reads, and where its episode stands.
async fn readable(app: &App, caller: &Caller, id: i64) -> Answer<(Subtitle, Standing)> {
  let subtitle = subtitle(app, id).await?;
  let stored = episodes::readable(app, caller, &subtitle.episode_id.to_string())
    .await
    .map_err(|_| no_subtitle())?;
  Ok((subtitle, stored.standing))
}

/// A subtitle of an episode the caller watches, a granted one among them.
async fn watchable(app: &App, caller: &Caller, id: i64) -> Answer<Subtitle> {
  let subtitle = subtitle(app, id).await?;
  episodes::watchable(app, caller, &subtitle.episode_id.to_string())
    .await
    .map_err(|_| no_subtitle())?;
  Ok(subtitle)
}

/// A subtitle's WebVTT answered.
pub async fn vtt_of(app: &App, subtitle: i64) -> Answer<Response> {
  let text = tokio::fs::read(app.vtt_path(subtitle))
    .await
    .map_err(|_| ApiError::not_found("The subtitle's WebVTT is not on disk."))?;
  let mut response = text.into_response();
  response
    .headers_mut()
    .insert(header::CONTENT_TYPE, HeaderValue::from_static("text/vtt; charset=utf-8"));
  response
    .headers_mut()
    .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
  Ok(response)
}

/// The subtitle a path segment names, and whether it asks for its WebVTT.
pub fn subtitle_file(segment: &str) -> Answer<(i64, bool)> {
  match segment.strip_suffix(".vtt") {
    Some(id) => Ok((id_of(id, "subtitle")?, true)),
    None => Ok((id_of(segment, "subtitle")?, false)),
  }
}

pub async fn read(State(app): Shared, headers: HeaderMap, Path(segment): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (id, vtt) = subtitle_file(&segment)?;
  let subtitle = watchable(&app, &caller, id).await?;
  if vtt { vtt_of(&app, subtitle.id).await } else { Ok(Json(subtitle).into_response()) }
}

pub async fn original(State(app): Shared, headers: HeaderMap, Path(id): Path<String>) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let subtitle = watchable(&app, &caller, id_of(&id, "subtitle")?).await?;
  let bytes = tokio::fs::read(app.subtitle_folder(subtitle.id).join(original_name(subtitle.format)))
    .await
    .map_err(|_| ApiError::not_found("The subtitle's original is not on disk."))?;
  let mut response = bytes.into_response();
  response
    .headers_mut()
    .insert(header::CONTENT_TYPE, HeaderValue::from_static("application/octet-stream"));
  let name: String = subtitle
    .file_name
    .chars()
    .filter(|c| c.is_ascii_graphic() && *c != '"' && *c != '\\')
    .collect();
  if let Ok(value) = HeaderValue::from_str(&format!("attachment; filename=\"{name}\"")) {
    response.headers_mut().insert(header::CONTENT_DISPOSITION, value);
  }
  Ok(response)
}

pub async fn delete(
  State(app): Shared,
  headers: HeaderMap,
  Path(segment): Path<String>,
) -> Answer<Response> {
  let caller = caller(&app.settings, &headers)?;
  let (id, vtt) = subtitle_file(&segment)?;
  if vtt {
    return Err(ApiError::no_route());
  }
  let (subtitle, standing) = readable(&app, &caller, id).await?;
  let own = Standing {
    creator: subtitle.creator.clone(),
    ..standing.clone()
  };
  caller.may_change(&own, "subtitle")?;
  sqlx::query("DELETE FROM subtitles WHERE id = $1")
    .bind(subtitle.id)
    .execute(&app.pool)
    .await?;
  remove_folder(&app.subtitle_folder(subtitle.id)).await;
  app.publish("subtitle.deleted", Deleted { id: subtitle.id }, &standing);
  Ok(no_content())
}
