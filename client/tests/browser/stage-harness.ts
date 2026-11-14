import ui from "@nuxt/ui/vue-plugin";
import { createApp, defineComponent, h, nextTick, onScopeDispose, ref, watch } from "vue";
import { createI18n } from "vue-i18n";

import UApp from "@nuxt/ui/components/App.vue";
import { markPainted } from "bmo-ui/app/domain/loading";
import { watchInputModality } from "bmo-ui/app/domain/modality";
import { watchOverlays } from "bmo-ui/app/domain/overlays";
import { TOOLTIP_PROVIDER } from "bmo-ui/app/domain/tooltip";

import PlayerStage from "../../app/components/theater/PlayerStage.vue";
import sheet from "./stage-harness.css?inline";

import type { Ref } from "vue";
import type { StagePlayer } from "../../app/components/theater/PlayerStage.vue";

// The theater's player stage (`docs/design.md`, 「The console」) on a page that says nothing else, in the three ways
// a screen draws it: an episode read and playing on the layer's player, an episode still being read, and one that
// cannot play. Every media element's clock, length and playing are the page's own, so nothing is fetched and
// nothing is played: a picture answers its metadata the moment it stands, and what the stage tells its screen is
// written down where the test reads it. The page stands outside the console, so it wires itself as the console
// does: the library's app root holding the layer's tooltip setting, the overlays the back gesture closes, the page
// taken over by the browser so a skeleton says it is busy, and the console's words for the layer's player.

const states = new Map<string, Ref<unknown>>();

Object.assign(globalThis, {
  watch,
  nextTick,
  onScopeDispose,
  useState: (key: string, start: () => unknown) => {
    if (!states.has(key)) states.set(key, ref(start()));

    return states.get(key);
  },
  useRouter: () => ({ options: {}, beforeEach: () => undefined })
});

const WORDS = {
  common: {
    action: {
      close: "Close",
      more: "More",
      exit_fullscreen: "Exit full screen",
      fullscreen: "Full screen",
      mute: "Mute",
      pause: "Pause",
      play: "Play",
      skip_back: "Back {n} seconds",
      skip_forward: "Forward {n} seconds",
      subtitles: "Subtitles",
      turn_picture: "Turn the picture",
      unmute: "Unmute"
    },
    aria: { play_named: "Play {name}", player_named: "{name} player", speed_named: "Playback speed {speed}" },
    error: { video_failed: "The video could not be played" },
    label: { play_position: "Playback position", speed_normal: "{speed} normal", subtitles_off: "Off", volume: "Volume" },
    status: {
      brightness: "Brightness {n}%",
      muted: "Muted",
      speed: "Speed {speed}",
      subtitles_off: "Subtitles off",
      subtitles_on: "Subtitles {name}",
      turned: "Turned to {degrees} degrees",
      unmuted: "Unmuted",
      video_loading: "Loading the video",
      volume: "Volume {n}%",
      went_back: "Back {n} seconds",
      went_forward: "Forward {n} seconds"
    }
  }
};

/** The length every media element on the page says it has, in seconds. */
export const LENGTH_SECONDS = 100;

/** The episode the playing stage stands for. */
export const EPISODE = 7;

export interface StageCall {
  name: string;
  args: unknown[];
}

declare global {
  interface Window {
    stageCalls: StageCall[];
  }
}

window.stageCalls = [];

// What one media element holds in place of a file: its clock, its rate and whether it plays.
interface Held {
  time: number;
  rate: number;
  paused: boolean;
}

const held = new WeakMap<HTMLMediaElement, Held>();

function heldBy(element: HTMLMediaElement): Held {
  let one = held.get(element);

  if (one === undefined) {
    one = { time: 0, rate: 1, paused: true };
    held.set(element, one);
  }

  return one;
}

const media = HTMLMediaElement.prototype;

Object.defineProperty(media, "duration", { configurable: true, get: () => LENGTH_SECONDS });
Object.defineProperty(media, "ended", { configurable: true, get: () => false });
Object.defineProperty(media, "paused", {
  configurable: true,
  get(this: HTMLMediaElement) {
    return heldBy(this).paused;
  }
});
Object.defineProperty(media, "currentTime", {
  configurable: true,
  get(this: HTMLMediaElement) {
    return heldBy(this).time;
  },
  set(this: HTMLMediaElement, seconds: number) {
    heldBy(this).time = seconds;
  }
});
Object.defineProperty(media, "playbackRate", {
  configurable: true,
  get(this: HTMLMediaElement) {
    return heldBy(this).rate;
  },
  set(this: HTMLMediaElement, rate: number) {
    heldBy(this).rate = rate;
  }
});

media.play = function play(this: HTMLMediaElement): Promise<void> {
  if (heldBy(this).paused) {
    heldBy(this).paused = false;
    this.dispatchEvent(new Event("play"));
    this.dispatchEvent(new Event("playing"));
  }

  return Promise.resolve();
};

media.pause = function pause(this: HTMLMediaElement): void {
  if (!heldBy(this).paused) {
    heldBy(this).paused = true;
    this.dispatchEvent(new Event("pause"));
  }
};

// The browser's own attempt at an address nothing answers is not the page's failure, so it never reaches the player.
window.addEventListener(
  "error",
  (event) => {
    if (event.target instanceof HTMLMediaElement) event.stopImmediatePropagation();
  },
  true
);

// A video answers its metadata the moment it stands on the page, its listeners already bound.
new MutationObserver((changes) => {
  for (const change of changes) {
    for (const node of change.addedNodes) {
      const videos = node instanceof HTMLVideoElement ? [node] : node instanceof Element ? Array.from(node.querySelectorAll("video")) : [];

      for (const video of videos) video.dispatchEvent(new Event("loadedmetadata"));
    }
  }
}).observe(document.body, { childList: true, subtree: true });

function called(name: string, ...args: unknown[]): void {
  window.stageCalls.push({ name, args });
}

const PLAYING: StagePlayer = {
  episode: EPISODE,
  src: "/episodes/7/stream",
  poster: null,
  title: "Harbour, episode 1",
  tracks: [{ id: 3, src: "/subtitles/3.vtt", label: "English", language: "en" }],
  autoplay: false,
  next: "Next: Harbour, episode 2"
};

const Page = defineComponent(() => {
  const track = ref<number | null>(null);

  return () =>
    h(UApp, { tooltip: TOOLTIP_PROVIDER }, () =>
      h("div", { class: "flex flex-col gap-4" }, [
        h(PlayerStage, {
          "data-stage": "playing",
          class: "aspect-video w-full",
          failed: "",
          player: PLAYING,
          track: track.value,
          "onUpdate:track": (value: number | null) => {
            track.value = value;
            called("track", value);
          },
          onPosition: (...args: unknown[]) => called("position", ...args),
          onLeaving: (...args: unknown[]) => called("leaving", ...args),
          onEnded: (...args: unknown[]) => called("ended", ...args),
          onNext: () => called("next")
        }),
        h(PlayerStage, { "data-stage": "reading", class: "aspect-video w-full", failed: "", player: null }),
        h(PlayerStage, { "data-stage": "failed", class: "aspect-video w-full", failed: "This episode cannot be found", player: null })
      ])
    );
});

const style = document.createElement("style");

style.textContent = sheet;
document.head.append(style);

watchInputModality(document);
watchOverlays();
markPainted();

Object.assign(useAppConfig(), { bmoUi: { zone: "UTC" } });

createApp(Page)
  .use(createI18n({ legacy: false, locale: "en", messages: { en: WORDS }, missingWarn: false, fallbackWarn: false }))
  .use(ui)
  .mount("#page");
