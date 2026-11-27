# The theater tool

The theater keeps videos as titles made of episodes: shows of the libraries on disk, films, and clips filed as
outputs. Each title, library and playlist stands in a realm: `mine`, the owner's own, or `organization`, the
organization's. A work is one video, an episode: its title is the shelf it stands on, and it may also stand in
playlists. Every entry acts for the owner, and a position is always the owner's own.

- **`theater_titles`** finds titles: by `realm`, `kind` (`show`, `film`, `clip`) and `query`, a part of the
  name. One line each: its id, its name, its kind, its realm and how many episodes it holds.
- **`theater_show`** with `titleId` answers the title and its episodes in order, each with its id, season and
  number, name, duration and the owner's progress; with `episodeId`, the episode and its subtitles.
- **`theater_file`** files a local video: `path` is the file on this host, copied into the theater. Name the
  title by `titleId`, or by `title`, its exact name in the realm, made as a new clip title where none bears it.
  With `share`, a share link is opened and its address answered: send a room the link, not the file.
- **`theater_work_rename`** renames a work: `episodeId` and its new `name`, with an optional `label`. Nothing
  is filed again, so a revision is corrected where it stands.
- **`theater_work_move`** moves a work: with `titleId` it stands under another title, the shelf it is on; with
  `playlistId` it is added to a playlist. Name exactly one of the two. A move deletes nothing: the work, its
  file, its subtitles, its positions and every playlist it stood in are kept.
- **`theater_work_delete`** deletes a filed or burned work and its file. A library's work goes with the file it
  was indexed from and is refused, as the service keeps a library's files in place. Deletion is its own call
  and never follows a move.
- **`theater_playlist_remove`** takes a work out of a playlist: `playlistId` and `episodeId`, with `at`, the
  place from 0, where the playlist holds it more than once. The work and the playlist stand afterwards.
- **`theater_subtitle_add`** with `episodeId` and `path` (both required) adds a local subtitle file (`srt`, `ass`,
  `smi` or `vtt`) to an episode; the player shows it as a track.
- **`theater_playlist_make`** makes a playlist of `episodeIds` in order; **`theater_playlist_add`** adds
  episodes to one, at `at` or at the end.
- **`theater_progress`** answers how far the owner has come through a playlist (`playlistId`) or a title
  (`titleId`): each episode's seconds and whether it is done, and the next to watch.
- **`theater_burn`** with `episodeId` and `subtitleId` (both required) burns one of the episode's subtitles into
  its picture, for a room or someone outside where no player draws subtitles. The `subtitleId` is one of the
  subtitles `theater_show` lists for that `episodeId`. It is answered at once; the burned video is filed later as a clip under the
  same title.

Examples:

- A finished introduction video, sent to the room: `theater_file` with `{"path": "/tmp/intro.mp4", "title":
  "Introductions", "share": true}`.
- Where the owner stopped in a show: `theater_titles` with `{"query": "pilot"}`, then `theater_progress` with
  `{"titleId": 12}`.
- A revised cut corrected in place: `theater_work_rename` with `{"episodeId": 334, "name": "Introductions",
  "label": "rev.4"}`, then `theater_playlist_remove` with `{"playlistId": 1, "episodeId": 332}` for the copy
  it replaced.
