//! A subtitle written into an episode's picture (`docs/api.md`, 「Burns」): ffmpeg run in the background with no
//! clock bound, its output filed as a `clip` episode under the same title.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sqlx::Row;

use crate::app::{App, remove_folder};
use crate::error::ApiError;
use crate::scan::{read_file, write_read};
use crate::store;
use crate::subtitles::decode;
use crate::wire::{Burn, SubtitleFormat, now_ms};

/// The file a subtitle's original is kept as, in its folder.
pub fn original_name(format: SubtitleFormat) -> String {
  format!("original.{}", format.word())
}

/// A burn started: its row made and its work run in the background.
pub async fn start(app: &Arc<App>, episode: i64, subtitle: i64, creator: String) -> Result<Burn, ApiError> {
  let row = sqlx::query(
    "INSERT INTO burns (episode_id, subtitle_id, state, creator, started_at) VALUES ($1, $2, 'running', $3, $4) \
     RETURNING id",
  )
  .bind(episode)
  .bind(subtitle)
  .bind(&creator)
  .bind(now_ms())
  .fetch_one(&app.pool)
  .await?;
  let id: i64 = row.try_get("id")?;
  let burn = store::burn(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::failed("the burn was not kept"))?;
  let standing = store::episode(&app.pool, episode, None)
    .await?
    .map(|stored| stored.standing)
    .ok_or_else(|| ApiError::not_found("No episode stands there."))?;
  app.publish("burn.running", &burn, &standing);
  let app = Arc::clone(app);
  tokio::spawn(async move {
    let ended = run(&app, id, episode, subtitle, &creator).await;
    let (state, clip, error) = match ended {
      Ok(clip) => ("finished", Some(clip), None),
      Err(why) => {
        eprintln!("bmo-theater: the burn {id} failed: {why}");
        ("failed", None, Some(why))
      }
    };
    remove_folder(&app.burn_folder(id)).await;
    let _ =
      sqlx::query("UPDATE burns SET state = $2, clip_id = $3, error = $4, ended_at = $5 WHERE id = $1")
        .bind(id)
        .bind(state)
        .bind(clip)
        .bind(error)
        .bind(now_ms())
        .execute(&app.pool)
        .await;
    if let Ok(Some(burn)) = store::burn(&app.pool, id).await {
      app.publish(&format!("burn.{state}"), &burn, &standing);
    }
  });
  Ok(burn)
}

/// The subtitle the burn reads, written in its work folder in UTF-8: the ASS original, which keeps its styles,
/// or the WebVTT. Its file name in the folder.
async fn subtitle_file(
  app: &App,
  work: &Path,
  subtitle: i64,
  format: SubtitleFormat,
) -> Result<String, String> {
  let folder = app.subtitle_folder(subtitle);
  let (from, name) = match format {
    SubtitleFormat::Ass => (folder.join(original_name(format)), "subtitle.ass"),
    _ => (app.vtt_path(subtitle), "subtitle.vtt"),
  };
  let bytes = tokio::fs::read(&from)
    .await
    .map_err(|why| format!("{} could not be read: {why}", from.display()))?;
  tokio::fs::write(work.join(name), decode(&bytes))
    .await
    .map_err(|why| format!("the burn's subtitle could not be written: {why}"))?;
  Ok(name.to_owned())
}

/// One burn run to its end: the clip it filed.
async fn run(app: &Arc<App>, id: i64, episode: i64, subtitle: i64, creator: &str) -> Result<i64, String> {
  let fault = |why: sqlx::Error| format!("the database failed: {why}");
  let source = store::episode(&app.pool, episode, None)
    .await
    .map_err(fault)?
    .ok_or("the episode is gone")?;
  let subtitle_row = store::subtitle(&app.pool, subtitle)
    .await
    .map_err(fault)?
    .ok_or("the subtitle is gone")?;
  let work = app.burn_folder(id);
  tokio::fs::create_dir_all(&work)
    .await
    .map_err(|why| format!("{} could not be made: {why}", work.display()))?;
  let name = subtitle_file(app, &work, subtitle, subtitle_row.format).await?;
  crate::media::burn(&app.settings.media, Path::new(&source.path), &work, &name).await?;

  let label = if subtitle_row.label.is_empty() { &subtitle_row.language } else { &subtitle_row.label };
  let stem = crate::names::stem(&source.episode.file_name);
  let file_name = format!("{stem}.mp4");
  let now = now_ms();
  let row = sqlx::query(
    "INSERT INTO episodes (title_id, kind, name, label, path, file_name, media_type, byte_count, modified_at, \
     burned_episode, burned_subtitle, creator, created_at, changed_at) \
     VALUES ($1, 'clip', $2, $3, '', $4, 'video/mp4', 0, 0, $5, $6, $7, $8, $8) RETURNING id",
  )
  .bind(source.episode.title_id)
  .bind(&source.episode.name)
  .bind(format!("burned: {label}"))
  .bind(&file_name)
  .bind(episode)
  .bind(subtitle)
  .bind(creator)
  .bind(now)
  .fetch_one(&app.pool)
  .await
  .map_err(fault)?;
  let clip: i64 = row.try_get("id").map_err(fault)?;
  let filed = file_into(app, clip, &work.join("output.mp4"), &file_name).await;
  let (size, modified) = match filed {
    Ok(written) => written,
    Err(why) => {
      let _ = sqlx::query("DELETE FROM episodes WHERE id = $1").bind(clip).execute(&app.pool).await;
      return Err(why);
    }
  };
  let path = app.episode_folder(clip).join(&file_name);
  sqlx::query("UPDATE episodes SET path = $2, byte_count = $3, modified_at = $4 WHERE id = $1")
    .bind(clip)
    .bind(path.to_string_lossy().as_ref())
    .bind(size)
    .bind(modified)
    .execute(&app.pool)
    .await
    .map_err(fault)?;
  let (probe, poster) = read_file(app, clip, &path).await;
  write_read(app, clip, &probe, poster).await.map_err(fault)?;
  if let Ok(Some(stored)) = store::episode(&app.pool, clip, None).await {
    app.publish("episode.created", &stored.episode, &stored.standing);
  }
  Ok(clip)
}

/// A file moved into an episode's folder under its name: its size and modification time.
pub async fn file_into(
  app: &App,
  episode: i64,
  from: &Path,
  file_name: &str,
) -> Result<(i64, i64), String> {
  let folder = app.episode_folder(episode);
  tokio::fs::create_dir_all(&folder)
    .await
    .map_err(|why| format!("{} could not be made: {why}", folder.display()))?;
  let to: PathBuf = folder.join(file_name);
  if tokio::fs::rename(from, &to).await.is_err() {
    tokio::fs::copy(from, &to)
      .await
      .map_err(|why| format!("{} could not be filed: {why}", to.display()))?;
    let _ = tokio::fs::remove_file(from).await;
  }
  let metadata = tokio::fs::metadata(&to).await.map_err(|why| format!("{}: {why}", to.display()))?;
  let modified = metadata
    .modified()
    .ok()
    .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
    .map(|at| at.as_millis() as i64)
    .unwrap_or_default();
  Ok((metadata.len() as i64, modified))
}
