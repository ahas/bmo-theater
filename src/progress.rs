//! A viewer's progress through an ordered list of episodes (`docs/design.md`, 「Playing」; `docs/api.md`,
//! 「Positions and progress」): each one's position, the one to watch next, and the one to go on with along a
//! title's main line.

use crate::wire::{Position, Progress, ProgressItem, TitleKind};

/// The share of an episode's duration past which it counts as watched.
pub const DONE_SHARE: f64 = 0.9;

/// Whether a position of `seconds` counts as watched in an episode lasting `duration_ms`.
pub fn reaches_done(seconds: f64, duration_ms: Option<i64>) -> bool {
  duration_ms.is_some_and(|duration| duration > 0 && seconds * 1000.0 >= duration as f64 * DONE_SHARE)
}

/// One episode in order: its id, its duration, and the viewer's position where it has one.
pub type Item = (i64, Option<i64>, Option<Position>);

/// Whether an episode of a title of `kind` stands on its main line: a show's numbered episodes outside season 0,
/// every episode of a film or a clip title.
pub fn on_main_line(kind: TitleKind, season: Option<i32>, number: Option<i32>) -> bool {
  match kind {
    TitleKind::Show => number.is_some() && season != Some(0),
    TitleKind::Film | TitleKind::Clip => true,
  }
}

/// The place of the episode last played, the earlier one where two were set at once.
fn last_played<'a>(items: impl Iterator<Item = (usize, &'a Item)>) -> Option<usize> {
  items
    .filter_map(|(at, (_, _, position))| position.as_ref().map(|position| (at, position.updated_at)))
    .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(&a.0)))
    .map(|(at, _)| at)
}

fn done(item: &Item) -> bool {
  item.2.as_ref().is_some_and(|position| position.done)
}

/// The place of the episode to go on with along the main line `main` marks: the one last played where it is not
/// finished; else the last main-line one played where it is not finished, and the first unfinished main-line one
/// after it where it is; else, where no main-line one was played, the first unfinished one of the main line. It
/// never starts over.
pub fn up_next(items: &[Item], main: &[bool]) -> Option<usize> {
  if let Some(at) = last_played(items.iter().enumerate())
    && !done(&items[at])
  {
    return Some(at);
  }
  let on_line = |at: &usize| main.get(*at).copied().unwrap_or_default();
  let from = match last_played(items.iter().enumerate().filter(|(at, _)| on_line(at))) {
    Some(at) if done(&items[at]) => at + 1,
    Some(at) => at,
    None => 0,
  };
  (from..items.len()).find(|at| on_line(at) && !done(&items[*at]))
}

/// The seconds to start the episode at `at` from: 0 for one done or never played.
fn start_of(items: &[Item], at: Option<usize>) -> f64 {
  at.and_then(|at| items[at].2.as_ref())
    .filter(|position| !position.done)
    .map(|position| position.seconds)
    .unwrap_or_default()
}

/// A title's progress, its up next read along the main line `main` marks.
pub fn title_progress(items: &[Item], main: &[bool]) -> Progress {
  let mut answer = progress(items);
  let up_next = up_next(items, main);
  answer.up_next = up_next.map(|at| items[at].0);
  answer.up_next_seconds = start_of(items, up_next);
  answer
}

/// The progress of the episodes in order, each with its duration and the viewer's position where it has one:
/// the one to watch next is the one last played where it is not finished; else the first unfinished one after
/// it, and from the start where none follows; else the first, where nothing was played. A playlist being all
/// main line, its up next is its next.
pub fn progress(items: &[Item]) -> Progress {
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
  let last = last_played(items.iter().enumerate());
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
  let next_seconds = next.map(|at| if rows[at].done { 0.0 } else { rows[at].seconds }).unwrap_or_default();
  Progress {
    done_count,
    next: next.map(|at| rows[at].episode_id),
    next_seconds,
    up_next: next.map(|at| rows[at].episode_id),
    up_next_seconds: next_seconds,
    items: rows,
  }
}
