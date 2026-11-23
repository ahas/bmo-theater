import { describe, expect, test } from "bun:test";
import { readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { parse } from "vue/compiler-sfc";

import { LOCALES, PACKS, keysOf, word } from "./words";

// The bundle as a console finds it: the words every key the screens ask for in every locale
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
  test("every locale holds the same keys in the same order, all under the bundle's namespace", () => {
    for (const locale of LOCALES) expect([locale, keysOf(PACKS[locale])]).toEqual([locale, keysOf(PACKS.en)]);
    expect(keysOf(PACKS.en).every((key) => key.startsWith("theater."))).toBe(true);
  });

  test("every locale's word holds the marks the English one does", () => {
    const marks = (text: string | undefined) => [...(text ?? "").matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();

    for (const locale of LOCALES) {
      for (const key of keysOf(PACKS.en)) expect([locale, key, marks(word(PACKS[locale], key))]).toEqual([locale, key, marks(word(PACKS.en, key))]);
    }
  });

  test("every key the screens ask for is held", () => {
    const asked = new Set(sources.flatMap((path) => [...readFileSync(path, "utf8").matchAll(/["'`](theater\.(?:label|action|aria|heading|status|message|error|question)\.[a-z0-9_.]+)["'`]/g)].map((match) => match[1]!)));

    expect(asked.size).toBeGreaterThan(100);

    for (const key of asked) expect([key, word(PACKS.ko, key)]).toEqual([key, expect.any(String)]);
  });

  test("the sidebar's name and the share head's locale and site name stand in every locale", () => {
    for (const locale of LOCALES) {
      for (const key of ["theater.title", "theater.share.locale", "theater.share.siteName"]) expect([locale, key, typeof word(PACKS[locale], key)]).toEqual([locale, key, "string"]);

      expect([locale, word(PACKS[locale], "theater.share.locale")]).toEqual([locale, expect.stringMatching(/^[a-z]{2}_[A-Z]{2}$/)]);
    }
  });

  test("no word carries a mark the translator would read as its own syntax", () => {
    for (const locale of LOCALES) {
      for (const key of keysOf(PACKS.en)) expect([locale, key, /[@|$]/.test(word(PACKS[locale], key)!)]).toEqual([locale, key, false]);
    }
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

  test("a screen makes from the layer's one create button, once, standing in its own row", () => {
    const creating = templates.filter(({ elements }) => elements.some((element) => element.tag === "CreateButton"));

    expect(creating.length).toBeGreaterThan(0);
    for (const { path, elements } of creating) {
      const rows = elements.filter((element) => element.tag === "ToolBar" || (element.tag === "template" && element.attributes.has(":create")));
      const held = new Set(rows.flatMap((row) => within(row.children)));
      const buttons = elements.filter((element) => element.tag === "CreateButton");

      expect([path, buttons.length]).toEqual([path, 1]);
      expect([path, held.has(buttons[0]!)]).toEqual([path, true]);
    }
  });

  test("a toolbar's way back leads it, and its buttons are the library's small ones at their own size", () => {
    const toolbars = templates.flatMap(({ path, elements }) => elements.filter((element) => element.tag === "ToolBar").map((toolbar) => ({ path, toolbar })));

    expect(toolbars.length).toBeGreaterThan(0);
    for (const { path, toolbar } of toolbars) {
      const controls = within(toolbar.children).filter((element) => element.tag === "UButton" || element.tag === "ChoiceSelect");
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

describe("the selects", () => {
  // The component library's own selects, which offer a search by no rule of the layer's.
  const BARE = new Set(["USelect", "USelectMenu", "UInputMenu"]);

  test("a value is chosen from a list in the layer's one select, never in the library's bare", () => {
    const chosen = templates.filter(({ elements }) => elements.some((element) => element.tag === "ChoiceSelect"));

    expect(chosen.length).toBeGreaterThan(0);
    for (const { path, elements } of templates) expect([path, elements.filter((element) => BARE.has(element.tag)).map((element) => element.tag)]).toEqual([path, []]);
  });
});
