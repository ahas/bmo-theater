//! A library's folder read into titles and episodes (`docs/design.md`, 「Libraries and scans」): the walk, the
//! plan it makes of the names alone, and the scan that brings the tables to the plan without deleting a row.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use sqlx::Row;

use crate::app::App;
use crate::caller::Standing;
use crate::error::ApiError;
use crate::media;
use crate::names::{self, clean, parse, season_folder, video_type};
use crate::store;
use crate::wire::{TitleKind, now_ms};

/// One video file found under a library's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
  /// Its path under the folder, its parts joined by `/`.
  pub relative: String,
  pub size: i64,
  pub modified_ms: i64,
}

/// One episode the plan files.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedEpisode {
  pub relative: String,
  pub season: Option<i32>,
  pub number: Option<i32>,
  pub name: String,
}

/// One title the plan files: its key within the library, its name, its kind and its episodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTitle {
  pub key: String,
  pub name: String,
  pub kind: TitleKind,
  pub episodes: Vec<PlannedEpisode>,
}

/// The video files under a folder, its own hidden entries and a folder's links passed over.
pub fn walk(folder: &Path) -> std::io::Result<Vec<Found>> {
  let mut found = Vec::new();
  let mut folders = vec![(folder.to_path_buf(), String::new())];
  while let Some((at, relative)) = folders.pop() {
    for entry in std::fs::read_dir(&at)? {
      let entry = entry?;
      let name = entry.file_name().to_string_lossy().into_owned();
      if name.starts_with('.') {
        continue;
      }
      let path = entry.path();
      let inner = if relative.is_empty() { name.clone() } else { format!("{relative}/{name}") };
      let kind = entry.file_type()?;
      if kind.is_dir() {
        folders.push((path, inner));
        continue;
      }
      if video_type(names::extension(&name)).is_none() {
        continue;
      }
      let Ok(metadata) = std::fs::metadata(&path) else {
        continue;
      };
      if !metadata.is_file() {
        continue;
      }
      let modified_ms = metadata
        .modified()
        .ok()
        .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
        .map(|at| at.as_millis() as i64)
        .unwrap_or_default();
      found.push(Found {
        relative: inner,
        size: metadata.len() as i64,
        modified_ms,
      });
    }
  }
  found.sort_by(|a, b| a.relative.cmp(&b.relative));
  Ok(found)
}

/// The group a file stands in: the folders above it, a season's folder giving its number rather than a group of
/// its own; and the season the nearest such folder gives.
fn group_of(relative: &str) -> (Vec<String>, Option<i32>) {
  let mut parts: Vec<&str> = relative.split('/').collect();
  parts.pop();
  let mut group = Vec::new();
  let mut season = None;
  for part in parts {
    match season_folder(part) {
      Some(number) => season = Some(number),
      None => {
        group.push(part.to_owned());
        season = None;
      }
    }
  }
  (group, season)
}

/// The name the library's own group takes where it is given none: the show name most of its files share, else
/// the folder's name.
pub fn root_name(files: &[String], folder_name: &str) -> String {
  let mut shows: BTreeMap<String, usize> = BTreeMap::new();
  for relative in files {
    if !group_of(relative).0.is_empty() {
      continue;
    }
    let file = relative.rsplit('/').next().unwrap_or(relative);
    if let Some(show) = parse(names::stem(file)).show {
      *shows.entry(show).or_default() += 1;
    }
  }
  shows
    .into_iter()
    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    .map(|(show, _)| show)
    .unwrap_or_else(|| folder_name.to_owned())
}

/// The titles the files make, by their folders and their names alone.
pub fn plan(files: &[String], library_name: &str) -> Vec<PlannedTitle> {
  let mut groups: BTreeMap<Vec<String>, Vec<(String, Option<i32>)>> = BTreeMap::new();
  for relative in files {
    let (group, season) = group_of(relative);
    groups.entry(group).or_default().push((relative.clone(), season));
  }
  let mut titles = Vec::new();
  for (group, files) in groups {
    let read: Vec<(String, Option<i32>, names::Parsed)> = files
      .into_iter()
      .map(|(relative, season)| {
        let file = relative.rsplit('/').next().unwrap_or(&relative).to_owned();
        let parsed = parse(names::stem(&file));
        (relative, season, parsed)
      })
      .collect();
    let named = |parsed: &names::Parsed, relative: &str| {
      if parsed.name.is_empty() {
        let file = relative.rsplit('/').next().unwrap_or(relative);
        clean(names::stem(file))
      } else {
        parsed.name.clone()
      }
    };
    if read.iter().any(|(_, _, parsed)| parsed.number.is_some()) {
      let name = group.last().cloned().unwrap_or_else(|| library_name.to_owned());
      titles.push(PlannedTitle {
        key: format!("folder:{}", group.join("/")),
        name,
        kind: TitleKind::Show,
        episodes: read
          .iter()
          .map(|(relative, season, parsed)| PlannedEpisode {
            relative: relative.clone(),
            season: parsed.number.and(parsed.season.or(*season)),
            number: parsed.number,
            name: named(parsed, relative),
          })
          .collect(),
      });
    } else {
      for (relative, _, parsed) in &read {
        let name = named(parsed, relative);
        titles.push(PlannedTitle {
          key: format!("file:{relative}"),
          name: name.clone(),
          kind: TitleKind::Film,
          episodes: vec![PlannedEpisode {
            relative: relative.clone(),
            season: None,
            number: None,
            name,
          }],
        });
      }
    }
  }
  titles
}

/// The counts a scan keeps as it goes.
#[derive(Default)]
struct Counts {
  found: i64,
  added: i64,
  changed: i64,
  missing: i64,
}

/// A scan of a library started: the scan already running, or a new one, run in the background. Whether it is new.
pub async fn start(app: &Arc<App>, library: i64) -> Result<(crate::wire::Scan, bool), ApiError> {
  let inserted = sqlx::query(
    "INSERT INTO scans (library_id, state, started_at) VALUES ($1, 'running', $2) \
     ON CONFLICT (library_id) WHERE state = 'running' DO NOTHING RETURNING id",
  )
  .bind(library)
  .bind(now_ms())
  .fetch_optional(&app.pool)
  .await?;
  let fresh = inserted.is_some();
  let id: i64 = match inserted {
    Some(row) => row.try_get("id")?,
    None => {
      sqlx::query_scalar("SELECT id FROM scans WHERE library_id = $1 AND state = 'running'")
        .bind(library)
        .fetch_one(&app.pool)
        .await?
    }
  };
  let scan = store::scan(&app.pool, id)
    .await?
    .ok_or_else(|| ApiError::failed("the scan was not kept"))?;
  if fresh {
    let standing = library_standing(app, library).await?;
    app.publish("scan.running", &scan, &standing);
    let app = Arc::clone(app);
    tokio::spawn(async move {
      let ended = run(&app, library, id).await;
      let (state, error) = match ended {
        Ok(()) => ("finished", None),
        Err(why) => {
          eprintln!("bmo-theater: the scan {id} of library {library} failed: {why}");
          ("failed", Some(why))
        }
      };
      let now = now_ms();
      let _ = sqlx::query("UPDATE scans SET state = $2, error = $3, ended_at = $4 WHERE id = $1")
        .bind(id)
        .bind(state)
        .bind(error)
        .bind(now)
        .execute(&app.pool)
        .await;
      if state == "finished" {
        let _ = sqlx::query("UPDATE libraries SET scanned_at = $2 WHERE id = $1")
          .bind(library)
          .bind(now)
          .execute(&app.pool)
          .await;
      }
      if let (Ok(Some(scan)), Ok(standing)) =
        (store::scan(&app.pool, id).await, library_standing(&app, library).await)
      {
        app.publish(&format!("scan.{state}"), &scan, &standing);
      }
    });
  }
  Ok((scan, fresh))
}

async fn library_standing(app: &App, library: i64) -> Result<Standing, ApiError> {
  store::library(&app.pool, library)
    .await?
    .map(|(_, standing)| standing)
    .ok_or_else(|| ApiError::not_found("No library stands there."))
}

/// What ffprobe reads of a file, and the poster it is given; a failure leaves the fields empty.
pub async fn read_file(app: &App, episode: i64, path: &Path) -> (media::Probe, Option<media::Poster>) {
  let probe = match media::probe(&app.settings.media, path).await {
    Ok(probe) => probe,
    Err(why) => {
      eprintln!("bmo-theater: {why}");
      media::Probe::default()
    }
  };
  let poster = if probe.has_video() {
    media::poster(&app.settings.media, path, probe.duration_ms, &app.poster_path(episode)).await
  } else {
    None
  };
  (probe, poster)
}

/// The fields a probe and a poster write into an episode's row.
pub async fn write_read(
  app: &App,
  episode: i64,
  probe: &media::Probe,
  poster: Option<media::Poster>,
) -> Result<(), sqlx::Error> {
  let size = poster.unwrap_or_default();
  sqlx::query(
    "UPDATE episodes SET duration_ms = $2, width = $3, height = $4, video_codec = $5, audio_codec = $6, \
     has_poster = $7, poster_width = $8, poster_height = $9, changed_at = $10 WHERE id = $1",
  )
  .bind(episode)
  .bind(probe.duration_ms)
  .bind(probe.width)
  .bind(probe.height)
  .bind(&probe.video_codec)
  .bind(&probe.audio_codec)
  .bind(poster.is_some())
  .bind(size.width)
  .bind(size.height)
  .bind(now_ms())
  .execute(&app.pool)
  .await?;
  Ok(())
}

/// The size of every poster kept before its size was read, read with ffprobe one at a time; a poster that does
/// not read is noted and passed over (`docs/design.md`, 「Libraries and scans」).
pub async fn read_poster_sizes(app: Arc<App>) {
  let unread: Vec<i64> = match sqlx::query_scalar(
    "SELECT id FROM episodes WHERE has_poster AND (poster_width IS NULL OR poster_height IS NULL) ORDER BY id",
  )
  .fetch_all(&app.pool)
  .await
  {
    Ok(unread) => unread,
    Err(why) => {
      eprintln!("bmo-theater: the posters of unread size could not be listed: {why}");
      return;
    }
  };
  for episode in unread {
    let (width, height) = match media::poster_size(&app.settings.media, &app.poster_path(episode)).await {
      Ok(size) => size,
      Err(why) => {
        eprintln!("bmo-theater: {why}");
        continue;
      }
    };
    let written =
      sqlx::query("UPDATE episodes SET poster_width = $2, poster_height = $3 WHERE id = $1 AND has_poster")
        .bind(episode)
        .bind(width)
        .bind(height)
        .execute(&app.pool)
        .await;
    if let Err(why) = written {
      eprintln!("bmo-theater: the poster size of episode {episode} could not be kept: {why}");
    }
  }
}

async fn publish_episode(app: &App, kind: &str, episode: i64) {
  if let Ok(Some(stored)) = store::episode(&app.pool, episode, None).await {
    app.publish(kind, &stored.episode, &stored.standing);
  }
}

/// One scan run to its end.
async fn run(app: &Arc<App>, library: i64, scan: i64) -> Result<(), String> {
  let fault = |why: sqlx::Error| format!("the database failed: {why}");
  let row = sqlx::query("SELECT * FROM libraries WHERE id = $1")
    .bind(library)
    .fetch_optional(&app.pool)
    .await
    .map_err(fault)?
    .ok_or("the library is gone")?;
  let folder = PathBuf::from(row.try_get::<String, _>("path").map_err(fault)?);
  let library_name: String = row.try_get("name").map_err(fault)?;
  let realm: String = row.try_get("realm").map_err(fault)?;
  let owner: String = row.try_get("owner").map_err(fault)?;
  let creator: String = row.try_get("creator").map_err(fault)?;

  let walked = folder.clone();
  let found = tokio::task::spawn_blocking(move || walk(&walked))
    .await
    .map_err(|why| format!("the walk failed: {why}"))?
    .map_err(|why| format!("{} could not be read: {why}", folder.display()))?;
  let by_path: HashMap<&str, &Found> = found.iter().map(|file| (file.relative.as_str(), file)).collect();
  let relatives: Vec<String> = found.iter().map(|file| file.relative.clone()).collect();
  let planned = plan(&relatives, &library_name);
  let mut counts = Counts {
    found: found.len() as i64,
    ..Counts::default()
  };

  let standing_rows = sqlx::query(
    "SELECT id, library_key, byte_count, modified_at, available FROM episodes WHERE library_id = $1",
  )
  .bind(library)
  .fetch_all(&app.pool)
  .await
  .map_err(fault)?;
  let mut standing_episodes: HashMap<String, (i64, i64, i64, bool)> = HashMap::new();
  for row in &standing_rows {
    let key: Option<String> = row.try_get("library_key").map_err(fault)?;
    if let Some(key) = key {
      standing_episodes.insert(
        key,
        (
          row.try_get("id").map_err(fault)?,
          row.try_get("byte_count").map_err(fault)?,
          row.try_get("modified_at").map_err(fault)?,
          row.try_get("available").map_err(fault)?,
        ),
      );
    }
  }

  for title in &planned {
    let now = now_ms();
    let made = sqlx::query(
      "INSERT INTO titles (realm, owner, creator, name, kind, library_id, library_key, created_at, changed_at) \
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $8) ON CONFLICT (library_id, library_key) DO NOTHING RETURNING id",
    )
    .bind(&realm)
    .bind(&owner)
    .bind(&creator)
    .bind(&title.name)
    .bind(title.kind.word())
    .bind(library)
    .bind(&title.key)
    .bind(now)
    .fetch_optional(&app.pool)
    .await
    .map_err(fault)?;
    let (title_id, retired, new_title) = match made {
      Some(row) => (row.try_get::<i64, _>("id").map_err(fault)?, false, true),
      None => {
        let row = sqlx::query("SELECT id, retired FROM titles WHERE library_id = $1 AND library_key = $2")
          .bind(library)
          .bind(&title.key)
          .fetch_one(&app.pool)
          .await
          .map_err(fault)?;
        (row.try_get("id").map_err(fault)?, row.try_get("retired").map_err(fault)?, false)
      }
    };
    if retired {
      continue;
    }
    if new_title && let Ok(Some((made, standing))) = store::title(&app.pool, title_id).await {
      app.publish("title.created", &made, &standing);
    }
    for planned in &title.episodes {
      let Some(file) = by_path.get(planned.relative.as_str()) else {
        continue;
      };
      let path = folder.join(&planned.relative);
      match standing_episodes.get(&planned.relative) {
        Some((id, size, modified, available)) => {
          let reread = *size != file.size || *modified != file.modified_ms;
          if !reread && *available {
            continue;
          }
          sqlx::query(
            "UPDATE episodes SET available = TRUE, byte_count = $2, modified_at = $3, path = $4, changed_at = $5 \
             WHERE id = $1",
          )
          .bind(id)
          .bind(file.size)
          .bind(file.modified_ms)
          .bind(path.to_string_lossy().as_ref())
          .bind(now_ms())
          .execute(&app.pool)
          .await
          .map_err(fault)?;
          if reread {
            let (probe, poster) = read_file(app, *id, &path).await;
            write_read(app, *id, &probe, poster).await.map_err(fault)?;
          }
          counts.changed += 1;
          publish_episode(app, "episode.changed", *id).await;
        }
        None => {
          let file_name = planned.relative.rsplit('/').next().unwrap_or(&planned.relative).to_owned();
          let media_type = video_type(names::extension(&file_name)).unwrap_or("application/octet-stream");
          let now = now_ms();
          let made = sqlx::query(
            "INSERT INTO episodes (title_id, kind, season, number, name, path, library_id, library_key, file_name, \
             media_type, byte_count, modified_at, creator, created_at, changed_at) \
             VALUES ($1, 'episode', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $13) \
             ON CONFLICT (library_id, library_key) DO NOTHING RETURNING id",
          )
          .bind(title_id)
          .bind(planned.season)
          .bind(planned.number)
          .bind(&planned.name)
          .bind(path.to_string_lossy().as_ref())
          .bind(library)
          .bind(&planned.relative)
          .bind(&file_name)
          .bind(media_type)
          .bind(file.size)
          .bind(file.modified_ms)
          .bind(&creator)
          .bind(now)
          .fetch_optional(&app.pool)
          .await
          .map_err(fault)?;
          let Some(made) = made else {
            continue;
          };
          let id: i64 = made.try_get("id").map_err(fault)?;
          let (probe, poster) = read_file(app, id, &path).await;
          write_read(app, id, &probe, poster).await.map_err(fault)?;
          counts.added += 1;
          publish_episode(app, "episode.created", id).await;
        }
      }
      write_counts(app, scan, &counts).await.map_err(fault)?;
    }
  }

  let present: HashSet<&str> = found.iter().map(|file| file.relative.as_str()).collect();
  for (key, (id, _, _, available)) in &standing_episodes {
    if *available && !present.contains(key.as_str()) {
      sqlx::query("UPDATE episodes SET available = FALSE, changed_at = $2 WHERE id = $1")
        .bind(id)
        .bind(now_ms())
        .execute(&app.pool)
        .await
        .map_err(fault)?;
      counts.missing += 1;
      publish_episode(app, "episode.changed", *id).await;
    }
  }
  write_counts(app, scan, &counts).await.map_err(fault)
}

async fn write_counts(app: &App, scan: i64, counts: &Counts) -> Result<(), sqlx::Error> {
  sqlx::query(
    "UPDATE scans SET found_count = $2, added_count = $3, changed_count = $4, missing_count = $5 WHERE id = $1",
  )
  .bind(scan)
  .bind(counts.found)
  .bind(counts.added)
  .bind(counts.changed)
  .bind(counts.missing)
  .execute(&app.pool)
  .await?;
  Ok(())
}
