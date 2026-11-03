//! A refusal or a failure, answered in the platform's error shape (`docs/api.md`, 「Codes」).

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::wire::WireError;

/// Every code the service answers, each with its status and what it means.
pub const CODES: [(&str, u16, &str); 18] = [
  ("bad-request", 400, "the body or the query is not the shape the route takes"),
  ("unauthorized", 401, "the token is missing or wrong"),
  ("forbidden", 403, "the token is right but the caller may not do this"),
  ("no-route", 404, "no route answers this method and path"),
  ("not-found", 404, "nothing stands at the key the path names"),
  ("conflict", 409, "the member does not take the change, or the key is taken"),
  (
    "gone",
    410,
    "a share link whose time has passed, or a resume from before the events kept",
  ),
  (
    "no-account",
    422,
    "a mine list or write, or a position, on a call that names no account",
  ),
  (
    "not-a-folder",
    422,
    "a library's path that is not an absolute folder the service reads outside its file root",
  ),
  (
    "not-a-video",
    422,
    "a filed file whose extension is no video's or in which no video stream is found",
  ),
  ("not-a-file", 422, "a path that is not an absolute file the service reads"),
  ("unknown-format", 422, "a subtitle format outside srt, ass, smi and vtt"),
  ("no-cues", 422, "a subtitle that converts to no cue"),
  ("unknown-subtitle", 422, "a burn naming a subtitle that is not the episode's"),
  ("unknown-episode", 422, "a playlist naming an episode the caller does not read"),
  ("not-in-realm", 422, "a playlist naming an episode its readers would not all read"),
  ("failed", 500, "the service failed: its database, its file root or a command it runs"),
  ("not-ready", 503, "the service cannot answer yet"),
];

/// A refusal or a failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApiError {
  pub status: u16,
  pub code: &'static str,
  pub error: String,
}

impl ApiError {
  /// A refusal under one of the codes.
  pub fn of(code: &'static str, error: impl Into<String>) -> Self {
    let status = CODES
      .iter()
      .find(|(name, ..)| *name == code)
      .map(|(_, status, _)| *status)
      .unwrap_or(500);

    Self {
      status,
      code,
      error: error.into(),
    }
  }

  pub fn bad_request(error: impl Into<String>) -> Self {
    Self::of("bad-request", error)
  }

  pub fn unauthorized() -> Self {
    Self::of("unauthorized", "The bearer token is missing or wrong.")
  }

  pub fn forbidden(error: impl Into<String>) -> Self {
    Self::of("forbidden", error)
  }

  pub fn no_route() -> Self {
    Self::of("no-route", "No route answers this method and path.")
  }

  pub fn not_found(error: impl Into<String>) -> Self {
    Self::of("not-found", error)
  }

  pub fn conflict(error: impl Into<String>) -> Self {
    Self::of("conflict", error)
  }

  pub fn gone(error: impl Into<String>) -> Self {
    Self::of("gone", error)
  }

  pub fn no_account() -> Self {
    Self::of("no-account", "The call names no account; send Owner-Key.")
  }

  pub fn failed(error: impl Into<String>) -> Self {
    Self::of("failed", error)
  }
}

impl From<sqlx::Error> for ApiError {
  fn from(why: sqlx::Error) -> Self {
    Self::failed(format!("the database failed: {why}"))
  }
}

impl From<std::io::Error> for ApiError {
  fn from(why: std::io::Error) -> Self {
    Self::failed(format!("the file root failed: {why}"))
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    let body = WireError {
      code: self.code.to_owned(),
      error: self.error,
    };
    let mut response = (status, axum::Json(body)).into_response();
    if status == StatusCode::SERVICE_UNAVAILABLE {
      response.headers_mut().insert(header::RETRY_AFTER, HeaderValue::from_static("2"));
    }
    response
  }
}
