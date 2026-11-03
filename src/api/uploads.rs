//! A file handed to a route that files one (`docs/api.md`, 「Titles」, 「Subtitles」): the bytes of a multipart
//! upload, written into the file root as they arrive, or a path on the service's own host.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use axum::body::Bytes;
use axum::extract::{FromRequest, Multipart, Request};
use axum::http::header;
use rand::Rng;
use tokio::io::AsyncWriteExt;

use super::Answer;
use crate::app::App;
use crate::error::ApiError;

/// A file in the uploads folder, removed when it goes unless it was kept.
pub struct Temporary {
  pub path: PathBuf,
  kept: bool,
}

impl Temporary {
  /// A new name in the uploads folder.
  pub async fn new(app: &App) -> Answer<Self> {
    tokio::fs::create_dir_all(app.uploads()).await?;
    let name = format!("{:032x}", rand::rng().random::<u128>());
    Ok(Self {
      path: app.uploads().join(name),
      kept: false,
    })
  }

  /// The file let stand, moved elsewhere by its holder.
  pub fn keep(mut self) -> PathBuf {
    self.kept = true;
    self.path.clone()
  }
}

impl Drop for Temporary {
  fn drop(&mut self) {
    if !self.kept {
      let _ = std::fs::remove_file(&self.path);
    }
  }
}

/// What a route that files a file was handed.
pub enum Handed {
  /// The bytes of a multipart upload, already in the uploads folder, under the name they were sent with, and
  /// the upload's text parts.
  Upload {
    file: Temporary,
    file_name: String,
    fields: HashMap<String, String>,
  },
  /// A JSON body.
  Json(Bytes),
}

/// A file's name as a sender gave it: its last part, with no character a path would read.
pub fn file_name_of(given: &str) -> String {
  let last = given.rsplit(['/', '\\']).next().unwrap_or("").trim();
  let clean: String = last.chars().filter(|character| !character.is_control()).collect();
  if clean.is_empty() || clean == "." || clean == ".." {
    "upload".to_owned()
  } else {
    clean
  }
}

/// The request read: a multipart upload's file streamed into the uploads folder, else its JSON body.
pub async fn receive(app: &App, request: Request) -> Answer<Handed> {
  let multipart = request
    .headers()
    .get(header::CONTENT_TYPE)
    .and_then(|value| value.to_str().ok())
    .is_some_and(|value| value.to_ascii_lowercase().starts_with("multipart/form-data"));
  if !multipart {
    let bytes = Bytes::from_request(request, &())
      .await
      .map_err(|why| ApiError::bad_request(format!("The body could not be read: {why}")))?;
    return Ok(Handed::Json(bytes));
  }
  let mut parts = Multipart::from_request(request, &())
    .await
    .map_err(|why| ApiError::bad_request(format!("The upload does not read: {why}")))?;
  let mut file: Option<(Temporary, String)> = None;
  let mut fields = HashMap::new();
  while let Some(mut part) = parts
    .next_field()
    .await
    .map_err(|why| ApiError::bad_request(format!("The upload broke off: {why}")))?
  {
    let name = part.name().unwrap_or_default().to_owned();
    if name == "file" && file.is_none() {
      let file_name = file_name_of(part.file_name().unwrap_or_default());
      let temporary = Temporary::new(app).await?;
      let mut written = tokio::fs::File::create(&temporary.path).await?;
      while let Some(chunk) = part
        .chunk()
        .await
        .map_err(|why| ApiError::bad_request(format!("The upload broke off: {why}")))?
      {
        written.write_all(&chunk).await?;
      }
      written.flush().await?;
      file = Some((temporary, file_name));
    } else {
      let text = part
        .text()
        .await
        .map_err(|why| ApiError::bad_request(format!("The part {name} does not read: {why}")))?;
      fields.insert(name, text);
    }
  }
  let (file, file_name) =
    file.ok_or_else(|| ApiError::bad_request("The upload carries no part named file."))?;
  Ok(Handed::Upload {
    file,
    file_name,
    fields,
  })
}

/// A path on the service's own host checked as an absolute file it reads, or `not-a-file`.
pub async fn local_file(path: &str) -> Answer<PathBuf> {
  let path = Path::new(path);
  let readable = path.is_absolute()
    && tokio::fs::metadata(path).await.is_ok_and(|metadata| metadata.is_file())
    && tokio::fs::File::open(path).await.is_ok();
  if readable {
    Ok(path.to_path_buf())
  } else {
    Err(ApiError::of(
      "not-a-file",
      format!("{} is not an absolute file the service reads.", path.display()),
    ))
  }
}

/// A local file copied into the uploads folder.
pub async fn copy_in(app: &App, path: &Path) -> Answer<Temporary> {
  let temporary = Temporary::new(app).await?;
  tokio::fs::copy(path, &temporary.path).await?;
  Ok(temporary)
}

/// A text part read as a whole number, where it is given.
pub fn number_field(fields: &HashMap<String, String>, name: &str) -> Answer<Option<i32>> {
  match fields.get(name).map(|value| value.trim()).filter(|value| !value.is_empty()) {
    None => Ok(None),
    Some(value) => value
      .parse()
      .map(Some)
      .map_err(|_| ApiError::bad_request(format!("{name} is no whole number"))),
  }
}

/// A text part, where it is given and not blank.
pub fn text_field(fields: &HashMap<String, String>, name: &str) -> Option<String> {
  fields.get(name).map(|value| value.trim().to_owned()).filter(|value| !value.is_empty())
}
