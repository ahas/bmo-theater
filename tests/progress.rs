//! A viewer's progress: each episode's position and the one to watch next, resumed where it was left, on past a
//! finished one, from the start where nothing follows, and the first where nothing was played.

use bmo_theater::progress::{progress, reaches_done};
use bmo_theater::wire::Position;

fn at(episode: i64, seconds: f64, done: bool, updated_at: i64) -> Option<Position> {
  Some(Position {
    episode_id: episode,
    seconds,
    done,
    updated_at,
  })
}

#[test]
fn nothing_played_starts_at_the_first() {
  let answer = progress(&[(1, Some(60_000), None), (2, Some(60_000), None)]);
  assert_eq!((answer.next, answer.next_seconds, answer.done_count), (Some(1), 0.0, 0));
  assert_eq!(progress(&[]).next, None);
}

#[test]
fn an_unfinished_last_played_is_resumed() {
  let answer =
    progress(&[(1, Some(60_000), at(1, 60.0, true, 10)), (2, Some(60_000), at(2, 12.5, false, 20))]);
  assert_eq!((answer.next, answer.next_seconds, answer.done_count), (Some(2), 12.5, 1));
}

#[test]
fn a_finished_last_played_goes_on_to_the_next_unfinished_and_wraps_to_the_start() {
  let items = [
    (1, Some(60_000), None),
    (2, Some(60_000), at(2, 60.0, true, 10)),
    (3, Some(60_000), at(3, 60.0, true, 30)),
    (4, Some(60_000), at(4, 5.0, false, 5)),
  ];
  let answer = progress(&items);
  assert_eq!((answer.next, answer.next_seconds), (Some(4), 5.0));

  let wrapped = progress(&[(1, Some(60_000), None), (2, Some(60_000), at(2, 60.0, true, 10))]);
  assert_eq!((wrapped.next, wrapped.next_seconds), (Some(1), 0.0));

  let all = progress(&[(1, Some(60_000), at(1, 60.0, true, 10)), (2, Some(60_000), at(2, 60.0, true, 20))]);
  assert_eq!((all.next, all.done_count), (None, 2));
}

#[test]
fn nine_tenths_of_the_duration_counts_as_watched() {
  assert!(reaches_done(54.0, Some(60_000)));
  assert!(!reaches_done(53.9, Some(60_000)));
  assert!(!reaches_done(1000.0, None));
}
