//! The settings (`docs/api.md`, 「Settings」): the defaults built in from `settings.toml`, then the settings
//! folder's `settings.toml` and `settings.local.toml` where they stand, then the environment.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use config::{Config, Environment, File, FileFormat};
use serde::Deserialize;

/// The defaults, as the repository's own settings file states them.
const DEFAULTS: &str = include_str!("../settings.toml");
/// The settings file a deployment may give.
pub const SETTINGS_FILE: &str = "settings.toml";
/// The file a deployment overrides it with.
pub const LOCAL_SETTINGS_FILE: &str = "settings.local.toml";
/// The environment variable naming the settings folder.
pub const SETTINGS_FOLDER: &str = "BMO_THEATER_SETTINGS";
/// What every environment variable that sets a key begins with.
pub const ENVIRONMENT_PREFIX: &str = "BMO_THEATER";

/// Everything the service and its command are started with.
#[derive(Clone, Debug, Deserialize)]
pub struct Settings {
  pub server: Server,
  pub console: Console,
  pub database: Database,
  pub files: Files,
  pub media: Media,
  pub share: Share,
  pub tool: Tool,
}

/// The HTTP server.
#[derive(Clone, Debug, Deserialize)]
pub struct Server {
  /// Where the service answers.
  pub listen: SocketAddr,
  /// The service token; an empty one lets no caller in.
  pub token: String,
  /// The data root; the deployment's conventional one under the home where empty.
  pub data_root: PathBuf,
}

/// The console bundle and the console's credential.
#[derive(Clone, Debug, Deserialize)]
pub struct Console {
  /// The console's credential; an empty one reaches nothing.
  pub token: String,
  /// The bundle's built folder; a relative one is read from where the process starts.
  pub folder: PathBuf,
}

/// The database the theater's tables stand in.
#[derive(Clone, Debug, Deserialize)]
pub struct Database {
  pub url: String,
}

/// The file root.
#[derive(Clone, Debug, Deserialize)]
pub struct Files {
  pub root: PathBuf,
}

/// The commands the theater runs.
#[derive(Clone, Debug, Deserialize)]
pub struct Media {
  pub ffmpeg: String,
  pub ffprobe: String,
}

/// Where a share link's address stands.
#[derive(Clone, Debug, Deserialize)]
pub struct Share {
  pub address: String,
}

/// What the tool is handed.
#[derive(Clone, Debug, Deserialize)]
pub struct Tool {
  /// The account the tool acts for.
  pub account: String,
}

impl Settings {
  /// The settings of this process: its settings folder, then its environment.
  pub fn load() -> Result<Self, String> {
    Self::read(&folder(), None)
  }

  /// The settings read from `dir`, then from `environment` where it is given and from the process's own
  /// environment where it is not.
  pub fn read(dir: &Path, environment: Option<HashMap<String, String>>) -> Result<Self, String> {
    let environment = Environment::with_prefix(ENVIRONMENT_PREFIX)
      .prefix_separator("_")
      .separator("__")
      .try_parsing(true)
      .source(environment);

    Config::builder()
      .add_source(File::from_str(DEFAULTS, FileFormat::Toml))
      .add_source(File::from(dir.join(SETTINGS_FILE)).format(FileFormat::Toml).required(false))
      .add_source(File::from(dir.join(LOCAL_SETTINGS_FILE)).format(FileFormat::Toml).required(false))
      .add_source(environment)
      .build()
      .and_then(Config::try_deserialize)
      .map_err(|why| format!("the settings could not be read: {why}"))
  }

  /// The data root: the settings', else the deployment's conventional one under the home.
  pub fn data_root(&self) -> PathBuf {
    if !self.server.data_root.as_os_str().is_empty() {
      return self.server.data_root.clone();
    }
    std::env::var_os("HOME")
      .map(|home| PathBuf::from(home).join(".local/share/bmo/data"))
      .unwrap_or_else(|| PathBuf::from("data"))
  }

  /// The service's own base address, as a caller on this machine reaches it.
  pub fn base_address(&self) -> String {
    let listen = self.server.listen;
    let host = if listen.ip().is_unspecified() {
      "127.0.0.1".to_owned()
    } else {
      listen.ip().to_string()
    };
    format!("http://{host}:{}", listen.port())
  }
}

/// The settings folder: the one the environment names, else the folder above the one holding the binary, a
/// deployment's root.
fn folder() -> PathBuf {
  std::env::var_os(SETTINGS_FOLDER)
    .filter(|named| !named.is_empty())
    .map(PathBuf::from)
    .or_else(|| {
      let binary = std::env::current_exe().ok()?;

      Some(binary.parent()?.parent()?.to_path_buf())
    })
    .unwrap_or_else(|| PathBuf::from("."))
}
