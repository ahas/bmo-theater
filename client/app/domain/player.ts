import type { Subtitle } from "../../shared/types";

// What the theater decides around the layer's video player (bmo-ui's `docs/components.md`, "The video player"),
// which leaves these to its host: where an episode starts, when its position is set again, and which subtitle
// shows first and by what name.

/** The share of an episode past which it counts as watched (`docs/api.md`, 「Positions and progress」). */
export const DONE_SHARE = 0.9;

/** Where an episode starts playing: where the viewer stopped, unless it was watched or that is at its end. */
export function startAt(position: { seconds: number; done: boolean } | null, lengthSeconds: number | null): number {
  if (position === null || position.done) return 0;
  if (lengthSeconds !== null && position.seconds >= lengthSeconds * DONE_SHARE) return 0;

  return Math.max(0, position.seconds);
}

/** Whether a position is worth setting again: it moved by a second or more since the last one set. */
export function positionMoved(last: number | null, now: number): boolean {
  return last === null || Math.abs(now - last) >= 1;
}

/** A language tag's primary subtag, in lower case: `ko` of `ko-KR`. */
export function primaryLanguage(tag: string): string {
  return tag.trim().split(/[-_]/)[0]?.toLowerCase() ?? "";
}

/** The subtitle shown first: the first whose language is the screen's own, else none, so subtitles start off. */
export function firstTrack(subtitles: Pick<Subtitle, "id" | "language">[], uiLanguage: string): number | null {
  const wanted = primaryLanguage(uiLanguage);

  if (!wanted || wanted === "und") return null;

  return subtitles.find((subtitle) => primaryLanguage(subtitle.language) === wanted)?.id ?? null;
}

/** A subtitle's name in a list: its label, else its language in the reader's words, else its file's name. */
export function trackName(subtitle: Pick<Subtitle, "label" | "language" | "fileName">, uiLanguage: string): string {
  if (subtitle.label.trim()) return subtitle.label;

  if (subtitle.language && subtitle.language !== "und") {
    try {
      return new Intl.DisplayNames([uiLanguage], { type: "language" }).of(subtitle.language) ?? subtitle.language;
    } catch {
      return subtitle.language;
    }
  }

  return subtitle.fileName;
}
