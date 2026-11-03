# bmo-theater

The platform's store of videos as one service: titles made of episodes, from videos filed as outputs into its
own file root and from libraries of video files already on disk, indexed in place; each episode streamed as its
file with HTTP ranges, subtitles converted to WebVTT and burned in on request, playlists, each viewer's watch
position and what to watch next, and share links that open a video with no sign-in; in two realms kept apart,
each account's own and the organization's; and the console bundle that shows them under its sidebar entry.
`docs/design.md` states what it is and `docs/api.md` its boundary: every route, the events it publishes, the
tool it places, the commands it runs and its settings.

## Tests

`cargo test` (through the testbed) runs the crate's one harness, `tests/main.rs`, each test over a PostgreSQL
database of its own on the suite's server and a file root of its own: a file's names read for its numbers
(`names.rs`), the subtitle formats converted (`subtitles.rs`), ranges served (`range.rs`), progress and the next
episode (`progress.rs`), a library planned, scanned, streamed, rescanned and burned over tiny videos ffmpeg makes,
and a video filed by upload and by path (`scan.rs`, skipped where ffmpeg is not installed), the realms, the
guard, playlists and positions (`realms.rs`), share links (`shares.rs`), the console bundle (`console.rs`), the
OpenAPI document (`openapi.rs`) and the repository's own checks (`repository.rs`). The tool's crate carries its
own harness, `tools/theater/tests/main.rs`, against a stand-in theater service. The console bundle's tests stand in
`client/tests/`, run by `bun run check` against a Bun stand-in of the service.

## Layout

| Folder | Holds |
|---|---|
| `src/` | The service and its command: the routes, the tables and their migrations, the scans, the burns, the subtitle converters, the range serving, the console bundle's serving and the OpenAPI document |
| `tools/` | The theater tool the deployment places on the tool host: its crate, its manifest, its input schemas and its manual |
| `client/` | The console plugin bundle, a project of its own |
| `deploy/` | The service's user unit |
| `docs/` | The canonical statements of what the service is and its boundary, and the OpenAPI document |
| `tests/` | The crate's one test harness and what its modules share |

At the root stand the Cargo manifest, `Cargo.lock`, `testbed.toml`, `rustfmt.toml`, `settings.toml` (the
defaults built into the binary), `CLAUDE.md`, `PERSONAL-DATA.md` and `LICENSE`.

## Commands

- `bmo-theater serve` brings the database to its schema and answers the routes until it is told to stop.
- `bmo-theater migrate` brings the database to its schema and ends.
- `bmo-theater health` ends 0 where the service its settings name answers ready.
- `bmo-theater openapi` prints the OpenAPI document, which `docs/openapi.json` holds.
- `bmo-theater tool-env` writes the data root's `env/theater.env`, where the tool finds the service.

The settings are read from the binary's built-in `settings.toml`, then the deployment's `settings.toml` and
`settings.local.toml` beside the release (the data root's `env/theater.toml`), then `BMO_THEATER_<TABLE>__<KEY>`
in the environment (`docs/api.md`, 「Settings」).
