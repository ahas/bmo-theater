//! The service's wire types (`docs/api.md`): what each route takes and answers, in the platform's format, each
//! with its JSON Schema for the OpenAPI document.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

/// The moment now, in epoch milliseconds.
pub fn now_ms() -> i64 {
  std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .map(|since| since.as_millis() as i64)
    .unwrap_or_default()
}

/// A field a change may name as a value, as null, or not at all.
fn present<'de, D, T>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
  D: Deserializer<'de>,
  T: Deserialize<'de>,
{
  Ok(Some(Option::deserialize(deserializer)?))
}

/// One of the two realms (`docs/design.md`, 「Two realms」).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Realm {
  /// An account's own.
  Mine,
  /// The organization's, read by every account.
  Organization,
}

impl Realm {
  /// The word the tables and the queries write it as.
  pub fn word(self) -> &'static str {
    match self {
      Self::Mine => "mine",
      Self::Organization => "organization",
    }
  }

  /// The realm a word names.
  pub fn of(word: &str) -> Option<Self> {
    match word {
      "mine" => Some(Self::Mine),
      "organization" => Some(Self::Organization),
      _ => None,
    }
  }
}

/// What a title is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum TitleKind {
  /// A series, its episodes numbered by season and episode.
  Show,
  /// One video.
  Film,
  /// Filed outputs.
  Clip,
}

impl TitleKind {
  pub fn word(self) -> &'static str {
    match self {
      Self::Show => "show",
      Self::Film => "film",
      Self::Clip => "clip",
    }
  }

  pub fn of(word: &str) -> Option<Self> {
    match word {
      "show" => Some(Self::Show),
      "film" => Some(Self::Film),
      "clip" => Some(Self::Clip),
      _ => None,
    }
  }
}

/// What an episode is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum EpisodeKind {
  /// A library's video.
  Episode,
  /// A filed output, or a burned one.
  Clip,
}

impl EpisodeKind {
  pub fn word(self) -> &'static str {
    match self {
      Self::Episode => "episode",
      Self::Clip => "clip",
    }
  }

  pub fn of(word: &str) -> Option<Self> {
    match word {
      "episode" => Some(Self::Episode),
      "clip" => Some(Self::Clip),
      _ => None,
    }
  }
}

/// Where a scan or a burn stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
  Running,
  Finished,
  Failed,
}

impl JobState {
  pub fn word(self) -> &'static str {
    match self {
      Self::Running => "running",
      Self::Finished => "finished",
      Self::Failed => "failed",
    }
  }

  pub fn of(word: &str) -> Option<Self> {
    match word {
      "running" => Some(Self::Running),
      "finished" => Some(Self::Finished),
      "failed" => Some(Self::Failed),
      _ => None,
    }
  }
}

/// A subtitle's original format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SubtitleFormat {
  Srt,
  Ass,
  Smi,
  Vtt,
}

impl SubtitleFormat {
  pub fn word(self) -> &'static str {
    match self {
      Self::Srt => "srt",
      Self::Ass => "ass",
      Self::Smi => "smi",
      Self::Vtt => "vtt",
    }
  }

  /// The format a word or an extension names, in any letter case; `ssa` reads as `ass`, `sami` as `smi`.
  pub fn of(word: &str) -> Option<Self> {
    match word.to_ascii_lowercase().as_str() {
      "srt" => Some(Self::Srt),
      "ass" | "ssa" => Some(Self::Ass),
      "smi" | "sami" => Some(Self::Smi),
      "vtt" | "webvtt" => Some(Self::Vtt),
      _ => None,
    }
  }
}

/// A role a reader holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Role {
  Administrator,
  Member,
}

/// A failure, in the platform's shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WireError {
  pub code: String,
  pub error: String,
}

/// Whether the service answers.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Health {
  pub ready: bool,
  pub reason: Option<String>,
  pub version: String,
}

/// A page of a list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Page<T> {
  pub items: Vec<T>,
  pub more: bool,
}

/// The key of a member deleted, as its event carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Deleted {
  pub id: i64,
}

/// The token of a link ended, as its event carries it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShareDeleted {
  pub token: String,
}

// ---------------------------------------------------------------------------------------------------------
// Libraries and scans

/// A registered folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Library {
  pub id: i64,
  pub realm: Realm,
  pub name: String,
  /// The folder, absolute.
  pub path: String,
  pub creator: String,
  pub created_at: i64,
  /// The end of its last finished scan.
  pub scanned_at: Option<i64>,
  /// Whether a scan runs.
  pub scanning: bool,
  pub title_count: i64,
  pub episode_count: i64,
}

/// A folder to register.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewLibrary {
  /// An absolute folder the service reads, outside its file root.
  pub path: String,
  pub realm: Realm,
  /// Its own title's name; the show name most of its folder's files share, else the folder's, where none.
  #[serde(default)]
  pub name: Option<String>,
}

/// A library's change.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LibraryChange {
  #[serde(default)]
  pub name: Option<String>,
}

/// One reading of a library's folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Scan {
  pub id: i64,
  pub library_id: i64,
  pub state: JobState,
  /// Video files found.
  pub found_count: i64,
  /// Episodes added.
  pub added_count: i64,
  /// Episodes read again, or found back on disk.
  pub changed_count: i64,
  /// Episodes whose file is gone.
  pub missing_count: i64,
  pub error: Option<String>,
  pub started_at: i64,
  pub ended_at: Option<i64>,
}

// ---------------------------------------------------------------------------------------------------------
// Titles

/// A title.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Title {
  pub id: i64,
  pub realm: Realm,
  pub name: String,
  pub kind: TitleKind,
  pub description: String,
  pub labels: BTreeMap<String, String>,
  /// The library it was indexed from; null for a title made by hand.
  pub library_id: Option<i64>,
  pub episode_count: i64,
  pub has_poster: bool,
  pub creator: String,
  pub created_at: i64,
  pub changed_at: i64,
}

/// A title to make.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewTitle {
  pub realm: Realm,
  pub name: String,
  /// `clip` where none.
  #[serde(default)]
  pub kind: Option<TitleKind>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub labels: Option<BTreeMap<String, String>>,
}

/// A title's change: the fields named.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TitleChange {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub kind: Option<TitleKind>,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub labels: Option<BTreeMap<String, String>>,
}

// ---------------------------------------------------------------------------------------------------------
// Episodes

/// What a burned clip was burned from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BurnedFrom {
  pub episode_id: i64,
  pub subtitle_id: i64,
}

/// One video under a title.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
  pub id: i64,
  pub title_id: i64,
  pub realm: Realm,
  pub kind: EpisodeKind,
  pub season: Option<i32>,
  pub number: Option<i32>,
  pub name: String,
  pub label: String,
  pub file_name: String,
  pub media_type: String,
  pub byte_count: i64,
  pub duration_ms: Option<i64>,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub video_codec: Option<String>,
  pub audio_codec: Option<String>,
  /// Whether its file is on disk.
  pub available: bool,
  pub has_poster: bool,
  pub subtitle_count: i64,
  /// For a burned clip, what it was burned from.
  pub burned_from: Option<BurnedFrom>,
  /// The caller's position in it.
  pub position: Option<Position>,
  pub creator: String,
  pub created_at: i64,
  pub changed_at: i64,
}

/// A video filed by a path on the service's own host.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FiledByPath {
  /// An absolute file the service reads, copied into its file root.
  pub path: String,
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub label: Option<String>,
  #[serde(default)]
  pub season: Option<i32>,
  #[serde(default)]
  pub number: Option<i32>,
}

/// An episode's change: the fields named; `season` and `number` null to clear, `titleId` against another title.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EpisodeChange {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub label: Option<String>,
  #[serde(default, deserialize_with = "present")]
  pub season: Option<Option<i32>>,
  #[serde(default, deserialize_with = "present")]
  pub number: Option<Option<i32>>,
  /// The title the episode moves under; a library's episode is refused.
  #[serde(default)]
  pub title_id: Option<i64>,
}

// ---------------------------------------------------------------------------------------------------------
// Positions and progress

/// A viewer's place in an episode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Position {
  pub episode_id: i64,
  pub seconds: f64,
  pub done: bool,
  pub updated_at: i64,
}

/// A position to set.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PositionSet {
  /// The seconds watched to, 0 or more.
  pub seconds: f64,
  /// Whether it counts as watched; it becomes true by itself at nine tenths of the duration.
  #[serde(default)]
  pub done: Option<bool>,
}

/// One episode of a progress.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProgressItem {
  pub episode_id: i64,
  pub duration_ms: Option<i64>,
  pub seconds: f64,
  pub done: bool,
}

/// A viewer's progress through a playlist or a title.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
  pub items: Vec<ProgressItem>,
  pub done_count: i64,
  /// The episode to watch next, or null.
  pub next: Option<i64>,
  /// The seconds to start it from.
  pub next_seconds: f64,
}

// ---------------------------------------------------------------------------------------------------------
// Subtitles and burns

/// A subtitle of an episode.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Subtitle {
  pub id: i64,
  pub episode_id: i64,
  pub language: String,
  pub label: String,
  pub format: SubtitleFormat,
  pub file_name: String,
  pub cue_count: i64,
  pub creator: String,
  pub created_at: i64,
}

/// A subtitle added by a path on the service's own host.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubtitleByPath {
  pub path: String,
  #[serde(default)]
  pub language: Option<String>,
  #[serde(default)]
  pub label: Option<String>,
  /// The file's extension where none.
  #[serde(default)]
  pub format: Option<SubtitleFormat>,
}

/// A burn: a subtitle written into an episode's picture.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Burn {
  pub id: i64,
  pub episode_id: i64,
  pub subtitle_id: i64,
  pub state: JobState,
  /// The clip it filed, once finished.
  pub clip_id: Option<i64>,
  pub error: Option<String>,
  pub creator: String,
  pub started_at: i64,
  pub ended_at: Option<i64>,
}

/// A burn to start.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewBurn {
  pub subtitle_id: i64,
}

// ---------------------------------------------------------------------------------------------------------
// Playlists

/// A playlist.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Playlist {
  pub id: i64,
  pub realm: Realm,
  pub name: String,
  pub episode_ids: Vec<i64>,
  pub duration_ms: i64,
  pub creator: String,
  pub created_at: i64,
  pub changed_at: i64,
}

/// A playlist to make.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewPlaylist {
  pub realm: Realm,
  pub name: String,
  #[serde(default)]
  pub episode_ids: Option<Vec<i64>>,
}

/// A playlist's change: its name, or its whole order.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaylistChange {
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub episode_ids: Option<Vec<i64>>,
}

/// Episodes added to a playlist.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlaylistItems {
  pub episode_ids: Vec<i64>,
  /// The place the first goes, from 0; the end where none.
  #[serde(default)]
  pub at: Option<usize>,
}

// ---------------------------------------------------------------------------------------------------------
// Share links

/// A share link.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Share {
  pub token: String,
  pub episode_id: Option<i64>,
  pub playlist_id: Option<i64>,
  /// The share address; null where the settings name none.
  pub url: Option<String>,
  pub creator: String,
  pub created_at: i64,
  pub expires_at: Option<i64>,
}

/// A link to make: one of the two members.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NewShare {
  #[serde(default)]
  pub episode_id: Option<i64>,
  #[serde(default)]
  pub playlist_id: Option<i64>,
  #[serde(default)]
  pub expires_at: Option<i64>,
}

/// What a share address reads as before it is opened (bmo-ui's `docs/console-bundles.md`, 「Share pages」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShareHead {
  pub title: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image: Option<String>,
  pub url: String,
  pub locale: String,
  pub site_name: String,
  #[serde(rename = "type")]
  pub kind: String,
}

// ---------------------------------------------------------------------------------------------------------
// Queries

/// A list's realm and page.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RealmQuery {
  /// The realm; both the caller reads where none.
  #[serde(default)]
  pub realm: Option<Realm>,
  /// The id the page starts after.
  #[serde(default)]
  pub after: Option<i64>,
  /// At most this many, 1 to 1000; 200 where none.
  #[serde(default)]
  pub limit: Option<i64>,
}

/// The titles asked for.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TitlesQuery {
  #[serde(default)]
  pub realm: Option<Realm>,
  #[serde(default)]
  pub kind: Option<TitleKind>,
  #[serde(default)]
  pub library_id: Option<i64>,
  /// A part of the name, in any letter case.
  #[serde(default)]
  pub q: Option<String>,
  #[serde(default)]
  pub after: Option<i64>,
  #[serde(default)]
  pub limit: Option<i64>,
}

/// A page.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PageQuery {
  #[serde(default)]
  pub after: Option<i64>,
  #[serde(default)]
  pub limit: Option<i64>,
}

/// The links asked for.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SharesQuery {
  #[serde(default)]
  pub episode_id: Option<i64>,
  #[serde(default)]
  pub playlist_id: Option<i64>,
}

/// A share head's language.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HeadQuery {
  /// The primary language subtag of the language the page is shown in.
  #[serde(default)]
  pub language: Option<String>,
}

/// The event stream's reading.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventsQuery {
  /// The event number the reading starts after.
  #[serde(default)]
  pub after: Option<i64>,
  /// The kinds wanted, comma-separated.
  #[serde(default)]
  pub kinds: Option<String>,
  /// A page's size, 1 to 500; 50 where none.
  #[serde(default)]
  pub limit: Option<i64>,
}

/// One published event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StreamEvent {
  pub id: i64,
  pub kind: String,
  pub at: i64,
  pub data: Value,
}

/// The console bundle's manifest (bmo-ui's `docs/console-bundles.md`, 「The manifest」).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleManifest {
  pub name: String,
  pub title: String,
  pub icon: String,
  pub screen_path: String,
  pub service_name: String,
  pub bundle: String,
  pub stylesheet: String,
  pub words: String,
  pub api_prefix: String,
  pub role: Role,
  pub owner_key: bool,
  pub events: bool,
  pub websocket: bool,
  pub push_kinds: Vec<String>,
  pub calls: Vec<String>,
  pub relays: Vec<Value>,
  pub share_reads: Vec<String>,
  pub share_head: String,
}
