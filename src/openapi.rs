//! The OpenAPI document, `docs/openapi.json`: every route `docs/api.md` states, with its parameters, its body,
//! its answers and the codes it can give, the schemas generated from the service's own types, the events it
//! publishes, and the tool's entries with the input schemas the tool's folder holds.

use std::collections::BTreeMap;

use schemars::generate::{SchemaGenerator, SchemaSettings};
use schemars::{JsonSchema, Schema};
use serde_json::{Map, Value, json};

use crate::error::CODES;
use crate::events::KINDS;
use crate::wire::{
  Burn, Deleted, Episode, EpisodeChange, EventsQuery, FiledByPath, Grant, GrantChange, HeadQuery, Health,
  Library, LibraryChange, NewBurn, NewGrants, NewLibrary, NewPlaylist, NewShare, NewTitle, Page, PageQuery,
  Playlist, PlaylistChange, PlaylistItems, Position, PositionSet, Progress, RealmQuery, Scan, Share,
  ShareDeleted, ShareHead, SharesQuery, StreamEvent, Subtitle, SubtitleByPath, Title, TitleChange,
  TitlesQuery, WireError,
};

const SCHEMAS: &str = "/components/schemas";

const DESCRIPTION: &str = "The platform's store of videos: titles made of episodes, filed outputs kept in the \
  service's file root and libraries of video files indexed in place, streamed as they are with HTTP ranges, with \
  subtitles converted to WebVTT and burned in on request, playlists, each viewer's watch position, and share links. \
  Titles, libraries and playlists stand in one of two realms, an account's own (mine) or the organization's. Bodies \
  are JSON with camelCase keys, times are epoch milliseconds, lists are pages of items and more, and every failure \
  is the error shape with its status. The tool's entries, placed on the tool host, are stated under x-tools.";

/// The tool's entries: name, description, the routes it calls, and the answer it carries.
pub const TOOLS: [(&str, &str, &str, &str); 12] = [
  (
    "theater_titles",
    "Finds titles by realm, kind and a part of the name.",
    "GET /titles",
    "titles",
  ),
  (
    "theater_show",
    "A title with its episodes, or an episode with its subtitles.",
    "GET /titles/{titleId}",
    "title",
  ),
  (
    "theater_file",
    "Files a local video under a title, made as a clip title where the name is new; with share, answers a link.",
    "POST /titles/{titleId}/episodes",
    "episode",
  ),
  (
    "theater_work_rename",
    "Renames a work without filing it again.",
    "PATCH /episodes/{episodeId}",
    "episode",
  ),
  (
    "theater_work_move",
    "Moves a work under another title or into a playlist; nothing is deleted.",
    "PATCH /episodes/{episodeId}",
    "episode",
  ),
  (
    "theater_work_delete",
    "Deletes a filed or burned work and its file; a library's work is refused.",
    "DELETE /episodes/{episodeId}",
    "deleted",
  ),
  (
    "theater_playlist_remove",
    "Takes a work out of a playlist, leaving the work and the playlist standing.",
    "PATCH /playlists/{playlistId}",
    "playlist",
  ),
  (
    "theater_subtitle_add",
    "Adds a local subtitle file to an episode.",
    "POST /episodes/{episodeId}/subtitles",
    "subtitle",
  ),
  ("theater_playlist_make", "Makes a playlist.", "POST /playlists", "playlist"),
  (
    "theater_playlist_add",
    "Adds episodes to a playlist.",
    "POST /playlists/{playlistId}/items",
    "playlist",
  ),
  (
    "theater_progress",
    "How far the owner has come through a playlist or a title, and what is next.",
    "GET /playlists/{playlistId}/progress",
    "progress",
  ),
  (
    "theater_burn",
    "Burns a subtitle into an episode; the clip is filed when the burn ends.",
    "POST /episodes/{episodeId}/burns",
    "burn",
  ),
];

/// The input schema of a tool entry, as the tool's folder holds it.
pub fn tool_input(name: &str) -> Value {
  let text = match name {
    "theater_titles" => include_str!("../tools/theater/theater_titles.json"),
    "theater_show" => include_str!("../tools/theater/theater_show.json"),
    "theater_file" => include_str!("../tools/theater/theater_file.json"),
    "theater_work_rename" => include_str!("../tools/theater/theater_work_rename.json"),
    "theater_work_move" => include_str!("../tools/theater/theater_work_move.json"),
    "theater_work_delete" => include_str!("../tools/theater/theater_work_delete.json"),
    "theater_playlist_remove" => include_str!("../tools/theater/theater_playlist_remove.json"),
    "theater_subtitle_add" => include_str!("../tools/theater/theater_subtitle_add.json"),
    "theater_playlist_make" => include_str!("../tools/theater/theater_playlist_make.json"),
    "theater_playlist_add" => include_str!("../tools/theater/theater_playlist_add.json"),
    "theater_progress" => include_str!("../tools/theater/theater_progress.json"),
    "theater_burn" => include_str!("../tools/theater/theater_burn.json"),
    _ => "null",
  };
  serde_json::from_str(text).unwrap_or(Value::Null)
}

/// How far the console's credential reaches a route: with the reader's key and role, or with the credential
/// alone.
#[derive(Clone, Copy)]
enum Reach {
  Screen,
  Open,
}

struct Route {
  method: &'static str,
  path: &'static str,
  id: &'static str,
  tag: &'static str,
  summary: &'static str,
  parameters: Vec<Value>,
  body: Option<Value>,
  answers: Vec<(u16, Option<Value>)>,
  codes: Vec<&'static str>,
  reach: Reach,
}

fn error_answer(error: &Schema, codes: &[&str]) -> Value {
  let description = codes
    .iter()
    .map(|code| {
      let meaning =
        CODES.iter().find(|(name, ..)| name == code).map(|(.., meaning)| *meaning).unwrap_or("");
      format!("`{code}`: {meaning}")
    })
    .collect::<Vec<_>>()
    .join("\n");
  json!({ "description": description, "content": { "application/json": { "schema": error } } })
}

impl Route {
  fn operation(self, error: &Schema) -> Value {
    let mut codes = vec!["unauthorized"];
    if matches!(self.reach, Reach::Screen) {
      codes.push("forbidden");
    }
    codes.extend(self.codes);
    codes.sort_by_key(|code| CODES.iter().position(|(name, ..)| name == code));
    codes.dedup();
    let mut statuses: BTreeMap<u16, Vec<&str>> = BTreeMap::new();
    for code in &codes {
      let status = CODES
        .iter()
        .find(|(name, ..)| name == code)
        .map(|(_, status, _)| *status)
        .unwrap_or(500);
      statuses.entry(status).or_default().push(code);
    }
    let mut responses = Map::new();
    for (status, content) in self.answers {
      let description = match status {
        201 => "Created.",
        204 => "Done; no content.",
        206 => "A stretch of the bytes.",
        304 => "The caller holds the current bytes.",
        416 => "The stretch starts past the end.",
        _ => "Answered.",
      };
      let mut answer = json!({ "description": description });
      if let Some(content) = content {
        answer["content"] = content;
      }
      responses.insert(status.to_string(), answer);
    }
    for (status, codes) in statuses {
      responses.insert(status.to_string(), error_answer(error, &codes));
    }
    let mut operation = json!({
      "operationId": self.id,
      "tags": [self.tag],
      "summary": self.summary,
      "responses": responses,
      "x-codes": codes,
      "x-console": match self.reach { Reach::Screen => "screen", Reach::Open => "open" },
    });
    if !self.parameters.is_empty() {
      operation["parameters"] = self.parameters.into();
    }
    if let Some(body) = self.body {
      operation["requestBody"] = json!({ "required": true, "content": body });
    }
    operation
  }
}

fn content<T: JsonSchema>(g: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({ "application/json": { "schema": g.subschema_for::<T>() } }))
}

fn json_body<T: JsonSchema>(g: &mut SchemaGenerator) -> Option<Value> {
  Some(json!({ "application/json": { "schema": g.subschema_for::<T>() } }))
}

/// A body that is a multipart upload or a path on the service's own host.
fn upload_body<T: JsonSchema>(g: &mut SchemaGenerator, fields: &[&str]) -> Option<Value> {
  let mut properties = Map::new();
  properties.insert(
    "file".into(),
    json!({ "type": "string", "contentMediaType": "application/octet-stream" }),
  );
  for field in fields {
    properties.insert((*field).into(), json!({ "type": "string" }));
  }
  Some(json!({
    "multipart/form-data": { "schema": { "type": "object", "required": ["file"], "properties": properties } },
    "application/json": { "schema": g.subschema_for::<T>() },
  }))
}

fn bytes(media: &[&str]) -> Option<Value> {
  let mut types = Map::new();
  for media in media {
    types.insert((*media).into(), json!({ "schema": { "type": "string", "contentMediaType": media } }));
  }
  Some(Value::Object(types))
}

fn path_id(name: &str, description: &str) -> Value {
  json!({
    "name": name,
    "in": "path",
    "required": true,
    "description": description,
    "schema": { "type": "integer", "format": "int64" },
  })
}

fn path_text(name: &str, description: &str) -> Value {
  json!({ "name": name, "in": "path", "required": true, "description": description, "schema": { "type": "string" } })
}

fn header(name: &str, description: &str) -> Value {
  json!({ "name": name, "in": "header", "required": false, "description": description, "schema": { "type": "string" } })
}

fn range_headers() -> Vec<Value> {
  vec![
    header("Range", "One stretch of bytes: bytes=first-last, bytes=first- or bytes=-suffix."),
    header("If-Range", "The tag a Range holds to; another answers the whole file."),
    header("If-None-Match", "The tag the caller holds; the current one answers 304."),
  ]
}

fn query<Q: JsonSchema>(g: &mut SchemaGenerator) -> Vec<Value> {
  let schema = Q::json_schema(g).to_value();
  let required: Vec<&str> = schema["required"]
    .as_array()
    .map(|names| names.iter().filter_map(Value::as_str).collect())
    .unwrap_or_default();
  let Some(properties) = schema["properties"].as_object() else {
    return Vec::new();
  };
  properties
    .iter()
    .map(|(name, property)| {
      let mut parameter = json!({
        "name": name,
        "in": "query",
        "required": required.contains(&name.as_str()),
        "schema": property,
      });
      if let Some(description) = property.get("description") {
        parameter["description"] = description.clone();
      }
      parameter
    })
    .collect()
}

fn with(mut first: Vec<Value>, rest: Vec<Value>) -> Vec<Value> {
  first.extend(rest);
  first
}

#[allow(clippy::too_many_arguments)]
fn route(
  method: &'static str,
  path: &'static str,
  id: &'static str,
  tag: &'static str,
  summary: &'static str,
  parameters: Vec<Value>,
  body: Option<Value>,
  answers: Vec<(u16, Option<Value>)>,
  codes: Vec<&'static str>,
) -> Route {
  Route {
    method,
    path,
    id,
    tag,
    summary,
    parameters,
    body,
    answers,
    codes,
    reach: Reach::Screen,
  }
}

fn open(mut route: Route) -> Route {
  route.reach = Reach::Open;
  route
}

fn routes(g: &mut SchemaGenerator) -> Vec<Route> {
  let library = || path_id("libraryId", "The library's id.");
  let title = || path_id("titleId", "The title's id.");
  let episode = || path_id("episodeId", "The episode's id.");
  let playlist = || path_id("playlistId", "The playlist's id.");
  let token = || path_text("token", "The share link's token.");
  let user = || path_text("user", "The account key the episode is granted to.");
  let streamed = bytes(&[
    "video/mp4",
    "video/webm",
    "video/x-matroska",
    "video/quicktime",
    "video/x-msvideo",
    "video/ogg",
  ]);
  let jpeg = bytes(&["image/jpeg"]);
  let vtt = bytes(&["text/vtt"]);
  let read = vec!["not-found"];
  vec![
    open(route(
      "get",
      "/health",
      "readHealth",
      "health",
      "Whether the service answers, and its version.",
      vec![],
      None,
      vec![(200, content::<Health>(g))],
      vec![],
    )),
    route(
      "get",
      "/libraries",
      "listLibraries",
      "libraries",
      "The libraries the caller reads.",
      query::<RealmQuery>(g),
      None,
      vec![(200, content::<Page<Library>>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "post",
      "/libraries",
      "createLibrary",
      "libraries",
      "Register a folder as a library; its first scan starts. The administrator alone.",
      vec![],
      json_body::<NewLibrary>(g),
      vec![(201, content::<Library>(g))],
      vec!["bad-request", "conflict", "no-account", "not-a-folder"],
    ),
    route(
      "get",
      "/libraries/{libraryId}",
      "readLibrary",
      "libraries",
      "One library.",
      vec![library()],
      None,
      vec![(200, content::<Library>(g))],
      read.clone(),
    ),
    route(
      "patch",
      "/libraries/{libraryId}",
      "changeLibrary",
      "libraries",
      "Rename a library. The administrator alone.",
      vec![library()],
      json_body::<LibraryChange>(g),
      vec![(200, content::<Library>(g))],
      vec!["bad-request", "not-found"],
    ),
    route(
      "delete",
      "/libraries/{libraryId}",
      "deleteLibrary",
      "libraries",
      "Remove a library and everything indexed from it; its files stay. The administrator alone.",
      vec![library()],
      None,
      vec![(204, None)],
      read.clone(),
    ),
    route(
      "post",
      "/libraries/{libraryId}/scans",
      "startScan",
      "libraries",
      "Start a scan of a library, or answer the one running. The administrator alone.",
      vec![library()],
      None,
      vec![(201, content::<Scan>(g)), (200, content::<Scan>(g))],
      read.clone(),
    ),
    route(
      "get",
      "/libraries/{libraryId}/scans",
      "listScans",
      "libraries",
      "A library's scans, newest first.",
      with(vec![library()], query::<PageQuery>(g)),
      None,
      vec![(200, content::<Page<Scan>>(g))],
      vec!["bad-request", "not-found"],
    ),
    route(
      "get",
      "/titles",
      "listTitles",
      "titles",
      "The titles the caller reads, in the order asked.",
      query::<TitlesQuery>(g),
      None,
      vec![(200, content::<Page<Title>>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "post",
      "/titles",
      "createTitle",
      "titles",
      "Make a title.",
      vec![],
      json_body::<NewTitle>(g),
      vec![(201, content::<Title>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "get",
      "/titles/{titleId}",
      "readTitle",
      "titles",
      "One title.",
      vec![title()],
      None,
      vec![(200, content::<Title>(g))],
      read.clone(),
    ),
    route(
      "patch",
      "/titles/{titleId}",
      "changeTitle",
      "titles",
      "Change the fields named of a title.",
      vec![title()],
      json_body::<TitleChange>(g),
      vec![(200, content::<Title>(g))],
      vec!["bad-request", "not-found"],
    ),
    route(
      "delete",
      "/titles/{titleId}",
      "deleteTitle",
      "titles",
      "Delete a title; a library's is retired and its files left.",
      vec![title()],
      None,
      vec![(204, None)],
      read.clone(),
    ),
    route(
      "get",
      "/titles/{titleId}/poster",
      "readTitlePoster",
      "titles",
      "The poster of a title's first episode that has one.",
      vec![title()],
      None,
      vec![(200, jpeg.clone())],
      read.clone(),
    ),
    route(
      "get",
      "/titles/{titleId}/episodes",
      "listTitleEpisodes",
      "titles",
      "A title's episodes in order, each with the caller's position.",
      vec![title()],
      None,
      vec![(200, content::<Page<Episode>>(g))],
      read.clone(),
    ),
    route(
      "post",
      "/titles/{titleId}/episodes",
      "fileEpisode",
      "titles",
      "File a video under a title as a clip: an upload, or a path on the service's host.",
      vec![title()],
      upload_body::<FiledByPath>(g, &["name", "label", "season", "number"]),
      vec![(201, content::<Episode>(g))],
      vec!["bad-request", "not-found", "not-a-video", "not-a-file", "failed"],
    ),
    route(
      "get",
      "/titles/{titleId}/progress",
      "readTitleProgress",
      "titles",
      "The caller's progress through a title's episodes, the episode to watch next and the one to go on with.",
      vec![title()],
      None,
      vec![(200, content::<Progress>(g))],
      vec!["not-found", "no-account"],
    ),
    route(
      "get",
      "/episodes/{episodeId}",
      "readEpisode",
      "episodes",
      "One episode, with the caller's position.",
      vec![episode()],
      None,
      vec![(200, content::<Episode>(g))],
      read.clone(),
    ),
    route(
      "patch",
      "/episodes/{episodeId}",
      "changeEpisode",
      "episodes",
      "Change the fields named of an episode.",
      vec![episode()],
      json_body::<EpisodeChange>(g),
      vec![(200, content::<Episode>(g))],
      vec!["bad-request", "not-found"],
    ),
    route(
      "delete",
      "/episodes/{episodeId}",
      "deleteEpisode",
      "episodes",
      "Delete a clip and its file; a library's episode is refused.",
      vec![episode()],
      None,
      vec![(204, None)],
      vec!["not-found", "conflict"],
    ),
    route(
      "get",
      "/episodes/{episodeId}/stream",
      "streamEpisode",
      "episodes",
      "The episode's file as it stands, with HTTP ranges.",
      with(vec![episode()], range_headers()),
      None,
      vec![(200, streamed.clone()), (206, streamed.clone()), (304, None), (416, None)],
      read.clone(),
    ),
    route(
      "get",
      "/episodes/{episodeId}/poster",
      "readEpisodePoster",
      "episodes",
      "The episode's poster frame.",
      vec![episode()],
      None,
      vec![(200, jpeg.clone())],
      read.clone(),
    ),
    route(
      "get",
      "/episodes/{episodeId}/position",
      "readPosition",
      "positions",
      "The caller's position in an episode.",
      vec![episode()],
      None,
      vec![(200, content::<Position>(g))],
      vec!["not-found", "no-account"],
    ),
    route(
      "put",
      "/episodes/{episodeId}/position",
      "setPosition",
      "positions",
      "Set the caller's position in an episode.",
      vec![episode()],
      json_body::<PositionSet>(g),
      vec![(200, content::<Position>(g))],
      vec!["bad-request", "not-found", "no-account"],
    ),
    route(
      "get",
      "/watching",
      "listWatching",
      "watching",
      "The caller's continue watching: the episodes it started and neither finished nor set aside, the one set last first.",
      query::<RealmQuery>(g),
      None,
      vec![(200, content::<Page<Episode>>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "delete",
      "/watching/{episodeId}",
      "setAsideWatching",
      "watching",
      "Set the caller's position in an episode aside from continue watching; the position stays.",
      vec![episode()],
      None,
      vec![(204, None)],
      vec!["not-found", "no-account"],
    ),
    route(
      "get",
      "/next-up",
      "listNextUp",
      "watching",
      "The caller's next up: for each show it played within thirty days, the episode to go on with, not yet started.",
      query::<RealmQuery>(g),
      None,
      vec![(200, content::<Page<Episode>>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "get",
      "/episodes/{episodeId}/subtitles",
      "listSubtitles",
      "subtitles",
      "An episode's subtitles.",
      vec![episode()],
      None,
      vec![(200, content::<Page<Subtitle>>(g))],
      read.clone(),
    ),
    route(
      "post",
      "/episodes/{episodeId}/subtitles",
      "addSubtitle",
      "subtitles",
      "Add a subtitle: an upload, or a path on the service's host.",
      vec![episode()],
      upload_body::<SubtitleByPath>(g, &["language", "label", "format"]),
      vec![(201, content::<Subtitle>(g))],
      vec!["bad-request", "not-found", "not-a-file", "unknown-format", "no-cues"],
    ),
    route(
      "get",
      "/episodes/{episodeId}/grants",
      "listGrants",
      "grants",
      "Who an episode is granted to.",
      vec![episode()],
      None,
      vec![(200, content::<Page<Grant>>(g))],
      vec!["forbidden", "not-found"],
    ),
    route(
      "post",
      "/episodes/{episodeId}/grants",
      "grantEpisode",
      "grants",
      "Grant an episode to one or more accounts; answers every grant of it.",
      vec![episode()],
      json_body::<NewGrants>(g),
      vec![(200, content::<Page<Grant>>(g))],
      vec!["bad-request", "forbidden", "not-found"],
    ),
    route(
      "put",
      "/episodes/{episodeId}/grants/{user}",
      "changeGrant",
      "grants",
      "Set a grant's permission.",
      vec![episode(), user()],
      json_body::<GrantChange>(g),
      vec![(200, content::<Grant>(g))],
      vec!["bad-request", "forbidden", "not-found"],
    ),
    route(
      "delete",
      "/episodes/{episodeId}/grants/{user}",
      "endGrant",
      "grants",
      "End a grant.",
      vec![episode(), user()],
      None,
      vec![(204, None)],
      vec!["forbidden", "not-found"],
    ),
    route(
      "post",
      "/episodes/{episodeId}/burns",
      "startBurn",
      "burns",
      "Burn a subtitle into an episode, in the background.",
      vec![episode()],
      json_body::<NewBurn>(g),
      vec![(201, content::<Burn>(g))],
      vec!["bad-request", "not-found", "unknown-subtitle"],
    ),
    route(
      "get",
      "/episodes/{episodeId}/burns",
      "listBurns",
      "burns",
      "An episode's burns, newest first.",
      with(vec![episode()], query::<PageQuery>(g)),
      None,
      vec![(200, content::<Page<Burn>>(g))],
      vec!["bad-request", "not-found"],
    ),
    route(
      "get",
      "/burns/{burnId}",
      "readBurn",
      "burns",
      "One burn.",
      vec![path_id("burnId", "The burn's id.")],
      None,
      vec![(200, content::<Burn>(g))],
      read.clone(),
    ),
    route(
      "get",
      "/subtitles/{subtitleId}",
      "readSubtitle",
      "subtitles",
      "One subtitle.",
      vec![path_id("subtitleId", "The subtitle's id.")],
      None,
      vec![(200, content::<Subtitle>(g))],
      read.clone(),
    ),
    route(
      "delete",
      "/subtitles/{subtitleId}",
      "deleteSubtitle",
      "subtitles",
      "Remove a subtitle and its files.",
      vec![path_id("subtitleId", "The subtitle's id.")],
      None,
      vec![(204, None)],
      read.clone(),
    ),
    route(
      "get",
      "/subtitles/{subtitleId}.vtt",
      "readSubtitleVtt",
      "subtitles",
      "A subtitle as WebVTT.",
      vec![path_id("subtitleId", "The subtitle's id.")],
      None,
      vec![(200, vtt.clone())],
      read.clone(),
    ),
    route(
      "get",
      "/subtitles/{subtitleId}/original",
      "readSubtitleOriginal",
      "subtitles",
      "A subtitle's file as it was given.",
      vec![path_id("subtitleId", "The subtitle's id.")],
      None,
      vec![(200, bytes(&["application/octet-stream"]))],
      read.clone(),
    ),
    route(
      "get",
      "/playlists",
      "listPlaylists",
      "playlists",
      "The playlists the caller reads.",
      query::<RealmQuery>(g),
      None,
      vec![(200, content::<Page<Playlist>>(g))],
      vec!["bad-request", "no-account"],
    ),
    route(
      "post",
      "/playlists",
      "createPlaylist",
      "playlists",
      "Make a playlist.",
      vec![],
      json_body::<NewPlaylist>(g),
      vec![(201, content::<Playlist>(g))],
      vec!["bad-request", "no-account", "unknown-episode", "not-in-realm"],
    ),
    route(
      "get",
      "/playlists/{playlistId}",
      "readPlaylist",
      "playlists",
      "One playlist.",
      vec![playlist()],
      None,
      vec![(200, content::<Playlist>(g))],
      read.clone(),
    ),
    route(
      "patch",
      "/playlists/{playlistId}",
      "changePlaylist",
      "playlists",
      "Rename a playlist, or write its whole order.",
      vec![playlist()],
      json_body::<PlaylistChange>(g),
      vec![(200, content::<Playlist>(g))],
      vec!["bad-request", "not-found", "unknown-episode", "not-in-realm"],
    ),
    route(
      "delete",
      "/playlists/{playlistId}",
      "deletePlaylist",
      "playlists",
      "Delete a playlist.",
      vec![playlist()],
      None,
      vec![(204, None)],
      read.clone(),
    ),
    route(
      "post",
      "/playlists/{playlistId}/items",
      "addPlaylistItems",
      "playlists",
      "Add episodes to a playlist, at a place or at the end.",
      vec![playlist()],
      json_body::<PlaylistItems>(g),
      vec![(201, content::<Playlist>(g))],
      vec!["bad-request", "not-found", "unknown-episode", "not-in-realm"],
    ),
    route(
      "get",
      "/playlists/{playlistId}/progress",
      "readPlaylistProgress",
      "playlists",
      "The caller's progress through a playlist, and the episode to watch next.",
      vec![playlist()],
      None,
      vec![(200, content::<Progress>(g))],
      vec!["not-found", "no-account"],
    ),
    route(
      "get",
      "/shares",
      "listShares",
      "shares",
      "The live links of the members the caller reads.",
      query::<SharesQuery>(g),
      None,
      vec![(200, content::<Page<Share>>(g))],
      vec!["bad-request"],
    ),
    route(
      "post",
      "/shares",
      "createShare",
      "shares",
      "Make a link to an episode or a playlist, or answer the live one.",
      vec![],
      json_body::<NewShare>(g),
      vec![(201, content::<Share>(g)), (200, content::<Share>(g))],
      vec!["bad-request", "not-found"],
    ),
    open(route(
      "get",
      "/shares/{token}",
      "readShare",
      "shares",
      "The link; a share read.",
      vec![token()],
      None,
      vec![(200, content::<Share>(g))],
      vec!["not-found", "gone"],
    )),
    route(
      "delete",
      "/shares/{token}",
      "deleteShare",
      "shares",
      "End a link.",
      vec![token()],
      None,
      vec![(204, None)],
      read.clone(),
    ),
    open(route(
      "get",
      "/shares/{token}/head",
      "readShareHead",
      "shares",
      "What the link reads as before it is opened.",
      with(vec![token()], query::<HeadQuery>(g)),
      None,
      vec![(200, content::<ShareHead>(g))],
      vec!["bad-request", "not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/playlist",
      "readSharedPlaylist",
      "shares",
      "A playlist link's playlist; a share read.",
      vec![token()],
      None,
      vec![(200, content::<Playlist>(g))],
      vec!["not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/episodes",
      "listSharedEpisodes",
      "shares",
      "The episodes a link opens; a share read.",
      vec![token()],
      None,
      vec![(200, content::<Page<Episode>>(g))],
      vec!["not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/episodes/{episodeId}/stream",
      "streamSharedEpisode",
      "shares",
      "An episode's stream; a share read.",
      with(vec![token(), episode()], range_headers()),
      None,
      vec![(200, streamed.clone()), (206, streamed), (304, None), (416, None)],
      vec!["not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/episodes/{episodeId}/poster",
      "readSharedPoster",
      "shares",
      "An episode's poster; a share read.",
      vec![token(), episode()],
      None,
      vec![(200, jpeg)],
      vec!["not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/episodes/{episodeId}/subtitles",
      "listSharedSubtitles",
      "shares",
      "An episode's subtitles; a share read.",
      vec![token(), episode()],
      None,
      vec![(200, content::<Page<Subtitle>>(g))],
      vec!["not-found", "gone"],
    )),
    open(route(
      "get",
      "/shares/{token}/subtitles/{subtitleId}.vtt",
      "readSharedVtt",
      "shares",
      "A subtitle as WebVTT; a share read.",
      vec![token(), path_id("subtitleId", "The subtitle's id.")],
      None,
      vec![(200, vtt)],
      vec!["not-found", "gone"],
    )),
    route(
      "get",
      "/events",
      "readEvents",
      "events",
      "The events the theater publishes, as a server-sent stream, or a page to Accept: application/json.",
      with(
        query::<EventsQuery>(g),
        vec![
          json!({ "name": "Last-Event-ID", "in": "header", "required": false, "description": "The last event number seen, as a reconnecting stream sends it.", "schema": { "type": "integer", "format": "int64" } }),
        ],
      ),
      None,
      vec![(
        200,
        Some(
          json!({ "text/event-stream": { "schema": { "type": "string", "description": "Each event's data is a StreamEvent." } }, "application/json": { "schema": g.subschema_for::<Page<StreamEvent>>() } }),
        ),
      )],
      vec!["bad-request", "gone"],
    ),
  ]
}

/// The OpenAPI document.
pub fn openapi() -> Value {
  let mut generator = SchemaSettings::draft2020_12()
    .for_serialize()
    .with(|settings| {
      settings.definitions_path = SCHEMAS.into();
      settings.meta_schema = None;
    })
    .into_generator();
  let g = &mut generator;
  let error = g.subschema_for::<WireError>();
  let mut tags: Vec<&str> = Vec::new();
  let mut paths = Map::new();
  for route in routes(g) {
    if !tags.contains(&route.tag) {
      tags.push(route.tag);
    }
    let (path, method) = (route.path, route.method);
    paths.entry(path).or_insert_with(|| json!({}))[method] = route.operation(&error);
  }
  let mut events = Map::new();
  for kind in KINDS {
    let (resource, change) = kind.split_once('.').unwrap_or((kind, ""));
    let schema = match (resource, change) {
      ("share", "deleted") => g.subschema_for::<ShareDeleted>(),
      (_, "deleted") => g.subschema_for::<Deleted>(),
      ("library", _) => g.subschema_for::<Library>(),
      ("scan", _) => g.subschema_for::<Scan>(),
      ("title", _) => g.subschema_for::<Title>(),
      ("episode", _) => g.subschema_for::<Episode>(),
      ("subtitle", _) => g.subschema_for::<Subtitle>(),
      ("playlist", _) => g.subschema_for::<Playlist>(),
      ("position", _) => g.subschema_for::<Position>(),
      ("burn", _) => g.subschema_for::<Burn>(),
      _ => g.subschema_for::<Share>(),
    };
    events.insert(kind.to_owned(), schema.to_value());
  }
  let mut outputs: BTreeMap<&str, Value> = BTreeMap::new();
  outputs.insert("titles", g.subschema_for::<Page<Title>>().to_value());
  outputs.insert("title", g.subschema_for::<Title>().to_value());
  outputs.insert("episode", g.subschema_for::<Episode>().to_value());
  outputs.insert("subtitle", g.subschema_for::<Subtitle>().to_value());
  outputs.insert("playlist", g.subschema_for::<Playlist>().to_value());
  outputs.insert("progress", g.subschema_for::<Progress>().to_value());
  outputs.insert("burn", g.subschema_for::<Burn>().to_value());
  outputs.insert("deleted", g.subschema_for::<Deleted>().to_value());
  let tools: Vec<Value> = TOOLS
    .iter()
    .map(|(name, description, route, answer)| {
      json!({
        "name": name,
        "namespace": "bmo",
        "capabilities": ["room.tools"],
        "description": description,
        "route": route,
        "input": tool_input(name),
        "output": outputs.get(answer).cloned().unwrap_or(Value::Null),
      })
    })
    .collect();
  json!({
    "openapi": "3.1.0",
    "info": { "title": "bmo-theater", "version": env!("CARGO_PKG_VERSION"), "description": DESCRIPTION },
    "servers": [{ "url": "/", "description": "the theater service" }],
    "security": [{ "bearer": [] }],
    "x-layer": "downstream",
    "x-calls": [],
    "x-events": KINDS,
    "x-event-data": events,
    "tags": tags.iter().map(|tag| json!({ "name": tag })).collect::<Vec<_>>(),
    "paths": paths,
    "components": {
      "schemas": generator.take_definitions(true),
      "securitySchemes": {
        "bearer": {
          "type": "http",
          "scheme": "bearer",
          "description": "The service token (naming its account with Owner-Key), or the console's credential with Owner-Key and Owner-Role; the console's reaches each operation as its x-console says.",
        },
      },
    },
    "x-tools": tools,
  })
}

/// The document as the repository keeps it: two-space indented JSON with a closing newline.
pub fn openapi_json() -> String {
  let mut text = serde_json::to_string_pretty(&openapi()).expect("a JSON value always renders");
  text.push('\n');
  text
}
