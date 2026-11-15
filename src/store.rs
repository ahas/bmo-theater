//! The theater's rows read into the wire's members, and the reads every route shares: each member with where it
//! stands, the episodes with the caller's position, a playlist with its order.

use std::collections::BTreeMap;

use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, QueryBuilder, Row};

use crate::caller::Standing;
use crate::wire::{
  Burn, BurnedFrom, Episode, EpisodeKind, Grant, JobState, Library, Permission, Playlist, Position, Realm,
  Scan, Share, Subtitle, SubtitleFormat, Title, TitleKind,
};

type Read<T> = Result<T, sqlx::Error>;

fn decode(what: &str, word: &str) -> sqlx::Error {
  sqlx::Error::Decode(format!("{what} {word} is none the service writes").into())
}

fn realm(row: &PgRow) -> Read<Realm> {
  let word: String = row.try_get("realm")?;
  Realm::of(&word).ok_or_else(|| decode("the realm", &word))
}

fn state(row: &PgRow) -> Read<JobState> {
  let word: String = row.try_get("state")?;
  JobState::of(&word).ok_or_else(|| decode("the state", &word))
}

/// Where the row's member stands, read from its `realm`, `owner` and the named creator column.
fn standing(row: &PgRow, creator: &str) -> Read<Standing> {
  Ok(Standing {
    realm: realm(row)?,
    owner: row.try_get("owner")?,
    creator: row.try_get(creator)?,
  })
}

// ---------------------------------------------------------------------------------------------------------
// Libraries and scans

/// What a library is read with.
pub const LIBRARY: &str = "SELECT l.*, \
  EXISTS (SELECT 1 FROM scans s WHERE s.library_id = l.id AND s.state = 'running') AS scanning, \
  (SELECT COUNT(*) FROM titles t WHERE t.library_id = l.id AND NOT t.retired) AS title_count, \
  (SELECT COUNT(*) FROM episodes e JOIN titles t ON t.id = e.title_id \
    WHERE e.library_id = l.id AND NOT t.retired) AS episode_count \
  FROM libraries l";

pub fn library_of(row: &PgRow) -> Read<(Library, Standing)> {
  Ok((
    Library {
      id: row.try_get("id")?,
      realm: realm(row)?,
      name: row.try_get("name")?,
      path: row.try_get("path")?,
      creator: row.try_get("creator")?,
      created_at: row.try_get("created_at")?,
      scanned_at: row.try_get("scanned_at")?,
      scanning: row.try_get("scanning")?,
      title_count: row.try_get("title_count")?,
      episode_count: row.try_get("episode_count")?,
    },
    standing(row, "creator")?,
  ))
}

pub async fn library(pool: &PgPool, id: i64) -> Read<Option<(Library, Standing)>> {
  sqlx::query(&format!("{LIBRARY} WHERE l.id = $1"))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(library_of)
    .transpose()
}

pub fn scan_of(row: &PgRow) -> Read<Scan> {
  Ok(Scan {
    id: row.try_get("id")?,
    library_id: row.try_get("library_id")?,
    state: state(row)?,
    found_count: row.try_get("found_count")?,
    added_count: row.try_get("added_count")?,
    changed_count: row.try_get("changed_count")?,
    missing_count: row.try_get("missing_count")?,
    error: row.try_get("error")?,
    started_at: row.try_get("started_at")?,
    ended_at: row.try_get("ended_at")?,
  })
}

pub async fn scan(pool: &PgPool, id: i64) -> Read<Option<Scan>> {
  sqlx::query("SELECT * FROM scans WHERE id = $1")
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(scan_of)
    .transpose()
}

// ---------------------------------------------------------------------------------------------------------
// Titles

/// What a title is read with; every read adds its own condition after `WHERE NOT t.retired`. Its poster's size is
/// that of its first episode with a poster, in the order of its episodes.
pub const TITLE: &str = "SELECT t.*, \
  (SELECT COUNT(*) FROM episodes e WHERE e.title_id = t.id) AS episode_count, \
  EXISTS (SELECT 1 FROM episodes e WHERE e.title_id = t.id AND e.has_poster) AS has_poster, \
  GREATEST(t.created_at, (SELECT MAX(e.created_at) FROM episodes e WHERE e.title_id = t.id)) AS added_at, \
  poster.poster_width, poster.poster_height \
  FROM titles t LEFT JOIN LATERAL (SELECT e.poster_width, e.poster_height FROM episodes e \
    WHERE e.title_id = t.id AND e.has_poster \
    ORDER BY (e.number IS NULL), e.season NULLS LAST, e.number, e.name, e.id LIMIT 1) poster ON TRUE \
  WHERE NOT t.retired";

pub fn title_of(row: &PgRow) -> Read<(Title, Standing)> {
  let kind: String = row.try_get("kind")?;
  let labels: String = row.try_get("labels")?;
  Ok((
    Title {
      id: row.try_get("id")?,
      realm: realm(row)?,
      name: row.try_get("name")?,
      kind: TitleKind::of(&kind).ok_or_else(|| decode("the title kind", &kind))?,
      description: row.try_get("description")?,
      labels: serde_json::from_str::<BTreeMap<String, String>>(&labels).unwrap_or_default(),
      library_id: row.try_get("library_id")?,
      episode_count: row.try_get("episode_count")?,
      has_poster: row.try_get("has_poster")?,
      creator: row.try_get("creator")?,
      created_at: row.try_get("created_at")?,
      changed_at: row.try_get("changed_at")?,
      added_at: row.try_get("added_at")?,
      poster_width: row.try_get("poster_width")?,
      poster_height: row.try_get("poster_height")?,
    },
    standing(row, "creator")?,
  ))
}

pub async fn title(pool: &PgPool, id: i64) -> Read<Option<(Title, Standing)>> {
  sqlx::query(&format!("{TITLE} AND t.id = $1"))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(title_of)
    .transpose()
}

// ---------------------------------------------------------------------------------------------------------
// Episodes

/// What an episode is read with, its caller's position named by the first bound value; every read adds its own
/// condition after `WHERE TRUE`.
pub const EPISODE: &str = "SELECT e.*, t.realm, t.owner, t.creator AS title_creator, \
  (SELECT COUNT(*) FROM subtitles s WHERE s.episode_id = e.id) AS subtitle_count, \
  p.seconds AS position_seconds, p.done AS position_done, p.updated_at AS position_updated_at, \
  p.set_aside AS position_set_aside \
  FROM episodes e JOIN titles t ON t.id = e.title_id AND NOT t.retired \
  LEFT JOIN positions p ON p.episode_id = e.id AND p.account = ";

/// The order of a title's episodes: numbered ones by season and number, then the rest, by name.
pub const EPISODE_ORDER: &str = " ORDER BY (e.number IS NULL), e.season NULLS LAST, e.number, e.name, e.id";

/// An episode as it is stored, beside the wire's: its path and where it was indexed from.
#[derive(Clone, Debug)]
pub struct Stored {
  pub episode: Episode,
  pub standing: Standing,
  pub path: String,
  pub library_id: Option<i64>,
}

pub fn episode_of(row: &PgRow) -> Read<Stored> {
  let kind: String = row.try_get("kind")?;
  let id: i64 = row.try_get("id")?;
  let seconds: Option<f64> = row.try_get("position_seconds")?;
  let position = match seconds {
    Some(seconds) => Some(Position {
      episode_id: id,
      seconds,
      done: row.try_get::<Option<bool>, _>("position_done")?.unwrap_or_default(),
      updated_at: row.try_get::<Option<i64>, _>("position_updated_at")?.unwrap_or_default(),
      set_aside: row.try_get::<Option<bool>, _>("position_set_aside")?.unwrap_or_default(),
    }),
    None => None,
  };
  let burned_episode: Option<i64> = row.try_get("burned_episode")?;
  let burned_subtitle: Option<i64> = row.try_get("burned_subtitle")?;
  Ok(Stored {
    episode: Episode {
      id,
      title_id: row.try_get("title_id")?,
      realm: realm(row)?,
      kind: EpisodeKind::of(&kind).ok_or_else(|| decode("the episode kind", &kind))?,
      season: row.try_get("season")?,
      number: row.try_get("number")?,
      name: row.try_get("name")?,
      label: row.try_get("label")?,
      file_name: row.try_get("file_name")?,
      media_type: row.try_get("media_type")?,
      byte_count: row.try_get("byte_count")?,
      duration_ms: row.try_get("duration_ms")?,
      width: row.try_get("width")?,
      height: row.try_get("height")?,
      video_codec: row.try_get("video_codec")?,
      audio_codec: row.try_get("audio_codec")?,
      available: row.try_get("available")?,
      has_poster: row.try_get("has_poster")?,
      subtitle_count: row.try_get("subtitle_count")?,
      burned_from: burned_episode.zip(burned_subtitle).map(|(episode_id, subtitle_id)| BurnedFrom {
        episode_id,
        subtitle_id,
      }),
      position,
      granted: false,
      poster_width: row.try_get("poster_width")?,
      poster_height: row.try_get("poster_height")?,
      creator: row.try_get("creator")?,
      created_at: row.try_get("created_at")?,
      changed_at: row.try_get("changed_at")?,
    },
    standing: standing(row, "title_creator")?,
    path: row.try_get("path")?,
    library_id: row.try_get("library_id")?,
  })
}

/// One episode, with the position of `account` where it names one.
pub async fn episode(pool: &PgPool, id: i64, account: Option<&str>) -> Read<Option<Stored>> {
  sqlx::query(&format!("{EPISODE}$1 WHERE e.id = $2"))
    .bind(account.unwrap_or_default())
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(episode_of)
    .transpose()
}

/// A title's episodes in order.
pub async fn title_episodes(pool: &PgPool, title: i64, account: Option<&str>) -> Read<Vec<Stored>> {
  sqlx::query(&format!("{EPISODE}$1 WHERE e.title_id = $2{EPISODE_ORDER}"))
    .bind(account.unwrap_or_default())
    .bind(title)
    .fetch_all(pool)
    .await?
    .iter()
    .map(episode_of)
    .collect()
}

/// The episodes named, in the order named, those that stand.
pub async fn episodes_named(pool: &PgPool, ids: &[i64], account: Option<&str>) -> Read<Vec<Stored>> {
  if ids.is_empty() {
    return Ok(Vec::new());
  }
  let found: Vec<Stored> = sqlx::query(&format!("{EPISODE}$1 WHERE e.id = ANY($2)"))
    .bind(account.unwrap_or_default())
    .bind(ids.to_vec())
    .fetch_all(pool)
    .await?
    .iter()
    .map(episode_of)
    .collect::<Read<_>>()?;
  Ok(
    ids
      .iter()
      .filter_map(|id| found.iter().find(|stored| stored.episode.id == *id).cloned())
      .collect(),
  )
}

// ---------------------------------------------------------------------------------------------------------
// Subtitles and burns

pub fn subtitle_of(row: &PgRow) -> Read<Subtitle> {
  let format: String = row.try_get("format")?;
  Ok(Subtitle {
    id: row.try_get("id")?,
    episode_id: row.try_get("episode_id")?,
    language: row.try_get("language")?,
    label: row.try_get("label")?,
    format: SubtitleFormat::of(&format).ok_or_else(|| decode("the subtitle format", &format))?,
    file_name: row.try_get("file_name")?,
    cue_count: row.try_get("cue_count")?,
    creator: row.try_get("creator")?,
    created_at: row.try_get("created_at")?,
  })
}

pub async fn subtitle(pool: &PgPool, id: i64) -> Read<Option<Subtitle>> {
  sqlx::query("SELECT * FROM subtitles WHERE id = $1")
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(subtitle_of)
    .transpose()
}

pub async fn episode_subtitles(pool: &PgPool, episode: i64) -> Read<Vec<Subtitle>> {
  sqlx::query("SELECT * FROM subtitles WHERE episode_id = $1 ORDER BY id")
    .bind(episode)
    .fetch_all(pool)
    .await?
    .iter()
    .map(subtitle_of)
    .collect()
}

pub fn burn_of(row: &PgRow) -> Read<Burn> {
  Ok(Burn {
    id: row.try_get("id")?,
    episode_id: row.try_get("episode_id")?,
    subtitle_id: row.try_get("subtitle_id")?,
    state: state(row)?,
    clip_id: row.try_get("clip_id")?,
    error: row.try_get("error")?,
    creator: row.try_get("creator")?,
    started_at: row.try_get("started_at")?,
    ended_at: row.try_get("ended_at")?,
  })
}

pub async fn burn(pool: &PgPool, id: i64) -> Read<Option<Burn>> {
  sqlx::query("SELECT * FROM burns WHERE id = $1")
    .bind(id)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(burn_of)
    .transpose()
}

// ---------------------------------------------------------------------------------------------------------
// Playlists

pub async fn playlist_from(pool: &PgPool, row: &PgRow) -> Read<(Playlist, Standing)> {
  let id: i64 = row.try_get("id")?;
  let items = sqlx::query(
    "SELECT i.episode_id, e.duration_ms FROM playlist_items i JOIN episodes e ON e.id = i.episode_id \
     JOIN titles t ON t.id = e.title_id AND NOT t.retired WHERE i.playlist_id = $1 ORDER BY i.ord",
  )
  .bind(id)
  .fetch_all(pool)
  .await?;
  let mut episode_ids = Vec::new();
  let mut duration_ms = 0;
  for item in &items {
    episode_ids.push(item.try_get::<i64, _>("episode_id")?);
    duration_ms += item.try_get::<Option<i64>, _>("duration_ms")?.unwrap_or_default();
  }
  Ok((
    Playlist {
      id,
      realm: realm(row)?,
      name: row.try_get("name")?,
      episode_ids,
      duration_ms,
      creator: row.try_get("creator")?,
      created_at: row.try_get("created_at")?,
      changed_at: row.try_get("changed_at")?,
    },
    standing(row, "creator")?,
  ))
}

pub async fn playlist(pool: &PgPool, id: i64) -> Read<Option<(Playlist, Standing)>> {
  match sqlx::query("SELECT * FROM playlists WHERE id = $1")
    .bind(id)
    .fetch_optional(pool)
    .await?
  {
    Some(row) => Ok(Some(playlist_from(pool, &row).await?)),
    None => Ok(None),
  }
}

/// A playlist's order written whole in place of the one standing.
pub async fn write_items(
  transaction: &mut sqlx::Transaction<'_, Postgres>,
  playlist: i64,
  episodes: &[i64],
) -> Read<()> {
  sqlx::query("DELETE FROM playlist_items WHERE playlist_id = $1")
    .bind(playlist)
    .execute(&mut **transaction)
    .await?;
  if episodes.is_empty() {
    return Ok(());
  }
  let mut insert =
    QueryBuilder::<Postgres>::new("INSERT INTO playlist_items (playlist_id, ord, episode_id) ");
  insert.push_values(episodes.iter().enumerate(), |mut values, (ord, episode)| {
    values.push_bind(playlist).push_bind(ord as i32).push_bind(*episode);
  });
  insert.build().execute(&mut **transaction).await?;
  Ok(())
}

// ---------------------------------------------------------------------------------------------------------
// Positions

pub async fn position(pool: &PgPool, account: &str, episode: i64) -> Read<Option<Position>> {
  let row = sqlx::query("SELECT * FROM positions WHERE account = $1 AND episode_id = $2")
    .bind(account)
    .bind(episode)
    .fetch_optional(pool)
    .await?;
  row
    .map(|row| {
      Ok(Position {
        episode_id: row.try_get("episode_id")?,
        seconds: row.try_get("seconds")?,
        done: row.try_get("done")?,
        updated_at: row.try_get("updated_at")?,
        set_aside: row.try_get("set_aside")?,
      })
    })
    .transpose()
}

// ---------------------------------------------------------------------------------------------------------
// Share links

/// A link as stored, its address made from the share address.
pub fn share_of(row: &PgRow, address: &str) -> Read<Share> {
  let token: String = row.try_get("token")?;
  let address = address.trim_end_matches('/');
  Ok(Share {
    url: (!address.is_empty()).then(|| format!("{address}/{token}")),
    token,
    episode_id: row.try_get("episode_id")?,
    playlist_id: row.try_get("playlist_id")?,
    creator: row.try_get("creator")?,
    created_at: row.try_get("created_at")?,
    expires_at: row.try_get("expires_at")?,
  })
}

pub async fn share(pool: &PgPool, token: &str, address: &str) -> Read<Option<Share>> {
  sqlx::query("SELECT * FROM shares WHERE token = $1")
    .bind(token)
    .fetch_optional(pool)
    .await?
    .as_ref()
    .map(|row| share_of(row, address))
    .transpose()
}

// Grants

/// A grant as stored.
pub fn grant_of(row: &PgRow) -> Read<Grant> {
  let permission: String = row.try_get("permission")?;
  Ok(Grant {
    user: row.try_get("account")?,
    permission: Permission::of(&permission).ok_or_else(|| decode("the permission", &permission))?,
    grantor: row.try_get("grantor")?,
    granted_at: row.try_get("granted_at")?,
  })
}

/// An episode's grants, oldest first.
pub async fn grants(pool: &PgPool, episode: i64) -> Read<Vec<Grant>> {
  sqlx::query("SELECT * FROM grants WHERE episode_id = $1 ORDER BY granted_at, account")
    .bind(episode)
    .fetch_all(pool)
    .await?
    .iter()
    .map(grant_of)
    .collect()
}

/// Whether the episode is granted to the account.
pub async fn granted(pool: &PgPool, episode: i64, account: &str) -> Read<bool> {
  sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM grants WHERE episode_id = $1 AND account = $2)")
    .bind(episode)
    .bind(account)
    .fetch_one(pool)
    .await
}
