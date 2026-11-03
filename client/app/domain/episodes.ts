import type { Episode, Progress, ProgressItem } from "../../shared/types";

// How the screens read a title's episodes: the code a show's episode goes by, its seasons, how far the viewer
// has come through one, and the episode after another.

/** Where a viewer has come through an episode, from nought to one, and whether it counts as watched. */
export interface Watched {
  fraction: number;
  done: boolean;
  seconds: number;
}

export const UNWATCHED: Watched = { fraction: 0, done: false, seconds: 0 };

/** A show's episode by its numbers, `S06E35`, `E01` where it has no season, and nothing where it has none. */
export function episodeCode(episode: Pick<Episode, "season" | "number">): string {
  const pad = (value: number) => String(value).padStart(2, "0");

  if (episode.number === null) return "";
  if (episode.season === null) return `E${pad(episode.number)}`;

  return `S${pad(episode.season)}E${pad(episode.number)}`;
}

/** An episode's whole name: its code before its name where it has one. */
export function episodeTitle(episode: Pick<Episode, "season" | "number" | "name">): string {
  const code = episodeCode(episode);

  return code ? `${code} ${episode.name}` : episode.name;
}

/** The seasons a title's episodes stand in, in order: the numbered seasons first, then those with none. */
export interface Season {
  season: number | null;
  episodes: Episode[];
}

export function seasonsOf(episodes: Episode[]): Season[] {
  const groups = new Map<number | null, Episode[]>();

  for (const episode of episodes) {
    const group = groups.get(episode.season);

    if (group) group.push(episode);
    else groups.set(episode.season, [episode]);
  }

  return [...groups.entries()]
    .sort(([a], [b]) => (a === null ? 1 : b === null ? -1 : a - b))
    .map(([season, members]) => ({ season, episodes: members }));
}

/** How far a viewer has come through an item of a progress, against the episode's length. */
export function watchedOf(item: Pick<ProgressItem, "seconds" | "done" | "durationMs"> | null | undefined): Watched {
  if (!item) return UNWATCHED;

  const length = (item.durationMs ?? 0) / 1000;
  const fraction = item.done ? 1 : length > 0 ? Math.min(1, Math.max(0, item.seconds / length)) : 0;

  return { fraction, done: item.done, seconds: item.seconds };
}

/** How far a viewer has come through an episode, by its own position. */
export function watchedEpisode(episode: Pick<Episode, "position" | "durationMs">): Watched {
  return episode.position === null ? UNWATCHED : watchedOf({ ...episode.position, durationMs: episode.durationMs });
}

/** Each episode's progress by its id, from a progress's items. */
export function watchedById(progress: Progress | null): Map<number, Watched> {
  const found = new Map<number, Watched>();

  for (const item of progress?.items ?? []) found.set(item.episodeId, watchedOf(item));

  return found;
}

/** The episode after `id` in a title's order, or none at its end. */
export function episodeAfter(episodes: Pick<Episode, "id">[], id: number): number | null {
  const at = episodes.findIndex((episode) => episode.id === id);

  return at === -1 ? null : (episodes[at + 1]?.id ?? null);
}

/** The place after `at` in a playlist's order, or none at its end. */
export function placeAfter(episodeIds: number[], at: number): number | null {
  return at + 1 < episodeIds.length ? at + 1 : null;
}

/** The place in a playlist an episode is opened at: the place given where it holds the episode, else its first. */
export function placeOf(episodeIds: number[], episodeId: number, asked: number | null): number | null {
  if (asked !== null && episodeIds[asked] === episodeId) return asked;

  const found = episodeIds.indexOf(episodeId);

  return found === -1 ? null : found;
}

/** A length in milliseconds as a clock: `42:05`, or `1:02:05` past an hour. */
export function lengthClock(ms: number | null): string {
  if (ms === null) return "";

  const whole = Math.max(0, Math.round(ms / 1000));
  const hours = Math.floor(whole / 3600);
  const minutes = Math.floor((whole % 3600) / 60);
  const seconds = String(whole % 60).padStart(2, "0");

  return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}
