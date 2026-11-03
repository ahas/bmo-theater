import type { TitleKind } from "../../shared/types";

// The marks the screens draw, each declared once.

/** The mark of a title's kind, drawn where it has no poster. */
export const KIND_ICONS: Record<TitleKind, string> = {
  show: "i-material-symbols-live-tv-outline",
  film: "i-material-symbols-movie-outline",
  clip: "i-material-symbols-video-file-outline"
};
