import en from "../i18n/en.json";
import ko from "../i18n/ko.json";

// The bundle's words as a test reads them: a key looked up in a locale's pack and its marks filled, the
// way the console's translator fills them.

export const PACKS = { ko, en } as const;

export function word(pack: unknown, key: string): string | undefined {
  let held: unknown = pack;

  for (const part of key.split(".")) {
    if (typeof held !== "object" || held === null) return undefined;

    held = (held as Record<string, unknown>)[part];
  }

  return typeof held === "string" ? held : undefined;
}

export function translator(locale: keyof typeof PACKS) {
  return (key: string, params: Record<string, unknown> = {}): string => {
    const text = word(PACKS[locale], key);

    if (text === undefined) throw new Error(`no word for ${key}`);

    return text.replaceAll(/\{(\w+)\}/g, (_match, name: string) => String(params[name] ?? ""));
  };
}

/** Every key a pack holds, written whole. */
export function keysOf(pack: unknown, prefix = ""): string[] {
  if (typeof pack !== "object" || pack === null) return [prefix];

  return Object.entries(pack as Record<string, unknown>).flatMap(([name, value]) => keysOf(value, prefix ? `${prefix}.${name}` : name));
}
