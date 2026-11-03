import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { consoleManifest } from "bmo-ui/shared/console-manifest";

import { PACKS, keysOf, word } from "./words";

// The bundle as a console finds it: its declaration, the words every key the screens ask for in both locales
// under the bundle's namespace, the words the service's share head and sidebar read, and no Korean anywhere in
// the code.

const root = new URL("..", import.meta.url).pathname;

function files(folder: string): string[] {
  return readdirSync(folder).flatMap((name) => {
    const path = join(folder, name);

    return statSync(path).isDirectory() ? files(path) : [path];
  });
}

const sources = [...files(join(root, "app")), ...files(join(root, "shared")), ...files(join(root, "console"))].filter((path) => /\.(ts|vue|css)$/.test(path));

describe("the declaration", () => {
  const declared = JSON.parse(readFileSync(join(root, "console-bundle.json"), "utf8")) as Record<string, unknown>;

  test("names the bundle, its screen, its stream and its share head", () => {
    expect(declared).toMatchObject({ name: "theater", title: "theater.title", screenPath: "/theater", serviceName: "theater", events: true, shareHead: "/shares/{token}/head" });
  });

  test("reads as a manifest by the contract, its title a word of the bundle", () => {
    const { languagePack: _pack, ...declaration } = declared;
    const manifest = consoleManifest({ ...declaration, bundle: "/console/bundle.js", words: "/console/words", role: "member", ownerKey: true });

    expect(manifest.name).toBe("theater");
    expect(word(PACKS.ko, manifest.title)).toBeString();
  });
});

describe("the words", () => {
  test("both locales hold the same keys, all under the bundle's namespace", () => {
    expect(keysOf(PACKS.en).sort()).toEqual(keysOf(PACKS.ko).sort());
    expect(keysOf(PACKS.ko).every((key) => key.startsWith("theater."))).toBe(true);
  });

  test("every key the screens ask for is held", () => {
    const asked = new Set(sources.flatMap((path) => [...readFileSync(path, "utf8").matchAll(/["'`](theater\.(?:label|action|aria|heading|status|message|error|question)\.[a-z0-9_.]+)["'`]/g)].map((match) => match[1]!)));

    expect(asked.size).toBeGreaterThan(100);

    for (const key of asked) expect([key, word(PACKS.ko, key)]).toEqual([key, expect.any(String)]);
  });

  test("the sidebar's name and the share head's locale and site name stand in both locales", () => {
    for (const key of ["theater.title", "theater.share.locale", "theater.share.siteName"]) {
      expect([key, typeof word(PACKS.ko, key), typeof word(PACKS.en, key)]).toEqual([key, "string", "string"]);
    }

    expect(word(PACKS.en, "theater.share.locale")).toMatch(/^[a-z]{2}_[A-Z]{2}$/);
    expect(word(PACKS.ko, "theater.share.locale")).toMatch(/^[a-z]{2}_[A-Z]{2}$/);
  });

  test("no word carries a mark the translator would read as its own syntax", () => {
    for (const key of keysOf(PACKS.ko)) expect([key, /[@|$]/.test(word(PACKS.ko, key)! + word(PACKS.en, key)!)]).toEqual([key, false]);
  });
});

describe("the code", () => {
  test("holds no Korean: the words stand in the pack", () => {
    const korean = new RegExp(`[${String.fromCharCode(0x1100)}-${String.fromCharCode(0x11ff)}${String.fromCharCode(0x3131)}-${String.fromCharCode(0x318e)}${String.fromCharCode(0xac00)}-${String.fromCharCode(0xd7a3)}]`);

    for (const path of [...sources, ...files(join(root, "tests")), join(root, "nuxt.config.ts"), join(root, "console-bundle.json")]) expect([path, korean.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
  });

  test("writes no motion length of its own: every movement takes the layer's shared ones", () => {
    for (const path of sources) expect([path, /\b(?:duration|transition)-\[?\d+m?s|\d+ms\b/.test(readFileSync(path, "utf8").replace(/_MS\s*=\s*[\d_]+/g, ""))]).toEqual([path, false]);
  });
});
