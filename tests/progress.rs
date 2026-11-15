//! A viewer's progress: each episode's position and the one to watch next, resumed where it was left, on past a
//! finished one, from the start where nothing follows, and the first where nothing was played; and the one to go
//! on with along a title's main line, which passes over specials and extras and never starts over.

use bmo_theater::progress::{on_main_line, progress, reaches_done, title_progress, up_next};
use bmo_theater::wire::{Position, TitleKind};

fn at(episode: i64, seconds: f64, done: bool, updated_at: i64) -> Option<Position> {
  Some(Position {
    episode_id: episode,
    seconds,
    done,
    updated_at,
    set_aside: false,
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

#[test]
fn a_shows_main_line_leaves_out_season_0_and_the_numberless() {
  assert!(on_main_line(TitleKind::Show, Some(1), Some(1)));
  assert!(on_main_line(TitleKind::Show, None, Some(3)));
  assert!(!on_main_line(TitleKind::Show, Some(0), Some(1)));
  assert!(!on_main_line(TitleKind::Show, Some(1), None));
  assert!(on_main_line(TitleKind::Film, Some(0), None));
  assert!(on_main_line(TitleKind::Clip, None, None));
}

#[test]
fn up_next_goes_along_the_main_line_and_never_starts_over() {
  // A special, two episodes of the main line, and an extra, in a title's order.
  let main = [false, true, true, false];
  let items = |special, first, second, extra| {
    [
      (1, Some(60_000), special),
      (2, Some(60_000), first),
      (3, Some(60_000), second),
      (4, Some(60_000), extra),
    ]
  };
  let up = |items: &[(i64, Option<i64>, Option<Position>)]| up_next(items, &main).map(|at| items[at].0);

  assert_eq!(up(&items(None, None, None, None)), Some(2));
  assert_eq!(up(&items(at(1, 60.0, true, 10), None, None, None)), Some(2), "a special watched alone");
  assert_eq!(
    up(&items(None, at(2, 60.0, true, 10), None, at(4, 5.0, false, 20))),
    Some(4),
    "an extra left part way"
  );
  assert_eq!(
    up(&items(None, at(2, 60.0, true, 10), None, at(4, 60.0, true, 20))),
    Some(3),
    "an extra finished"
  );
  assert_eq!(
    up(&items(at(1, 60.0, true, 30), at(2, 60.0, true, 10), None, None)),
    Some(3),
    "a special after"
  );
  assert_eq!(
    up(&items(at(1, 60.0, true, 30), at(2, 20.0, false, 10), None, None)),
    Some(2),
    "a main-line episode left part way before a special"
  );
  assert_eq!(
    up(&items(None, at(2, 60.0, true, 10), at(3, 60.0, true, 20), None)),
    None,
    "the main line finished"
  );
  assert_eq!(
    up(&items(None, None, at(3, 60.0, true, 20), None)),
    None,
    "nothing after the last one played"
  );

  let finished = items(None, at(2, 60.0, true, 10), at(3, 60.0, true, 20), None);
  let answer = title_progress(&finished, &main);
  assert_eq!((answer.next, answer.up_next, answer.up_next_seconds), (Some(4), None, 0.0));
  let resumed = title_progress(&items(None, at(2, 60.0, true, 10), at(3, 12.5, false, 20), None), &main);
  assert_eq!((resumed.up_next, resumed.up_next_seconds), (Some(3), 12.5));
}

#[test]
fn a_playlists_up_next_is_its_next() {
  let answer = progress(&[(1, Some(60_000), None), (2, Some(60_000), at(2, 60.0, true, 10))]);
  assert_eq!((answer.up_next, answer.up_next_seconds), (answer.next, answer.next_seconds));
  assert_eq!(answer.up_next, Some(1));
}
