import { kindWord } from "./a11y";

import type { Title, TitleKind } from "../../shared/types";

// The library's rows of one kind (`docs/design.md`, 「The console」): a row for each kind of title the realm holds,
// shows, films and clips in that order, each keeping the order the titles were read in.

/** One kind's row: its kind, the key of the words it is named by, and its titles. */
export interface KindRow {
  kind: TitleKind;
  name: string;
  titles: Title[];
}

/** The order the kinds' rows stand in. */
export const ROW_KINDS: readonly TitleKind[] = ["show", "film", "clip"];

/** The rows of the kinds a realm's titles hold, each in the order the titles were read in. */
export function kindRow(titles: Title[]): KindRow[] {
  return ROW_KINDS.map((kind) => ({ kind, name: kindWord(kind), titles: titles.filter((title) => title.kind === kind) })).filter((row) => row.titles.length > 0);
}
