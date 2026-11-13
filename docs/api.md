# The boundary API

Every route bmo-theater answers, the events it publishes, the console face it serves, the tool entries it
places, the commands it runs and its settings. The wire format is the platform's (bmo-chat
`docs/conventions.md`): camelCase keys, times in epoch milliseconds, ids as signed 64-bit integers, a
failure a non-2xx status with `code` (kebab-case) and `error` (an English sentence), a list `{items, more}`.
`docs/openapi.json` is generated from the service's own types by `bmo-theater openapi`, and a test fails
where the committed document differs.

## Callers

The service listens on the loopback at the address its settings name (`[server] listen`, by convention
`127.0.0.1:13128`). Every route but the console face's own (「Console」) takes `Authorization: Bearer <token>`,
one of two:

- **The service token** (`[server] token`), the tool's and any program's. It acts as the administrator and
  reaches every member by its id. It names the account it acts for with the `Owner-Key` header, as the
  console does; a call that names none acts for no account, so it may read and write the `organization`
  realm (recording no creator) and any member by id, but a list of the `mine` realm, a `mine` write and a
  position are refused 422 `no-account`.
- **The console credential** (`[console] token`), with the headers the console sets on every call it relays:
  `Owner-Key`, the signed-in account's key, and `Owner-Role`, `administrator` or `member`. A console call with
  no `Owner-Key` (or one longer than 200 characters), or with an `Owner-Role` that is neither, is 403
  `forbidden`, but for the share reads, which take the credential alone (「Share links」).

A missing or wrong token is 401 `unauthorized`; an empty token in the settings lets no caller in by it.
**A share token is never a bearer.** It is carried in the path of the share reads, which the console relays
for a reader with no sign-in.

## Realms

A realm is `mine` or `organization`. A route that makes a member takes `realm` in its body; a list takes it
in its query and, where none is given, answers both realms the caller reads. In `mine` the member belongs to
the calling account. A member of another account's `mine` realm is answered 404 `not-found`, as if it did not
stand, but for the reads a grant opens to the account it names (「Grants」). In `organization` every account reads; a title, a playlist, a subtitle and a share link are changed or
deleted by the administrator and by the account that made it (`creator`), anyone else being refused 403
`forbidden`. A library is registered, changed, scanned and deleted by the administrator alone.

## Lists

A list is ordered by id, oldest first, unless its route says otherwise, at most `limit` items (200 where none
is given, 1000 at most), from the item after the id `after` names in the list's own order (for a list newest
first, the older ones). `more` says whether anything stands past the last item.

## Health

| Method | Path | Does |
|---|---|---|
| GET | `/health` | `ready`, `reason` (null when ready), `version` |

The service is ready once it has reached its database and brought it to the schema it carries, and its file
root can be written. It answers not ready, naming why, where the file root cannot be written.

## Libraries

| Method | Path | Does |
|---|---|---|
| GET | `/libraries?realm=` | The libraries the caller reads — `{items, more}` |
| POST | `/libraries` | `path`, `realm`, `name?` — 201, the library, its first scan started |
| GET | `/libraries/{libraryId}` | One library |
| PATCH | `/libraries/{libraryId}` | `name` — 200 |
| DELETE | `/libraries/{libraryId}` | Removes it and every title, episode, subtitle, position and share link indexed from it; its folder's files stay — 204 |
| POST | `/libraries/{libraryId}/scans` | Starts a scan — 201, the scan; 200, the scan already running |
| GET | `/libraries/{libraryId}/scans` | Its scans, newest first — `{items, more}` |

A library is `id`, `realm`, `name`, `path`, `creator`, `createdAt`, `scannedAt` (the end of its last finished
scan, or null), `scanning` (whether a scan runs), `titleCount`, `episodeCount`. `path` is an absolute folder
the service can read, outside its file root; anything else is 422 `not-a-folder`. A realm already holding a
library at the same path is 409 `conflict`. `name` is the library's own title's name (docs/design.md,
「Libraries and scans」); where none is given, the show name most of its own folder's files share, else the
folder's name. Registering, changing, scanning and deleting a library is the administrator's; a member is
refused 403 `forbidden`.

A scan is `id`, `libraryId`, `state` (`running`, `finished` or `failed`), `foundCount` (video files found),
`addedCount` (episodes added), `changedCount` (episodes read again), `missingCount` (episodes whose file is
gone), `error` (why it failed, or null), `startedAt`, `endedAt` (null while it runs). A scan is told by
`scan.running` as it starts and `scan.finished` or `scan.failed` as it ends; every title and episode it adds
or changes by its own events.

## Titles

| Method | Path | Does |
|---|---|---|
| GET | `/titles?realm=&kind=&libraryId=&q=` | The titles the caller reads; `q` matches a part of the name, in any letter case — `{items, more}` |
| POST | `/titles` | `realm`, `name`, `kind?` (`clip` where none), `description?`, `labels?` — 201 |
| GET | `/titles/{titleId}` | One title |
| PATCH | `/titles/{titleId}` | `name`, `kind`, `description`, `labels` — 200 |
| DELETE | `/titles/{titleId}` | A library's title is retired, its files left; any other is removed with its filed files — 204 |
| GET | `/titles/{titleId}/poster` | The poster of its first episode that has one, `image/jpeg`; 404 `not-found` where none has |
| GET | `/titles/{titleId}/episodes` | Its episodes in order (season, then episode, then name; numberless last) — `{items, more}`, `more` false |
| POST | `/titles/{titleId}/episodes` | Files a video as a `clip` episode — 201 |
| GET | `/titles/{titleId}/progress` | The viewer's progress through its episodes (「Positions and progress」) |

A title is `id`, `realm`, `name`, `kind` (`show`, `film` or `clip`), `description` (may be empty), `labels`
(an object of strings), `libraryId` (null for a title made by hand), `episodeCount`, `hasPoster`, `creator`,
`createdAt`, `changedAt`. A blank name or one of more than one line is 400 `bad-request`. A retired title is
not found by any route.

**Filing a video** takes one of two bodies:

- `multipart/form-data`: the part `file`, the video's bytes under its file name, and the optional text parts
  `name`, `label`, `season` and `number`. The bytes are written into the file root as they arrive; no ceiling
  is set on their size.
- JSON `{path, name?, label?, season?, number?}`: `path` an absolute file the service can read on its own
  host, which it copies into the file root. The service token and the administrator alone file by path; a
  member is refused 403 `forbidden`.

The episode takes the file's name, its media type by its extension and its size, is read with ffprobe and
given a poster frame. `name` defaults to the file's name without its extension. A file whose extension is
not a video's (docs/design.md, 「Libraries and scans」), or that ffprobe finds no video stream in, is 422
`not-a-video`, and nothing is kept of it.

## Episodes

| Method | Path | Does |
|---|---|---|
| GET | `/episodes/{episodeId}` | One episode |
| PATCH | `/episodes/{episodeId}` | `name`, `label`, `season`, `number`, `titleId` — 200 |
| DELETE | `/episodes/{episodeId}` | Removes a `clip` episode and its file — 204; a library's episode is 409 `conflict` |
| GET | `/episodes/{episodeId}/stream` | The file (「Streams」) |
| GET | `/episodes/{episodeId}/poster` | Its poster frame, `image/jpeg`; 404 `not-found` where it has none |

An episode is `id`, `titleId`, `realm`, `kind` (`episode` or `clip`), `season`, `number` (each null where it
has none), `name`, `label` (may be empty), `fileName`, `mediaType`, `byteCount`, `durationMs`, `width`,
`height`, `videoCodec`, `audioCodec` (each null where ffprobe gave none), `available` (whether its file is on
disk), `hasPoster`, `subtitleCount`, `burnedFrom` (for a burned clip, `{episodeId, subtitleId}`; null
otherwise), `position` (the caller's position in it, as 「Positions and progress」 answers one, or null where
the caller has none or names no account), `granted` (true where the caller reads it by a grant alone, false
otherwise), `creator`, `createdAt`, `changedAt`. An episode's changes are made by whoever changes its title.

`titleId` moves a `clip` episode under another title of its realm and answers it under the new title. The
caller changes both the episode's title and the title it joins, so a caller without the right is 403
`forbidden`; a title it does not read is 422 `unknown-title`, and a title of the other realm is 422
`not-in-realm`. A library's episode is 409 `conflict`, since the title follows its file. A move keeps the
episode, its file, its subtitles, its burns, its positions and every playlist it stands in, and deletes
nothing.

## Streams

`GET /episodes/{episodeId}/stream` and the share read `GET /shares/{token}/episodes/{episodeId}/stream` answer
the episode's file as it stands, nothing transcoded:

- `Content-Type` the media type by the file's extension; `Accept-Ranges: bytes`; `ETag` a strong tag made of
  the file's size and modification time; `Cache-Control: private, no-cache`.
- With no `Range`, 200 and the whole file. With `Range: bytes=<first>-<last>`, `bytes=<first>-` or
  `bytes=-<suffix>`, 206 and that stretch, with `Content-Range`. A range starting past the end is 416, with
  `Content-Range: bytes */<size>`. A header naming several ranges is answered with the first. A `Range` whose
  `If-Range` names another tag is answered 200 whole.
- `If-None-Match` naming the current tag is 304.
- `HEAD` answers the same headers with no body.
- A file no longer on disk is 404 `not-found`, and its episode is marked unavailable.

## Subtitles

| Method | Path | Does |
|---|---|---|
| GET | `/episodes/{episodeId}/subtitles` | The episode's subtitles, oldest first — `{items, more}`, `more` false |
| POST | `/episodes/{episodeId}/subtitles` | Adds one — 201 |
| GET | `/subtitles/{subtitleId}` | One subtitle |
| GET | `/subtitles/{subtitleId}.vtt` | Its WebVTT, `text/vtt; charset=utf-8` |
| GET | `/subtitles/{subtitleId}/original` | The file as it was given, under its name |
| DELETE | `/subtitles/{subtitleId}` | Removes it and its files — 204 |

A subtitle is `id`, `episodeId`, `language` (a language tag, `und` where none is given), `label` (may be
empty), `format` (`srt`, `ass`, `smi` or `vtt`), `fileName`, `cueCount`, `creator`, `createdAt`.

Adding one takes `multipart/form-data` (the part `file`, the subtitle under its file name, and the optional
text parts `language`, `label` and `format`) or JSON `{path, language?, label?, format?}`, `path` a file the
service can read on its own host (the service token and the administrator alone). `format` defaults to the
file's extension. A format outside the four is 422 `unknown-format`; a file that converts to no cue is 422
`no-cues`. Every account that reads the episode adds subtitles to it; deleting one is its creator's and the
administrator's, and in `mine` the account's.

How each format converts is docs/design.md, 「Playing」. The cues are written in WebVTT's own text form: each
cue's start and end, then its lines; an SRT's and a WebVTT's markup is kept where WebVTT knows it (`b`, `i`,
`u`) and dropped elsewhere; a SAMI's and an ASS's lines are plain text.

## Burns

| Method | Path | Does |
|---|---|---|
| POST | `/episodes/{episodeId}/burns` | `subtitleId` — 201, the burn, running |
| GET | `/episodes/{episodeId}/burns` | The episode's burns, newest first — `{items, more}` |
| GET | `/burns/{burnId}` | One burn |

A burn is `id`, `episodeId`, `subtitleId`, `state` (`running`, `finished` or `failed`), `clipId` (the episode
it filed, once finished), `error` (why it failed, or null), `creator`, `startedAt`, `endedAt`. A subtitle of
another episode is 422 `unknown-subtitle`. Whoever reads the episode burns it. The burn runs in the
background with no clock bound (「Calls the theater makes」); once ffmpeg ends well the output is filed under
the same title as a `clip` episode named as the source episode, its label `burned: <the subtitle's label or
language>`, its `burnedFrom` naming both. It is told by `burn.running`, then `burn.finished` with the clip's
`episode.created`, or `burn.failed`. A burn running when the service stops is failed as the service starts
again.

## Playlists

| Method | Path | Does |
|---|---|---|
| GET | `/playlists?realm=` | The playlists the caller reads — `{items, more}` |
| POST | `/playlists` | `realm`, `name`, `episodeIds?` — 201 |
| GET | `/playlists/{playlistId}` | One playlist |
| PATCH | `/playlists/{playlistId}` | `name`; `episodeIds`, the whole order in place of the one standing — 200 |
| DELETE | `/playlists/{playlistId}` | 204 |
| POST | `/playlists/{playlistId}/items` | `episodeIds`, `at?` (the place the first goes, from 0; the end where none) — 201, the playlist |
| GET | `/playlists/{playlistId}/progress` | The viewer's progress (「Positions and progress」) |

A playlist is `id`, `realm`, `name`, `episodeIds` (in order), `durationMs` (the sum of its episodes'),
`creator`, `createdAt`, `changedAt`. An episode the caller does not read is 422 `unknown-episode`; one the
playlist's readers would not all read (a `mine` episode in an `organization` playlist, or another account's)
is 422 `not-in-realm`. An episode deleted leaves every playlist it stood in.

## Positions and progress

| Method | Path | Does |
|---|---|---|
| GET | `/episodes/{episodeId}/position` | The caller's position in it; 404 `not-found` where it has none |
| PUT | `/episodes/{episodeId}/position` | `seconds`, `done?` — 200, the position |

A position is `episodeId`, `seconds` (a number, 0 or more), `done`, `updatedAt`. A PUT sets `seconds`; `done`
becomes true once `seconds` reaches nine tenths of the episode's duration, or where the body says `done`, and
stays true until a body says `done: false`. A position is the calling account's own; a call naming no
account is 422 `no-account`. Every account that reads the episode keeps one.

A progress, of a playlist or of a title, is `items`, each `episodeId`, `durationMs`, `seconds` (0 where the
viewer has no position), `done`; `doneCount`; `next`, the episode to watch next as docs/design.md
「Playing」 states, or null where every one is done or there is none; and `nextSeconds`, the seconds to start
`next` from (0 for an episode done or never played). A playlist naming one episode twice answers it twice.

## Share links

| Method | Path | Does |
|---|---|---|
| GET | `/shares?episodeId=&playlistId=` | The live links of the members the caller reads — `{items, more}` |
| POST | `/shares` | `episodeId` or `playlistId`, `expiresAt?` — 201, the link; 200, the live link already standing |
| GET | `/shares/{token}` | The link |
| DELETE | `/shares/{token}` | Ends it — 204 |

A link is `token` (32 letters and digits), `episodeId` or `playlistId` (the other null), `url` (the share
address the settings name, a `/` and the token; null where none is named), `creator`, `createdAt`,
`expiresAt` (null for a link with no end). Every account that reads the member makes a link to it; ending one
is its creator's and the administrator's. Naming both members or neither is 400 `bad-request`; an
`expiresAt` not in the future is 400 `bad-request`.

The **share reads** open, to whoever holds a live token, what it opens and nothing else. They take either
bearer with no `Owner-Key` and no `Owner-Role`, since the console relays them for a reader with no sign-in,
and are answered the same whoever asks:

| Method | Path | Does |
|---|---|---|
| GET | `/shares/{token}` | The link |
| GET | `/shares/{token}/head` | `language?` — the share head (bmo-ui's `docs/console-bundles.md`, 「Share pages」) |
| GET | `/shares/{token}/playlist` | A playlist link's playlist; 404 `not-found` for an episode link |
| GET | `/shares/{token}/episodes` | The episodes it opens, in the playlist's order — `{items, more}`, `more` false |
| GET | `/shares/{token}/episodes/{episodeId}/stream` | The episode's stream (「Streams」) |
| GET | `/shares/{token}/episodes/{episodeId}/poster` | The episode's poster |
| GET | `/shares/{token}/episodes/{episodeId}/subtitles` | The episode's subtitles — `{items, more}`, `more` false |
| GET | `/shares/{token}/subtitles/{subtitleId}.vtt` | A subtitle of an episode it opens, as WebVTT |

An episode or a subtitle the link does not open is 404 `not-found`, as is a token that names no link. A link
whose `expiresAt` has passed is 410 `gone`, and is removed by the first read that finds it so. An episode in a
share read carries no `position`.

The share head is `title` (the episode's name led by its title's, or the playlist's name), `url` (the link's),
`type` (`video.episode` for a show's episode, `video.movie` for any other episode, `video.other` for a
playlist), `image` (the poster's share read, where the episode or the playlist's first episode has a poster),
`description` (the title's description where it has one), `locale` and `siteName`, the bundle's words
`theater.share.locale` and `theater.share.siteName` in the asked language (English where it holds none of
it; `en_US` and `Theater` where no face is served). A link has no public address where the settings name no
share address, so its head is then 404 `not-found` and a console writes no head for it.

## Grants

| Method | Path | Does |
|---|---|---|
| GET | `/episodes/{episodeId}/grants` | Who the episode is granted to, oldest first — `{items, more}`, `more` false |
| POST | `/episodes/{episodeId}/grants` | `users`, `permission?` — 200, every grant of the episode as the GET answers them |
| PUT | `/episodes/{episodeId}/grants/{user}` | `permission` — 200, the grant |
| DELETE | `/episodes/{episodeId}/grants/{user}` | Ends it — 204 |

A grant is `user` (the account key, as the console's `Owner-Key` carries it), `permission` (`read` or `edit`),
`grantor` (the account that granted it, empty where the service token named none) and `grantedAt`. The rule
is docs/design.md, 「Grants」.

- **Who calls:** whoever changes the episode (「Realms」); any other reader is 403 `forbidden`, and a caller
  who does not read the episode is 404 `not-found`, a grant giving no reach to these routes.
- **Granting:** `users` is one or more account keys, each 1 to 200 characters; none, an empty one or a longer
  one is 400 `bad-request`. `permission` is `read` where none is given. An account already granted takes the
  permission given; the account the episode belongs to in `mine` is passed over.
- **Changing and ending:** an account the episode is not granted to is 404 `not-found`.
- **What it opens:** to a console call whose `Owner-Key` is the grant's account, `GET /episodes/{episodeId}`,
  its `stream` and `poster`, `GET` and `PUT` of its `position`, `GET /episodes/{episodeId}/subtitles`, and
  `GET /subtitles/{subtitleId}`, its `.vtt` and its `original` for a subtitle of the episode, each answered as
  it is to the owner. Every other route answers that account as it answers anyone outside the realm.
- **Events:** a grant publishes none; the share dialog reads the grants afresh each time it opens.

## Events the theater publishes

`GET /events` is a server-sent stream in the platform's shape (`after` or `Last-Event-ID`, `kinds`; the same
events as a page to `Accept: application/json`, with `after` and `limit` from 1 to 500, 50 where none is
given). Each event's `data` is the member as its GET answers it (an episode with no `position`), or `{id}`
for a `deleted` one (`{token}` for a link):

| Kind | When |
|---|---|
| `library.created`, `library.changed`, `library.deleted` | a library registered, renamed, removed |
| `scan.running`, `scan.finished`, `scan.failed` | a scan started and ended |
| `title.created`, `title.changed`, `title.deleted` | a title made, changed, deleted or retired |
| `episode.created`, `episode.changed`, `episode.deleted` | an episode filed or indexed, changed (a scan finding it gone or back among them), deleted |
| `subtitle.created`, `subtitle.deleted` | a subtitle added, removed |
| `playlist.created`, `playlist.changed`, `playlist.deleted` | a playlist made, changed (its items among it), deleted |
| `position.changed` | the viewer's position set |
| `burn.running`, `burn.finished`, `burn.failed` | a burn started and ended |
| `share.created`, `share.deleted` | a link made, ended |

A move publishes `episode.changed` with the work under its new title, and `title.changed` for the title it
left and the one it joined.

A console reader is given the `organization` realm's events and those of its own account's `mine` realm and
positions; the service token reads every one. The service keeps the events since it started, at most the
last 1000, numbered upward from its start's moment in milliseconds times 1000; a resume from before them is
410 `gone`. A comment line keeps the stream open every fifteen seconds.

## Console

The theater brings a console screen by bmo-ui's console plugin contract (bmo-ui `docs/console-bundles.md`),
which states the manifest, how the module, the stylesheet and the words are addressed and guarded, and the
share pages. It serves that face through bmo-chat's `bmo-console-face` (bmo-chat `docs/console-face.md`),
which answers the manifest, the bundle's files and their guard, and lays the screens' guard over the theater's
own routes; this section names only what the theater hands it.

- **The declaration**, the manifest fields the theater decides: `name` `theater`, `title` `theater.title`,
  `icon` `i-material-symbols-theaters-outline`, `screenPath` `/theater`, `serviceName` `theater`, `apiPrefix`
  `/`, `role` `member`, `ownerKey` true, `events` true, `websocket` false, `shareReads` every share read of
  「Share links」 written as its pattern (the WebVTT read as `/shares/{token}/subtitles/{subtitleFile}`), and
  `shareHead` `/shares/{token}/head`.
- **The bundle's built folder** (`[console] folder`), as `bun run bundle` writes it with bmo-ui's bundle
  build: the module `bundle.js`, the stylesheet `bundle.css`, the words `words.json` and the layer the bundle
  was built on.
- **The console credential** (`[console] token`).
- **The screens' routes:** every route but the health, under the screens' guard.

Where the settings name no console credential, or the folder does not make a face the crate accepts, the
service says why as it starts and serves no face: its manifest and its files are answered 404 `no-route`, the
console leaves the theater out, and the theater's other routes are answered as ever.

The words carry, beside the screens' own, `theater.title` (the sidebar's name) and `theater.share.locale`
and `theater.share.siteName` for the share head.

## The tool

Placed on the tool host by the deployment (`bmo-pocket put --service theater --placed --allow room.tools
tools/theater/.placed`), namespace `bmo`, capability `room.tools`, each entry's input and output schema in
`docs/openapi.json` under `x-tools`:

| Entry | Calls | Does |
|---|---|---|
| `theater_titles` | `GET /titles` | Finds titles by realm, kind and a part of the name |
| `theater_show` | `GET /titles/{titleId}` and its episodes, or `GET /episodes/{episodeId}` and its subtitles | A title with its episodes, or an episode with its subtitles |
| `theater_file` | `POST /titles` where the title is new, `POST /titles/{titleId}/episodes` by path, `POST /shares` where asked | Files a local video under a title, made as a `clip` title where the name is new; with `share`, answers a link |
| `theater_work_rename` | `PATCH /episodes/{episodeId}` | Renames a work, without uploading it again |
| `theater_work_move` | `PATCH /episodes/{episodeId}` with `titleId`, or `POST /playlists/{playlistId}/items` | Moves a work to another title or into a playlist; nothing is deleted |
| `theater_work_delete` | `DELETE /episodes/{episodeId}` | Deletes a filed or burned work and its file; a library's work is 409 `conflict` |
| `theater_playlist_remove` | `GET /playlists/{playlistId}` and `PATCH /playlists/{playlistId}` | Takes a work out of a playlist, leaving the work and the playlist standing |
| `theater_subtitle_add` | `POST /episodes/{episodeId}/subtitles` by path | Adds a local subtitle file to an episode |
| `theater_playlist_make` | `POST /playlists` | Makes a playlist |
| `theater_playlist_add` | `POST /playlists/{playlistId}/items` | Adds episodes to a playlist |
| `theater_progress` | `GET /playlists/{playlistId}/progress` or `GET /titles/{titleId}/progress` | How far the owner has come, and what is next |
| `theater_burn` | `POST /episodes/{episodeId}/burns` | Burns a subtitle into an episode, answered at once; the clip is filed when the burn ends |

A work is one episode, the video the theater keeps; its title is the shelf it stands on. A move changes
where it stands or which playlists hold it, never whether it stands: deleting a work is its own entry, and
the service's realm rule decides who may change or delete it.

Every holder of the tool acts for the owner: the tool names the account `THEATER_ACCOUNT` holds as its
`Owner-Key`. It reaches the service with the service token from `env/theater.env` under the data root
(`THEATER_ADDRESS`, `THEATER_TOKEN`, `THEATER_ACCOUNT`; the same names in the call's environment come first),
which the deployment writes before the flip with `bmo-theater tool-env` from the settings.

## Calls the theater makes

The theater calls no other unit. It runs two commands, each named by the settings (`[media] ffprobe` and
`[media] ffmpeg`), each waited on however long it takes:

| What | Command |
|---|---|
| what a file holds | `ffprobe -v error -print_format json -show_format -show_streams <file>` |
| a poster frame | `ffmpeg -nostdin -v error -ss <a tenth of the duration> -i <file> -frames:v 1 -vf scale=640:-2 -q:v 3 -y <poster>.jpg` |
| a burn | `ffmpeg -nostdin -v error -i <file> -vf subtitles=<subtitle> -c:v libx264 -preset veryfast -crf 20 -c:a copy -movflags +faststart -y output.mp4`, run in the burn's work folder, the subtitle the ASS original or the WebVTT written there in UTF-8 |

A poster that fails leaves the episode without one; a probe that fails leaves its fields null for a library's
file and refuses a filed one (`not-a-video`).

## Codes

Beside the common codes of bmo-chat's `docs/conventions.md` (`bad-request`, `unauthorized`, `forbidden`,
`no-route`, `not-found`, `conflict`, `gone`, `failed`), each answered 422:

| Code | When |
|---|---|
| `no-account` | a `mine` list or write, or a position, on a call that names no account |
| `not-a-folder` | a library's path that is not an absolute folder the service reads, or one inside its file root |
| `not-a-video` | a filed file whose extension is no video's or in which ffprobe finds no video |
| `not-a-file` | a path given to file or to add that is not an absolute file the service reads |
| `unknown-format` | a subtitle format outside `srt`, `ass`, `smi` and `vtt` |
| `no-cues` | a subtitle that converts to no cue |
| `unknown-subtitle` | a burn naming a subtitle that is not the episode's |
| `unknown-episode` | a playlist naming an episode the caller does not read |
| `unknown-title` | a move naming a title the caller does not read |
| `not-in-realm` | a playlist naming an episode its readers would not all read, or a move naming a title of the other realm |

## Commands

- `bmo-theater serve` brings the database to its schema, fails the scans and burns a stop cut short, and
  answers the routes until it is told to stop.
- `bmo-theater migrate` brings the database the settings name to the schema the binary carries, and ends.
- `bmo-theater health` ends 0 where the service its settings name answers ready.
- `bmo-theater openapi` prints the OpenAPI document, which `docs/openapi.json` holds.
- `bmo-theater tool-env` writes the data root's `env/theater.env`, where the tool finds the service.
- `bmo-theater console-reach` prints how a console reaches the service, one JSON object of `address` (the
  service's base address), `consoleToken` and `token`, for a deployment's console probe (bmo-testbed
  `docs/declaration.md`, `[deploy.console]`).

## Settings

The settings are read at start in layers, each overriding the one before: the defaults built into the binary
(the repository's `settings.toml`); then `settings.toml` and `settings.local.toml` in the settings folder,
the one `BMO_THEATER_SETTINGS` names, else the folder above the one holding the binary (a deployment's root,
where `settings.local.toml` links to the data root's `env/theater.toml`); then `BMO_THEATER_<TABLE>__<KEY>` in
the environment.

| Table | Key | What it is |
|---|---|---|
| `server` | `listen` | where the service answers, `host:port` |
| `server` | `token` | the service token; an empty one lets no caller in |
| `server` | `data_root` | the data root under which `tool-env` writes `env/theater.env` |
| `console` | `token` | the console credential; empty, the console reaches nothing |
| `console` | `folder` | the console bundle's built folder (`client/dist` where none is set; a relative one read from where the process starts) |
| `database` | `url` | the PostgreSQL database the theater's tables stand in |
| `files` | `root` | the file root (docs/design.md, 「The file root」); made where it does not stand |
| `media` | `ffmpeg`, `ffprobe` | the commands run (「Calls the theater makes」) |
| `share` | `address` | the address a link's `url` stands under, the console's share screen; empty, a link has no `url` and no share head |
| `tool` | `account` | the account the tool acts for, written into `env/theater.env` as `THEATER_ACCOUNT` |
