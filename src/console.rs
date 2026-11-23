//! The console face the service serves (`docs/api.md`, 「Console」), through bmo-chat's `bmo-console-face`: the
//! service's declaration of the manifest fields it decides, over the bundle build's folder, guarded by the
//! console's credential.

use std::path::Path;

use bmo_console_face::{BundleFolder, ConsoleFace, ConsoleFaceError};
use serde_json::{Value, json};

use crate::settings::Settings;

/// The manifest fields the service decides, as bmo-ui's console plugin contract names them.
pub const DECLARATION: &str = include_str!("console.json");

// What the bundle's build writes into its folder.
const MODULE_FILE: &str = "bundle.js";
const STYLESHEET_FILE: &str = "bundle.css";
const WORDS_FILE: &str = "words.json";

/// The words the face holds are read in English where the asked language holds none of a word.
const FALLBACK_LOCALE: &str = "en";

/// The one Chinese the words speak, under the code its pack is named by: a console asks by the primary
/// language subtag alone, `zh`, which this locale answers.
const CHINESE_LOCALE: &str = "zh-CN";

/// The locales a language tag is looked up under, most exact first: the tag as asked, its primary subtag in
/// lower case, and the pack's own Chinese where that subtag is `zh`.
fn locales_of(language: &str) -> Vec<String> {
  let asked = language.trim();
  let primary = asked.split(['-', '_']).next().unwrap_or_default().to_ascii_lowercase();
  let mut locales = vec![asked.to_owned(), primary.clone()];
  if primary == "zh" {
    locales.push(CHINESE_LOCALE.to_owned());
  }
  locales.retain(|locale| !locale.is_empty());
  locales
}

/// The face over the bundle built into `folder`, guarded by `credential`, or why it cannot be served.
pub fn face(folder: &Path, credential: &str) -> Result<ConsoleFace, ConsoleFaceError> {
  let bundle = BundleFolder::new(folder, MODULE_FILE, WORDS_FILE).with_stylesheet(STYLESHEET_FILE);
  ConsoleFace::new(DECLARATION, &bundle, credential)
}

/// A word of the face's words in `language`, else in English, where either holds it: `key` a dotted path.
pub fn word(face: &ConsoleFace, language: Option<&str>, key: &str) -> Option<String> {
  language
    .map(locales_of)
    .unwrap_or_default()
    .iter()
    .find_map(|locale| face.word(locale, key))
    .or_else(|| face.word(FALLBACK_LOCALE, key))
    .map(str::to_owned)
}

/// How a console reaches the service, as a deployment's console probe asks for it: the base address the settings
/// name, the console's credential and the service token, which the console's routes refuse.
pub fn reach(settings: &Settings) -> Value {
  json!({
    "address": settings.base_address(),
    "consoleToken": settings.console.token,
    "token": settings.server.token,
  })
}
