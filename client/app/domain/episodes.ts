import type { Episode, Progress, ProgressItem, TitleKind } from "../../shared/types";

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

type Translate = (key: string, params?: Record<string, unknown>) => string;

/**
 * A part of a title's episodes its page shows under one name (`docs/design.md`, 「Playing」 and 「The console」):
 * a season, the specials, the extras, or the whole of a title that has no seasons. Each says its own name, the
 * word the address chooses it by, and how what plays next goes on from one of its episodes.
 */
export interface Section {
  /** The word the address chooses it by. */
  key: string;
  episodes: Episode[];
  /** The words it is named by. */
  name: (t: Translate) => string;
  /**
   * Where play goes on from its last episode: `main`, into the next section of the main line; `own`, nowhere past
   * its own episodes; `none`, nowhere at all, not even to its next episode.
   */
  line: "main" | "own" | "none";
}

function season(number: number | null, episodes: Episode[]): Section {
  return number === null
    ? { key: "episodes", episodes, name: (t) => t("theater.label.episodes"), line: "main" }
    : { key: String(number), episodes, name: (t) => t("theater.label.season_number", { n: String(number) }), line: "main" };
}

function specials(episodes: Episode[]): Section {
  return { key: "specials", episodes, name: (t) => t("theater.label.specials"), line: "own" };
}

function extras(episodes: Episode[]): Section {
  return { key: "extras", episodes, name: (t) => t("theater.label.extras"), line: "none" };
}

function whole(episodes: Episode[]): Section {
  return { key: "all", episodes, name: (t) => t("theater.label.episodes"), line: "main" };
}

/** A title's episodes as its page shows them: what is chosen between, and the extras standing apart after them. */
export interface Sections {
  /** The seasons in order, the specials last; a title with no seasons holds its whole here. */
  chosen: Section[];
  /** A show's episodes with no number, which stand apart after the seasons; none where it has none. */
  extras: Section | null;
}

/**
 * A title's episodes, in the service's order, as its page shows them: a show's numbered episodes by season, those
 * with a number and no season first, season 0 last as the specials, and those with no number apart as the extras;
 * any other title's as one whole.
 */
export function sectionsOf(kind: TitleKind, episodes: Episode[]): Sections {
  if (kind !== "show") return { chosen: episodes.length ? [whole(episodes)] : [], extras: null };

  const seasons = new Map<number | null, Episode[]>();
  const special: Episode[] = [];
  const extra: Episode[] = [];

  for (const episode of episodes) {
    if (episode.number === null) extra.push(episode);
    else if (episode.season === 0) special.push(episode);
    else seasons.set(episode.season, [...(seasons.get(episode.season) ?? []), episode]);
  }

  const chosen = [...seasons.entries()].sort(([a], [b]) => (a === null ? -1 : b === null ? 1 : a - b)).map(([number, members]) => season(number, members));

  if (special.length) chosen.push(specials(special));

  return { chosen, extras: extra.length ? extras(extra) : null };
}

/** The section holding an episode, among the chosen ones and the extras. */
export function sectionOf(sections: Sections, episodeId: number): Section | null {
  return [...sections.chosen, ...(sections.extras ? [sections.extras] : [])].find((one) => one.episodes.some((episode) => episode.id === episodeId)) ?? null;
}

/**
 * What plays after an episode of a title: the next of its section, and past a main-line section's end the first of
 * the next main-line section; nothing after an extra, nor past the specials' end.
 */
export function followingOf(sections: Sections, episodeId: number): Episode | null {
  const holding = sectionOf(sections, episodeId);

  if (!holding || holding.line === "none") return null;

  const at = holding.episodes.findIndex((episode) => episode.id === episodeId);
  const next = holding.episodes[at + 1];

  if (next) return next;
  if (holding.line !== "main") return null;

  const later = sections.chosen.slice(sections.chosen.indexOf(holding) + 1).find((one) => one.line === "main");

  return later?.episodes[0] ?? null;
}

/** How a title reads in one line: how many seasons, how many episodes and how long they all run, in milliseconds. */
export interface TitleFacts {
  seasons: number;
  episodes: number;
  durationMs: number;
}

export function factsOf(sections: Sections, episodes: Episode[]): TitleFacts {
  return {
    seasons: sections.chosen.filter((one) => one.line === "main" && one.key !== "all" && one.key !== "episodes").length,
    episodes: episodes.length,
    durationMs: episodes.reduce((sum, episode) => sum + (episode.durationMs ?? 0), 0)
  };
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

/** The minutes left of an episode the viewer has started and not finished, rounded up; none otherwise. */
export function minutesLeft(episode: Pick<Episode, "position" | "durationMs">): number | null {
  const position = episode.position;

  if (position === null || position.done || position.seconds <= 0 || !episode.durationMs) return null;

  return Math.max(1, Math.ceil((episode.durationMs / 1000 - position.seconds) / 60));
}
