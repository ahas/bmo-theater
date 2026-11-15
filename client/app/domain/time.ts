import { countText } from "bmo-ui/app/domain/numbers";

// How long a title runs in all, in the words the layer's pack says a span in.

type Translate = (key: string, params?: Record<string, unknown>) => string;

/** A running time in milliseconds as hours and minutes, or minutes alone under an hour; a minute at least. */
export function runningTime(t: Translate, ms: number): string {
  const minutes = Math.max(1, Math.round(ms / 60_000));
  const hours = Math.floor(minutes / 60);

  return hours > 0 ? t("common.label.hours_minutes", { h: countText(hours), m: String(minutes % 60) }) : t("common.label.minutes", { m: String(minutes) });
}
