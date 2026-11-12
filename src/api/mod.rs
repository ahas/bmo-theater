//! The routes (`docs/api.md`): the helpers every route shares, the health and the event stream, each resource's
//! routes in a module of its own, and the console face's routes beside them where a face stands.

mod episodes;
mod libraries;
mod playlists;
mod shares;
mod subtitles;
mod titles;
mod uploads;

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::rejection::QueryRejection;
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::sse::{Event as Sent, KeepAlive, Sse};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use futures_util::{StreamExt, stream};
use serde::de::DeserializeOwned;

use crate::app::App;
use crate::caller::{caller, credential, header_text};
use crate::error::ApiError;
use crate::events::Record;
use crate::wire::{EventsQuery, Health, Page, StreamEvent};

/// How often a comment line keeps a stream open.
const KEEP_OPEN: Duration = Duration::from_secs(15);

/// The most items a page answers, and how many where it names none.
const MOST: i64 = 1000;
const PAGE: i64 = 200;

type Answer<T> = Result<T, ApiError>;
type Shared = State<Arc<App>>;

/// Every route, the unknown ones answered `no-route`: the health, the routes the screens call under the console
/// face's guard, and the face's own routes, where a face stands (`docs/api.md`, 「Console」).
pub fn router(app: Arc<App>) -> Router {
  let face = app.face.clone();
  let screens = match &face {
    Some(face) => face.guard_screens(screens()),
    None => screens(),
  };
  let mut routes = Router::new().route("/health", get(health)).merge(screens);
  if let Some(face) = &face {
    routes = routes.merge(face.router());
  }
  routes
    .fallback(|| async { ApiError::no_route() })
    .method_not_allowed_fallback(|| async { ApiError::no_route() })
    .with_state(app)
}

/// The routes the screens call, which the service's other callers reach as well.
fn screens() -> Router<Arc<App>> {
  Router::new()
    .route("/events", get(events))
    .route("/libraries", get(libraries::list).post(libraries::create))
    .route(
      "/libraries/{libraryId}",
      get(libraries::read).patch(libraries::change).delete(libraries::delete),
    )
    .route("/libraries/{libraryId}/scans", get(libraries::scans).post(libraries::scan))
    .route("/titles", get(titles::list).post(titles::create))
    .route("/titles/{titleId}", get(titles::read).patch(titles::change).delete(titles::delete))
    .route("/titles/{titleId}/poster", get(titles::poster))
    .route(
      "/titles/{titleId}/episodes",
      get(titles::episodes).post(episodes::file).layer(DefaultBodyLimit::disable()),
    )
    .route("/titles/{titleId}/progress", get(titles::progress))
    .route(
      "/episodes/{episodeId}",
      get(episodes::read).patch(episodes::change).delete(episodes::delete),
    )
    .route("/episodes/{episodeId}/stream", get(episodes::stream))
    .route("/episodes/{episodeId}/poster", get(episodes::poster))
    .route("/episodes/{episodeId}/position", get(episodes::position).put(episodes::set_position))
    .route(
      "/episodes/{episodeId}/subtitles",
      get(subtitles::list).post(subtitles::add).layer(DefaultBodyLimit::disable()),
    )
    .route("/episodes/{episodeId}/burns", get(episodes::burns).post(episodes::burn))
    .route("/burns/{burnId}", get(episodes::read_burn))
    .route("/subtitles/{subtitleFile}", get(subtitles::read).delete(subtitles::delete))
    .route("/subtitles/{subtitleId}/original", get(subtitles::original))
    .route("/playlists", get(playlists::list).post(playlists::create))
    .route(
      "/playlists/{playlistId}",
      get(playlists::read).patch(playlists::change).delete(playlists::delete),
    )
    .route("/playlists/{playlistId}/items", post(playlists::add))
    .route("/playlists/{playlistId}/progress", get(playlists::progress))
    .route("/shares", get(shares::list).post(shares::create))
    .route("/shares/{token}", get(shares::read).delete(shares::delete))
    .route("/shares/{token}/head", get(shares::head))
    .route("/shares/{token}/playlist", get(shares::playlist))
    .route("/shares/{token}/episodes", get(shares::episodes))
    .route("/shares/{token}/episodes/{episodeId}/stream", get(shares::stream))
    .route("/shares/{token}/episodes/{episodeId}/poster", get(shares::poster))
    .route("/shares/{token}/episodes/{episodeId}/subtitles", get(shares::subtitles))
    .route("/shares/{token}/subtitles/{subtitleFile}", get(shares::vtt))
}

fn query<T: DeserializeOwned>(given: Result<Query<T>, QueryRejection>) -> Answer<T> {
  given.map(|Query(query)| query).map_err(|why| ApiError::bad_request(why.body_text()))
}

fn body<T: DeserializeOwned>(bytes: &Bytes) -> Answer<T> {
  serde_json::from_slice(bytes)
    .map_err(|why| ApiError::bad_request(format!("The body does not read: {why}")))
}

fn created(value: impl serde::Serialize) -> Response {
  (StatusCode::CREATED, Json(value)).into_response()
}

fn no_content() -> Response {
  StatusCode::NO_CONTENT.into_response()
}

/// A page's start and size, refused where the size is outside 1 to 1000.
fn bounds(after: Option<i64>, limit: Option<i64>) -> Answer<(i64, i64)> {
  let limit = limit.unwrap_or(PAGE);
  if !(1..=MOST).contains(&limit) {
    return Err(ApiError::bad_request("limit is 1 to 1000"));
  }
  Ok((after.unwrap_or(i64::MIN), limit))
}

/// The first `limit` of rows read one past it, and whether more stand.
fn paged<T>(mut items: Vec<T>, limit: i64) -> Page<T> {
  let more = items.len() as i64 > limit;
  items.truncate(limit as usize);
  Page { items, more }
}

/// A name: one line, not blank.
fn one_line(text: &str, what: &str) -> Answer<String> {
  let text = text.trim();
  if text.is_empty() || text.contains('\n') {
    return Err(ApiError::bad_request(format!("The {what} is blank or more than one line.")));
  }
  Ok(text.to_owned())
}

/// An id a path names, `not-found` where it does not read as one.
fn id_of(text: &str, what: &str) -> Answer<i64> {
  text.parse().map_err(|_| ApiError::not_found(format!("No {what} stands at {text}.")))
}

// ---------------------------------------------------------------------------------------------------------
// Health

async fn health(State(app): Shared, headers: HeaderMap) -> Answer<Json<Health>> {
  credential(&app.settings, &headers)?;
  let reason = app.unready();
  Ok(Json(Health {
    ready: reason.is_none(),
    reason,
    version: env!("CARGO_PKG_VERSION").to_owned(),
  }))
}

// ---------------------------------------------------------------------------------------------------------
// The event stream

fn sent(event: &StreamEvent) -> Sent {
  Sent::default()
    .event(event.kind.clone())
    .id(event.id.to_string())
    .json_data(event)
    .unwrap_or_else(|_| Sent::default().comment("an event that did not render"))
}

async fn events(
  State(app): Shared,
  headers: HeaderMap,
  given: Result<Query<EventsQuery>, QueryRejection>,
) -> Answer<Response> {
  let given = query(given)?;
  let caller = caller(&app.settings, &headers)?;
  let resumed = header_text(&headers, "last-event-id").and_then(|id| id.parse::<i64>().ok());
  let after = resumed.or(given.after);
  let kinds: Option<Vec<String>> = given
    .kinds
    .as_deref()
    .map(|kinds| kinds.split(',').map(str::trim).map(str::to_owned).collect());
  let visible = move |record: &Record| {
    let kind_wanted = kinds.as_ref().is_none_or(|kinds| kinds.contains(&record.event.kind));
    let reader_may = caller.service || record.account.is_none() || record.account == caller.account;
    kind_wanted && reader_may
  };
  let gone =
    || ApiError::gone("The events after that number are no longer kept; read the collections again.");
  let paged = header_text(&headers, "accept").is_some_and(|accepted| accepted.contains("application/json"));
  if paged {
    let limit = given.limit.unwrap_or(50);
    if !(1..=500).contains(&limit) {
      return Err(ApiError::bad_request("limit is 1 to 500"));
    }
    let (records, is_gone) = app.events.after(after.unwrap_or(app.events.first()));
    if is_gone && after.is_some() {
      return Err(gone());
    }
    let records: Vec<_> = records.into_iter().filter(|record| visible(record)).collect();
    let more = records.len() > limit as usize;
    let items = records.into_iter().take(limit as usize).map(|record| record.event).collect();
    return Ok(Json(Page::<StreamEvent> { items, more }).into_response());
  }
  let live = app.events.subscribe();
  let backlog = match after {
    Some(after) => {
      let (records, is_gone) = app.events.after(after);
      if is_gone {
        return Err(gone());
      }
      records
    }
    None => Vec::new(),
  };
  let cursor = backlog
    .last()
    .map(|record| record.event.id)
    .or(after)
    .unwrap_or_else(|| app.events.latest());
  let live = stream::unfold((live, cursor), |(mut live, cursor)| async move {
    loop {
      match live.recv().await {
        Ok(record) if record.event.id > cursor => {
          let id = record.event.id;
          return Some((record, (live, id)));
        }
        Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
        Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
      }
    }
  });
  let stream = stream::iter(backlog)
    .chain(live)
    .filter(move |record| std::future::ready(visible(record)))
    .map(|record| Ok::<_, Infallible>(sent(&record.event)))
    .take_until(app.events.stopped());
  Ok(Sse::new(stream).keep_alive(KeepAlive::new().interval(KEEP_OPEN)).into_response())
}
