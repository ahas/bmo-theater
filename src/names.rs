//! What a file's and a folder's names say (`docs/design.md`, 「Libraries and scans」): whether a file is a
//! video and of which media type, a season's folder and its number, and a file's show, season, episode and
//! name.

use std::sync::LazyLock;

use regex::Regex;

/// The extensions read as videos, each with the media type it is served as.
pub const VIDEOS: [(&str, &str); 7] = [
  ("mp4", "video/mp4"),
  ("m4v", "video/mp4"),
  ("mkv", "video/x-matroska"),
  ("webm", "video/webm"),
  ("mov", "video/quicktime"),
  ("avi", "video/x-msvideo"),
  ("ogv", "video/ogg"),
];

/// The media type a video's extension is served as, where it is a video's.
pub fn video_type(extension: &str) -> Option<&'static str> {
  let extension = extension.to_ascii_lowercase();
  VIDEOS.iter().find(|(known, _)| *known == extension).map(|(_, media)| *media)
}

/// The extension of a file's name, without its dot.
pub fn extension(file_name: &str) -> &str {
  match file_name.rsplit_once('.') {
    Some((stem, extension)) if !stem.is_empty() => extension,
    _ => "",
  }
}

/// A file's name without its extension.
pub fn stem(file_name: &str) -> &str {
  match file_name.rsplit_once('.') {
    Some((stem, _)) if !stem.is_empty() => stem,
    _ => file_name,
  }
}

static SEASON_FOLDER: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?i)^(?:s|season[ ._-]*)(\d{1,3})$").expect("the season folder's form"));
static SEASON_EPISODE: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?i)\bs(\d{1,3})[ ._-]?e(\d{1,4})(?:-?e\d{1,4})*\b").expect("the SxxEyy form")
});
static CROSS: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?i)\b(\d{1,2})x(\d{2,3})\b").expect("the NxNN form"));
static EPISODE_ALONE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?i)(?:^|[\s._-])e(?:p)?(\d{1,4})\b").expect("the Eyy form"));
static TAG: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"\[[^\]]*\]|\{[^}]*\}").expect("a bracketed tag's form"));
static ROUND_TAG: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(
    r"(?i)\((?:\d{3,4}p|[0-9]+k|x26[45]|h\.?26[45]|hevc|web(?:-?dl|rip)?|bluray|hdtv|kr|en|eng|kor)\)",
  )
  .expect("a round tag's form")
});
static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("a run of spaces"));

/// The season number a folder's name gives, where it is a season's folder.
pub fn season_folder(name: &str) -> Option<i32> {
  SEASON_FOLDER.captures(name.trim())?.get(1)?.as_str().parse().ok()
}

/// What a file's name says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
  /// The show's name, what stands before the numbers, where anything does.
  pub show: Option<String>,
  pub season: Option<i32>,
  pub number: Option<i32>,
  /// The episode's name, what stands after the numbers; the whole name, cleaned, where it has no numbers.
  pub name: String,
}

/// A name's tags dropped, its dots and underscores read as spaces where it has no space, and its edges trimmed of
/// separators.
pub fn clean(text: &str) -> String {
  let text = TAG.replace_all(text, " ");
  let text = ROUND_TAG.replace_all(&text, " ");
  let text = if text.contains(' ') { text.into_owned() } else { text.replace(['.', '_'], " ") };
  let text = SPACES.replace_all(&text, " ");
  text
    .trim_matches(|character: char| character.is_whitespace() || "-–—_.:|".contains(character))
    .to_owned()
}

/// A file's stem read for its show, season, episode and name.
pub fn parse(stem: &str) -> Parsed {
  let found = SEASON_EPISODE
    .captures(stem)
    .map(|captures| {
      let whole = captures.get(0).expect("a match");
      (whole.start(), whole.end(), captures[1].parse().ok(), captures[2].parse().ok())
    })
    .or_else(|| {
      CROSS.captures(stem).map(|captures| {
        let whole = captures.get(0).expect("a match");
        (whole.start(), whole.end(), captures[1].parse().ok(), captures[2].parse().ok())
      })
    })
    .or_else(|| {
      EPISODE_ALONE.captures(stem).map(|captures| {
        let number = captures.get(1).expect("the episode's digits");
        let start = captures.get(0).expect("a match").start();
        let start = if stem[start..].starts_with(|c: char| c.is_whitespace() || "._-".contains(c)) {
          start + 1
        } else {
          start
        };
        (start, number.end(), None, number.as_str().parse().ok())
      })
    });
  match found {
    None => Parsed {
      show: None,
      season: None,
      number: None,
      name: clean(stem),
    },
    Some((start, end, season, number)) => {
      let show = clean(&stem[..start]);
      let name = clean(&stem[end..]);
      Parsed {
        show: (!show.is_empty()).then_some(show),
        season,
        number,
        name,
      }
    }
  }
}
