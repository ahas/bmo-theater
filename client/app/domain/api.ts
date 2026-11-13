import type { Burn, Episode, Grant, Library, ListPage, Permission, Playlist, Position, Progress, Realm, Scan, Share, Subtitle, Title } from "../../shared/types";

// The screens' calls to the theater service, each at the path the service's own API writes it (`docs/api.md`).
// The caller hands the call itself — through the console's relay in the browser, to a stand-in in a test — so
// this module names no address and no fetcher. An upload that reports how far it has come is handed the same
// way, as a sender.

export type Method = "GET" | "POST" | "PUT" | "PATCH" | "DELETE";

export type Call = <T>(path: string, options?: { method?: Method; body?: unknown; query?: Record<string, string> }) => Promise<T>;

/** Sends a form to a path and answers what the service answered, telling `progress` the bytes sent so far. */
export type Send = <T>(path: string, form: FormData, progress: (sent: number, total: number) => void) => Promise<T>;

/** A call the service answered with a refusal: its status and its code (`docs/api.md`, 「Codes」). */
export class Refusal extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    message: string
  ) {
    super(message);
  }
}

/** A call that never reached the service: the network, not the service, said no. */
export class Unreachable extends Error {}

// What a fetcher threw, sorted into a refusal the service gave and a call that never landed.
export function sorted(error: unknown): Error {
  if (error instanceof Refusal || error instanceof Unreachable) return error;

  const failed = error as { response?: { status?: number }; status?: number; statusCode?: number; data?: { code?: unknown; error?: unknown } } | null;
  const status = failed?.response?.status ?? failed?.statusCode ?? failed?.status;

  if (typeof status === "number" && status > 0) {
    const code = typeof failed?.data?.code === "string" ? failed.data.code : "failed";
    const message = typeof failed?.data?.error === "string" ? failed.data.error : String(status);

    return new Refusal(status, code, message);
  }

  return new Unreachable(error instanceof Error ? error.message : String(error));
}

/** Where a member's own path stands under the service. */
export const paths = {
  titlePoster: (titleId: number) => `/titles/${titleId}/poster`,
  episodePoster: (episodeId: number) => `/episodes/${episodeId}/poster`,
  stream: (episodeId: number) => `/episodes/${episodeId}/stream`,
  vtt: (subtitleId: number) => `/subtitles/${subtitleId}.vtt`,
  position: (episodeId: number) => `/episodes/${episodeId}/position`,
  shared: {
    poster: (token: string, episodeId: number) => `/shares/${encodeURIComponent(token)}/episodes/${episodeId}/poster`,
    stream: (token: string, episodeId: number) => `/shares/${encodeURIComponent(token)}/episodes/${episodeId}/stream`,
    vtt: (token: string, subtitleId: number) => `/shares/${encodeURIComponent(token)}/subtitles/${subtitleId}.vtt`
  }
} as const;

/** What a share link opens, as its page reads it with nothing but its token. */
export interface SharedReads {
  link: (token: string) => Promise<Share>;
  playlist: (token: string) => Promise<Playlist>;
  episodes: (token: string) => Promise<Episode[]>;
  subtitles: (token: string, episodeId: number) => Promise<Subtitle[]>;
}

export interface TheaterApi {
  titles: (realm: Realm) => Promise<Title[]>;
  title: (id: number) => Promise<Title>;
  makeTitle: (realm: Realm, name: string) => Promise<Title>;
  episodes: (titleId: number) => Promise<Episode[]>;
  episode: (id: number) => Promise<Episode>;
  titleProgress: (titleId: number) => Promise<Progress>;
  file: (titleId: number, form: FormData, progress: (sent: number, total: number) => void) => Promise<Episode>;
  deleteEpisode: (id: number) => Promise<void>;
  subtitles: (episodeId: number) => Promise<Subtitle[]>;
  addSubtitle: (episodeId: number, form: FormData, progress: (sent: number, total: number) => void) => Promise<Subtitle>;
  deleteSubtitle: (id: number) => Promise<void>;
  burns: (episodeId: number) => Promise<Burn[]>;
  burn: (episodeId: number, subtitleId: number) => Promise<Burn>;
  setPosition: (episodeId: number, seconds: number, done?: boolean) => Promise<Position>;
  playlists: (realm: Realm) => Promise<Playlist[]>;
  playlist: (id: number) => Promise<Playlist>;
  playlistProgress: (id: number) => Promise<Progress>;
  makePlaylist: (realm: Realm, name: string, episodeIds?: number[]) => Promise<Playlist>;
  changePlaylist: (id: number, change: { name?: string; episodeIds?: number[] }) => Promise<Playlist>;
  addToPlaylist: (id: number, episodeIds: number[]) => Promise<Playlist>;
  deletePlaylist: (id: number) => Promise<void>;
  shareOf: (member: { episodeId: number } | { playlistId: number }) => Promise<Share | null>;
  makeShare: (member: { episodeId: number } | { playlistId: number }, expiresAt: number | null) => Promise<Share>;
  endShare: (token: string) => Promise<void>;
  grants: (episodeId: number) => Promise<Grant[]>;
  grant: (episodeId: number, users: string[], permission: Permission) => Promise<Grant[]>;
  changeGrant: (episodeId: number, user: string, permission: Permission) => Promise<Grant>;
  endGrant: (episodeId: number, user: string) => Promise<void>;
  libraries: () => Promise<Library[]>;
  library: (id: number) => Promise<Library>;
  makeLibrary: (realm: Realm, path: string, name: string | null) => Promise<Library>;
  renameLibrary: (id: number, name: string) => Promise<Library>;
  scanLibrary: (id: number) => Promise<Scan>;
  deleteLibrary: (id: number) => Promise<void>;
  administers: () => Promise<boolean>;
  shared: SharedReads;
}

export function createTheaterApi(call: Call, send: Send): TheaterApi {
  async function sent<T>(path: string, options?: Parameters<Call>[1]): Promise<T> {
    try {
      return await call<T>(path, options);
    } catch (error) {
      throw sorted(error);
    }
  }

  async function uploaded<T>(path: string, form: FormData, progress: (sent: number, total: number) => void): Promise<T> {
    try {
      return await send<T>(path, form, progress);
    } catch (error) {
      throw sorted(error);
    }
  }

  // Every page of a list, oldest first, each read from the id after the last one held.
  async function all<T extends { id: number }>(path: string, query: Record<string, string> = {}): Promise<T[]> {
    const items: T[] = [];
    let after: number | null = null;

    for (;;) {
      const page: ListPage<T> = await sent<ListPage<T>>(path, { query: after === null ? query : { ...query, after: String(after) } });

      items.push(...page.items);

      const last = page.items.at(-1);

      if (!page.more || last === undefined) return items;

      after = last.id;
    }
  }

  async function items<T>(path: string): Promise<T[]> {
    return (await sent<ListPage<T>>(path)).items;
  }

  function memberQuery(member: { episodeId: number } | { playlistId: number }): Record<string, string> {
    return "episodeId" in member ? { episodeId: String(member.episodeId) } : { playlistId: String(member.playlistId) };
  }

  // Whether the reader administers the theater. No route says so; registering a library is the administrator's
  // alone and is refused before its body is read, so an empty one answers 403 to a member and a refusal of the
  // body to the administrator, and makes nothing either way.
  async function administers(): Promise<boolean> {
    try {
      await sent("/libraries", { method: "POST", body: {} });

      return true;
    } catch (error) {
      if (error instanceof Refusal) return error.status !== 403 && error.status !== 401;

      throw error;
    }
  }

  const token = (value: string) => encodeURIComponent(value);

  return {
    titles: (realm) => all<Title>("/titles", { realm }),
    title: (id) => sent<Title>(`/titles/${id}`),
    makeTitle: (realm, name) => sent<Title>("/titles", { method: "POST", body: { realm, name, kind: "clip" } }),
    episodes: (titleId) => items<Episode>(`/titles/${titleId}/episodes`),
    episode: (id) => sent<Episode>(`/episodes/${id}`),
    titleProgress: (titleId) => sent<Progress>(`/titles/${titleId}/progress`),
    file: (titleId, form, progress) => uploaded<Episode>(`/titles/${titleId}/episodes`, form, progress),
    deleteEpisode: (id) => sent<void>(`/episodes/${id}`, { method: "DELETE" }),
    subtitles: (episodeId) => items<Subtitle>(`/episodes/${episodeId}/subtitles`),
    addSubtitle: (episodeId, form, progress) => uploaded<Subtitle>(`/episodes/${episodeId}/subtitles`, form, progress),
    deleteSubtitle: (id) => sent<void>(`/subtitles/${id}`, { method: "DELETE" }),
    burns: (episodeId) => items<Burn>(`/episodes/${episodeId}/burns`),
    burn: (episodeId, subtitleId) => sent<Burn>(`/episodes/${episodeId}/burns`, { method: "POST", body: { subtitleId } }),
    setPosition: (episodeId, seconds, done) => sent<Position>(paths.position(episodeId), { method: "PUT", body: done === undefined ? { seconds } : { seconds, done } }),
    playlists: (realm) => all<Playlist>("/playlists", { realm }),
    playlist: (id) => sent<Playlist>(`/playlists/${id}`),
    playlistProgress: (id) => sent<Progress>(`/playlists/${id}/progress`),
    makePlaylist: (realm, name, episodeIds = []) => sent<Playlist>("/playlists", { method: "POST", body: { realm, name, episodeIds } }),
    changePlaylist: (id, change) => sent<Playlist>(`/playlists/${id}`, { method: "PATCH", body: change }),
    addToPlaylist: (id, episodeIds) => sent<Playlist>(`/playlists/${id}/items`, { method: "POST", body: { episodeIds } }),
    deletePlaylist: (id) => sent<void>(`/playlists/${id}`, { method: "DELETE" }),
    shareOf: async (member) => (await sent<ListPage<Share>>("/shares", { query: memberQuery(member) })).items[0] ?? null,
    makeShare: (member, expiresAt) => sent<Share>("/shares", { method: "POST", body: { ...member, expiresAt } }),
    endShare: (value) => sent<void>(`/shares/${token(value)}`, { method: "DELETE" }),
    grants: (episodeId) => items<Grant>(`/episodes/${episodeId}/grants`),
    grant: async (episodeId, users, permission) => (await sent<ListPage<Grant>>(`/episodes/${episodeId}/grants`, { method: "POST", body: { users, permission } })).items,
    changeGrant: (episodeId, user, permission) => sent<Grant>(`/episodes/${episodeId}/grants/${encodeURIComponent(user)}`, { method: "PUT", body: { permission } }),
    endGrant: (episodeId, user) => sent<void>(`/episodes/${episodeId}/grants/${encodeURIComponent(user)}`, { method: "DELETE" }),
    libraries: () => all<Library>("/libraries"),
    library: (id) => sent<Library>(`/libraries/${id}`),
    makeLibrary: (realm, path, name) => sent<Library>("/libraries", { method: "POST", body: name ? { realm, path, name } : { realm, path } }),
    renameLibrary: (id, name) => sent<Library>(`/libraries/${id}`, { method: "PATCH", body: { name } }),
    scanLibrary: (id) => sent<Scan>(`/libraries/${id}/scans`, { method: "POST" }),
    deleteLibrary: (id) => sent<void>(`/libraries/${id}`, { method: "DELETE" }),
    administers,
    shared: {
      link: (value) => sent<Share>(`/shares/${token(value)}`),
      playlist: (value) => sent<Playlist>(`/shares/${token(value)}/playlist`),
      episodes: (value) => items<Episode>(`/shares/${token(value)}/episodes`),
      subtitles: (value, episodeId) => items<Subtitle>(`/shares/${token(value)}/episodes/${episodeId}/subtitles`)
    }
  };
}
