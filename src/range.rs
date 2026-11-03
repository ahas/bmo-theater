//! A file served as it stands, with HTTP ranges (`docs/api.md`, 「Streams」): 200 whole, 206 a stretch, 416 a
//! stretch past the end, 304 where the caller holds the current tag.

use std::io::SeekFrom;
use std::path::Path;
use std::time::UNIX_EPOCH;

use axum::body::Body;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use futures_util::stream;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// How many bytes one read of the file hands on.
const CHUNK: usize = 256 * 1024;

/// The stretch a `Range` header asks of a file of `size` bytes: inclusive first and last.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wanted {
  /// The whole file.
  Whole,
  /// A stretch within it.
  Stretch(u64, u64),
  /// A stretch that starts past its end.
  Unsatisfiable,
}

/// The stretch a `Range` header's value asks for, the first where it names several; a value that does not read
/// as bytes is the whole file.
pub fn wanted(range: Option<&str>, size: u64) -> Wanted {
  let Some(range) = range.and_then(|range| range.trim().strip_prefix("bytes=")) else {
    return Wanted::Whole;
  };
  let first = range.split(',').next().unwrap_or("").trim();
  let Some((from, to)) = first.split_once('-') else {
    return Wanted::Whole;
  };
  let (from, to) = (from.trim(), to.trim());
  if from.is_empty() {
    let Ok(suffix) = to.parse::<u64>() else {
      return Wanted::Whole;
    };
    if suffix == 0 || size == 0 {
      return Wanted::Unsatisfiable;
    }
    return Wanted::Stretch(size.saturating_sub(suffix), size - 1);
  }
  let Ok(from) = from.parse::<u64>() else {
    return Wanted::Whole;
  };
  if from >= size {
    return Wanted::Unsatisfiable;
  }
  let last = if to.is_empty() {
    size - 1
  } else {
    match to.parse::<u64>() {
      Ok(to) if to >= from => to.min(size - 1),
      _ => return Wanted::Whole,
    }
  };
  Wanted::Stretch(from, last)
}

/// The strong tag of a file: its size and its modification time.
pub fn tag(size: u64, modified_ns: u128) -> String {
  format!("\"{size:x}-{modified_ns:x}\"")
}

fn header_text(headers: &HeaderMap, name: header::HeaderName) -> Option<&str> {
  headers.get(name)?.to_str().ok()
}

/// The file at `path` answered to a request's headers, or none where it is not on disk.
pub async fn serve(
  path: &Path,
  media_type: &str,
  headers: &HeaderMap,
) -> std::io::Result<Option<Response>> {
  let mut file = match tokio::fs::File::open(path).await {
    Ok(file) => file,
    Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(why) => return Err(why),
  };
  let metadata = file.metadata().await?;
  if !metadata.is_file() {
    return Ok(None);
  }
  let size = metadata.len();
  let modified = metadata
    .modified()
    .ok()
    .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
    .map(|at| at.as_nanos());
  let etag = tag(size, modified.unwrap_or_default());
  let common = |status: StatusCode| {
    let mut response = Response::builder()
      .status(status)
      .header(header::ACCEPT_RANGES, "bytes")
      .header(header::ETAG, &etag)
      .header(header::CACHE_CONTROL, "private, no-cache");
    if status != StatusCode::NOT_MODIFIED {
      response = response.header(header::CONTENT_TYPE, media_type);
    }
    response
  };
  if header_text(headers, header::IF_NONE_MATCH)
    .is_some_and(|tags| tags.split(',').any(|given| given.trim().trim_start_matches("W/") == etag))
  {
    return Ok(Some(
      common(StatusCode::NOT_MODIFIED).body(Body::empty()).map_err(std::io::Error::other)?,
    ));
  }
  let range = header_text(headers, header::RANGE)
    .filter(|_| header_text(headers, header::IF_RANGE).is_none_or(|given| given.trim() == etag));
  let (status, first, last) = match wanted(range, size) {
    Wanted::Whole => (StatusCode::OK, 0, size.saturating_sub(1)),
    Wanted::Stretch(first, last) => (StatusCode::PARTIAL_CONTENT, first, last),
    Wanted::Unsatisfiable => {
      let mut response = common(StatusCode::RANGE_NOT_SATISFIABLE)
        .header(header::CONTENT_RANGE, format!("bytes */{size}"))
        .body(Body::empty())
        .map_err(std::io::Error::other)?;
      response.headers_mut().remove(header::CONTENT_TYPE);
      return Ok(Some(response));
    }
  };
  let length = if size == 0 { 0 } else { last - first + 1 };
  let mut response = common(status).header(header::CONTENT_LENGTH, length);
  if status == StatusCode::PARTIAL_CONTENT {
    response = response.header(header::CONTENT_RANGE, format!("bytes {first}-{last}/{size}"));
  }
  file.seek(SeekFrom::Start(first)).await?;
  let body = stream::unfold((file, length), |(mut file, left)| async move {
    if left == 0 {
      return None;
    }
    let mut buffer = vec![0u8; CHUNK.min(left as usize)];
    match file.read(&mut buffer).await {
      Ok(0) => None,
      Ok(read) => {
        buffer.truncate(read);
        Some((Ok::<_, std::io::Error>(buffer), (file, left - read as u64)))
      }
      Err(why) => Some((Err(why), (file, 0))),
    }
  });
  Ok(Some(response.body(Body::from_stream(body)).map_err(std::io::Error::other)?))
}

/// Bytes answered whole under a media type, kept by nobody.
pub fn bytes(media_type: &'static str, bytes: Vec<u8>) -> Response {
  let mut response = bytes.into_response();
  response
    .headers_mut()
    .insert(header::CONTENT_TYPE, HeaderValue::from_static(media_type));
  response
    .headers_mut()
    .insert(header::CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
  response
}
