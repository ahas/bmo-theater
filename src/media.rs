//! The commands the theater runs (`docs/api.md`, 「Calls the theater makes」): ffprobe for what a file holds and
//! for a poster's size, ffmpeg for a poster frame and for a burn. Each is waited on however long it takes.

use std::path::Path;

use serde_json::Value;
use tokio::process::Command;

use crate::settings::Media;

/// What ffprobe read of a file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Probe {
  pub duration_ms: Option<i64>,
  pub width: Option<i32>,
  pub height: Option<i32>,
  pub video_codec: Option<String>,
  pub audio_codec: Option<String>,
}

impl Probe {
  /// Whether it found a video stream.
  pub fn has_video(&self) -> bool {
    self.video_codec.is_some()
  }
}

/// A poster frame kept: its pixel size, where ffprobe read it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Poster {
  pub width: Option<i32>,
  pub height: Option<i32>,
}

/// The last lines a command wrote to its error output, as the reason it failed.
fn said(output: &std::process::Output) -> String {
  let text = String::from_utf8_lossy(&output.stderr);
  let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
  let tail = lines[lines.len().saturating_sub(3)..].join(" / ");
  if tail.is_empty() { format!("it ended {}", output.status) } else { tail }
}

/// What ffprobe's JSON says of the streams and the format.
pub fn read_probe(json: &Value) -> Probe {
  let streams = json["streams"].as_array().cloned().unwrap_or_default();
  let video = streams.iter().find(|stream| {
    stream["codec_type"] == "video" && stream["disposition"]["attached_pic"].as_i64() != Some(1)
  });
  let audio = streams.iter().find(|stream| stream["codec_type"] == "audio");
  let duration = json["format"]["duration"]
    .as_str()
    .and_then(|seconds| seconds.parse::<f64>().ok())
    .or_else(|| video.and_then(|video| video["duration"].as_str()?.parse::<f64>().ok()))
    .map(|seconds| (seconds * 1000.0).round() as i64);
  Probe {
    duration_ms: duration,
    width: video.and_then(|video| video["width"].as_i64()).map(|width| width as i32),
    height: video.and_then(|video| video["height"].as_i64()).map(|height| height as i32),
    video_codec: video.and_then(|video| video["codec_name"].as_str()).map(str::to_owned),
    audio_codec: audio.and_then(|audio| audio["codec_name"].as_str()).map(str::to_owned),
  }
}

/// What a file holds, as ffprobe reads it.
pub async fn probe(media: &Media, file: &Path) -> Result<Probe, String> {
  let output = Command::new(&media.ffprobe)
    .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
    .arg(file)
    .kill_on_drop(true)
    .output()
    .await
    .map_err(|why| format!("{} could not be run: {why}", media.ffprobe))?;
  if !output.status.success() {
    return Err(format!("ffprobe refused {}: {}", file.display(), said(&output)));
  }
  let json: Value = serde_json::from_slice(&output.stdout)
    .map_err(|why| format!("ffprobe's answer does not read: {why}"))?;
  Ok(read_probe(&json))
}

/// A poster frame taken a tenth of the way in and written as JPEG to `poster`, with its size read, where it was
/// taken.
pub async fn poster(media: &Media, file: &Path, duration_ms: Option<i64>, poster: &Path) -> Option<Poster> {
  if !take_poster(media, file, duration_ms, poster).await {
    return None;
  }
  match poster_size(media, poster).await {
    Ok((width, height)) => Some(Poster {
      width: Some(width),
      height: Some(height),
    }),
    Err(why) => {
      eprintln!("bmo-theater: {why}");
      Some(Poster::default())
    }
  }
}

/// A poster frame taken a tenth of the way in and written as JPEG to `poster`; whether it was.
async fn take_poster(media: &Media, file: &Path, duration_ms: Option<i64>, poster: &Path) -> bool {
  let at = duration_ms.unwrap_or_default().max(0) as f64 / 10_000.0;
  if let Some(folder) = poster.parent()
    && tokio::fs::create_dir_all(folder).await.is_err()
  {
    return false;
  }
  let output = Command::new(&media.ffmpeg)
    .args(["-nostdin", "-v", "error", "-ss", &format!("{at:.3}"), "-i"])
    .arg(file)
    .args(["-frames:v", "1", "-vf", "scale=640:-2", "-q:v", "3", "-y"])
    .arg(poster)
    .kill_on_drop(true)
    .output()
    .await;
  match output {
    Ok(output) if output.status.success() => {
      tokio::fs::metadata(poster).await.is_ok_and(|written| written.len() > 0)
    }
    Ok(output) => {
      eprintln!("bmo-theater: no poster of {}: {}", file.display(), said(&output));
      false
    }
    Err(why) => {
      eprintln!("bmo-theater: {} could not be run: {why}", media.ffmpeg);
      false
    }
  }
}

/// A poster's pixel width and height, as ffprobe reads its first stream.
pub async fn poster_size(media: &Media, poster: &Path) -> Result<(i32, i32), String> {
  let output = Command::new(&media.ffprobe)
    .args(["-v", "error", "-print_format", "json", "-show_streams"])
    .arg(poster)
    .kill_on_drop(true)
    .output()
    .await
    .map_err(|why| format!("{} could not be run: {why}", media.ffprobe))?;
  if !output.status.success() {
    return Err(format!("ffprobe refused {}: {}", poster.display(), said(&output)));
  }
  let json: Value = serde_json::from_slice(&output.stdout)
    .map_err(|why| format!("ffprobe's answer does not read: {why}"))?;
  let first = &json["streams"][0];
  match (first["width"].as_i64(), first["height"].as_i64()) {
    (Some(width), Some(height)) => Ok((width as i32, height as i32)),
    _ => Err(format!("ffprobe read no size of {}", poster.display())),
  }
}

/// A subtitle written into a video's picture: ffmpeg run in `work`, reading `subtitle` (a file name in it) and
/// writing `output.mp4` there.
pub async fn burn(media: &Media, file: &Path, work: &Path, subtitle: &str) -> Result<(), String> {
  let output = Command::new(&media.ffmpeg)
    .current_dir(work)
    .args(["-nostdin", "-v", "error", "-i"])
    .arg(file)
    .args(["-vf", &format!("subtitles={subtitle}")])
    .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20", "-c:a", "copy"])
    .args(["-movflags", "+faststart", "-y", "output.mp4"])
    .kill_on_drop(true)
    .output()
    .await
    .map_err(|why| format!("{} could not be run: {why}", media.ffmpeg))?;
  if output.status.success() {
    Ok(())
  } else {
    Err(format!("ffmpeg failed: {}", said(&output)))
  }
}
