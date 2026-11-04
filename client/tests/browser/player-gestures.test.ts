import { afterAll, afterEach, beforeAll, describe, expect, test } from "bun:test";
import { existsSync } from "node:fs";
import { chromium, devices } from "playwright-core";

import type { Browser, BrowserContext, CDPSession, Page } from "playwright-core";
import type { Harness, HarnessState } from "./gesture-harness";
import type { PlayerCue } from "../../app/domain/gestures";

// A real finger on the player's picture, driven through the browser's own input pipeline: a tap that is a tap and
// not a double tap, a double tap on a half, a drag that moves the clock as it travels, a vertical drag that sets
// the brightness on the left and the volume on the right, a held finger that plays faster, and a dialog standing
// over the picture that no gesture reaches through. The picture's clock, volume and speed are the harness's own;
// the touches and the browser's reading of them are real.

const HOOK_MS = 60_000;

function ready(): boolean {
  try {
    return existsSync(chromium.executablePath());
  } catch {
    return false;
  }
}

const available = ready();
const built = available ? await Bun.build({ entrypoints: [new URL("./gesture-harness.ts", import.meta.url).pathname], target: "browser" }) : null;
const script = built === null ? "" : (await Promise.all(built.outputs.map((output) => output.text()))).join("\n");

interface Point {
  x: number;
  y: number;
}

type PlayerWindow = Window & { mountPlayer?: () => void; player?: Harness };

let browser: Browser | null = null;
let context: BrowserContext;
let page: Page;
let cdp: CDPSession;

beforeAll(async () => {
  if (!available) return;

  browser = await chromium.launch();
  context = await browser.newContext({ ...devices["Pixel 5"] });
  page = await context.newPage();
  cdp = await context.newCDPSession(page);
  await page.setContent("<!doctype html><html><body style='margin:0;overflow:hidden'></body></html>");
  await page.addScriptTag({ type: "module", content: `${script}\nwindow.mountPlayer = mount;` });
  await page.waitForFunction(() => "mountPlayer" in window);
}, HOOK_MS);

afterEach(async () => {
  if (!available) return;

  await page.evaluate(() => (window as PlayerWindow).player?.unmount());
  await page.evaluate(() => document.querySelectorAll("[data-dismissable-layer]").forEach((one) => one.remove()));
});

afterAll(async () => {
  await browser?.close();
}, HOOK_MS);

async function reset(values: Partial<HarnessState> = {}): Promise<void> {
  await page.evaluate((next) => {
    const scope = window as PlayerWindow;

    scope.player?.unmount();
    scope.mountPlayer?.();
    Object.assign(scope.player!.state, next);
  }, values);
}

async function state(): Promise<HarnessState> {
  return page.evaluate(() => (window as PlayerWindow).player!.state);
}

async function touch(type: "touchStart" | "touchMove" | "touchEnd", at?: Point): Promise<void> {
  await cdp.send("Input.dispatchTouchEvent", {
    type,
    touchPoints: at === undefined ? [] : [{ x: Math.round(at.x), y: Math.round(at.y), radiusX: 6, radiusY: 6, force: 1, id: 1 }]
  });
}

async function tap(at: Point): Promise<void> {
  await touch("touchStart", at);
  await touch("touchEnd");
}

async function doubleTap(at: Point): Promise<void> {
  await tap(at);
  await page.waitForTimeout(80);
  await tap(at);
}

async function drag(from: Point, to: Point, steps = 8): Promise<void> {
  await touch("touchStart", from);

  for (let step = 1; step <= steps; step += 1) {
    await touch("touchMove", { x: from.x + ((to.x - from.x) * step) / steps, y: from.y + ((to.y - from.y) * step) / steps });
    await page.waitForTimeout(16);
  }
}

function settledSeek(settled: PlayerCue[]): Extract<PlayerCue, { kind: "seek" }> | null {
  const last = settled.at(-1);

  return last?.kind === "seek" ? last : null;
}

describe.skipIf(!available)("a finger on the player's picture", () => {
  test("a single tap is a tap only once the wait for a second one is over", async () => {
    await reset();

    await tap({ x: 100, y: 100 });
    await page.waitForTimeout(350);

    expect((await state()).taps).toBe(1);
  });

  test("two taps on the left half go ten seconds back, with a cue of how far", async () => {
    await reset({ time: 50 });

    await doubleTap({ x: 50, y: 100 });
    await page.waitForTimeout(350);

    const now = await state();

    expect(now.time).toBe(40);
    expect(now.taps).toBe(0);
    expect(settledSeek(now.settled)).toMatchObject({ kind: "seek", by: -10, seconds: 40 });
    expect(now.cue).toMatchObject({ kind: "seek", by: -10 });

    await page.waitForTimeout(600);
    expect((await state()).cue).toBeNull();
  });

  test("two taps on the right half go ten seconds forward", async () => {
    await reset({ time: 50 });

    await doubleTap({ x: 300, y: 100 });
    await page.waitForTimeout(350);

    const now = await state();

    expect(now.time).toBe(60);
    expect(settledSeek(now.settled)).toMatchObject({ kind: "seek", by: 10, seconds: 60 });
  });

  test("a horizontal drag moves the position as it travels and settles where it lifts", async () => {
    await reset({ time: 50 });

    await touch("touchStart", { x: 180, y: 100 });

    for (let step = 1; step <= 7; step += 1) {
      await touch("touchMove", { x: 180 + step * 10, y: 100 });
      await page.waitForTimeout(16);
    }

    const moving = await state();

    expect(moving.time).toBeGreaterThan(50);
    expect(moving.shown).toBeCloseTo(moving.time);
    expect(moving.cue).toMatchObject({ kind: "seek", by: null });

    await touch("touchEnd");
    await page.waitForTimeout(50);

    const settled = await state();

    expect(settled.time).toBeGreaterThan(50);
    expect(settledSeek(settled.settled)).toMatchObject({ kind: "seek", by: null, seconds: settled.time });
  });

  test("a vertical drag up on the left half brightens the picture and leaves the volume", async () => {
    await reset({ time: 50, brightness: 1, volume: 0.5 });

    await drag({ x: 90, y: 140 }, { x: 90, y: 40 });
    await touch("touchEnd");
    await page.waitForTimeout(50);

    const now = await state();

    expect(now.brightness).toBeGreaterThan(1);
    expect(now.volume).toBe(0.5);
    expect(now.time).toBe(50);
    expect(now.settled.at(-1)).toMatchObject({ kind: "brightness" });
  });

  test("a vertical drag down on the right half turns the volume down and leaves the brightness", async () => {
    await reset({ time: 50, brightness: 1, volume: 0.5 });

    await drag({ x: 270, y: 80 }, { x: 270, y: 160 });
    await touch("touchEnd");
    await page.waitForTimeout(50);

    const now = await state();

    expect(now.volume).toBeLessThan(0.5);
    expect(now.brightness).toBe(1);
    expect(now.time).toBe(50);
    expect(now.settled.at(-1)).toMatchObject({ kind: "volume" });
  });

  test("a drag keeps the way it first went, so a vertical turn moves neither the volume nor the brightness", async () => {
    await reset({ time: 50, brightness: 1, volume: 0.5 });

    await touch("touchStart", { x: 180, y: 80 });
    await touch("touchMove", { x: 240, y: 80 });
    await page.waitForTimeout(16);
    await touch("touchMove", { x: 240, y: 180 });
    await page.waitForTimeout(16);
    await touch("touchEnd");
    await page.waitForTimeout(50);

    const now = await state();

    expect(now.time).toBeGreaterThan(50);
    expect(now.volume).toBe(0.5);
    expect(now.brightness).toBe(1);
    expect(now.settled.at(-1)).toMatchObject({ kind: "seek" });
  });

  test("a held finger plays faster and gives the chosen speed back when it lifts", async () => {
    await reset({ time: 50, baseSpeed: 1 });

    await touch("touchStart", { x: 180, y: 100 });
    await page.waitForTimeout(650);

    const held = await state();

    expect(held.rate).toBe(2);
    expect(held.cue).toMatchObject({ kind: "hold", speed: 2 });
    expect(held.time).toBe(50);

    await touch("touchEnd");
    await page.waitForTimeout(50);

    expect((await state()).rate).toBe(1);
  });

  test("no gesture answers while a dialog or the console's overlay stands open", async () => {
    await reset({ time: 50 });

    await page.evaluate(() => {
      const layer = document.createElement("div");

      layer.setAttribute("data-dismissable-layer", "");
      layer.setAttribute("data-state", "open");
      document.body.appendChild(layer);
    });

    await doubleTap({ x: 50, y: 100 });
    await page.waitForTimeout(350);

    const now = await state();

    expect(now.time).toBe(50);
    expect(now.taps).toBe(0);
    expect(now.settled).toHaveLength(0);
  });

  test("a mouse drag moves nothing", async () => {
    await reset({ time: 50 });

    await page.mouse.move(180, 100);
    await page.mouse.down();
    await page.mouse.move(270, 100, { steps: 5 });
    await page.mouse.up();
    await page.waitForTimeout(50);

    const now = await state();

    expect(now.time).toBe(50);
    expect(now.settled).toHaveLength(0);
  });
});
