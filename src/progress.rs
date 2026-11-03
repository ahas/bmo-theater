//! A viewer's progress through an ordered list of episodes (`docs/design.md`, 「Playing」; `docs/api.md`,
//! 「Positions and progress」): each one's position, and the one to watch next.

use crate::wire::{Position, Progress, ProgressItem};

/// The share of an episode's duration past which it counts as watched.
pub const DONE_SHARE: f64 = 0.9;

/// Whether a position of `seconds` counts as watched in an episode lasting `duration_ms`.
pub fn reaches_done(seconds: f64, duration_ms: Option<i64>) -> bool {
  duration_ms.is_some_and(|duration| duration > 0 && seconds * 1000.0 >= duration as f64 * DONE_SHARE)
}

/// The progress of the episodes in order, each with its duration and the viewer's position where it has one:
/// the one to watch next is the one last played where it is not finished; else the first unfinished one after
/// it, and from the start where none follows; else the first, where nothing was played.
pub fn progress(items: &[(i64, Option<i64>, Option<Position>)]) -> Progress {
  let rows: Vec<ProgressItem> = items
    .iter()
    .map(|(episode_id, duration_ms, position)| ProgressItem {
      episode_id: *episode_id,
      duration_ms: *duration_ms,
      seconds: position.as_ref().map(|position| position.seconds).unwrap_or_default(),
      done: position.as_ref().is_some_and(|position| position.done),
    })
    .collect();
  let done_count = rows.iter().filter(|row| row.done).count() as i64;
  let last = items
    .iter()
    .enumerate()
    .filter_map(|(at, (_, _, position))| position.as_ref().map(|position| (at, position.updated_at)))
    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    .map(|(at, _)| at);
  let next = match last {
    Some(at) if !rows[at].done => Some(at),
    Some(at) => rows
      .iter()
      .enumerate()
      .skip(at + 1)
      .chain(rows.iter().enumerate().take(at))
      .find(|(_, row)| !row.done)
      .map(|(at, _)| at),
    None => (!rows.is_empty()).then_some(0),
  };
  Progress {
    done_count,
    next: next.map(|at| rows[at].episode_id),
    next_seconds: next.map(|at| if rows[at].done { 0.0 } else { rows[at].seconds }).unwrap_or_default(),
    items: rows,
  }
}
