//! The console bundle the service serves (`docs/api.md`, 「Console」): its manifest by bmo-ui's console plugin
//! contract, and the module, the stylesheet and the words the manifest names, read from the bundle's built
//! folder once at start. Each file is answered under a path made from its bytes' digest.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::wire::{ConsoleManifest, Role};

/// The route the manifest is answered at.
pub const MANIFEST_PATH: &str = "/console-manifest";
/// The route the words are answered at.
pub const WORDS_PATH: &str = "/console/words";

// What the bundle's build writes into its folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

// How many hexadecimal digits of a file's digest stand in its path.
const DIGEST_DIGITS: usize = 16;

/// The reads a share token alone permits, as the manifest names them (`docs/api.md`, 「Share links」).
pub const SHARE_READS: [&str; 7] = [
  "/shares/{token}",
  "/shares/{token}/playlist",
  "/shares/{token}/episodes",
  "/shares/{token}/episodes/{episodeId}/stream",
  "/shares/{token}/episodes/{episodeId}/poster",
  "/shares/{token}/episodes/{episodeId}/subtitles",
  "/shares/{token}/subtitles/{subtitleFile}",
];

/// The read a console asks for a share address's head.
pub const SHARE_HEAD: &str = "/shares/{token}/head";

/// The cache line of a file whose path follows its bytes.
pub const KEPT_FOR_GOOD: &str = "private, max-age=31536000, immutable";

/// One file of the bundle, as it is answered.
#[derive(Clone, Debug)]
pub struct Served {
  /// Its name under `/console/`: `bundle-<digest>.<extension>`.
  pub name: String,
  pub content_type: &'static str,
  pub bytes: Vec<u8>,
}

impl Served {
  fn read(folder: &Path, file: &str, extension: &str, content_type: &'static str) -> Result<Self, String> {
    let path = folder.join(file);
    let bytes =
      std::fs::read(&path).map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    Ok(Self {
      name: format!("bundle-{}.{extension}", &digest[..DIGEST_DIGITS]),
      content_type,
      bytes,
    })
  }

  fn path(&self) -> String {
    format!("/console/{}", self.name)
  }
}

/// The bundle as the service serves it.
#[derive(Clone, Debug)]
pub struct Bundle {
  pub module: Served,
  pub stylesheet: Served,
  pub words: serde_json::Value,
}

impl Bundle {
  /// The bundle built into `folder`, or why it cannot be served.
  pub fn read(folder: &Path) -> Result<Self, String> {
    let module = Served::read(folder, MODULE_FILE, "js", "text/javascript")?;
    let stylesheet = Served::read(folder, STYLESHEET_FILE, "css", "text/css")?;
    let path = folder.join(WORDS_FILE);
    let words =
      std::fs::read(&path).map_err(|why| format!("{} could not be read: {why}", path.display()))?;
    let words: serde_json::Value =
      serde_json::from_slice(&words).map_err(|why| format!("{} is not JSON: {why}", path.display()))?;
    if !words.is_object() {
      return Err(format!("{} is not one object keyed by locale", path.display()));
    }
    Ok(Self {
      module,
      stylesheet,
      words,
    })
  }

  /// The manifest naming this bundle's files.
  pub fn manifest(&self) -> ConsoleManifest {
    ConsoleManifest {
      name: "theater".into(),
      title: "theater.title".into(),
      icon: "i-material-symbols-theaters-outline".into(),
      screen_path: "/theater".into(),
      service_name: "theater".into(),
      bundle: self.module.path(),
      stylesheet: self.stylesheet.path(),
      words: WORDS_PATH.into(),
      api_prefix: "/".into(),
      role: Role::Member,
      owner_key: true,
      events: true,
      websocket: false,
      push_kinds: Vec::new(),
      calls: Vec::new(),
      relays: Vec::new(),
      share_reads: SHARE_READS.iter().map(|read| (*read).to_owned()).collect(),
      share_head: SHARE_HEAD.into(),
    }
  }

  /// A word of the bundle's words in a language, else in English, where either holds it: `key` a dotted path.
  pub fn word(&self, language: Option<&str>, key: &str) -> Option<String> {
    let find = |locale: &str| {
      let mut at = self.words.get(locale)?;
      for part in key.split('.') {
        at = at.get(part)?;
      }
      at.as_str().map(str::to_owned)
    };
    let language = language
      .map(|language| language.trim().to_ascii_lowercase())
      .filter(|language| !language.is_empty());
    language.as_deref().and_then(find).or_else(|| find("en"))
  }

  /// The file answered under `name`, where the bundle has one.
  pub fn file(&self, name: &str) -> Option<&Served> {
    [&self.module, &self.stylesheet].into_iter().find(|served| served.name == name)
  }
}
