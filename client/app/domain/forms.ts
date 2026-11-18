import type { Playlist, Realm, Title } from "../../shared/types";

// What the screens' forms send and offer: a file with the text parts the service reads beside it, how far a
// sending has come, and the targets a form's select offers.

/** A multipart body: the part `file` under the file's own name, and each text part given that is not blank. */
export function fileForm(file: File, parts: Record<string, string | number | null | undefined>): FormData {
  const form = new FormData();

  form.append("file", file, file.name);

  for (const [name, value] of Object.entries(parts)) {
    if (value === null || value === undefined) continue;

    const text = String(value).trim();

    if (text) form.append(name, text);
  }

  return form;
}

/** A file's name without its extension, as the service names an episode where no name is given. */
export function bareName(fileName: string): string {
  const dot = fileName.lastIndexOf(".");

  return dot > 0 ? fileName.slice(0, dot) : fileName;
}

/** A size in bytes in megabytes, to one place. */
export function megabytes(bytes: number, locale?: string): string {
  return (bytes / 1_000_000).toLocaleString(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
}

/** How far a sending has come: `12.3/450.0 MB 2%`, the share floored so a hundred stands only once all is sent. */
export function sentText(sent: number, total: number, locale?: string): string {
  const share = total > 0 ? Math.floor((Math.min(sent, total) / total) * 100) : 0;

  return `${megabytes(sent, locale)}/${megabytes(total, locale)} MB ${share}%`;
}

/** Whether a name is one the service takes: not blank and on one line. */
export function nameOk(name: string): boolean {
  return name.trim() !== "" && !/[\r\n]/.test(name);
}

/** The titles a video may be filed under: the realm's, by name. */
export function fileTargets(titles: Title[]): Title[] {
  return [...titles].sort((a, b) => a.name.localeCompare(b.name));
}

type Translate = (key: string, params?: Record<string, unknown>) => string;

/** A select's choice: a group's heading, or a value picked. */
export type Choice = { type: "label"; label: string } | { label: string; value: string };

/** The value of the playlist choice that makes a new one. */
export const NEW_PLAYLIST = "new";

/** The realms whose playlists may hold an episode of a realm: either for an organization episode, the reader's own otherwise. */
export function playlistRealms(episodeRealm: Realm): Realm[] {
  return episodeRealm === "organization" ? ["mine", "organization"] : ["mine"];
}

/**
 * The playlists an episode may go into, as a select offers them: a new one in a group of its own, then each
 * realm's playlists in a group of theirs. A realm is named over its group only where both are offered, since one
 * realm alone is the one the reader is in.
 */
export function playlistChoices(t: Translate, realms: Realm[], held: Record<Realm, Playlist[]>): Choice[][] {
  const realmWord = (realm: Realm) => t(realm === "mine" ? "theater.label.mine" : "theater.label.organization");

  return [
    [{ label: t("theater.label.new_playlist"), value: NEW_PLAYLIST }],
    ...realms
      .filter((realm) => held[realm].length > 0)
      .map((realm) => [
        ...(realms.length > 1 ? [{ type: "label" as const, label: realmWord(realm) }] : []),
        ...held[realm].map((playlist) => ({ label: playlist.name, value: String(playlist.id) }))
      ])
  ];
}
