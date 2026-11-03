import { computed, ref } from "vue";

import { Refusal, Unreachable } from "./api";
import { moved, removedAt } from "./order";

import type { TheaterApi } from "./api";
import type { Burn, Episode, Library, Playlist, Position, Progress, Realm, Share, StreamEvent, Subtitle, Title } from "../../shared/types";

// What the screens hold of the theater service: the titles and playlists of both realms, the episodes, the
// progress and the subtitles of what has been opened, the burns of what is being burned, and the libraries.
//
// A list is read once and kept, and the service's event stream keeps it current: a member made, changed or
// deleted anywhere is put in or taken out where it is held. A change of a playlist's order is drawn at once and
// sent behind it; a refusal takes it back and says why.

/** A word the store has for the reader, as a key of the bundle's words and what fills it. */
export interface Notice {
  tone: "info" | "success" | "warning" | "error";
  title: string;
  params?: Record<string, string>;
}

export interface StoreOptions {
  api: TheaterApi;
  tell: (notice: Notice) => void;
}

// The words a refusal is told in, by the service's code; any other is told as a change that failed.
const REFUSAL_WORDS: Record<string, string> = {
  forbidden: "theater.error.forbidden",
  "not-found": "theater.error.not_found",
  conflict: "theater.error.conflict",
  gone: "theater.error.gone",
  "not-a-folder": "theater.error.not_a_folder",
  "not-a-video": "theater.error.not_a_video",
  "not-a-file": "theater.error.not_a_file",
  "unknown-format": "theater.error.unknown_format",
  "no-cues": "theater.error.no_cues",
  "unknown-subtitle": "theater.error.unknown_subtitle",
  "unknown-episode": "theater.error.unknown_episode",
  "not-in-realm": "theater.error.not_in_realm",
  "bad-request": "theater.error.bad_request"
};

/** The key of the words a failure is told in. */
export function refusalWord(error: unknown): string {
  if (error instanceof Unreachable) return "theater.error.unreachable";

  return error instanceof Refusal ? (REFUSAL_WORDS[error.code] ?? "theater.error.change_failed") : "theater.error.change_failed";
}

function upsert<T extends { id: number }>(list: T[], item: T): T[] {
  const at = list.findIndex((one) => one.id === item.id);

  return at === -1 ? [...list, item].sort((a, b) => a.id - b.id) : list.map((one) => (one.id === item.id ? item : one));
}

function without<T extends { id: number }>(list: T[], id: number): T[] {
  return list.filter((one) => one.id !== id);
}

/** The key a progress is held under. */
export function progressKey(of: "title" | "playlist", id: number): string {
  return `${of}:${id}`;
}

export function createTheaterStore(options: StoreOptions) {
  const { api, tell } = options;

  const titles = ref<Record<Realm, Title[]>>({ mine: [], organization: [] });
  const titlesRead = ref<Record<Realm, boolean>>({ mine: false, organization: false });
  const titlesFailed = ref<Record<Realm, boolean>>({ mine: false, organization: false });
  // Titles opened by their id, where no list of them has been read.
  const opened = ref<Record<number, Title>>({});
  const episodes = ref<Record<number, Episode[]>>({});
  // Episodes read by their id alone, under no title's list.
  const loose = ref<Record<number, Episode>>({});
  const progress = ref<Record<string, Progress>>({});
  const playlists = ref<Record<Realm, Playlist[]>>({ mine: [], organization: [] });
  const playlistsRead = ref<Record<Realm, boolean>>({ mine: false, organization: false });
  const playlistsFailed = ref<Record<Realm, boolean>>({ mine: false, organization: false });
  const subtitles = ref<Record<number, Subtitle[]>>({});
  const burns = ref<Record<number, Burn[]>>({});
  const libraries = ref<Library[]>([]);
  const librariesRead = ref(false);
  const librariesFailed = ref(false);
  const administrator = ref<boolean | null>(null);

  const allTitles = computed(() => [...titles.value.mine, ...titles.value.organization]);
  const allPlaylists = computed(() => [...playlists.value.mine, ...playlists.value.organization]);

  function titleOf(id: number): Title | undefined {
    return allTitles.value.find((one) => one.id === id) ?? opened.value[id];
  }

  function playlistOf(id: number): Playlist | undefined {
    return allPlaylists.value.find((one) => one.id === id);
  }

  /** An episode held under any title read. */
  function episodeOf(id: number): Episode | undefined {
    for (const list of Object.values(episodes.value)) {
      const found = list.find((one) => one.id === id);

      if (found) return found;
    }

    return loose.value[id];
  }

  function failWith(error: unknown): void {
    tell({ tone: "error", title: refusalWord(error) });
  }

  // ---- Titles and episodes ------------------------------------------------------------------------

  async function loadTitles(realm: Realm): Promise<void> {
    try {
      const read = await api.titles(realm);

      titles.value = { ...titles.value, [realm]: read };
      titlesFailed.value = { ...titlesFailed.value, [realm]: false };
    } catch {
      titlesFailed.value = { ...titlesFailed.value, [realm]: true };
    } finally {
      titlesRead.value = { ...titlesRead.value, [realm]: true };
    }
  }

  function putTitle(title: Title): void {
    const other: Realm = title.realm === "mine" ? "organization" : "mine";

    titles.value = { ...titles.value, [title.realm]: upsert(titles.value[title.realm], title), [other]: without(titles.value[other], title.id) };
    if (opened.value[title.id]) opened.value = { ...opened.value, [title.id]: title };
  }

  function dropTitle(id: number): void {
    titles.value = { mine: without(titles.value.mine, id), organization: without(titles.value.organization, id) };

    const { [id]: _gone, ...rest } = opened.value;

    opened.value = rest;
  }

  /** A title by its id, read where no list holds it; it throws where the service refuses it. */
  async function loadTitle(id: number): Promise<Title> {
    const title = await api.title(id);

    opened.value = { ...opened.value, [id]: title };

    return title;
  }

  async function loadEpisodes(titleId: number): Promise<Episode[]> {
    const list = await api.episodes(titleId);

    episodes.value = { ...episodes.value, [titleId]: list };

    return list;
  }

  /** An episode by its id, read and kept under its title. */
  async function loadEpisode(id: number): Promise<Episode> {
    const episode = await api.episode(id);

    putEpisode(episode);

    return episode;
  }

  // The reads by id on their way, so two asks for one episode are one call.
  const asking = new Map<number, Promise<void>>();

  /** The episodes of a list of ids, each read once where no title read holds it; one not read is left out. */
  async function loadEpisodesByIds(ids: number[]): Promise<void> {
    const AT_ONCE = 6;
    const wanted = [...new Set(ids)].filter((id) => !episodeOf(id));

    for (let at = 0; at < wanted.length; at += AT_ONCE) {
      await Promise.all(
        wanted.slice(at, at + AT_ONCE).map((id) => {
          const standing = asking.get(id);

          if (standing) return standing;

          const read = api
            .episode(id)
            .then(putEpisode, () => undefined)
            .finally(() => asking.delete(id));

          asking.set(id, read);

          return read;
        })
      );
    }
  }

  // An episode is held in its title's list where that list has been read, and on its own otherwise, so a list
  // read whole is never mistaken for one holding a single episode read by its id.
  function putEpisode(episode: Episode): void {
    const list = episodes.value[episode.titleId];
    const held = episodeOf(episode.id);
    const kept = { ...episode, position: episode.position ?? held?.position ?? null };

    if (list) {
      const at = list.findIndex((one) => one.id === episode.id);

      episodes.value = { ...episodes.value, [episode.titleId]: at === -1 ? [...list, kept] : list.map((one) => (one.id === episode.id ? kept : one)) };
    }

    if (!list || loose.value[episode.id]) loose.value = { ...loose.value, [episode.id]: kept };
  }

  function dropEpisode(id: number): void {
    const next: Record<number, Episode[]> = {};

    for (const [titleId, list] of Object.entries(episodes.value)) next[Number(titleId)] = without(list, id);

    episodes.value = next;

    const { [id]: _gone, ...rest } = loose.value;

    loose.value = rest;
  }

  async function loadProgress(of: "title" | "playlist", id: number): Promise<Progress | null> {
    try {
      const read = of === "title" ? await api.titleProgress(id) : await api.playlistProgress(id);

      progress.value = { ...progress.value, [progressKey(of, id)]: read };

      return read;
    } catch {
      return null;
    }
  }

  function progressOf(of: "title" | "playlist", id: number): Progress | null {
    return progress.value[progressKey(of, id)] ?? null;
  }

  async function deleteEpisode(episode: Episode): Promise<boolean> {
    try {
      await api.deleteEpisode(episode.id);
      dropEpisode(episode.id);

      return true;
    } catch (error) {
      failWith(error);

      return false;
    }
  }

  /**
   * Files a video under a title, made as a `clip` title in the realm where `title` is a new name. It throws
   * where the service refuses it, for the form that asked to say why.
   */
  async function file(target: { titleId: number } | { realm: Realm; name: string }, form: FormData, progressOf: (sent: number, total: number) => void): Promise<Episode> {
    let titleId: number;

    if ("titleId" in target) titleId = target.titleId;
    else {
      const made = await api.makeTitle(target.realm, target.name);

      putTitle(made);
      titleId = made.id;
    }

    const episode = await api.file(titleId, form, progressOf);

    putEpisode(episode);

    return episode;
  }

  // ---- The viewer's positions ---------------------------------------------------------------------

  function putPosition(position: Position): void {
    const held = episodeOf(position.episodeId);

    if (held) putEpisode({ ...held, position });

    // A progress holding the episode answers it at once; its next is read again behind it.
    const next: Record<string, Progress> = {};

    for (const [key, read] of Object.entries(progress.value)) {
      next[key] = read.items.some((item) => item.episodeId === position.episodeId)
        ? { ...read, items: read.items.map((item) => (item.episodeId === position.episodeId ? { ...item, seconds: position.seconds, done: position.done } : item)) }
        : read;
    }

    progress.value = next;
  }

  /** Sets the viewer's position, quietly: a position that did not land is set again by the next one. */
  async function setPosition(episodeId: number, seconds: number, done?: boolean): Promise<Position | null> {
    try {
      const position = await api.setPosition(episodeId, seconds, done);

      putPosition(position);

      return position;
    } catch {
      return null;
    }
  }

  // ---- Playlists ----------------------------------------------------------------------------------

  async function loadPlaylists(realm: Realm): Promise<void> {
    try {
      const read = await api.playlists(realm);

      playlists.value = { ...playlists.value, [realm]: read };
      playlistsFailed.value = { ...playlistsFailed.value, [realm]: false };
    } catch {
      playlistsFailed.value = { ...playlistsFailed.value, [realm]: true };
    } finally {
      playlistsRead.value = { ...playlistsRead.value, [realm]: true };
    }
  }

  function putPlaylist(playlist: Playlist): void {
    const other: Realm = playlist.realm === "mine" ? "organization" : "mine";

    playlists.value = { ...playlists.value, [playlist.realm]: upsert(playlists.value[playlist.realm], playlist), [other]: without(playlists.value[other], playlist.id) };
  }

  function dropPlaylist(id: number): void {
    playlists.value = { mine: without(playlists.value.mine, id), organization: without(playlists.value.organization, id) };
  }

  async function loadPlaylist(id: number): Promise<Playlist> {
    const playlist = await api.playlist(id);

    putPlaylist(playlist);

    return playlist;
  }

  /** Makes a playlist; it throws where the service refuses it, for the form that asked to say why. */
  async function makePlaylist(realm: Realm, name: string, episodeIds: number[] = []): Promise<Playlist> {
    const made = await api.makePlaylist(realm, name, episodeIds);

    putPlaylist(made);

    return made;
  }

  /** Renames a playlist; it throws where the service refuses it. */
  async function renamePlaylist(playlist: Playlist, name: string): Promise<Playlist> {
    const changed = await api.changePlaylist(playlist.id, { name });

    putPlaylist(changed);

    return changed;
  }

  // The order drawn at once and sent whole behind it; a refusal puts the order that stood back.
  async function sendOrder(playlist: Playlist, order: number[]): Promise<boolean> {
    const before = playlistOf(playlist.id) ?? playlist;

    putPlaylist({ ...before, episodeIds: order });

    try {
      putPlaylist(await api.changePlaylist(playlist.id, { episodeIds: order }));
      void loadProgress("playlist", playlist.id);

      return true;
    } catch (error) {
      putPlaylist(before);
      failWith(error);

      return false;
    }
  }

  function reorderPlaylist(playlist: Playlist, from: number, to: number): Promise<boolean> {
    return sendOrder(playlist, moved(playlist.episodeIds, from, to));
  }

  function removeFromPlaylist(playlist: Playlist, at: number): Promise<boolean> {
    return sendOrder(playlist, removedAt(playlist.episodeIds, at));
  }

  /** Adds episodes at a playlist's end; it throws where the service refuses it. */
  async function addToPlaylist(playlistId: number, episodeIds: number[]): Promise<Playlist> {
    const changed = await api.addToPlaylist(playlistId, episodeIds);

    putPlaylist(changed);

    return changed;
  }

  async function deletePlaylist(playlist: Playlist): Promise<boolean> {
    try {
      await api.deletePlaylist(playlist.id);
      dropPlaylist(playlist.id);

      return true;
    } catch (error) {
      failWith(error);

      return false;
    }
  }

  // ---- Subtitles and burns ------------------------------------------------------------------------

  async function loadSubtitles(episodeId: number): Promise<Subtitle[]> {
    const list = await api.subtitles(episodeId);

    subtitles.value = { ...subtitles.value, [episodeId]: list };

    return list;
  }

  function putSubtitle(subtitle: Subtitle): void {
    const list = subtitles.value[subtitle.episodeId];

    if (list) subtitles.value = { ...subtitles.value, [subtitle.episodeId]: upsert(list, subtitle) };
  }

  function dropSubtitle(id: number): void {
    const next: Record<number, Subtitle[]> = {};

    for (const [episodeId, list] of Object.entries(subtitles.value)) next[Number(episodeId)] = without(list, id);

    subtitles.value = next;
  }

  /** Adds a subtitle file to an episode; it throws where the service refuses it. */
  async function addSubtitle(episodeId: number, form: FormData, progressOf: (sent: number, total: number) => void = () => {}): Promise<Subtitle> {
    const added = await api.addSubtitle(episodeId, form, progressOf);

    subtitles.value = { ...subtitles.value, [episodeId]: upsert(subtitles.value[episodeId] ?? [], added) };

    return added;
  }

  async function deleteSubtitle(subtitle: Subtitle): Promise<boolean> {
    try {
      await api.deleteSubtitle(subtitle.id);
      dropSubtitle(subtitle.id);

      return true;
    } catch (error) {
      failWith(error);

      return false;
    }
  }

  async function loadBurns(episodeId: number): Promise<void> {
    try {
      const read = await api.burns(episodeId);

      burns.value = { ...burns.value, [episodeId]: read };
    } catch {
      // The burns are told again by the stream as they start and end.
    }
  }

  function putBurn(burn: Burn): void {
    const list = burns.value[burn.episodeId] ?? [];
    const at = list.findIndex((one) => one.id === burn.id);

    burns.value = { ...burns.value, [burn.episodeId]: at === -1 ? [burn, ...list] : list.map((one) => (one.id === burn.id ? burn : one)) };
  }

  /** The burn of a subtitle standing last, the running one first. */
  function burnOf(episodeId: number, subtitleId: number): Burn | null {
    const list = (burns.value[episodeId] ?? []).filter((one) => one.subtitleId === subtitleId);

    return list.find((one) => one.state === "running") ?? list.sort((a, b) => b.startedAt - a.startedAt)[0] ?? null;
  }

  async function burn(episodeId: number, subtitleId: number): Promise<Burn | null> {
    try {
      const started = await api.burn(episodeId, subtitleId);

      putBurn(started);

      return started;
    } catch (error) {
      failWith(error);

      return null;
    }
  }

  // ---- Libraries ----------------------------------------------------------------------------------

  async function checkAdministrator(): Promise<boolean> {
    if (administrator.value !== null) return administrator.value;

    try {
      administrator.value = await api.administers();
    } catch {
      return false;
    }

    return administrator.value;
  }

  async function loadLibraries(): Promise<void> {
    try {
      libraries.value = await api.libraries();
      librariesFailed.value = false;
    } catch {
      librariesFailed.value = true;
    } finally {
      librariesRead.value = true;
    }
  }

  function putLibrary(library: Library): void {
    libraries.value = upsert(libraries.value, library);
  }

  /** Registers a folder; it throws where the service refuses it. */
  async function makeLibrary(realm: Realm, path: string, name: string | null): Promise<Library> {
    const made = await api.makeLibrary(realm, path, name);

    putLibrary(made);

    return made;
  }

  /** Renames a library; it throws where the service refuses it. */
  async function renameLibrary(library: Library, name: string): Promise<Library> {
    const changed = await api.renameLibrary(library.id, name);

    putLibrary(changed);

    return changed;
  }

  async function scanLibrary(library: Library): Promise<boolean> {
    try {
      await api.scanLibrary(library.id);
      putLibrary({ ...library, scanning: true });

      return true;
    } catch (error) {
      failWith(error);

      return false;
    }
  }

  async function deleteLibrary(library: Library): Promise<boolean> {
    try {
      await api.deleteLibrary(library.id);
      libraries.value = without(libraries.value, library.id);

      return true;
    } catch (error) {
      failWith(error);

      return false;
    }
  }

  async function rereadLibrary(id: number): Promise<void> {
    try {
      putLibrary(await api.library(id));
    } catch {
      // A library no longer read is taken out by its own event.
    }
  }

  // ---- Shares -------------------------------------------------------------------------------------

  function shareRoad(member: { episodeId: number } | { playlistId: number }) {
    return {
      read: () => api.shareOf(member),
      make: (expiresAt: number | null) => api.makeShare(member, expiresAt),
      remove: (share: Pick<Share, "token">) => api.endShare(share.token)
    };
  }

  // ---- The event stream ---------------------------------------------------------------------------

  // The progress read for what an event changed, read again so its next stands as the service answers it.
  function rereadProgressHolding(episodeId: number): void {
    for (const [key, read] of Object.entries(progress.value)) {
      if (!read.items.some((item) => item.episodeId === episodeId)) continue;

      const [of, id] = key.split(":");

      void loadProgress(of as "title" | "playlist", Number(id));
    }
  }

  /** Puts what an event of the service's stream tells where it is held. */
  function apply(event: StreamEvent): void {
    const data = event.data as Record<string, unknown> & { id: number };

    switch (event.kind) {
      case "title.created":
      case "title.changed":
        putTitle(data as unknown as Title);
        break;
      case "title.deleted":
        dropTitle(data.id);
        break;
      case "episode.created":
      case "episode.changed": {
        const episode = data as unknown as Episode;

        if (episodes.value[episode.titleId] || loose.value[episode.id]) putEpisode(episode);
        if (event.kind === "episode.created") void refreshTitle(episode.titleId);
        break;
      }
      case "episode.deleted":
        dropEpisode(data.id);
        break;
      case "subtitle.created":
        putSubtitle(data as unknown as Subtitle);
        break;
      case "subtitle.deleted":
        dropSubtitle(data.id);
        break;
      case "playlist.created":
      case "playlist.changed":
        putPlaylist(data as unknown as Playlist);
        break;
      case "playlist.deleted":
        dropPlaylist(data.id);
        break;
      case "position.changed": {
        const position = data as unknown as Position;

        putPosition(position);
        rereadProgressHolding(position.episodeId);
        break;
      }
      case "burn.running":
      case "burn.finished":
      case "burn.failed":
        putBurn(data as unknown as Burn);
        break;
      case "library.created":
      case "library.changed":
        putLibrary(data as unknown as Library);
        break;
      case "library.deleted":
        libraries.value = without(libraries.value, data.id);
        break;
      case "scan.running":
      case "scan.finished":
      case "scan.failed":
        void rereadLibrary(Number(data.libraryId));
        break;
      default:
        break;
    }
  }

  // A title an episode was added under counts it, so the title is read again where it is held.
  async function refreshTitle(id: number): Promise<void> {
    if (!titleOf(id)) return;

    try {
      putTitle(await api.title(id));
    } catch {
      // A title no longer read is taken out by its own event.
    }
  }

  return {
    titles,
    titlesRead,
    titlesFailed,
    episodes,
    progress,
    playlists,
    playlistsRead,
    playlistsFailed,
    subtitles,
    burns,
    libraries,
    librariesRead,
    librariesFailed,
    administrator,
    titleOf,
    playlistOf,
    episodeOf,
    loadTitles,
    loadTitle,
    loadEpisodes,
    loadEpisode,
    loadEpisodesByIds,
    loadProgress,
    progressOf,
    deleteEpisode,
    file,
    setPosition,
    loadPlaylists,
    loadPlaylist,
    makePlaylist,
    renamePlaylist,
    reorderPlaylist,
    removeFromPlaylist,
    addToPlaylist,
    deletePlaylist,
    loadSubtitles,
    addSubtitle,
    deleteSubtitle,
    loadBurns,
    burnOf,
    burn,
    checkAdministrator,
    loadLibraries,
    makeLibrary,
    renameLibrary,
    scanLibrary,
    deleteLibrary,
    shareRoad,
    apply
  };
}

export type TheaterStore = ReturnType<typeof createTheaterStore>;
