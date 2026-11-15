import { LANDSCAPE, PORTRAIT, rowShape } from "bmo-ui/app/domain/artwork";

import type { ArtworkShape, PictureSize } from "bmo-ui/app/domain/artwork";
import type { Episode, Title } from "../../shared/types";

// How the theater's cards stand (`docs/design.md`, 「The console」, the cards): the picture size the service
// answers for each, the one shape a row of them takes, and the width a card of a shape stands at.

/** A title's or an episode's poster size, where the service read it. */
export function posterSize(member: Pick<Title | Episode, "hasPoster" | "posterWidth" | "posterHeight">): PictureSize | null {
  return member.hasPoster && member.posterWidth && member.posterHeight ? { width: member.posterWidth, height: member.posterHeight } : null;
}

/** The shape a row of titles stands at: the one most of their posters have, portrait where none is known. */
export function titlesShape(titles: Title[]): ArtworkShape {
  return rowShape(titles.map(posterSize), PORTRAIT);
}

/** The shape a row of episodes stands at: the one most of their posters have, landscape where none is known. */
export function episodesShape(episodes: Episode[]): ArtworkShape {
  return rowShape(episodes.map(posterSize), LANDSCAPE);
}

/**
 * The width a card of a shape stands at, against the screen's card measure (`--theater-card`): the measure times
 * the root of the shape's ratio, so a card of any shape covers about the same room — a wide card shorter, a tall
 * one narrower.
 */
export function cardWidth(shape: ArtworkShape): string {
  return `calc(var(--theater-card) * ${Math.sqrt(shape.ratio).toFixed(3)})`;
}

/** The screen's card measure, wider from the small breakpoint up. */
export const CARD_MEASURE = "[--theater-card:9rem] sm:[--theater-card:11rem]";

/** How the component library's page card stands as one of the theater's cards: its picture, a name, a line under it. */
export const CARD_UI = {
  root: "min-w-0 rounded-lg select-none active:bg-elevated",
  container: "min-w-0 gap-y-2 p-2 sm:p-2",
  wrapper: "w-full min-w-0",
  header: "mb-2 w-full",
  body: "w-full min-w-0",
  title: "truncate text-base font-medium text-nowrap",
  description: "mt-0 truncate text-sm text-muted tabular-nums text-nowrap"
} as const;
