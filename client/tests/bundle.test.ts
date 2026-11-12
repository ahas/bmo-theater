import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { parse } from "vue/compiler-sfc";

import { PACKS, keysOf, word } from "./words";

// The bundle as a console finds it: the words every key the screens ask for in both locales
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

// An element of a screen's template as Vue's own compiler reads it: its tag, its attributes written as they stand,
// and the elements under it.
interface Element {
  tag: string;
  attributes: Map<string, string>;
  children: Element[];
}

interface TemplateNode {
  type: number;
  tag?: string;
  props?: { type: number; name: string; value?: { content: string }; arg?: { content: string }; exp?: { content: string } }[];
  children?: TemplateNode[];
}

const ELEMENT = 1;
const ATTRIBUTE = 6;

function elementOf(node: TemplateNode): Element {
  const attributes = new Map<string, string>();

  for (const prop of node.props ?? []) {
    if (prop.type === ATTRIBUTE) attributes.set(prop.name, prop.value?.content ?? "");
    else if (prop.arg) attributes.set(`:${prop.arg.content}`, prop.exp?.content ?? "");
  }

  return { tag: node.tag ?? "", attributes, children: (node.children ?? []).filter((child) => child.type === ELEMENT).map(elementOf) };
}

function templateOf(path: string): Element[] {
  const ast = parse(readFileSync(path, "utf8")).descriptor.template?.ast as TemplateNode | undefined;

  return ast ? elementOf(ast).children : [];
}

function within(elements: Element[]): Element[] {
  return elements.flatMap((element) => [element, ...within(element.children)]);
}

const templates = sources.filter((path) => path.endsWith(".vue")).map((path) => ({ path, elements: within(templateOf(path)) }));

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

    for (const path of [...sources, ...files(join(root, "tests")), join(root, "nuxt.config.ts")]) expect([path, korean.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
  });

  test("writes no motion length of its own: every movement takes the layer's shared ones", () => {
    for (const path of sources) expect([path, /\b(?:duration|transition)-\[?\d+m?s|\d+ms\b/.test(readFileSync(path, "utf8").replace(/_MS\s*=\s*[\d_]+/g, ""))]).toEqual([path, false]);
  });
});

describe("the bars", () => {
  test("lay nothing into the console's title bar: no screen reads its places or teleports into it", () => {
    for (const path of sources) expect([path, /screenActions|screenLead/.test(readFileSync(path, "utf8"))]).toEqual([path, false]);
    for (const { path, elements } of templates) expect([path, elements.some((element) => element.tag === "Teleport")]).toEqual([path, false]);
  });

  test("the theater's own bar holds its name and mark alone", () => {
    const bars = templates.flatMap(({ elements }) => elements.filter((element) => element.tag === "UDashboardNavbar"));

    expect(bars.length).toBeGreaterThan(0);
    for (const bar of bars) expect([...bar.attributes.keys(), bar.children.length]).toEqual([":title", "icon", 0]);
  });

  test("a screen's actions stand in its own row: the row of screens or its toolbar", () => {
    for (const { path, elements } of templates) {
      const rows = elements.filter((element) => element.tag === "ToolBar" || element.tag === "nav");
      const held = new Set(rows.flatMap((row) => within(row.children)));

      for (const group of elements.filter((element) => element.tag === "ActionGroup")) expect([path, held.has(group)]).toEqual([path, true]);
    }
  });

  test("a toolbar's way back leads it, and its buttons are the library's small ones at their own size", () => {
    const toolbars = templates.flatMap(({ path, elements }) => elements.filter((element) => element.tag === "ToolBar").map((toolbar) => ({ path, toolbar })));

    expect(toolbars.length).toBeGreaterThan(0);
    for (const { path, toolbar } of toolbars) {
      const controls = within(toolbar.children).filter((element) => element.tag === "UButton" || element.tag === "USelect");
      const back = controls.filter((control) => control.attributes.get("icon") === "i-material-symbols-arrow-back");

      expect([path, toolbar.attributes.has("class")]).toEqual([path, false]);
      for (const control of back) expect([path, within(toolbar.children[0] ? [toolbar.children[0]] : []).includes(control)]).toEqual([path, true]);
      for (const control of controls) {
        expect([path, control.tag, control.attributes.get("size")]).toEqual([path, control.tag, "sm"]);
        expect([path, control.tag, /(?:^|\s)(?:[\w:-]*:)?(?:h-|size-|p-0)/.test(control.attributes.get("class") ?? "")]).toEqual([path, control.tag, false]);
      }
    }
  });
});
