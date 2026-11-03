import { episodeTitle, lengthClock } from "./episodes";

import type { Watched } from "./episodes";
import type { Episode, Title } from "../../shared/types";

// What the screens say to a reader who does not see them: a title card's name, an episode's name with how far
// the viewer has come, a share of an episode in words.

type Translate = (key: string, params?: Record<string, unknown>) => string;

const KIND_WORDS: Record<Title["kind"], string> = {
  show: "theater.label.kind_show",
  film: "theater.label.kind_film",
  clip: "theater.label.kind_clip"
};

/** The words a title's kind is named by. */
export function kindWord(kind: Title["kind"]): string {
  return KIND_WORDS[kind];
}

/** A share from nought to one as a whole percentage, floored, so a hundred stands only once it is whole. */
export function percent(fraction: number): number {
  return Math.floor(Math.min(1, Math.max(0, fraction)) * 100);
}

/** A title card's name: the title, its kind and how many episodes it holds. */
export function titleName(t: Translate, title: Pick<Title, "name" | "kind" | "episodeCount">, countText: (n: number) => string = String): string {
  return [title.name, t(KIND_WORDS[title.kind]), t("theater.label.episodes_count", { n: countText(title.episodeCount) })].join(", ");
}

/** How far the viewer has come through an episode, in words: watched, a share of it, or not begun. */
export function watchedWords(t: Translate, watched: Watched): string {
  if (watched.done) return t("theater.label.watched");
  if (watched.fraction <= 0) return t("theater.label.unwatched");

  return t("theater.label.watched_share", { n: String(percent(watched.fraction)) });
}

// The label the service gives a burned clip names what it was burned from, in the service's own words.
const BURNED_LABEL = /^burned: /;

/** The words an episode's label is shown in: a burned clip's in the screen's language, any other as it stands. */
export function episodeLabel(t: Translate, episode: Pick<Episode, "label" | "burnedFrom">): string {
  return episode.burnedFrom && BURNED_LABEL.test(episode.label) ? t("theater.label.burned_from", { name: episode.label.replace(BURNED_LABEL, "") }) : episode.label;
}

/** An episode row's name: its code and name, its length, its label and how far the viewer has come. */
export function episodeName(t: Translate, episode: Pick<Episode, "season" | "number" | "name" | "durationMs" | "label" | "burnedFrom" | "available">, watched: Watched | null): string {
  const parts = [episodeTitle(episode)];
  const length = lengthClock(episode.durationMs);

  if (length) parts.push(length);
  if (episode.label) parts.push(episodeLabel(t, episode));
  if (!episode.available) parts.push(t("theater.label.unavailable"));
  if (watched) parts.push(watchedWords(t, watched));

  return parts.join(", ");
}
