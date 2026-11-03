//! What a file's and a folder's names say: the season and episode numbers, the show's and the episode's names,
//! the tags dropped, a season's folder, and the videos by their extensions.

use bmo_theater::names::{clean, parse, season_folder, video_type};

#[test]
fn a_season_and_an_episode_are_read_with_the_names_around_them() {
  let parsed = parse("Show Name - S06E35 Grand Finale 1000 Plus [KR] [1080p]");
  assert_eq!(parsed.show.as_deref(), Some("Show Name"));
  assert_eq!((parsed.season, parsed.number), (Some(6), Some(35)));
  assert_eq!(parsed.name, "Grand Finale 1000 Plus");

  let lower = parse("show.s01e02.pilot");
  assert_eq!((lower.season, lower.number), (Some(1), Some(2)));
  assert_eq!(lower.show.as_deref(), Some("show"));
  assert_eq!(lower.name, "pilot");

  let crossed = parse("Show 2x05 The Trip");
  assert_eq!((crossed.season, crossed.number), (Some(2), Some(5)));
  assert_eq!(crossed.name, "The Trip");
}

#[test]
fn an_episode_alone_has_no_season() {
  let parsed = parse("Show Name - Far Lands E01 Robot [KR] [1080p]");
  assert_eq!(parsed.show.as_deref(), Some("Show Name - Far Lands"));
  assert_eq!((parsed.season, parsed.number), (None, Some(1)));
  assert_eq!(parsed.name, "Robot");
}

#[test]
fn a_second_episode_number_belongs_to_neither_name() {
  let parsed = parse("Show S01E01E02 Double Bill");
  assert_eq!((parsed.season, parsed.number), (Some(1), Some(1)));
  assert_eq!(parsed.name, "Double Bill");
}

#[test]
fn a_name_without_numbers_is_cleaned_whole_and_a_round_part_that_says_something_stays() {
  let parsed = parse("Show Name - Season 2 Official Trailer (Teaser) [subs]");
  assert_eq!((parsed.season, parsed.number, parsed.show), (None, None, None));
  assert_eq!(parsed.name, "Show Name - Season 2 Official Trailer (Teaser)");
  assert_eq!(clean("A_Film_Name.2019"), "A Film Name 2019");
  assert_eq!(clean("Film (1080p) [x264]"), "Film");
  let pilot = parse("Show - S00E01 Show (Pilot) [480p]");
  assert_eq!(pilot.name, "Show (Pilot)");
  assert_eq!(pilot.season, Some(0));
}

#[test]
fn a_season_folder_gives_its_number() {
  assert_eq!(season_folder("S06"), Some(6));
  assert_eq!(season_folder("s1"), Some(1));
  assert_eq!(season_folder("Season 2"), Some(2));
  assert_eq!(season_folder("season_10"), Some(10));
  assert_eq!(season_folder("Side Quests"), None);
  assert_eq!(season_folder("Specials"), None);
}

#[test]
fn videos_are_known_by_their_extensions() {
  assert_eq!(video_type("mp4"), Some("video/mp4"));
  assert_eq!(video_type("MKV"), Some("video/x-matroska"));
  assert_eq!(video_type("webm"), Some("video/webm"));
  assert_eq!(video_type("htm"), None);
  assert_eq!(video_type("srt"), None);
}
