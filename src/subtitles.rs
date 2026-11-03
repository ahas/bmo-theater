//! A subtitle file read as text and converted to WebVTT (`docs/design.md`, 「Playing」; `docs/api.md`,
//! 「Subtitles」): SRT nearly as it is, SAMI by its timed blocks, ASS to its text alone, and WebVTT kept.

use std::sync::LazyLock;

use regex::Regex;

use crate::wire::SubtitleFormat;

/// One cue: its start and end in milliseconds and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cue {
  pub start: i64,
  pub end: i64,
  pub text: String,
}

/// A converted subtitle: its WebVTT and how many cues it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Converted {
  pub vtt: String,
  pub cues: usize,
}

/// The longest a SAMI file's last block lasts, with no block after it to end it.
const LAST_BLOCK_MS: i64 = 4000;

/// A file's bytes as text: UTF-8 (its mark dropped), UTF-16 where it carries that mark, else the Korean code
/// page SAMI files are usually written in.
pub fn decode(bytes: &[u8]) -> String {
  if let Some((encoding, mark)) = encoding_rs::Encoding::for_bom(bytes) {
    let (text, _) = encoding.decode_without_bom_handling(&bytes[mark..]);
    return text.into_owned();
  }
  match std::str::from_utf8(bytes) {
    Ok(text) => text.to_owned(),
    Err(_) => encoding_rs::EUC_KR.decode_without_bom_handling(bytes).0.into_owned(),
  }
}

/// A time in milliseconds as WebVTT writes it.
pub fn vtt_time(ms: i64) -> String {
  let ms = ms.max(0);
  format!("{:02}:{:02}:{:02}.{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
}

/// Cues written as a WebVTT file.
pub fn write_vtt(cues: &[Cue]) -> String {
  let mut vtt = String::from("WEBVTT\n");
  for cue in cues {
    vtt.push_str(&format!("\n{} --> {}\n{}\n", vtt_time(cue.start), vtt_time(cue.end), cue.text));
  }
  vtt
}

/// A subtitle's text converted to WebVTT, or why it holds no cue.
pub fn convert(format: SubtitleFormat, text: &str) -> Result<Converted, String> {
  let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n").replace('\r', "\n");
  let cues = match format {
    SubtitleFormat::Srt | SubtitleFormat::Vtt => timed_blocks(&text),
    SubtitleFormat::Smi => sami(&text),
    SubtitleFormat::Ass => ass(&text),
  };
  if cues.is_empty() {
    return Err(format!("the {} file holds no cue", format.word()));
  }
  Ok(Converted {
    vtt: write_vtt(&cues),
    cues: cues.len(),
  })
}

static TIMING: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"^\s*((?:\d+:)?\d{1,2}:\d{1,2}[.,]\d{1,3})\s*-->\s*((?:\d+:)?\d{1,2}:\d{1,2}[.,]\d{1,3})")
    .expect("a timing line's form")
});
static MARKUP: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"</?([A-Za-z][A-Za-z0-9]*)[^>]*>").expect("a tag"));
static ASS_OVERRIDE: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"\{[^}]*\}").expect("an override block"));

/// A time written `[h:]mm:ss.mmm` or with a comma, in milliseconds.
fn clock(text: &str) -> Option<i64> {
  let text = text.replace(',', ".");
  let (clock, fraction) = text.split_once('.').unwrap_or((&text, "0"));
  let mut ms = 0i64;
  for part in clock.split(':') {
    ms = ms * 60 + part.trim().parse::<i64>().ok()?;
  }
  let fraction = format!("{fraction:0<3}");
  Some(ms * 1000 + fraction[..3].parse::<i64>().ok()?)
}

/// A line's markup kept where WebVTT knows it, as `b`, `i` and `u`, and dropped elsewhere.
fn known_markup(line: &str) -> String {
  MARKUP
    .replace_all(line, |captures: &regex::Captures| {
      let name = captures[1].to_ascii_lowercase();
      match name.as_str() {
        "b" | "i" | "u" if captures[0].starts_with("</") => format!("</{name}>"),
        "b" | "i" | "u" => format!("<{name}>"),
        _ => String::new(),
      }
    })
    .into_owned()
}

/// The cues of SRT or WebVTT text: each block whose timing line names a start and an end, its lines after it.
fn timed_blocks(text: &str) -> Vec<Cue> {
  let mut cues = Vec::new();
  for block in text.split("\n\n") {
    let mut lines = block.lines().skip_while(|line| !TIMING.is_match(line));
    let Some(timing) = lines.next() else {
      continue;
    };
    let Some(captures) = TIMING.captures(timing) else {
      continue;
    };
    let (Some(start), Some(end)) = (clock(&captures[1]), clock(&captures[2])) else {
      continue;
    };
    let body: Vec<String> = lines
      .map(|line| known_markup(line).replace("-->", "->"))
      .filter(|line| !line.trim().is_empty())
      .collect();
    if body.is_empty() || end <= start {
      continue;
    }
    cues.push(Cue {
      start,
      end,
      text: body.join("\n"),
    });
  }
  cues.sort_by_key(|cue| cue.start);
  cues
}

static SYNC: LazyLock<Regex> =
  LazyLock::new(|| Regex::new(r"(?is)<sync[^>]*?start\s*=\s*['\x22]?(\d+)[^>]*>").expect("a SYNC tag"));
static P_CLASS: LazyLock<Regex> = LazyLock::new(|| {
  Regex::new(r"(?is)<p[^>]*?class\s*=\s*['\x22]?([A-Za-z0-9_-]+)").expect("a P tag's class")
});
static P_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<p\b[^>]*>").expect("a P tag"));
static BREAK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<br\s*/?>").expect("a line break"));
static COMMENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").expect("a comment"));

/// An HTML entity's text.
fn entities(text: &str) -> String {
  text
    .replace("&nbsp;", " ")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#39;", "'")
    .replace("&apos;", "'")
    .replace("&amp;", "&")
}

/// Plain text made safe to stand in a cue.
fn plain(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// The text of one SAMI block's paragraph: its breaks as new lines, every other tag dropped, its lines trimmed.
fn sami_text(block: &str) -> String {
  let text = BREAK.replace_all(block, "\n");
  let text = MARKUP.replace_all(&text, "");
  entities(&text)
    .lines()
    .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
    .filter(|line| !line.is_empty())
    .collect::<Vec<_>>()
    .join("\n")
}

/// The cues of SAMI text: each SYNC block's paragraph of the first language class lasting until the next block
/// begins, a block holding only spaces ending the one before it.
fn sami(text: &str) -> Vec<Cue> {
  let text = COMMENT.replace_all(text, "");
  let body = match text.to_ascii_lowercase().find("<body") {
    Some(at) => &text[at..],
    None => &text[..],
  };
  let class = P_CLASS.captures(body).map(|captures| captures[1].to_ascii_lowercase());
  let starts: Vec<(usize, usize, i64)> = SYNC
    .captures_iter(body)
    .filter_map(|captures| {
      let whole = captures.get(0)?;
      Some((whole.start(), whole.end(), captures[1].parse().ok()?))
    })
    .collect();
  let mut blocks: Vec<(i64, String)> = Vec::new();
  for (index, (_, end, start)) in starts.iter().enumerate() {
    let until = starts.get(index + 1).map(|next| next.0).unwrap_or(body.len());
    let block = &body[*end..until];
    let mut paragraphs: Vec<(Option<String>, &str)> = Vec::new();
    let tags: Vec<(usize, usize)> =
      P_TAG.find_iter(block).map(|found| (found.start(), found.end())).collect();
    if tags.is_empty() {
      paragraphs.push((None, block));
    }
    for (at, (tag_start, tag_end)) in tags.iter().enumerate() {
      let until = tags.get(at + 1).map(|next| next.0).unwrap_or(block.len());
      let tag_class = P_CLASS
        .captures(&block[*tag_start..*tag_end])
        .map(|captures| captures[1].to_ascii_lowercase());
      paragraphs.push((tag_class, &block[*tag_end..until]));
    }
    let chosen = paragraphs
      .iter()
      .find(|(tag_class, _)| class.is_none() || tag_class.is_none() || *tag_class == class)
      .map(|(_, paragraph)| sami_text(paragraph))
      .unwrap_or_default();
    match blocks.last_mut() {
      Some(last) if last.0 == *start => {
        if last.1.is_empty() {
          last.1 = chosen;
        }
      }
      _ => blocks.push((*start, chosen)),
    }
  }
  let mut cues = Vec::new();
  for (index, (start, text)) in blocks.iter().enumerate() {
    if text.is_empty() {
      continue;
    }
    let end = blocks.get(index + 1).map(|next| next.0).unwrap_or(start + LAST_BLOCK_MS);
    if end > *start {
      cues.push(Cue {
        start: *start,
        end,
        text: plain(text),
      });
    }
  }
  cues
}

/// The cues of ASS or SSA text: each `Dialogue` line of its events, read by the section's `Format` line, its
/// override blocks dropped and its breaks as new lines.
fn ass(text: &str) -> Vec<Cue> {
  let mut in_events = false;
  let mut fields: Vec<String> = Vec::new();
  let mut cues = Vec::new();
  for line in text.lines() {
    let line = line.trim();
    if line.starts_with('[') {
      in_events = line.eq_ignore_ascii_case("[events]");
      continue;
    }
    if !in_events {
      continue;
    }
    if let Some(format) = line.strip_prefix("Format:") {
      fields = format.split(',').map(|field| field.trim().to_ascii_lowercase()).collect();
      continue;
    }
    let Some(dialogue) = line.strip_prefix("Dialogue:") else {
      continue;
    };
    if fields.is_empty() {
      fields = [
        "layer", "start", "end", "style", "name", "marginl", "marginr", "marginv", "effect", "text",
      ]
      .map(str::to_owned)
      .to_vec();
    }
    let values: Vec<&str> = dialogue.splitn(fields.len(), ',').collect();
    let field = |name: &str| fields.iter().position(|field| field == name).and_then(|at| values.get(at));
    let (Some(start), Some(end), Some(said)) = (field("start"), field("end"), field("text")) else {
      continue;
    };
    let (Some(start), Some(end)) = (clock(start.trim()), clock(end.trim())) else {
      continue;
    };
    let said = ASS_OVERRIDE.replace_all(said, "");
    let said = said.replace("\\N", "\n").replace("\\n", "\n").replace("\\h", " ");
    let said: Vec<&str> = said.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    if said.is_empty() || end <= start {
      continue;
    }
    cues.push(Cue {
      start,
      end,
      text: plain(&said.join("\n")),
    });
  }
  cues.sort_by_key(|cue| cue.start);
  cues
}
