// The theater service's wire shapes as the screens read them (`docs/api.md`): camelCase keys, times in epoch
// milliseconds, ids as numbers, a list `{items, more}`.

export type Realm = "mine" | "organization";

export type TitleKind = "show" | "film" | "clip";

export type EpisodeKind = "episode" | "clip";

export type JobState = "running" | "finished" | "failed";

export type SubtitleFormat = "srt" | "ass" | "smi" | "vtt";

export interface ListPage<T> {
  items: T[];
  more: boolean;
}

export interface Title {
  id: number;
  realm: Realm;
  name: string;
  kind: TitleKind;
  description: string;
  labels: Record<string, string>;
  libraryId: number | null;
  episodeCount: number;
  hasPoster: boolean;
  creator: string;
  createdAt: number;
  changedAt: number;
}

export interface Position {
  episodeId: number;
  seconds: number;
  done: boolean;
  updatedAt: number;
}

export interface Episode {
  id: number;
  titleId: number;
  realm: Realm;
  kind: EpisodeKind;
  season: number | null;
  number: number | null;
  name: string;
  label: string;
  fileName: string;
  mediaType: string;
  byteCount: number;
  durationMs: number | null;
  width: number | null;
  height: number | null;
  videoCodec: string | null;
  audioCodec: string | null;
  available: boolean;
  hasPoster: boolean;
  subtitleCount: number;
  burnedFrom: { episodeId: number; subtitleId: number } | null;
  position: Position | null;
  /** Whether the reader watches it by a grant alone, so nothing but watching is open to them. */
  granted: boolean;
  creator: string;
  createdAt: number;
  changedAt: number;
}

export interface Subtitle {
  id: number;
  episodeId: number;
  language: string;
  label: string;
  format: SubtitleFormat;
  fileName: string;
  cueCount: number;
  creator: string;
  createdAt: number;
}

export interface Burn {
  id: number;
  episodeId: number;
  subtitleId: number;
  state: JobState;
  clipId: number | null;
  error: string | null;
  creator: string;
  startedAt: number;
  endedAt: number | null;
}

export interface Playlist {
  id: number;
  realm: Realm;
  name: string;
  episodeIds: number[];
  durationMs: number;
  creator: string;
  createdAt: number;
  changedAt: number;
}

export interface ProgressItem {
  episodeId: number;
  durationMs: number | null;
  seconds: number;
  done: boolean;
}

export interface Progress {
  items: ProgressItem[];
  doneCount: number;
  next: number | null;
  nextSeconds: number;
}

/** What a grant opens; both open the same reads (`docs/design.md`, 「Grants」). */
export type Permission = "read" | "edit";

/** One account an episode is shared with, by the account key. */
export interface Grant {
  user: string;
  permission: Permission;
  grantor: string;
  grantedAt: number;
}

export interface Share {
  token: string;
  episodeId: number | null;
  playlistId: number | null;
  url: string | null;
  creator: string;
  createdAt: number;
  expiresAt: number | null;
}

export interface Library {
  id: number;
  realm: Realm;
  name: string;
  path: string;
  creator: string;
  createdAt: number;
  scannedAt: number | null;
  scanning: boolean;
  titleCount: number;
  episodeCount: number;
}

export interface Scan {
  id: number;
  libraryId: number;
  state: JobState;
  foundCount: number;
  addedCount: number;
  changedCount: number;
  missingCount: number;
  error: string | null;
  startedAt: number;
  endedAt: number | null;
}

/** The kinds the service's event stream tells (`docs/api.md`, 「Events the theater publishes」). */
export const STREAM_KINDS = [
  "library.created",
  "library.changed",
  "library.deleted",
  "scan.running",
  "scan.finished",
  "scan.failed",
  "title.created",
  "title.changed",
  "title.deleted",
  "episode.created",
  "episode.changed",
  "episode.deleted",
  "subtitle.created",
  "subtitle.deleted",
  "playlist.created",
  "playlist.changed",
  "playlist.deleted",
  "position.changed",
  "burn.running",
  "burn.finished",
  "burn.failed",
  "share.created",
  "share.deleted"
] as const;

export type StreamKind = (typeof STREAM_KINDS)[number];

export interface StreamEvent {
  kind: StreamKind;
  data: unknown;
}
