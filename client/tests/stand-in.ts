import { ofetch } from "ofetch";

import type { Call, Send } from "../app/domain/api";
import type { Burn, Episode, Library, Playlist, Position, Progress, Realm, Share, Subtitle, Title } from "../shared/types";

// A stand-in of the theater service's routes the screens call (`docs/api.md`), served on the loopback for the
// length of a test: titles and playlists in two realms, episodes with the caller's positions, subtitles and
// burns, progress with the next episode, share links and their reads, and libraries the administrator alone
// registers. A write on a member named in `refused` is refused 403 `forbidden`, as one on another account's
// organization member is.

export interface StandIn {
  url: string;
  role: "administrator" | "member";
  titles: Title[];
  episodes: Episode[];
  playlists: Playlist[];
  subtitles: Subtitle[];
  burns: Burn[];
  positions: Map<number, Position>;
  shares: Share[];
  libraries: Library[];
  /** Members a write on is refused, as `kind:id`. */
  refused: Set<string>;
  /** Every call made, as `METHOD path?query`. */
  calls: string[];
  /** The bodies of the writes made, by `METHOD path`. */
  bodies: Map<string, unknown>;
  /** The text parts of every multipart body sent, by path. */
  forms: Map<string, Record<string, string>>;
  stop: () => void;
  call: Call;
  send: Send;
}

export function title(id: number, realm: Realm, name: string, extra: Partial<Title> = {}): Title {
  return { id, realm, name, kind: "show", description: "", labels: {}, libraryId: null, episodeCount: 0, hasPoster: false, creator: "2", createdAt: 1, changedAt: 1, ...extra };
}

export function episode(id: number, titleId: number, realm: Realm, extra: Partial<Episode> = {}): Episode {
  return {
    id,
    titleId,
    realm,
    kind: "episode",
    season: 1,
    number: id,
    name: `Pilot ${id}`,
    label: "",
    fileName: `pilot-${id}.mp4`,
    mediaType: "video/mp4",
    byteCount: 1000,
    durationMs: 100_000,
    width: 1920,
    height: 1080,
    videoCodec: "h264",
    audioCodec: "aac",
    available: true,
    hasPoster: false,
    subtitleCount: 0,
    burnedFrom: null,
    position: null,
    creator: "2",
    createdAt: 1,
    changedAt: 1,
    ...extra
  };
}

export function playlist(id: number, realm: Realm, name: string, episodeIds: number[]): Playlist {
  return { id, realm, name, episodeIds, durationMs: 0, creator: "2", createdAt: 1, changedAt: 1 };
}

function refusal(status: number, code: string, error: string): Response {
  return Response.json({ code, error }, { status });
}

// The episode to watch next, as docs/design.md 「Playing」 states it.
function progressOf(ids: number[], episodes: Episode[], positions: Map<number, Position>): Progress {
  const items = ids.map((id) => {
    const found = episodes.find((one) => one.id === id);
    const position = positions.get(id);

    return { episodeId: id, durationMs: found?.durationMs ?? null, seconds: position?.seconds ?? 0, done: position?.done ?? false };
  });
  const played = [...positions.values()].filter((one) => ids.includes(one.episodeId)).sort((a, b) => b.updatedAt - a.updatedAt)[0];
  let next: number | null = null;

  if (!played) next = ids[0] ?? null;
  else if (!played.done) next = played.episodeId;
  else {
    const at = ids.indexOf(played.episodeId);

    next = items.slice(at + 1).find((item) => !item.done)?.episodeId ?? items.find((item) => !item.done)?.episodeId ?? null;
  }

  const nextSeconds = next === null ? 0 : positions.get(next)?.done ? 0 : (positions.get(next)?.seconds ?? 0);

  return { items, doneCount: items.filter((item) => item.done).length, next, nextSeconds };
}

export function startStandIn(seed: Partial<Pick<StandIn, "titles" | "episodes" | "playlists" | "subtitles" | "shares" | "libraries" | "role">> = {}): StandIn {
  const state = {
    role: seed.role ?? "member",
    titles: [...(seed.titles ?? [])],
    episodes: [...(seed.episodes ?? [])],
    playlists: [...(seed.playlists ?? [])],
    subtitles: [...(seed.subtitles ?? [])],
    burns: [] as Burn[],
    positions: new Map<number, Position>(),
    shares: [...(seed.shares ?? [])],
    libraries: [...(seed.libraries ?? [])],
    refused: new Set<string>(),
    calls: [] as string[],
    bodies: new Map<string, unknown>(),
    forms: new Map<string, Record<string, string>>()
  };
  let ids = 1000;
  let clock = 10;

  function list<T>(items: T[]): Response {
    return Response.json({ items, more: false });
  }

  function guard(kind: string, id: number): Response | null {
    return state.refused.has(`${kind}:${id}`) ? refusal(403, "forbidden", "Not yours to change.") : null;
  }

  function readable(ids: number[], realm: Realm): Response | null {
    for (const id of ids) {
      const found = state.episodes.find((one) => one.id === id);

      if (!found) return refusal(422, "unknown-episode", "No such episode.");
      if (realm === "organization" && found.realm === "mine") return refusal(422, "not-in-realm", "Not every reader reads it.");
    }

    return null;
  }

  function live(token: string): Share | Response {
    const share = state.shares.find((one) => one.token === token);

    if (!share) return refusal(404, "not-found", "No such link.");
    if (share.expiresAt !== null && share.expiresAt <= Date.now()) return refusal(410, "gone", "The link has ended.");

    return share;
  }

  function opened(share: Share): Episode[] {
    const order = share.episodeId !== null ? [share.episodeId] : (state.playlists.find((one) => one.id === share.playlistId)?.episodeIds ?? []);

    return order.flatMap((id) => state.episodes.filter((one) => one.id === id)).map((one) => ({ ...one, position: null }));
  }

  const server = Bun.serve({
    port: 0,
    async fetch(request) {
      const url = new URL(request.url);
      const parts = url.pathname.split("/").filter(Boolean).map(decodeURIComponent);
      const method = request.method;
      const [head, second, third, fourth, fifth] = parts;
      const id = Number(second);

      state.calls.push(`${method} ${url.pathname}${url.search}`);

      let body: Record<string, unknown> = {};
      let form: FormData | null = null;

      if ((request.headers.get("content-type") ?? "").startsWith("multipart/form-data")) {
        form = await request.formData();

        const fields: Record<string, string> = {};

        form.forEach((value, name) => {
          fields[name] = typeof value === "string" ? value : `file:${(value as File).name}`;
        });
        state.forms.set(url.pathname, fields);
      } else if (method === "POST" || method === "PATCH" || method === "PUT") {
        const text = await request.text();

        body = text ? (JSON.parse(text) as Record<string, unknown>) : {};
        state.bodies.set(`${method} ${url.pathname}`, body);
      }

      // ---- Titles
      if (head === "titles" && parts.length === 1 && method === "GET") return list(state.titles.filter((one) => one.realm === url.searchParams.get("realm")));
      if (head === "titles" && parts.length === 1 && method === "POST") {
        const made = title(++ids, body.realm as Realm, String(body.name), { kind: (body.kind as Title["kind"]) ?? "clip" });

        state.titles.push(made);

        return Response.json(made, { status: 201 });
      }
      if (head === "titles" && parts.length === 2) {
        const found = state.titles.find((one) => one.id === id);

        return found ? Response.json(found) : refusal(404, "not-found", "No such title.");
      }
      if (head === "titles" && third === "episodes" && method === "GET") return list(state.episodes.filter((one) => one.titleId === id).map((one) => ({ ...one, position: state.positions.get(one.id) ?? null })));
      if (head === "titles" && third === "episodes" && method === "POST") {
        const found = state.titles.find((one) => one.id === id);

        if (!found) return refusal(404, "not-found", "No such title.");

        const file = form?.get("file") as File | null;

        if (!file || !/\.(mp4|mkv|webm|mov|m4v|avi|ogv)$/.test(file.name)) return refusal(422, "not-a-video", "Not a video.");

        const filed = episode(++ids, id, found.realm, { kind: "clip", season: null, number: null, name: String(form?.get("name") ?? file.name.replace(/\.[^.]+$/, "")), fileName: file.name });

        state.episodes.push(filed);

        return Response.json(filed, { status: 201 });
      }
      if (head === "titles" && third === "progress") return Response.json(progressOf(state.episodes.filter((one) => one.titleId === id).map((one) => one.id), state.episodes, state.positions));

      // ---- Episodes, positions, subtitles and burns
      if (head === "episodes") {
        const found = state.episodes.find((one) => one.id === id);

        if (!found) return refusal(404, "not-found", "No such episode.");
        if (parts.length === 2 && method === "GET") return Response.json({ ...found, position: state.positions.get(id) ?? null });
        if (parts.length === 2 && method === "DELETE") {
          if (found.kind !== "clip") return refusal(409, "conflict", "A library's episode goes with its file.");

          state.episodes = state.episodes.filter((one) => one.id !== id);

          return new Response(null, { status: 204 });
        }
        if (third === "position" && method === "PUT") {
          const seconds = Number(body.seconds);
          const held = state.positions.get(id);
          const done = body.done === false ? false : body.done === true || (held?.done ?? false) || (found.durationMs !== null && seconds >= (found.durationMs / 1000) * 0.9);
          const position = { episodeId: id, seconds, done, updatedAt: ++clock };

          state.positions.set(id, position);

          return Response.json(position);
        }
        if (third === "subtitles" && method === "GET") return list(state.subtitles.filter((one) => one.episodeId === id));
        if (third === "subtitles" && method === "POST") {
          const file = form?.get("file") as File | null;
          const format = String(form?.get("format") ?? file?.name.split(".").pop() ?? "");

          if (!["srt", "ass", "smi", "vtt"].includes(format)) return refusal(422, "unknown-format", "Not a subtitle format.");

          const added: Subtitle = { id: ++ids, episodeId: id, language: String(form?.get("language") ?? "und"), label: String(form?.get("label") ?? ""), format: format as Subtitle["format"], fileName: file?.name ?? "", cueCount: 3, creator: "2", createdAt: ++clock };

          state.subtitles.push(added);

          return Response.json(added, { status: 201 });
        }
        if (third === "burns" && method === "GET") return list(state.burns.filter((one) => one.episodeId === id));
        if (third === "burns" && method === "POST") {
          if (!state.subtitles.some((one) => one.id === body.subtitleId && one.episodeId === id)) return refusal(422, "unknown-subtitle", "Not this episode's subtitle.");

          const started: Burn = { id: ++ids, episodeId: id, subtitleId: Number(body.subtitleId), state: "running", clipId: null, error: null, creator: "2", startedAt: ++clock, endedAt: null };

          state.burns.push(started);

          return Response.json(started, { status: 201 });
        }
      }
      if (head === "subtitles" && method === "DELETE") {
        const blocked = guard("subtitle", id);

        if (blocked) return blocked;

        state.subtitles = state.subtitles.filter((one) => one.id !== id);

        return new Response(null, { status: 204 });
      }

      // ---- Playlists
      if (head === "playlists" && parts.length === 1 && method === "GET") return list(state.playlists.filter((one) => one.realm === url.searchParams.get("realm")));
      if (head === "playlists" && parts.length === 1 && method === "POST") {
        const order = (body.episodeIds as number[] | undefined) ?? [];
        const wrong = readable(order, body.realm as Realm);

        if (wrong) return wrong;

        const made = playlist(++ids, body.realm as Realm, String(body.name), order);

        state.playlists.push(made);

        return Response.json(made, { status: 201 });
      }
      if (head === "playlists") {
        const at = state.playlists.findIndex((one) => one.id === id);

        if (at === -1) return refusal(404, "not-found", "No such playlist.");

        const held = state.playlists[at]!;

        if (parts.length === 2 && method === "GET") return Response.json(held);
        if (third === "progress") return Response.json(progressOf(held.episodeIds, state.episodes, state.positions));
        if (method !== "GET") {
          const blocked = guard("playlist", id);

          if (blocked) return blocked;
        }
        if (parts.length === 2 && method === "PATCH") {
          const order = (body.episodeIds as number[] | undefined) ?? held.episodeIds;
          const wrong = readable(order, held.realm);

          if (wrong) return wrong;

          state.playlists[at] = { ...held, name: (body.name as string | undefined) ?? held.name, episodeIds: order, changedAt: ++clock };

          return Response.json(state.playlists[at]);
        }
        if (third === "items" && method === "POST") {
          const added = body.episodeIds as number[];
          const wrong = readable(added, held.realm);

          if (wrong) return wrong;

          state.playlists[at] = { ...held, episodeIds: [...held.episodeIds, ...added], changedAt: ++clock };

          return Response.json(state.playlists[at], { status: 201 });
        }
        if (parts.length === 2 && method === "DELETE") {
          state.playlists.splice(at, 1);

          return new Response(null, { status: 204 });
        }
      }

      // ---- Shares and their reads
      if (head === "shares" && parts.length === 1 && method === "GET") {
        const episodeId = url.searchParams.get("episodeId");
        const playlistId = url.searchParams.get("playlistId");

        return list(state.shares.filter((one) => (episodeId !== null && one.episodeId === Number(episodeId)) || (playlistId !== null && one.playlistId === Number(playlistId))));
      }
      if (head === "shares" && parts.length === 1 && method === "POST") {
        const made: Share = { token: `token${++ids}`, episodeId: (body.episodeId as number) ?? null, playlistId: (body.playlistId as number) ?? null, url: null, creator: "2", createdAt: ++clock, expiresAt: (body.expiresAt as number) ?? null };

        state.shares.push(made);

        return Response.json(made, { status: 201 });
      }
      if (head === "shares" && second) {
        const share = live(second);

        if (share instanceof Response) return share;
        if (parts.length === 2 && method === "DELETE") {
          state.shares = state.shares.filter((one) => one.token !== second);

          return new Response(null, { status: 204 });
        }
        if (parts.length === 2) return Response.json(share);
        if (third === "playlist") {
          const found = state.playlists.find((one) => one.id === share.playlistId);

          return found ? Response.json(found) : refusal(404, "not-found", "Not a playlist link.");
        }
        if (third === "episodes" && parts.length === 3) return list(opened(share));
        if (third === "episodes" && fifth === "subtitles") {
          if (!opened(share).some((one) => one.id === Number(fourth))) return refusal(404, "not-found", "Not opened by the link.");

          return list(state.subtitles.filter((one) => one.episodeId === Number(fourth)));
        }
      }

      // ---- Libraries: the administrator's alone, refused before the body is read.
      if (head === "libraries" && parts.length === 1 && method === "GET") return list(state.libraries);
      if (head === "libraries" && method !== "GET" && state.role !== "administrator") return refusal(403, "forbidden", "The administrator's alone.");
      if (head === "libraries" && parts.length === 1 && method === "POST") {
        if (typeof body.path !== "string" || typeof body.realm !== "string") return refusal(400, "bad-request", "A library takes a path and a realm.");

        const made: Library = { id: ++ids, realm: body.realm as Realm, name: String(body.name ?? "Library"), path: body.path, creator: "2", createdAt: ++clock, scannedAt: null, scanning: true, titleCount: 0, episodeCount: 0 };

        state.libraries.push(made);

        return Response.json(made, { status: 201 });
      }

      return refusal(404, "no-route", "No such route.");
    }
  });

  const base = `http://127.0.0.1:${server.port}`;
  const fetcher = ofetch.create({ baseURL: base });
  const send: Send = async <T>(path: string, form: FormData, progress: (sent: number, total: number) => void) => {
    const answer = await fetch(`${base}${path}`, { method: "POST", body: form });
    const data = answer.status === 204 ? null : await answer.json();

    if (!answer.ok) throw { status: answer.status, data };

    progress(1, 1);

    return data as T;
  };

  return Object.assign(state, {
    url: base,
    stop: () => void server.stop(true),
    call: <T>(path: string, options?: Parameters<Call>[1]) => fetcher<T>(path, options as never) as Promise<T>,
    send
  }) as StandIn;
}

/** A call that never reaches anything: what a browser with no network answers. */
export const unreachable: Call = () => ofetch("http://127.0.0.1:1/");
