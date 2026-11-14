import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdtempSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { chromium, devices } from "playwright-core";

import type { Browser, BrowserContext, CDPSession, Page } from "playwright-core";
import type { StageCall } from "./stage-harness";

// The theater's player stage (`docs/design.md`, 「The console」) driven by a real Chromium at a desktop's width and a
// phone's, in the light and the dark mode: the layer's one video player filling the stage a screen sizes, a skeleton
// while the episode is read and the reason where it cannot play, what the player tells handed to the screen by the
// episode's number, and a finger's tap on the stage bringing and hiding the controls. How the player itself behaves
// is the layer's, pinned by its own tests. The page's media elements keep their own clock, so nothing is fetched or
// played, and the timers the player waits on run on the browser's installed clock. Skipped where the browser build
// is not on the machine.

const HOOK_MS = 60_000;
// How long one test is given: a browser context opened while a whole suite holds the cores takes seconds to come up.
const TEST_MS = 30_000;
const ROOT = new URL("../../", import.meta.url).pathname;
const ORIGIN = "http://page.test";
const PAGE = `<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<link rel="stylesheet" href="/stage-harness.css"><script src="/stage-harness.js" defer></script></head><body><div id="page"></div></body></html>`;

// What the harness says of its media and of the episode on its playing stage.
const LENGTH_SECONDS = 100;
const EPISODE = 7;

const PLAYING = "[data-stage='playing']";
const READING = "[data-stage='reading']";
const FAILED = "[data-stage='failed']";
const PLAYER = "[data-video-player]";
const CONTROLS = "[data-video-controls]";

type Width = "desktop" | "mobile";
type Scheme = "light" | "dark";

const WIDTHS: Width[] = ["desktop", "mobile"];
const SCHEMES: Scheme[] = ["light", "dark"];

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const ready = browserReady();

let out = "";
let script = "";
// The components' own scoped sheets, which the build writes beside the script.
let sheet = "";
let browser: Browser;

beforeAll(async () => {
  if (!ready) return;

  out = mkdtempSync(join(tmpdir(), "theater-stage-"));

  const built = Bun.spawnSync(["bun", "x", "vite", "build", "--configLoader", "runner", "--config", "tests/browser/stage-harness.config.ts", "--outDir", out, "--logLevel", "error"], {
    cwd: ROOT,
    env: { ...process.env, NODE_ENV: "production", VITE_CONFIG_NATIVE_IGNORE_WARNING: "true" }
  });

  if (built.exitCode !== 0) throw new Error(`the harness build failed: ${built.stderr.toString()}`);

  script = await Bun.file(join(out, "stage-harness.js")).text();
  sheet = (await Promise.all(readdirSync(out).filter((name) => name.endsWith(".css")).map((name) => Bun.file(join(out, name)).text()))).join("\n");
  browser = await chromium.launch();
}, HOOK_MS);

afterAll(async () => {
  await browser?.close();

  if (out !== "") rmSync(out, { recursive: true, force: true });
}, HOOK_MS);

interface Session {
  context: BrowserContext;
  page: Page;
  cdp: CDPSession;
}

// The page at one width and in one mode, the browser's clock installed before anything runs. The phone is one of
// the width the screens are designed against, 360px in portrait.
async function session(width: Width, scheme: Scheme): Promise<Session> {
  const context = await browser.newContext(width === "desktop" ? { viewport: { width: 1280, height: 800 } } : { ...devices["Galaxy S24"] });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);

  await page.route(`${ORIGIN}/**`, (route) => {
    const path = new URL(route.request().url()).pathname;

    if (path === "/stage-harness.js") return route.fulfill({ contentType: "text/javascript", body: script });
    if (path === "/stage-harness.css") return route.fulfill({ contentType: "text/css", body: sheet });
    if (path === "/") return route.fulfill({ contentType: "text/html; charset=utf-8", body: PAGE });

    return route.fulfill({ status: 404, body: "" });
  });

  await page.clock.install();
  await page.goto(ORIGIN);
  await page.evaluate((dark) => document.documentElement.classList.toggle("dark", dark), scheme === "dark");
  await page.locator(PLAYER).first().waitFor();

  return { context, page, cdp };
}

interface Box {
  x: number;
  y: number;
  width: number;
  height: number;
}

async function box(page: Page, selector: string): Promise<Box> {
  const found = await page.locator(selector).first().boundingBox();

  if (found === null) throw new Error(`${selector} is not on the screen`);

  return found;
}

async function ground(page: Page, selector: string): Promise<string> {
  return page.$eval(selector, (element) => getComputedStyle(element).backgroundColor);
}

async function controlsShown(page: Page): Promise<boolean> {
  return (await page.$eval(`${PLAYING} ${CONTROLS}`, (element) => element.getAttribute("aria-hidden"))) === null;
}

async function calls(page: Page, name: string): Promise<unknown[][]> {
  return (await page.evaluate(() => window.stageCalls)).filter((call: StageCall) => call.name === name).map((call: StageCall) => call.args);
}

async function tap(cdp: CDPSession, at: { x: number; y: number }): Promise<void> {
  const point = { x: Math.round(at.x), y: Math.round(at.y), radiusX: 6, radiusY: 6, force: 1, id: 1 };

  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [point] });
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
}

describe.skipIf(!ready)("the player stage", () => {
  for (const width of WIDTHS) {
    for (const scheme of SCHEMES) {
      test(`draws the layer's player over its whole black stage, a skeleton while the episode is read and why where it cannot, at ${width} width in ${scheme}`, async () => {
        const { context, page } = await session(width, scheme);
        const stage = await box(page, PLAYING);
        const player = await box(page, `${PLAYING} ${PLAYER}`);

        expect(await page.locator(PLAYER).count()).toBe(1);
        expect(stage.width).toBeGreaterThan(0);
        expect([player.x, player.y, player.width, player.height]).toEqual([stage.x, stage.y, stage.width, stage.height]);
        expect([await ground(page, PLAYING), await ground(page, `${PLAYING} ${PLAYER}`)]).toEqual(["rgb(0, 0, 0)", "rgb(0, 0, 0)"]);
        expect(await page.getAttribute(`${PLAYING} ${PLAYER}`, "aria-label")).toBe("Harbour, episode 1 player");

        const controls = await page.$$eval(`${PLAYING} ${CONTROLS} button`, (buttons) => buttons.map((button) => button.getBoundingClientRect()).map((at) => ({ left: at.left, right: at.right })));

        expect(controls.length).toBeGreaterThan(0);
        expect(controls.filter((at) => at.left < stage.x || at.right > stage.x + stage.width)).toEqual([]);

        expect(await page.locator(`${READING} [role='status']`).getAttribute("aria-label")).toBe("Loading the video");
        expect(await ground(page, READING)).toBe("rgb(0, 0, 0)");

        expect(await page.locator(`${FAILED} [role='alert']`).textContent()).toBe("This episode cannot be found");
        expect(await page.locator(`${FAILED} ${PLAYER}`).count()).toBe(0);
        await context.close();
      }, TEST_MS);
    }
  }

  test("hands its screen the position, the end, the next episode and the subtitle, each by the episode's number", async () => {
    const { context, page } = await session("desktop", "light");

    await page.click(`${PLAYING} [data-video-start]`);
    await page.clock.runFor(5_100);
    expect(await calls(page, "position")).toEqual([[EPISODE, 0]]);

    await page.keyboard.press("c");
    expect(await calls(page, "track")).toEqual([[3]]);

    await page.click(`${PLAYING} button[aria-label='Next: Harbour, episode 2']`);
    expect(await calls(page, "next")).toEqual([[]]);

    await page.$eval(`${PLAYING} video`, (video) => video.dispatchEvent(new Event("ended")));
    expect(await calls(page, "ended")).toEqual([[EPISODE, LENGTH_SECONDS]]);
    await context.close();
  }, TEST_MS);

  test("keeps the controls a finger's tap on the stage brought, and hides them on the next tap", async () => {
    const { context, page, cdp } = await session("mobile", "light");
    const picture = await box(page, `${PLAYING} video`);
    const middle = { x: picture.x + picture.width / 2, y: picture.y + picture.height * 0.3 };

    await page.tap(`${PLAYING} [data-video-start]`);
    await page.clock.runFor(2_600);
    expect(await controlsShown(page)).toBe(false);

    await tap(cdp, middle);
    await page.clock.runFor(350);
    expect(await controlsShown(page)).toBe(true);
    await page.clock.runFor(1_000);
    expect(await controlsShown(page)).toBe(true);

    await tap(cdp, middle);
    await page.clock.runFor(350);
    expect(await controlsShown(page)).toBe(false);
    await context.close();
  }, TEST_MS);
});
