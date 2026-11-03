# Working in this repository

This repository is bmo-theater: the platform's dated events, kept as memories in the brain service
(the brain is the master, this unit the subordinate), shown in the console and told to the supervisor
when due. `docs/design.md` states what it is; `docs/api.md` is the boundary contract.

## Rules

- **`docs/` is canonical.** A change that would make a document wrong repairs the document first.
- **The brain holds the truth.** This unit keeps only what can be rebuilt from the brain: a cache and the
  told marks. No theater or event is written anywhere but the brain.
- **Two realms, never one:** an account's and the organization's are different brains.
- **No clock bound on any request.** The minute beat is a schedule, not a bound.
- **Tests live in `tests/`,** one harness `tests/main.rs`; checks: `cargo check --workspace --all-targets`,
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test` through
  the testbed; the client's `bun run check`.
- **English only** in code, documents, tests and commits; Korean in the bundle's `ko` words alone.
- **No personal data, no names of agents, owners or hosts** in code, documents or commits.
- **Versions:** `0.1.0-rev.N` until a release the owner declares; no sibling version pins.
- Depend only on the base libraries (bmo-chat, bmo-chat-client, bmo-ui) and this unit's own crates; other
  units through their documented APIs.
