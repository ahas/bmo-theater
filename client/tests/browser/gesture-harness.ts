import { createApp } from "vue";

import { usePlayerGestures } from "../../app/composables/usePlayerGestures";
import { overlayOpen } from "../../app/domain/gestures";

import type { PlayerCue } from "../../app/domain/gestures";

// The player's gesture reader in a browser with no screen around it: a video the reader drives, a record of what it
// asked of the player, and real touches sent at it from the test. The picture's clock, speed and length are the
// harness's own, so nothing is played and no file is fetched.

export interface HarnessState {
  length: number | null;
  baseSpeed: number;
  rate: number;
  time: number;
  shown: number;
  volume: number;
  brightness: number;
  cue: PlayerCue | null;
  cues: (PlayerCue | null)[];
  settled: PlayerCue[];
  taps: number;
  wakes: number;
}

export interface Harness {
  video: HTMLVideoElement;
  state: HarnessState;
  unmount: () => void;
}

export function mount(): void {
  const video = document.createElement("video");
  video.style.cssText = "position:absolute;left:0;top:0;width:360px;height:200px;background:#000;touch-action:none;user-select:none;-webkit-user-select:none;-webkit-touch-callout:none";
  document.body.appendChild(video);

  const state: HarnessState = { length: 100, baseSpeed: 1, rate: 1, time: 50, shown: 50, volume: 0.5, brightness: 1, cue: null, cues: [], settled: [], taps: 0, wakes: 0 };

  Object.defineProperty(video, "currentTime", {
    configurable: true,
    get: () => state.time,
    set: (to: number) => {
      state.time = to;
    }
  });
  Object.defineProperty(video, "playbackRate", {
    configurable: true,
    get: () => state.rate,
    set: (to: number) => {
      state.rate = to;
    }
  });
  // The picture the reader drives is playing, though nothing is fetched or played.
  Object.defineProperty(video, "paused", { configurable: true, get: () => false });

  const root = document.createElement("div");
  document.body.appendChild(root);

  const app = createApp({
    setup() {
      usePlayerGestures({
        surface: () => video,
        length: () => state.length,
        speed: () => state.baseSpeed,
        volume: () => state.volume,
        setVolume: (level) => {
          state.volume = level;
        },
        brightness: () => state.brightness,
        setBrightness: (level) => {
          state.brightness = level;
        },
        onTime: (seconds) => {
          state.shown = seconds;
        },
        onCue: (cue) => {
          state.cue = cue;
          state.cues.push(cue);
        },
        onSettled: (cue) => {
          state.settled.push(cue);
        },
        onTap: () => {
          state.taps += 1;
        },
        wake: () => {
          state.wakes += 1;
        },
        blocked: () => overlayOpen(document)
      });

      return () => null;
    }
  });

  app.mount(root);

  const harness: Harness = {
    video,
    state,
    unmount: () => {
      app.unmount();
      video.remove();
      root.remove();
      delete (window as unknown as { player?: Harness }).player;
    }
  };

  (window as unknown as { player?: Harness }).player = harness;
}
