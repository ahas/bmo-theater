# bmo-theater

The theater keeps the platform's videos and shows them. Where bmo-artifact keeps documents of words and
pictures, the theater keeps **titles** made of **episodes**: a series, a film, or an output the platform's
agents made. Videos come in two ways and are seen the same way afterwards:

- **Filed outputs.** A video an agent or a person made (an introduction, a report, a clip) is handed to the
  theater, which copies it into its own file root and keeps it there.
- **Libraries.** A folder of video files already on disk is registered, and the theater reads it in place
  and indexes it into titles, seasons and episodes. A library's files are never moved, copied or
  rewritten.

Each episode streams to a browser as the file it is, with subtitles laid over it by the player, a watch
position kept for each viewer, playlists that say how far a viewer has come and what to watch next, share
links that open a video to whoever holds the link, and grants that open a video to named people of the
company.

## The model

- **A library** is a registered folder: its path, a name, its realm and the moment it was last scanned.
  Registering one scans it; a scan may be asked for again at any time.
- **A title** is a name, a kind, a description, labels (the caller's own metadata, an object of strings),
  its realm, and where it came from: a library (and the folder within it that it stands for) or nothing,
  for a title made by hand. Its kind is one of:
  - `show`: a series, its episodes numbered by season and episode;
  - `film`: one video, a title of one episode;
  - `clip`: a title of filed outputs.
- **An episode** is one video under a title: its season and episode numbers where it has them, its name, a
  label, the file it is (path, name, media type, size), what the file holds as read when it was indexed
  (duration, width, height, video and audio codecs), whether the file is still on disk, a poster frame, and
  its subtitles. Its kind is `episode`, a library's video, or `clip`, a filed output or a burned one; a
  burned clip names the episode and the subtitle it was burned from. A `clip` episode may be moved to
  another title of its realm; a library's episode stays with the title its file was indexed under.
- **A subtitle** belongs to one episode: its language, a label, its original format (`srt`, `ass`, `smi` or
  `vtt`), the original file as it was given and the WebVTT the theater converted it to.
- **A playlist** is a name, a realm and an ordered list of episodes, which may stand under different titles
  and may name one episode more than once.
- **A position** is one viewer's place in one episode: the seconds watched to, whether the episode counts as
  watched, and when it was last set.
- **A share link** opens one episode or one playlist to whoever holds its token, with an end in time or
  none.
- **A grant** opens one episode to one account of the company, at a permission, `read` or `edit`.
- **A burn** is the work of writing a subtitle into an episode's picture, and its outcome.
- **A scan** is one reading of a library's folder, and its outcome.

## Two realms

Titles, libraries and playlists stand in one of two realms, never both:

- **`mine`:** an account's own. Only that account reads and writes them.
- **`organization`:** the organization's. Every account reads them. A title or a playlist in it is changed or
  deleted by the administrator and by the account that made it.

A position is always the viewer's own, in either realm: the same organization playlist shows each viewer
their own progress. A playlist holds only episodes its readers can read, so an organization playlist holds
organization episodes alone, and a `mine` playlist its account's episodes and the organization's.

An episode, a subtitle, a burn, a share link and a grant stand in the realm of the title or playlist they
belong to. A grant is the one way past the realm: the account it names watches that one episode, and reads
nothing else of the realm by it (「Grants」).

A library is registered by the administrator alone, in either realm, since registering one reads a folder
of the host's. The titles a scan makes stand in the library's realm and belong to the account that
registered it.

## Libraries and scans

A scan walks the library's folder and every folder under it, and indexes each video file it finds by its
extension (`mp4`, `m4v`, `mkv`, `webm`, `mov`, `avi`, `ogv`). It goes:

1. **Folders group the files.** The library's folder is one group, and so is every folder under it whose
   name is not a season's. A season's folder (`S1`, `S06`, `Season 2`, any letter case) is part of the group
   above it and gives its season number to the files in it that name none.
2. **Names give the numbers.** A file's name is read for its season and episode: `S06E35` (or `s6e35`),
   `6x35`, or an episode alone, `E01`. What stands after the numbers is the episode's name; what stands
   before them is the show's name; a part in square or round brackets that only tags the file (`[KR]`,
   `[1080p]`) is dropped, and dots and underscores read as spaces where the name has none.
3. **A group holding any numbered file is one `show` title.** Its numbered files are its episodes; a file
   in it with no numbers (a trailer, a special) is an episode with none, listed after the numbered ones.
   The library's own group takes the library's name; any other group takes its folder's name.
4. **A group holding no numbered file is one `film` title per file,** named by the file's name.
5. **What the file holds is read** with ffprobe (duration, width, height, codecs), and a **poster frame** is
   taken with ffmpeg at a tenth of the way in and kept in the file root.

A scan never deletes a row. It adds what is new, marks a file that is no longer on disk unavailable (and
available again once it returns), reads a file again where its size or its modification time changed, and
leaves the rest. A title deleted from a library is retired rather than removed, so a later scan does not
bring it back. One scan of a library runs at a time; a scan stopped by the service ending is failed when
the service starts again, and the next scan reads what it missed.

## Playing

- **Direct serving.** An episode's stream is its file as it stands, served with HTTP ranges, so a browser
  seeks by asking for the stretch it needs. Nothing is transcoded: the libraries the theater was built for
  hold H.264 and AAC in MP4, which browsers play as they are. A file in a format a browser cannot play is
  still served as it is, and transcoding stays out of scope until such files appear.
- **Subtitles are drawn by the player.** Each subtitle is kept as WebVTT, which the browser lays over the
  video as a text track. An SRT file converts nearly as it is; a SAMI file (`smi`) converts by its timed
  blocks, each lasting until the next begins, its first language class taken where it holds several; an ASS
  file converts to its text alone, its styles and positions dropped. A file whose bytes are not UTF-8 is read
  as UTF-16 where it carries that mark, and otherwise as the Korean code page that SAMI files are usually
  written in.
- **Burning** writes a subtitle into the picture, for a video that goes where no player draws text tracks:
  a room, or someone outside. A burn runs ffmpeg's subtitles filter over the episode and files the output as
  a new `clip` episode under the same title, labelled with what it was burned from; an ASS subtitle is burned
  with its styles. A burn is work in the background: it is answered at once, runs as long as it takes, and is
  told by events as it starts, finishes or fails.
- **Positions.** The player sets the viewer's position as it plays. An episode counts as watched once the
  position passes nine tenths of its duration, or when the player says so, and stays watched until the
  player says otherwise.
- **What to watch next.** A playlist's progress, and a show's, answer each episode's position for the viewer
  and the next one to watch: the episode last played where it is not finished; else the first unfinished
  one after it, and from the start where none follows; else the first, where nothing was played.

## Share links

A share link opens one episode or one playlist, with no sign-in, to whoever holds its token: the episode or
the playlist's episodes, their posters, their streams and their subtitles, and nothing else. An episode or a
playlist has at most one live link. A link ends at its `expiresAt` where it has one, or when it is deleted,
and with the episode or the playlist it opens.

## Grants

A grant shares one episode with one person of the company, named by the account key the console signs them
in with. The console's people directory finds the person; the theater keeps the key alone and names nobody.

- **What it holds:** the episode, the account, the permission (`read` where none is given), who granted it
  and when. An episode has at most one grant to an account; granting again sets its permission.
- **Who grants:** whoever changes the episode (「Two realms」) makes, changes and ends its grants, and reads
  them. A grant to the account the episode already belongs to adds nothing and is not kept.
- **What it opens:** to the signed-in account it names, the episode watched as its owner watches it: the
  episode, its stream, its poster, its subtitles and their files, and the account's own position in it. Both
  permissions open these reads alone; no write is opened by a grant, so the account does not change, delete,
  burn, subtitle, link, move or list the episode, nor reads its title, its other episodes or its burns.
- **What it does not touch:** anyone the episode is not granted to is answered as before, as if it did not
  stand. A grant and a share link are independent; neither makes, ends nor changes the other.
- **Where it matters:** an episode of a `mine` realm. An `organization` episode is read by every account
  already, so a grant on one opens nothing more.
- **When it ends:** when it is taken off, or with the episode.

## The file root

The theater's file root, a setting, holds what the theater itself keeps:

- the files of filed and burned episodes, one folder for each episode;
- the poster frames, one for each episode that has one;
- each subtitle's original file and its WebVTT, one folder for each subtitle;
- a burn's work while it runs, and an upload while it arrives.

A library's files stay where they are; the file root holds nothing of them but their posters. Deleting a
filed or burned episode deletes its file; deleting a library's episode is refused, since it goes with its
file.

## The console

A console bundle, built from `client/`, stands under the sidebar entry the words name:

- **The library:** a grid of posters, the reader's own realm's titles in one tab and the organization's in
  another, and a video filed from the browser as a clip, under a title of the realm or a new one, its bytes
  sent with how far they have come.
- **A title:** its seasons and episodes, each with the viewer's progress, the title's own progress and a button
  to go on from where the viewer stopped; each episode added to a playlist, shared, given subtitles, and, for a
  clip, deleted.
- **The playlists:** made, renamed, reordered (dragged by a handle, moved by its arrow keys, or on a phone by
  each item's up and down buttons), an item removed, each change sending the whole order, and a button to go
  on from where the viewer stopped.
- **The player:** the stream with its subtitles as text tracks (off unless one is in the screen's language),
  playback speed, ten-second skips, the next episode, which a playlist goes on to by itself, the volume,
  fullscreen, turning the picture, resuming, and the keys space, the arrows, f, m, c and r. Turning goes to full screen and locks the screen to the other orientation where the browser can; where it cannot, as on a desktop or an iPhone, it turns the picture itself a quarter at a time inside the player, fitted to the player's box. Leaving full screen or the player gives the screen's orientation back and sets the picture straight, and on a phone a second press does the same. A press on the picture plays or pauses with a mouse, and a double press turns it full screen; a finger shows or hides the controls. On a touch screen the picture also takes gestures: a double tap on its left or right half skips ten seconds back or forward, a held finger plays faster until it lifts, a horizontal drag moves the position as the finger travels and settles where it lifts, and a vertical drag sets the picture's brightness on the left half and the volume on the right, each with a cue over the picture. No gesture answers while a dialog, a menu or the console's overlay stands open. It sets the viewer's position every five
  seconds, as it pauses and as it is left.
- **Subtitles:** a file added to an episode or a subtitle deleted, and a subtitle burned in, the burn's state
  told by the event stream until its clip stands under the title.
- **The libraries,** shown to the administrator alone: a folder registered, renamed, scanned again or removed.
  No route says whether a reader administers the theater, so the bundle asks the one route only the
  administrator may call with a body it refuses, which makes nothing either way.
- **Sharing:** an episode and a playlist open the layer's one share dialog (bmo-ui `docs/components.md`,
  「Sharing」). Its link part makes, shows and ends the share link. Its people part stands for an episode of
  the reader's own `mine` realm where the console offers its people directory (bmo-ui
  `docs/console-bundles.md`, the people directory and sharing): people found through the directory's search,
  each grant drawn by the name the directory gives its account, else by the key. A playlist is shared by link
  alone.
- **Watching by a grant:** the player alone, its title shown only where the reader reads it, with no share,
  subtitle, burn or playlist action, since the service opens none of them to a grant.
- **The share page,** the screen path's `/share/` and a token: the same player for whoever holds a link, with no
  sign-in and no position set, a playlist's episodes following one another.

Every screen keeps to the event stream, so what changes anywhere is shown where it stands.

The bar over a screen and the row under it keep bmo-ui's console plugin contract (bmo-ui
`docs/console-bundles.md`, 「How a bundle mounts」, the title bar and the screen's own row). The theater's
row is the row of its screens on the library, the playlists and the libraries, and the screen's toolbar on a
title, a playlist and the player. Every button in that row is the component library's small button at its own size,
its hover confined to its own shape; on a narrow screen the row of screens names each screen by its mark
alone, so the actions keep standing beside it.

## The tool

The supervisor reaches the theater through the tool host: find titles, show a title or an episode, file a
local video as an output, rename, move and delete a work, add a subtitle, make a playlist, add to it and take
a work out of it, read a viewer's progress, and burn a subtitle in. A finished video is filed with the tool,
and the room is sent a share link rather than the file. A work is one video the theater keeps, an episode: it
stands on a title, the shelf that holds it, and may also stand in playlists. A move puts it under another
title or into a playlist and never deletes anything; a delete is its own call, and a library's episode is
neither moved nor deleted on its own. Every holder of the tool acts for the owner, and the console's realm
rule decides who may change or delete what.

## Out of scope

Transcoding and HLS (until a file appears that browsers cannot play as it is), subtitles transcribed by the
voice unit, metadata fetched from outside catalogues, and finer permissions such as forbidding a download.
