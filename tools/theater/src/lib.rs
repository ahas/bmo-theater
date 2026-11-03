//! The theater tool (`docs/api.md`, 「The tool」): what the tool host hands a call and what it answers
//! (bmo-pocket's `docs/mcp-host.md`, 「A call」), and each entry as one or a few calls of theater routes, made with
//! the service token and the owner's account the deployment writes into the data root's `env/theater.env`.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// The protocol version every manifest and every handed object names.
pub const PROTOCOL: u32 = 1;

/// The entries this executable answers, as its manifest declares them.
pub const ENTRIES: [&str; 8] = [
  "theater_titles",
  "theater_show",
  "theater_file",
  "theater_subtitle_add",
  "theater_playlist_make",
  "theater_playlist_add",
  "theater_progress",
  "theater_burn",
];

/// The data root handed in the record's variables.
pub const DATA_ROOT_VARIABLE: &str = "BMO_DATA_DIR";
/// The env file under the data root the service's address, token and account are read from.
pub const ENV_FILE: &str = "env/theater.env";

/// One session's record, as much of it as the tool reads.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
  #[serde(default)]
  pub variables: BTreeMap<String, String>,
}

/// One call handed by the host.
#[derive(Clone, Debug, Deserialize)]
pub struct Handed {
  pub protocol: u32,
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub entry: String,
  #[serde(default)]
  pub arguments: Value,
  #[serde(default)]
  pub session: Session,
}

/// The host's tool result.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Answer {
  pub content: Vec<Value>,
  #[serde(rename = "isError")]
  pub is_error: bool,
  #[serde(rename = "structuredContent", skip_serializing_if = "Option::is_none")]
  pub structured: Option<Value>,
}

impl Answer {
  /// Lines a person reads, and the value a program reads beside them.
  pub fn reading(lines: Vec<String>, structured: Value) -> Self {
    let text = if lines.is_empty() { "Nothing stands there.".to_owned() } else { lines.join("\n") };
    Self {
      content: vec![json!({ "type": "text", "text": text })],
      is_error: false,
      structured: Some(structured),
    }
  }

  /// A failure saying why in English.
  pub fn refusal(why: impl Into<String>) -> Self {
    Self {
      content: vec![json!({ "type": "text", "text": why.into() })],
      is_error: true,
      structured: None,
    }
  }

  /// The text the answer carries.
  pub fn text(&self) -> String {
    self
      .content
      .iter()
      .filter_map(|piece| piece["text"].as_str())
      .collect::<Vec<_>>()
      .join("\n")
  }
}

/// The theater service as the tool reaches it, and the account it acts for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Service {
  pub url: String,
  pub token: String,
  pub account: Option<String>,
}

/// The data root: the record's, else this process's, else the deployment's conventional one under the home.
pub fn data_root(session: &Session) -> PathBuf {
  session
    .variables
    .get(DATA_ROOT_VARIABLE)
    .map(PathBuf::from)
    .or_else(|| std::env::var(DATA_ROOT_VARIABLE).ok().map(PathBuf::from))
    .or_else(|| std::env::var("HOME").ok().map(|home| PathBuf::from(home).join(".local/share/bmo/data")))
    .unwrap_or_else(|| PathBuf::from("/nonexistent"))
}

fn env_file(path: &Path) -> BTreeMap<String, String> {
  std::fs::read_to_string(path)
    .map(|text| {
      text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_owned(), value.trim().to_owned()))
        .collect()
    })
    .unwrap_or_default()
}

/// The service: `THEATER_ADDRESS`, `THEATER_TOKEN` and `THEATER_ACCOUNT` of the call's environment, else of the
/// data root's `env/theater.env`.
pub fn service(session: &Session) -> Result<Service, String> {
  let file = env_file(&data_root(session).join(ENV_FILE));
  let variable = |name: &str| {
    std::env::var(name)
      .ok()
      .or_else(|| file.get(name).cloned())
      .map(|value| value.trim().to_owned())
      .filter(|value| !value.is_empty())
  };
  match (variable("THEATER_ADDRESS"), variable("THEATER_TOKEN")) {
    (Some(url), Some(token)) => Ok(Service {
      url: url.trim_end_matches('/').to_owned(),
      token,
      account: variable("THEATER_ACCOUNT"),
    }),
    _ => Err(format!(
      "the theater's address or token is set nowhere: no THEATER_ADDRESS and THEATER_TOKEN in {ENV_FILE}"
    )),
  }
}

/// The last part of a full name, `<namespace>/<name>`.
pub fn entry_name(entry: &str) -> &str {
  entry.rsplit('/').next().unwrap_or(entry)
}

/// One call of a route: its answer as JSON, or the service's refusal in words.
async fn call(
  service: &Service,
  method: reqwest::Method,
  path: &str,
  query: &[(&str, String)],
  body: Option<&Value>,
) -> Result<Value, String> {
  let mut request = reqwest::Client::builder()
    .no_proxy()
    .build()
    .map_err(|why| format!("no HTTP client: {why}"))?
    .request(method, format!("{}{path}", service.url))
    .bearer_auth(&service.token)
    .query(query);
  if let Some(account) = &service.account {
    request = request.header("Owner-Key", account);
  }
  if let Some(body) = body {
    request = request.json(body);
  }
  let response = request.send().await.map_err(|why| format!("the theater could not be reached: {why}"))?;
  let status = response.status();
  let text = response
    .text()
    .await
    .map_err(|why| format!("the theater's answer could not be read: {why}"))?;
  if !status.is_success() {
    let said: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let code = said["code"].as_str().unwrap_or("error");
    let error = said["error"].as_str().unwrap_or(text.as_str());
    return Err(format!("{error} ({code})"));
  }
  if text.trim().is_empty() {
    return Ok(Value::Null);
  }
  serde_json::from_str(&text).map_err(|why| format!("the theater's answer does not read: {why}"))
}

fn text<'a>(arguments: &'a Value, name: &str) -> Option<&'a str> {
  arguments[name].as_str().map(str::trim).filter(|value| !value.is_empty())
}

fn number(arguments: &Value, name: &str) -> Option<i64> {
  arguments[name].as_i64()
}

fn needed(arguments: &Value, name: &str) -> Result<i64, String> {
  number(arguments, name).ok_or_else(|| format!("{name} is required"))
}

/// A duration in milliseconds as minutes and seconds.
pub fn clock(ms: Option<i64>) -> String {
  match ms {
    Some(ms) => format!("{}:{:02}", ms / 60_000, ms / 1000 % 60),
    None => "?".to_owned(),
  }
}

/// One title as one line.
pub fn title_line(title: &Value) -> String {
  format!(
    "title {} · {} · {} · {} · {} episodes",
    title["id"],
    title["name"].as_str().unwrap_or(""),
    title["kind"].as_str().unwrap_or(""),
    title["realm"].as_str().unwrap_or(""),
    title["episodeCount"],
  )
}

/// One episode as one line.
pub fn episode_line(episode: &Value) -> String {
  let numbers = match (episode["season"].as_i64(), episode["number"].as_i64()) {
    (Some(season), Some(number)) => format!("S{season:02}E{number:02} "),
    (None, Some(number)) => format!("E{number:02} "),
    _ => String::new(),
  };
  let mut line = format!(
    "episode {} · {numbers}{} · {}",
    episode["id"],
    episode["name"].as_str().unwrap_or(""),
    clock(episode["durationMs"].as_i64())
  );
  if let Some(label) = episode["label"].as_str().filter(|label| !label.is_empty()) {
    line.push_str(&format!(" · {label}"));
  }
  if episode["position"]["done"] == true {
    line.push_str(" · watched");
  } else if let Some(seconds) = episode["position"]["seconds"].as_f64() {
    line.push_str(&format!(" · watched to {}", clock(Some((seconds * 1000.0) as i64))));
  }
  if episode["available"] == false {
    line.push_str(" · file missing");
  }
  line
}

/// One answer for every entry: run against the service.
pub async fn answer(handed: &Handed, service: &Service) -> Answer {
  match answered(handed, service).await {
    Ok(answer) => answer,
    Err(why) => Answer::refusal(why),
  }
}

fn realm_of(arguments: &Value) -> Result<&str, String> {
  match text(arguments, "realm").unwrap_or("mine") {
    realm @ ("mine" | "organization") => Ok(realm),
    _ => Err("realm is mine or organization".into()),
  }
}

/// The title a filing names: by id, else by its exact name in the realm, made as a clip title where none bears it.
async fn title_for(service: &Service, arguments: &Value) -> Result<Value, String> {
  use reqwest::Method;
  if let Some(id) = number(arguments, "titleId") {
    return call(service, Method::GET, &format!("/titles/{id}"), &[], None).await;
  }
  let name = text(arguments, "title").ok_or("titleId or title names the title")?;
  let realm = realm_of(arguments)?;
  let query = [("realm", realm.to_owned()), ("q", name.to_owned()), ("limit", "1000".to_owned())];
  let page = call(service, Method::GET, "/titles", &query, None).await?;
  if let Some(found) = page["items"]
    .as_array()
    .and_then(|items| items.iter().find(|title| title["name"].as_str() == Some(name)))
  {
    return Ok(found.clone());
  }
  call(
    service,
    Method::POST,
    "/titles",
    &[],
    Some(&json!({ "realm": realm, "name": name, "kind": "clip" })),
  )
  .await
}

async fn answered(handed: &Handed, service: &Service) -> Result<Answer, String> {
  use reqwest::Method;

  let arguments = &handed.arguments;
  match entry_name(&handed.entry) {
    "theater_titles" => {
      let mut query = Vec::new();
      if let Some(realm) = text(arguments, "realm") {
        query.push(("realm", realm.to_owned()));
      }
      if let Some(kind) = text(arguments, "kind") {
        query.push(("kind", kind.to_owned()));
      }
      if let Some(part) = text(arguments, "query") {
        query.push(("q", part.to_owned()));
      }
      let page = call(service, Method::GET, "/titles", &query, None).await?;
      let lines = page["items"]
        .as_array()
        .map(|items| items.iter().map(title_line).collect())
        .unwrap_or_default();
      Ok(Answer::reading(lines, page))
    }
    "theater_show" => {
      if let Some(id) = number(arguments, "titleId") {
        let title = call(service, Method::GET, &format!("/titles/{id}"), &[], None).await?;
        let episodes = call(service, Method::GET, &format!("/titles/{id}/episodes"), &[], None).await?;
        let mut lines = vec![title_line(&title)];
        if let Some(description) = title["description"].as_str().filter(|text| !text.is_empty()) {
          lines.push(description.to_owned());
        }
        lines.extend(episodes["items"].as_array().into_iter().flatten().map(episode_line));
        let mut structured = title.clone();
        structured["episodes"] = episodes["items"].clone();
        return Ok(Answer::reading(lines, structured));
      }
      let id =
        needed(arguments, "episodeId").map_err(|_| "titleId or episodeId names what to show".to_owned())?;
      let episode = call(service, Method::GET, &format!("/episodes/{id}"), &[], None).await?;
      let subtitles = call(service, Method::GET, &format!("/episodes/{id}/subtitles"), &[], None).await?;
      let mut lines = vec![episode_line(&episode)];
      for subtitle in subtitles["items"].as_array().into_iter().flatten() {
        lines.push(format!(
          "subtitle {} · {} · {} · {} cues",
          subtitle["id"],
          subtitle["language"].as_str().unwrap_or(""),
          subtitle["format"].as_str().unwrap_or(""),
          subtitle["cueCount"]
        ));
      }
      let mut structured = episode.clone();
      structured["subtitles"] = subtitles["items"].clone();
      Ok(Answer::reading(lines, structured))
    }
    "theater_file" => {
      let path = text(arguments, "path").ok_or("path is the local video file")?;
      let title = title_for(service, arguments).await?;
      let id = title["id"].as_i64().ok_or("the title has no id")?;
      let mut body = json!({ "path": path });
      for field in ["name", "label"] {
        if let Some(value) = text(arguments, field) {
          body[field] = json!(value);
        }
      }
      let episode =
        call(service, Method::POST, &format!("/titles/{id}/episodes"), &[], Some(&body)).await?;
      let mut lines = vec![format!("Filed under {}.", title_line(&title)), episode_line(&episode)];
      let mut structured = episode.clone();
      if arguments["share"] == true {
        let share =
          call(service, Method::POST, "/shares", &[], Some(&json!({ "episodeId": episode["id"] }))).await?;
        match share["url"].as_str() {
          Some(url) => lines.push(format!("Share link: {url}")),
          None => lines.push(format!("Share token: {}", share["token"].as_str().unwrap_or(""))),
        }
        structured["share"] = share;
      }
      Ok(Answer::reading(lines, structured))
    }
    "theater_subtitle_add" => {
      let id = needed(arguments, "episodeId")?;
      let path = text(arguments, "path").ok_or("path is the local subtitle file")?;
      let mut body = json!({ "path": path });
      for field in ["language", "label"] {
        if let Some(value) = text(arguments, field) {
          body[field] = json!(value);
        }
      }
      let subtitle =
        call(service, Method::POST, &format!("/episodes/{id}/subtitles"), &[], Some(&body)).await?;
      let line = format!(
        "Added subtitle {} ({}, {} cues) to episode {id}.",
        subtitle["id"],
        subtitle["language"].as_str().unwrap_or(""),
        subtitle["cueCount"]
      );
      Ok(Answer::reading(vec![line], subtitle))
    }
    "theater_playlist_make" => {
      let name = text(arguments, "name").ok_or("name is the playlist's name")?;
      let body = json!({
        "realm": realm_of(arguments)?,
        "name": name,
        "episodeIds": arguments.get("episodeIds").cloned().unwrap_or(json!([])),
      });
      let playlist = call(service, Method::POST, "/playlists", &[], Some(&body)).await?;
      Ok(Answer::reading(vec![playlist_line(&playlist)], playlist))
    }
    "theater_playlist_add" => {
      let id = needed(arguments, "playlistId")?;
      let mut body = json!({ "episodeIds": arguments.get("episodeIds").cloned().unwrap_or(json!([])) });
      if let Some(at) = number(arguments, "at") {
        body["at"] = json!(at);
      }
      let playlist =
        call(service, Method::POST, &format!("/playlists/{id}/items"), &[], Some(&body)).await?;
      Ok(Answer::reading(vec![playlist_line(&playlist)], playlist))
    }
    "theater_progress" => {
      let path = match (number(arguments, "playlistId"), number(arguments, "titleId")) {
        (Some(id), _) => format!("/playlists/{id}/progress"),
        (None, Some(id)) => format!("/titles/{id}/progress"),
        _ => return Err("playlistId or titleId names what to read".into()),
      };
      let progress = call(service, Method::GET, &path, &[], None).await?;
      let items = progress["items"].as_array().cloned().unwrap_or_default();
      let mut lines = vec![format!("{} of {} watched.", progress["doneCount"], items.len())];
      match progress["next"].as_i64() {
        Some(next) => lines.push(format!(
          "Next: episode {next}, from {}.",
          clock(progress["nextSeconds"].as_f64().map(|seconds| (seconds * 1000.0) as i64))
        )),
        None => lines.push("Nothing is left to watch.".to_owned()),
      }
      for item in &items {
        let state = if item["done"] == true {
          "watched".to_owned()
        } else {
          format!("at {}", clock(item["seconds"].as_f64().map(|seconds| (seconds * 1000.0) as i64)))
        };
        lines.push(format!(
          "episode {} · {state} of {}",
          item["episodeId"],
          clock(item["durationMs"].as_i64())
        ));
      }
      Ok(Answer::reading(lines, progress))
    }
    "theater_burn" => {
      let id = needed(arguments, "episodeId")?;
      let subtitle = needed(arguments, "subtitleId")?;
      let burn = call(
        service,
        Method::POST,
        &format!("/episodes/{id}/burns"),
        &[],
        Some(&json!({ "subtitleId": subtitle })),
      )
      .await?;
      let line = format!(
        "Burn {} started; the burned video is filed as a clip under the same title when it ends.",
        burn["id"]
      );
      Ok(Answer::reading(vec![line], burn))
    }
    other => Err(format!("the theater tool answers no entry {other}")),
  }
}

fn playlist_line(playlist: &Value) -> String {
  format!(
    "playlist {} · {} · {} · {} episodes · {}",
    playlist["id"],
    playlist["name"].as_str().unwrap_or(""),
    playlist["realm"].as_str().unwrap_or(""),
    playlist["episodeIds"].as_array().map(Vec::len).unwrap_or_default(),
    clock(playlist["durationMs"].as_i64())
  )
}

/// The one call this process was handed, answered on standard output.
pub async fn run() -> std::process::ExitCode {
  let mut text = String::new();
  let handed: Result<Handed, String> = std::io::stdin()
    .lock()
    .read_to_string(&mut text)
    .map_err(|why| format!("the call could not be read: {why}"))
    .and_then(|_| serde_json::from_str(&text).map_err(|why| format!("the call does not read: {why}")));
  let answer = match handed {
    Ok(handed) if handed.protocol != PROTOCOL => {
      Answer::refusal(format!("protocol {} is not spoken", handed.protocol))
    }
    Ok(handed) => match service(&handed.session) {
      Ok(service) => answer(&handed, &service).await,
      Err(why) => Answer::refusal(why),
    },
    Err(why) => Answer::refusal(why),
  };
  let text = serde_json::to_string(&answer).unwrap_or_default();
  match std::io::stdout().lock().write_all(text.as_bytes()) {
    Ok(()) => std::process::ExitCode::SUCCESS,
    Err(why) => {
      eprintln!("the answer could not be written: {why}");
      std::process::ExitCode::FAILURE
    }
  }
}
