import { afterAll, beforeAll, describe, expect, test } from "bun:test";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { chromium } from "playwright-core";

import { POSTERS, made, ordered, progressOf } from "./fixtures";
import { PACKS, word } from "../words";

import type { Browser, BrowserContext, Page, Route } from "playwright-core";
import type { Server } from "bun";
import type { PosterName } from "./fixtures";

// The theater's screens (`docs/design.md`, 「The console」) stood up in a Nuxt host of their own (`host/`) and driven by
// a real Chromium at a desktop's width and a phone's, in the light and the dark mode, over a realm shaped like the
// libraries the theater serves (`fixtures.ts`): the library's rows, a row in full, a show's page on a season, on its
// specials and on its extras, a clip's page, the player offering the next episode, the organization's empty realm,
// and a playlist; and the selects of a row's order, a show's seasons, the playlist an episode goes into and the
// title a video is filed under, opened.
// The host is generated once before the tests and served from its files, so no page waits on a development server
// compiling it, and a file is answered by the range the player asks for, so the video seeks near its end.
// The test answers the relay itself; the posters and the one video every episode streams are drawn
// with ffmpeg into the host's public folder under the client's packages. Where `THEATER_SHOTS` names a folder, each state is kept there as
// `<width>-<scheme>-<state>.png`; where `THEATER_COMMON_WORDS` names a JSON file of the layer's `common` words, the
// host shows them, and their keys otherwise. Skipped where the browser build or ffmpeg is not on the machine.

// The host's generation included, which a slower machine takes minutes over.
const HOOK_MS = 600_000;
const TEST_MS = 120_000;
const CLIENT = new URL("../../", import.meta.url).pathname;
const HOST = join(CLIENT, "tests/browser/host");
// What the test draws for the host, under the client's packages where the host's build stands.
const CACHE = join(CLIENT, "node_modules/.cache/theater-host");
const MEDIA = join(CACHE, "public/media");
const GENERATED = join(CACHE, ".output/public");
// The video every episode streams: long enough that the player offers the next one before its end.
const VIDEO_SECONDS = 601;

const WIDTHS = [
  { name: "1280", width: 1280, height: 800, touch: false },
  { name: "360", width: 360, height: 780, touch: true }
] as const;
const SCHEMES = ["light", "dark"] as const;

function browserReady(): boolean {
  try {
    return existsSync(chromium.executablePath()) && Bun.which("ffmpeg") !== null;
  } catch {
    return false;
  }
}

const ready = browserReady();

let server: Server | null = null;
let origin = "";
let browser: Browser;
const shots = process.env.THEATER_SHOTS ?? mkdtempSync(join(tmpdir(), "theater-shots-"));

function ffmpeg(args: string[]): void {
  const ran = Bun.spawnSync(["ffmpeg", "-nostdin", "-v", "error", "-y", ...args]);

  if (ran.exitCode !== 0) throw new Error(`ffmpeg failed: ${ran.stderr.toString()}`);
}

// The posters at their own sizes, each a picture of its own, and the one video.
function drawMedia(): void {
  mkdirSync(MEDIA, { recursive: true });

  const sources: Record<PosterName, string> = { wide: "testsrc2", bars: "smptehdbars", square: "mandelbrot" };

  for (const [name, poster] of Object.entries(POSTERS)) {
    const path = join(MEDIA, poster.file);

    if (!existsSync(path)) ffmpeg(["-f", "lavfi", "-i", `${sources[name as PosterName]}=size=${poster.width}x${poster.height}`, "-frames:v", "1", path]);
  }

  const video = join(MEDIA, "video.mp4");

  if (!existsSync(video))
    ffmpeg(["-f", "lavfi", "-i", "testsrc2=size=320x180:rate=2", "-f", "lavfi", "-i", "anullsrc=r=22050:cl=mono", "-t", String(VIDEO_SECONDS), "-c:v", "libx264", "-preset", "ultrafast", "-c:a", "aac", "-b:a", "16k", "-shortest", "-movflags", "+faststart", video]);
}

// A file the host's generation wrote, and the app's shell for any other path, which its router then reads.
async function served(request: Request): Promise<Response> {
  const file = Bun.file(join(GENERATED, decodeURIComponent(new URL(request.url).pathname)));

  return new Response((await file.exists()) ? file : Bun.file(join(GENERATED, "200.html")));
}

beforeAll(async () => {
  if (!ready) return;

  drawMedia();
  mkdirSync(shots, { recursive: true });

  const words = process.env.THEATER_COMMON_WORDS;

  writeFileSync(join(CACHE, "common.json"), words ? readFileSync(words, "utf8") : "{}");

  // Generated after the media and the words, which it carries into what it writes.
  const generated = Bun.spawnSync(["bun", "--bun", "x", "nuxi", "generate", HOST], { cwd: CLIENT, env: { ...process.env, NUXT_TELEMETRY_DISABLED: "1" } });

  if (generated.exitCode !== 0) throw new Error(`the host's generation failed: ${generated.stderr.toString()}`);

  server = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: served });
  origin = server.url.origin;
  browser = await chromium.launch();
}, HOOK_MS);

afterAll(async () => {
  await browser?.close();
  await server?.stop(true);
}, HOOK_MS);

// The relay answered from the realm: every read the screens make, a position set, an episode set aside.
async function answerRelay(context: BrowserContext): Promise<void> {
  const realm = made();

  const json = (route: Route, body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
  const page = <T>(items: T[]) => ({ items, more: false });
  const lastWatched = (titleId: number) => Math.max(0, ...realm.episodes.filter((one) => one.titleId === titleId && realm.positions.has(one.id)).map((one) => realm.positions.get(one.id)!.updatedAt));
  const withPosition = (one: (typeof realm.episodes)[number]) => ({ ...one, position: realm.positions.get(one.id) ?? null });

  // A pattern rather than a predicate, so the browser holds back the relay's calls alone; a predicate would hold
  // back every module the development host serves and pass each through this process.
  await context.route(
    "**/relay/theater/**",
    async (route) => {
      const url = new URL(route.request().url());
      const path = url.pathname.slice("/relay/theater".length);
      const method = route.request().method();
      const asked = url.searchParams.get("realm") === "organization" ? "organization" : "mine";
      const parts = path.split("/").filter(Boolean);
      const id = Number(parts[1]);

      if (path === "/events") return route.fulfill({ status: 200, contentType: "text/event-stream", body: ": open\n\n" });
      if (path === "/libraries" && method === "POST") return json(route, { code: "forbidden", error: "Only the administrator registers a library." }, 403);
      if (path === "/titles") return json(route, page(ordered(realm.titles, asked, url.searchParams.get("order") ?? "id", (title) => lastWatched(title.id))));
      if (path === "/playlists") return json(route, page(realm.playlists.filter((one) => one.realm === asked)));
      if (path === "/watching")
        return json(
          route,
          page(
            realm.episodes
              .filter((one) => one.realm === asked && realm.positions.get(one.id)?.seconds && !realm.positions.get(one.id)!.done && !realm.positions.get(one.id)!.setAside)
              .sort((a, b) => realm.positions.get(b.id)!.updatedAt - realm.positions.get(a.id)!.updatedAt)
              .map(withPosition)
          )
        );
      if (path === "/next-up") {
        const shows = realm.titles.filter((one) => one.realm === asked && one.kind === "show");
        const offered = shows.flatMap((show) => {
          const progress = progressOf(show.kind, realm.episodes.filter((one) => one.titleId === show.id), realm.positions);
          const upNext = realm.episodes.find((one) => one.id === progress.upNext);

          return upNext && !realm.positions.has(upNext.id) ? [withPosition(upNext)] : [];
        });

        return json(route, page(offered));
      }
      if (parts[0] === "watching" && method === "DELETE") {
        const held = realm.positions.get(id);

        if (held) realm.positions.set(id, { ...held, setAside: true });

        return route.fulfill({ status: 204 });
      }
      if (parts[0] === "titles" && parts.length === 2) return json(route, realm.titles.find((one) => one.id === id));
      if (parts[0] === "titles" && parts[2] === "episodes") return json(route, page(realm.episodes.filter((one) => one.titleId === id).map(withPosition)));
      if (parts[0] === "titles" && parts[2] === "progress") {
        const title = realm.titles.find((one) => one.id === id)!;

        return json(route, progressOf(title.kind, realm.episodes.filter((one) => one.titleId === id), realm.positions));
      }
      if (parts[2] === "poster") return route.fulfill({ status: 302, headers: { location: `/media/${POSTERS[realm.posters.get(`${parts[0] === "titles" ? "title" : "episode"}:${id}`)!].file}` } });
      if (parts[0] === "episodes" && parts[2] === "stream") return route.fulfill({ status: 302, headers: { location: "/media/video.mp4" } });
      if (parts[0] === "episodes" && parts[2] === "subtitles") return json(route, page([]));
      if (parts[0] === "episodes" && parts[2] === "position" && method === "PUT") {
        const body = route.request().postDataJSON() as { seconds: number };

        return json(route, { episodeId: id, seconds: body.seconds, done: false, updatedAt: Date.now(), setAside: false });
      }
      if (parts[0] === "episodes" && parts.length === 2) return json(route, withPosition(realm.episodes.find((one) => one.id === id)!));
      if (parts[0] === "playlists" && parts.length === 2) return json(route, realm.playlists.find((one) => one.id === id));
      if (parts[0] === "playlists" && parts[2] === "progress") {
        const playlist = realm.playlists.find((one) => one.id === id)!;

        return json(route, progressOf("clip", playlist.episodeIds.map((episodeId) => realm.episodes.find((one) => one.id === episodeId)!), realm.positions));
      }

      return json(route, { code: "no-route", error: "No route." }, 404);
    }
  );
}

async function open(width: (typeof WIDTHS)[number], scheme: (typeof SCHEMES)[number]): Promise<{ context: BrowserContext; page: Page }> {
  const context = await browser.newContext({ viewport: { width: width.width, height: width.height }, hasTouch: width.touch, isMobile: width.touch, deviceScaleFactor: 1, colorScheme: scheme });

  await context.addCookies([{ name: "nuxt-color-mode", value: scheme, url: origin }]);
  await answerRelay(context);

  return { context, page: await context.newPage() };
}

// A screen once what shows its reads have landed stands and the pictures in view have loaded or failed.
async function settled(page: Page, path: string, landed: string): Promise<void> {
  await page.goto(`${origin}${path}`, { waitUntil: "load" });
  await page.locator(landed).first().waitFor({ timeout: 60_000 });

  // The pictures in view, since one scrolled past the screen's edge loads only once it comes near.
  await page.waitForFunction(
    () =>
      [...document.images].every((image) => {
        const box = image.getBoundingClientRect();

        return image.complete || box.bottom < 0 || box.top > innerHeight || box.right < 0 || box.left > innerWidth || box.width === 0;
      }),
    undefined,
    { timeout: 60_000, polling: 250 }
  );
  // The movements that end, since a skeleton's shimmer runs until its picture lands.
  await page.evaluate(() => Promise.all(document.getAnimations().filter((one) => one.effect?.getComputedTiming().endTime !== Infinity).map((one) => one.finished.catch(() => undefined))));
}

async function shoot(page: Page, width: (typeof WIDTHS)[number], scheme: string, state: string): Promise<void> {
  await page.screenshot({ path: join(shots, `${width.name}-${scheme}-${state}.png`) });
}

async function noSidewaysScroll(page: Page): Promise<void> {
  expect(await page.evaluate(() => document.scrollingElement!.scrollWidth <= document.scrollingElement!.clientWidth)).toBe(true);
}

const headings = (page: Page) => page.locator("[data-card-row] h2").allTextContents();

describe.skipIf(!ready)("the theater's screens", () => {
  for (const width of WIDTHS) {
    for (const scheme of SCHEMES) {
      test(
        `stand at ${width.name} in the ${scheme} mode`,
        async () => {
          const { context, page } = await open(width, scheme);

          try {
            // The library: the rows in their order, continue watching first, each standing only with something in it.
            await settled(page, "/theater", "[data-card-row]");
            expect((await headings(page)).length).toBe(7);
            expect(await page.locator("[data-card-row]").first().locator("[data-artwork]").count()).toBe(2);
            // The four by three poster stands whole over a ground of itself in its row; the clip with none stands as its mark and name.
            expect(await page.locator("[data-artwork][data-shape='landscape'] [data-artwork-ground]").count()).toBeGreaterThan(0);
            expect(await page.locator("[data-artwork-placeholder]", { hasText: "Studio notes" }).count()).toBeGreaterThan(0);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "home");

            // A row in full, the order in the address.
            await settled(page, "/theater/browse?order=name", "ul li [data-artwork]");
            expect(await page.locator("ul li [data-artwork]").count()).toBe(10);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "all");

            // The long show, on the season of its up next; its specials last among the seasons; its extras apart.
            await settled(page, "/theater/titles/1", "section ul li");
            expect(await page.locator("[data-title-play]").textContent()).toContain("S01E06");
            expect(await page.locator("section[aria-label] ul li").count()).toBe(45 + 9);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "series");

            await settled(page, "/theater/titles/1?season=specials", "section ul li");
            expect(await page.locator("section ul").first().locator("li").count()).toBe(11);
            await shoot(page, width, scheme, "specials");
            await page.locator("section").last().evaluate((extras) => extras.scrollIntoView({ block: "start" }));
            expect(await page.locator("section").last().locator("ul li").count()).toBe(9);
            await shoot(page, width, scheme, "extras");

            // A clip of one episode lists no episodes.
            await settled(page, "/theater/titles/4", "[data-title-play]");
            expect(await page.locator("section ul li").count()).toBe(0);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "clip");

            // The player near the end of an episode of the long show offers the next one.
            const episodeId = made().episodes.find((one) => one.titleId === 1 && one.season === 1 && one.number === 6)!.id;

            await page.goto(`${origin}/theater/watch/${episodeId}`, { waitUntil: "load" });
            await page.waitForFunction(() => {
              const video = document.querySelector<HTMLVideoElement>("[data-player]");

              return video !== null && video.readyState >= 1;
            }, undefined, { timeout: 60_000 });
            await page.evaluate(async (seconds) => {
              const video = document.querySelector<HTMLVideoElement>("[data-player]")!;

              video.muted = true;
              video.currentTime = seconds;
              await video.play();
            }, VIDEO_SECONDS - 20);
            await page.locator("[data-video-next]").waitFor({ timeout: 30_000 });
            await page.evaluate(() => document.querySelector<HTMLVideoElement>("[data-player]")!.pause());
            await page.evaluate(() => Promise.all(document.getAnimations().filter((one) => one.effect?.getComputedTiming().endTime !== Infinity).map((one) => one.finished.catch(() => undefined))));
            // The count-down's last turn lands a moment after the pause.
            await page.waitForTimeout(1000);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "player-next");

            // The organization's realm, which holds nothing.
            await settled(page, "/theater?realm=organization", "main p, [data-slot='body'] p");
            expect(await page.locator("[data-card-row]").count()).toBe(0);
            await shoot(page, width, scheme, "empty");

            // A playlist.
            await settled(page, "/theater/playlists/1", "ul li");
            expect(await page.locator("ul li").count()).toBe(3);
            await noSidewaysScroll(page);
            await shoot(page, width, scheme, "playlist");
          } finally {
            await context.close();
          }
        },
        TEST_MS
      );
    }
  }
});

// A select's opened list, and the search field the layer stands at its top once it holds more than seven choices.
const LIST = "[data-slot=focusScope]";
const CHOICE = `${LIST} [role=option]`;
const FIELD = `${LIST} input`;

// A control the screen names by its words, pressed as the width's own pointer does: a finger on the phone, a
// mouse on the desktop.
async function press(page: Page, width: (typeof WIDTHS)[number], selector: string, key: string): Promise<void> {
  const control = page.locator(`${selector}[aria-label="${word(PACKS.ko, key)!}"]`).first();

  await (width.touch ? control.tap() : control.click());
}

// The select the screen names by its words opened: as many choices as the screen hands it, and the search field
// standing only past seven of them.
async function opened(page: Page, width: (typeof WIDTHS)[number], key: string, choices: number): Promise<void> {
  await press(page, width, "[data-slot=base]", key);
  await page.locator(CHOICE).first().waitFor({ timeout: 10_000 });
  expect([key, await page.locator(CHOICE).count()]).toEqual([key, choices]);
  expect([key, await page.locator(FIELD).count()]).toEqual([key, choices > 7 ? 1 : 0]);
  // Opened all the way, so a picture of it is not taken mid-fade; what animates without end is not awaited.
  await page.evaluate(() => Promise.all(document.getAnimations().filter((one) => Number.isFinite(Number(one.effect?.getComputedTiming().endTime))).map((one) => one.finished.catch(() => undefined))));
}

async function closed(page: Page): Promise<void> {
  await page.keyboard.press("Escape");
  await page.locator(LIST).waitFor({ state: "detached", timeout: 10_000 });
}

describe.skipIf(!ready)("the theater's selects", () => {
  for (const width of WIDTHS) {
    test(
      `offer their search by the layer's rule at ${width.name}`,
      async () => {
        const { context, page } = await open(width, "light");
        const titles = made().titles.filter((one) => one.realm === "mine").length;

        try {
          // A row's three orders.
          await settled(page, "/theater/browse", "ul li [data-artwork]");
          await opened(page, width, "theater.label.order", 3);
          await shoot(page, width, "light", "select-order");
          await closed(page);

          // The long show's six seasons and its specials: seven, the most a list holds with no search.
          await settled(page, "/theater/titles/1", "section ul li");
          await opened(page, width, "theater.label.season", 7);
          await shoot(page, width, "light", "select-season");
          await closed(page);

          // The playlist an episode goes into, its dialog made as it opens: a new one and each of the realm's.
          await press(page, width, "button", "theater.action.add_to_playlist");
          await opened(page, width, "theater.label.playlist", made().playlists.filter((one) => one.realm === "mine").length + 1);
          await shoot(page, width, "light", "select-playlist");
          await closed(page);

          // The title a video is filed under: a new one and each of the realm's.
          await settled(page, "/theater", "[data-card-row]");
          await press(page, width, "button", "theater.action.upload_video");
          await opened(page, width, "theater.label.title", titles + 1);
          await shoot(page, width, "light", "select-title");
          await closed(page);
        } finally {
          await context.close();
        }
      },
      TEST_MS
    );
  }
});
