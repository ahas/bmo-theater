# Working in this repository

This repository is bmo-theater: the platform's store of videos. Where bmo-artifact keeps documents of
words and pictures, this unit keeps titles made of episodes — videos filed as outputs, and libraries of
video files already on disk, indexed in place — with subtitles, playlists, each viewer's watch position, and
share links. `docs/design.md` states what it is; `docs/api.md` is the boundary contract.

## Rules

- **`docs/` is canonical.** A change that would make a document wrong repairs the document first.
- **A library file is never moved, copied or rewritten.** A registered folder is read; only what this unit
  derives (posters, converted subtitles, burned outputs) and what is filed as an output lives under its own
  file root.
- **Two realms, never one:** a title and a playlist stand in `mine` (an account's) or `organization`; a
  watch position is always the viewer's own.
- **No clock bound on any request.** A scan, a burn or a stream ends when it is done.
- **Tests live in `tests/`,** one harness `tests/main.rs`. The checks are the testbed's, through `bmo check`:
  `cargo check` on the crates a change marks, and its Cargo workspace check, `cargo fmt --check` and
  `cargo clippy --all-targets` with every warning an error; `cargo test` through the testbed; the client's
  `bun run check`.
- **English only** in code, documents, tests and commits; Korean in the bundle's `ko` words alone.
- **No personal data, no names of agents, owners or hosts** in code, documents or commits; a path that
  differs by installation is a setting.
- **Versions:** `0.1.0-rev.N` until a release the owner declares; no sibling version pins.
- Depend only on the base libraries (bmo-chat, bmo-chat-client, bmo-console-face, bmo-ui) and this unit's own
  crates; other units through their documented APIs. ffmpeg and ffprobe are reached as commands the settings
  name.
