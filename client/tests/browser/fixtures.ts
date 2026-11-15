import type { Episode, Playlist, Position, Progress, Realm, Title, TitleKind } from "../../shared/types";

// A realm shaped like a library the theater was built for, as the service would answer it (`docs/api.md`): one long
// show of 290 episodes with its specials in season 0 and its trailers numberless at the end, a short show, six clips
// that are versions of one video, a film whose poster is four by three, a clip with no poster, and two playlists;
// the viewer part way through the long show, done with the start of the short one and half through one clip. The
// organization's realm holds nothing. The screens' test answers the relay from it.

const MINUTE = 60_000;
const DAY = 24 * 60 * MINUTE;
const NOW = Date.UTC(2026, 10, 16, 12);

/** The posters the test draws, by the file the host serves each from, with their size. */
export const POSTERS = {
  wide: { file: "wide.jpg", width: 640, height: 360 },
  bars: { file: "bars.jpg", width: 640, height: 360 },
  square: { file: "square.jpg", width: 640, height: 480 }
} as const;

export type PosterName = keyof typeof POSTERS;

interface Made {
  titles: Title[];
  episodes: Episode[];
  positions: Map<number, Position>;
  posters: Map<string, PosterName>;
  playlists: Playlist[];
}

function title(id: number, name: string, kind: TitleKind, addedAt: number, poster: PosterName | null, episodeCount: number): Title {
  const size = poster ? POSTERS[poster] : null;

  return {
    id,
    realm: "mine",
    name,
    kind,
    description: "",
    labels: {},
    libraryId: kind === "clip" ? null : 1,
    episodeCount,
    hasPoster: poster !== null,
    posterWidth: size?.width ?? null,
    posterHeight: size?.height ?? null,
    creator: "viewer",
    createdAt: addedAt,
    changedAt: addedAt,
    addedAt
  };
}

function episode(id: number, titleId: number, season: number | null, number: number | null, name: string, minutes: number, poster: PosterName | null, createdAt: number): Episode {
  const size = poster ? POSTERS[poster] : null;

  return {
    id,
    titleId,
    realm: "mine",
    kind: titleId >= 3 && titleId <= 8 ? "clip" : titleId === 10 ? "clip" : "episode",
    season,
    number,
    name,
    label: "",
    fileName: `${name}.mp4`,
    mediaType: "video/mp4",
    byteCount: minutes * 9_000_000,
    durationMs: minutes * MINUTE,
    width: 1280,
    height: 720,
    videoCodec: "h264",
    audioCodec: "aac",
    available: true,
    hasPoster: poster !== null,
    posterWidth: size?.width ?? null,
    posterHeight: size?.height ?? null,
    subtitleCount: 0,
    burnedFrom: null,
    position: null,
    granted: false,
    creator: "viewer",
    createdAt,
    changedAt: createdAt
  };
}

/** The realm as the service holds it. */
export function made(): Made {
  const titles: Title[] = [];
  const episodes: Episode[] = [];
  const posters = new Map<string, PosterName>();
  let next = 1001;

  // The long show: season 0, six seasons of forty-five, two episodes named by their files, nine trailers.
  const longAdded = NOW - 40 * DAY;

  for (let number = 1; number <= 11; number++) episodes.push(episode(next++, 1, 0, number, number === 1 ? "Pilot" : `Special ${number - 1}`, 30, "wide", longAdded));

  for (let season = 1; season <= 6; season++) {
    for (let number = 1; number <= 45; number++) {
      const named = season === 3 && (number === 4 || number === 5) ? `Long.Show.S03E0${number}.1080p.WEB-DL.x264` : `Chapter ${number}`;

      episodes.push(episode(next++, 1, season, number, named, 24, "wide", longAdded));
    }
  }

  for (let at = 1; at <= 9; at++) episodes.push(episode(next++, 1, null, null, at === 1 ? "Teaser" : `Trailer ${at - 1}`, 2, "wide", longAdded));
  titles.push(title(1, "Long Show", "show", longAdded, "wide", 290));

  // The short show: two seasons of six, forty-five minutes each.
  const shortAdded = NOW - 20 * DAY;

  for (let season = 1; season <= 2; season++) for (let number = 1; number <= 6; number++) episodes.push(episode(next++, 2, season, number, `Part ${number}`, 45, "bars", shortAdded));
  titles.push(title(2, "Short Show", "show", shortAdded, "bars", 12));

  // Six clips that are versions of one video.
  ["Harbour walk", "Harbour walk rev.2", "Harbour walk rev.3", "Harbour walk (alternative cut)", "Harbour walk rev.4", "Harbour walk final"].forEach((name, at) => {
    const added = NOW - (6 - at) * DAY;

    episodes.push(episode(next++, 3 + at, null, null, name, 3, "bars", added));
    titles.push(title(3 + at, name, "clip", added, "bars", 1));
  });

  // A film whose poster is four by three, and a clip with no poster at all.
  episodes.push(episode(next++, 9, null, null, "Old Film", 95, "square", NOW - 60 * DAY));
  titles.push(title(9, "Old Film", "film", NOW - 60 * DAY, "square", 1));
  episodes.push(episode(next++, 10, null, null, "Studio notes", 12, null, NOW - 2 * 60 * MINUTE));
  titles.push(title(10, "Studio notes", "clip", NOW - 2 * 60 * MINUTE, null, 1));

  for (const one of titles) if (one.hasPoster) posters.set(`title:${one.id}`, one.posterWidth === 640 && one.posterHeight === 480 ? "square" : one.id === 1 ? "wide" : "bars");
  for (const one of episodes) if (one.hasPoster) posters.set(`episode:${one.id}`, one.posterHeight === 480 ? "square" : one.titleId === 1 ? "wide" : "bars");

  // Where the viewer stands: the long show done to its fifth episode of season 1 and part through its sixth, the
  // short show done with its third, and the third clip half through.
  const positions = new Map<number, Position>();
  const at = (titleId: number, season: number | null, number: number | null, name?: string) =>
    episodes.find((one) => one.titleId === titleId && one.season === season && one.number === number && (name === undefined || one.name === name))!;
  const put = (one: Episode, seconds: number, done: boolean, updatedAt: number) => positions.set(one.id, { episodeId: one.id, seconds, done, updatedAt, setAside: false });

  for (let number = 1; number <= 5; number++) put(at(1, 1, number), 1440, true, NOW - (10 - number) * MINUTE);
  put(at(1, 1, 6), 600, false, NOW - MINUTE);
  for (let number = 1; number <= 3; number++) put(at(2, 1, number), 2700, true, NOW - DAY - (4 - number) * MINUTE);
  put(at(5, null, null), 90, false, NOW - 2 * DAY);

  for (const one of episodes) one.position = positions.get(one.id) ?? null;

  const playlists: Playlist[] = [
    { id: 1, realm: "mine", name: "Weekend", episodeIds: [at(2, 1, 1).id, at(2, 1, 2).id, at(9, null, null).id], durationMs: (45 + 45 + 95) * MINUTE, creator: "viewer", createdAt: NOW - 3 * DAY, changedAt: NOW - 3 * DAY },
    { id: 2, realm: "mine", name: "Harbour cuts", episodeIds: [3, 4, 5, 6].map((titleId) => at(titleId, null, null).id), durationMs: 12 * MINUTE, creator: "viewer", createdAt: NOW - DAY, changedAt: NOW - DAY }
  ];

  return { titles, episodes, positions, posters, playlists };
}

// A show's main line: its numbered episodes outside season 0; any other title's every episode (`docs/design.md`,
// 「Playing」).
function mainLine(kind: TitleKind, episodes: Episode[]): Episode[] {
  return kind === "show" ? episodes.filter((one) => one.number !== null && one.season !== 0) : episodes;
}

/** A title's progress as the service answers it, its up next read along the main line. */
export function progressOf(kind: TitleKind, episodes: Episode[], positions: Map<number, Position>): Progress {
  const items = episodes.map((one) => {
    const position = positions.get(one.id);

    return { episodeId: one.id, durationMs: one.durationMs, seconds: position?.seconds ?? 0, done: position?.done ?? false };
  });
  const played = episodes.filter((one) => positions.has(one.id)).sort((a, b) => positions.get(b.id)!.updatedAt - positions.get(a.id)!.updatedAt);
  const last = played[0];
  const line = mainLine(kind, episodes);
  let upNext: Episode | undefined;

  if (last && !positions.get(last.id)!.done) upNext = last;
  else {
    const lastOnLine = played.find((one) => line.includes(one));
    const from = lastOnLine ? line.indexOf(lastOnLine) + 1 : 0;

    upNext = line.slice(from).find((one) => !positions.get(one.id)?.done);
  }

  const seconds = upNext ? (positions.get(upNext.id)?.done ? 0 : (positions.get(upNext.id)?.seconds ?? 0)) : 0;

  return { items, doneCount: items.filter((one) => one.done).length, next: upNext?.id ?? null, nextSeconds: seconds, upNext: upNext?.id ?? null, upNextSeconds: seconds };
}

/** The titles of a realm in the order the list names (`docs/api.md`, 「Titles」). */
export function ordered(titles: Title[], realm: Realm, order: string, lastWatched: (title: Title) => number): Title[] {
  const held = titles.filter((one) => one.realm === realm);
  const added = (a: Title, b: Title) => b.addedAt - a.addedAt || b.id - a.id;

  if (order === "name") return [...held].sort((a, b) => (a.name.toLowerCase() < b.name.toLowerCase() ? -1 : a.name.toLowerCase() > b.name.toLowerCase() ? 1 : a.id - b.id));
  if (order === "watched") return [...held].sort((a, b) => lastWatched(b) - lastWatched(a) || added(a, b));
  if (order === "added") return [...held].sort(added);

  return [...held].sort((a, b) => a.id - b.id);
}
