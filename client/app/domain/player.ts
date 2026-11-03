import type { Subtitle } from "../../shared/types";

// How the player behaves, apart from the element it drives: the keys it answers, the speeds it offers, how far
// a skip goes, when it sets the viewer's position, where it starts, and which subtitle it shows first.

/** How far a skip goes, in seconds. */
export const SKIP_SECONDS = 10;

/** How often the position is set while the video plays (`docs/design.md`, 「The console」). */
export const SAVE_EVERY_MS = 5_000;

/** The share of an episode past which it counts as watched (`docs/api.md`, 「Positions and progress」). */
export const DONE_SHARE = 0.9;

/** How much a volume key moves the volume. */
export const VOLUME_STEP = 0.1;

/** The playback speeds offered, the normal one among them. */
export const SPEEDS = [0.5, 0.75, 1, 1.25, 1.5, 2] as const;

/** The line a subtitle stands on while the controls stand, counted up from the picture's foot, clear of them. */
export const CUE_LINE_OVER_CONTROLS = -4;

/** How long the controls stand after the pointer last moved over a playing video. */
export const CONTROLS_IDLE_MS = 2_500;

export type PlayerAction = "toggle" | "back" | "forward" | "louder" | "quieter" | "fullscreen" | "mute" | "captions";

/** The action a key stands for, or none: space, the arrows, f, m and c. */
export function keyAction(key: string): PlayerAction | null {
  switch (key) {
    case " ":
    case "k":
      return "toggle";
    case "ArrowLeft":
    case "j":
      return "back";
    case "ArrowRight":
    case "l":
      return "forward";
    case "ArrowUp":
      return "louder";
    case "ArrowDown":
      return "quieter";
    case "f":
    case "F":
      return "fullscreen";
    case "m":
    case "M":
      return "mute";
    case "c":
    case "C":
      return "captions";
    default:
      return null;
  }
}

/** Where a skip lands: never before the start, and never past the end where the end is known. */
export function skipTo(now: number, by: number, length: number | null): number {
  const to = Math.max(0, now + by);

  return length === null ? to : Math.min(length, to);
}

/** A volume moved by a step, held between nought and one. */
export function steppedVolume(level: number, by: number): number {
  return Math.round(Math.min(1, Math.max(0, level + by)) * 100) / 100;
}

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

/** A speed as the player names it: `1.25×`. */
export function speedText(speed: number): string {
  return `${speed}×`;
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

/** The subtitle the captions key turns to: the last one shown where none shows, else none. */
export function toggledTrack(shown: number | null, remembered: number | null, subtitles: Pick<Subtitle, "id">[]): number | null {
  if (shown !== null) return null;
  if (remembered !== null && subtitles.some((one) => one.id === remembered)) return remembered;

  return subtitles[0]?.id ?? null;
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
