<template>
  <!-- The player (`docs/design.md`, 「Playing」 and 「The console」): the browser's video over the episode's
       stream, its subtitles laid over it as text tracks, and its own controls under it — play, ten-second skips,
       the next episode, the volume, the subtitle and the speed, turning the picture, and full screen. A finger on
       the picture takes the player's gestures, its cue standing over the picture. It stands on a black ground of
       its own, whatever the theme, so its words and marks are fixed light ones. The keys answer while the
       player stands: space, the arrows, f, m, c and r. -->
  <div
    ref="root"
    role="region"
    :aria-label="$t('theater.aria.player', { name: props.title })"
    class="theater-player relative flex size-full items-center justify-center overflow-hidden bg-black text-white"
    :class="controlsShown ? '' : 'cursor-none'"
    @pointermove="wake"
    @pointerdown.capture="onPress"
    @focusin="wake"
  >
    <video
      ref="video"
      :key="props.src"
      :src="props.src"
      :poster="props.poster ?? undefined"
      playsinline
      preload="metadata"
      class="theater-picture size-full object-contain"
      :style="videoStyle"
      @click="onVideoPress"
      @dblclick="onVideoDoubleClick"
      @loadedmetadata="onMetadata"
      @timeupdate="onTime"
      @play="onPlay"
      @pause="onPause"
      @ended="onEnded"
      @waiting="waiting = true"
      @playing="waiting = false"
      @canplay="waiting = false"
      @error="failed = true"
    >
      <track v-for="one in props.tracks" :key="one.id" kind="subtitles" :src="one.src" :srclang="one.language" :label="one.label" :data-track="one.id" @load="placeCues" />
    </video>

    <!-- Before the first play the poster stands with one large way to start. -->
    <UTooltip :portal="portal" v-if="!started && !failed" :text="$t('common.action.play')">
      <button
        type="button"
        class="absolute inset-0 m-auto flex size-18 items-center justify-center rounded-full bg-black/70 text-white transition-colors hover:bg-black/85 active:bg-black"
        :aria-label="$t('common.action.play')"
        @click="toggle"
      >
        <UIcon name="i-material-symbols-play-arrow" class="size-12" />
      </button>
    </UTooltip>

    <UIcon v-if="waiting && started && !failed" name="i-material-symbols-progress-activity" class="absolute inset-0 m-auto size-12 animate-spin motion-reduce:animate-none" aria-hidden="true" />

    <!-- What a finger on the picture is doing, over the picture. The live region under the player says the same
         once a gesture has ended, so this stands out of the reader's way. -->
    <Transition name="theater-cue">
      <div v-if="cueView" class="pointer-events-none absolute inset-x-0 top-1/2 z-10 mx-auto flex w-fit max-w-[calc(100%-2rem)] -translate-y-1/2 items-center gap-2 rounded-full bg-black/75 px-4 py-2 text-base text-white" aria-hidden="true">
        <UIcon :name="cueView.icon" class="size-6 shrink-0" />
        <span class="truncate tabular-nums">{{ cueView.text }}</span>
      </div>
    </Transition>

    <p v-if="failed" role="alert" class="absolute inset-x-4 top-1/2 -translate-y-1/2 text-center text-base">{{ $t("common.error.video_failed") }}</p>

    <div
      class="absolute inset-x-0 bottom-0 flex flex-col gap-1 bg-black/75 px-2 pt-1 pb-[max(0.25rem,var(--bmo-safe-bottom,0px))] transition-opacity sm:px-3"
      :class="controlsShown ? 'opacity-100' : 'pointer-events-none opacity-0'"
      :aria-hidden="controlsShown ? undefined : 'true'"
    >
      <div class="flex items-center gap-3 px-1 text-sm tabular-nums">
        <USlider
          :model-value="current"
          :min="0"
          :max="Math.max(1, length ?? 1)"
          :step="1"
          size="sm"
          class="theater-seek min-w-0 flex-1"
          :ui="{ thumb: THUMB }"
          :aria-label="$t('theater.aria.seek')"
          :aria-valuetext="`${playClock(current)} / ${playClock(length ?? 0)}`"
          :disabled="length === null"
          @update:model-value="seek"
        />
        <span class="shrink-0 text-white">{{ playClock(current) }} / {{ playClock(length ?? 0) }}</span>
      </div>

      <div class="flex min-w-0 items-center gap-1">
        <UTooltip :portal="portal" :text="$t(playing ? 'common.action.pause' : 'common.action.play')" :kbds="['space']">
          <UButton :icon="playing ? 'i-material-symbols-pause' : 'i-material-symbols-play-arrow'" v-bind="BUTTON" :aria-label="$t(playing ? 'common.action.pause' : 'common.action.play')" @click="toggle" />
        </UTooltip>
        <UTooltip :portal="portal" :text="$t('theater.action.back_10')" :kbds="['arrowleft']">
          <UButton icon="i-material-symbols-replay-10" v-bind="BUTTON" :aria-label="$t('theater.action.back_10')" @click="skip(-SKIP_SECONDS)" />
        </UTooltip>
        <UTooltip :portal="portal" :text="$t('theater.action.forward_10')" :kbds="['arrowright']">
          <UButton icon="i-material-symbols-forward-10" v-bind="BUTTON" :aria-label="$t('theater.action.forward_10')" @click="skip(SKIP_SECONDS)" />
        </UTooltip>
        <UTooltip :portal="portal" v-if="props.next" :text="props.next">
          <UButton icon="i-material-symbols-skip-next" v-bind="BUTTON" :aria-label="props.next" @click="emit('next')" />
        </UTooltip>
        <UTooltip :portal="portal" :text="$t(volume.loudness.value.muted ? 'common.action.unmute' : 'common.action.mute')" :kbds="['m']">
          <UButton :icon="muteIcon" v-bind="BUTTON" :aria-label="$t(volume.loudness.value.muted ? 'common.action.unmute' : 'common.action.mute')" @click="volume.toggleMute()" />
        </UTooltip>
        <USlider
          v-if="!narrow"
          :model-value="volume.heard.value * 100"
          :min="0"
          :max="100"
          :step="5"
          size="sm"
          class="theater-seek w-24 shrink-0"
          :ui="{ thumb: THUMB }"
          :aria-label="$t('common.label.volume')"
          @update:model-value="(level) => volume.slide((Number(level) || 0) / 100)"
        />

        <span class="flex-1" />

        <UDropdownMenu :modal="false" :ui="{ item: 'pointer-coarse:min-h-11' }" v-if="props.tracks.length" :items="trackItems" :content="{ align: 'end', side: 'top' }" :portal="portal">
          <UTooltip :portal="portal" :text="$t('theater.action.subtitles')" :kbds="['c']">
            <UButton :icon="shownTrack === null ? 'i-material-symbols-subtitles-off-outline' : 'i-material-symbols-subtitles'" v-bind="BUTTON" :aria-label="$t('theater.action.subtitles')" />
          </UTooltip>
        </UDropdownMenu>
        <UDropdownMenu :modal="false" :ui="{ item: 'pointer-coarse:min-h-11' }" :items="speedItems" :content="{ align: 'end', side: 'top' }" :portal="portal">
          <UTooltip :portal="portal" :text="$t('theater.aria.speed_named', { speed: speedText(speed) })">
            <UButton icon="i-material-symbols-speed-outline" v-bind="BUTTON" :aria-label="$t('theater.aria.speed_named', { speed: speedText(speed) })" />
          </UTooltip>
        </UDropdownMenu>
        <UTooltip :portal="portal" :text="$t('theater.action.turn')" :kbds="['r']">
          <UButton icon="i-material-symbols-screen-rotation-outline" v-bind="BUTTON" :aria-label="$t('theater.action.turn')" @click="turn" />
        </UTooltip>
        <UTooltip :portal="portal" :text="$t(fullscreen ? 'theater.action.exit_fullscreen' : 'theater.action.fullscreen')" :kbds="['f']">
          <UButton :icon="fullscreen ? 'i-material-symbols-fullscreen-exit' : 'i-material-symbols-fullscreen'" v-bind="BUTTON" :aria-label="$t(fullscreen ? 'theater.action.exit_fullscreen' : 'theater.action.fullscreen')" @click="toggleFullscreen" />
        </UTooltip>
      </div>
    </div>

    <!-- What a key or a press changed, said once it has changed. -->
    <p class="sr-only" aria-live="polite" aria-atomic="true">{{ announced }}</p>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, useTemplateRef, watch } from "vue";
import { useI18n } from "vue-i18n";
import { holdThroughFullscreen } from "bmo-ui/app/domain/fullscreen";
import { knownLength, playClock } from "bmo-ui/app/domain/player";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { useVolume } from "bmo-ui/app/domain/volume";

import { usePlayerGestures } from "../../composables/usePlayerGestures";
import { overlayOpen } from "../../domain/gestures";
import { CONTROLS_IDLE_MS, CUE_LINE_OVER_CONTROLS, SAVE_EVERY_MS, SKIP_SECONDS, SPEEDS, VOLUME_STEP, keyAction, nextTurn, otherOrientation, skipTo, turnedBox, speedText, steppedVolume, toggledTrack } from "../../domain/player";

import type { PlayerCue } from "../../domain/gestures";

import type { DropdownMenuItem } from "@nuxt/ui";

export interface PlayerTrack {
  id: number;
  src: string;
  label: string;
  language: string;
}

interface Props {
  /** The episode it plays, named on what it tells of it. */
  episode: number;
  src: string;
  poster?: string | null;
  /** The episode's name, which the player is named by. */
  title: string;
  tracks?: PlayerTrack[];
  /** Where it starts, in seconds. */
  start?: number;
  /** Whether it starts as soon as it can, as the next of a playlist does. */
  autoplay?: boolean;
  /** The next episode's words, where there is one to go on to. */
  next?: string | null;
}

const props = withDefaults(defineProps<Props>(), { poster: null, tracks: () => [], start: 0, autoplay: false, next: null });

const emit = defineEmits<{
  /**
   * Where the video stands, every few seconds while it plays, as it pauses and as the player is left, named by the
   * `episode` this player stands for, since the screen may already have moved on to another as it is left.
   */
  position: [episode: number, seconds: number];
  /** The same as the page itself goes away, when only a call that outlives the page lands. */
  leaving: [episode: number, seconds: number];
  ended: [episode: number, seconds: number];
  next: [];
}>();

/** The subtitle shown, or none. */
const track = defineModel<number | null>("track", { default: null });

// The control buttons stand on the player's own black ground in its fixed light words, a square a finger aims
// at on a touch screen.
const BUTTON = {
  color: "neutral",
  variant: "ghost",
  class: "text-white hover:bg-white/15 active:bg-white/25 hover:text-white pointer-coarse:size-11 pointer-coarse:justify-center"
} as const;

// A slider's thumb is aimed at over a reach a finger has, wider than the mark it draws.
const THUMB = "relative after:absolute after:-inset-4";

const rootRef = useTemplateRef<HTMLDivElement>("root");
const videoRef = useTemplateRef<HTMLVideoElement>("video");

const { t } = useI18n();
const narrow = useNarrow();
const volume = useVolume();

const announced = ref("");
const current = ref(0);
const length = ref<number | null>(null);
const speed = ref(1);
const remembered = ref<number | null>(null);
const playing = ref(false);
const started = ref(false);
const waiting = ref(false);
const failed = ref(false);
const awake = ref(true);
const fullscreen = ref(false);
const brightness = ref(1);
const cue = ref<PlayerCue | null>(null);
// Where a touch last landed, so the browser's own double-click after two taps opens no full screen.
let lastTouchAt = Number.NEGATIVE_INFINITY;

// A menu opened in full screen stands inside the player, which is all the browser shows.
const portal = computed(() => (fullscreen.value ? (rootRef.value ?? true) : true));
const controlsShown = computed(() => !playing.value || awake.value || failed.value);
const shownTrack = computed(() => track.value);
const muteIcon = computed(() => (volume.heard.value === 0 ? "i-material-symbols-volume-off" : volume.heard.value < 0.5 ? "i-material-symbols-volume-down" : "i-material-symbols-volume-up"));

// What the cue over the picture says of the gesture under the finger, and the mark that names it.
const cueView = computed<{ icon: string; text: string } | null>(() => {
  const shown = cue.value;

  if (shown === null) return null;

  if (shown.kind === "seek") {
    const forward = shown.by !== null ? shown.by >= 0 : shown.seconds >= shown.from;
    const byTap = shown.by !== null;

    return {
      icon: byTap ? (forward ? "i-material-symbols-forward-10" : "i-material-symbols-replay-10") : forward ? "i-material-symbols-fast-forward" : "i-material-symbols-fast-rewind",
      text: byTap ? t(forward ? "theater.status.went_forward" : "theater.status.went_back", { n: String(Math.abs(shown.by ?? 0)) }) : `${playClock(shown.seconds)} / ${playClock(shown.length ?? 0)}`
    };
  }

  if (shown.kind === "hold") return { icon: "i-material-symbols-fast-forward", text: t("theater.status.speed", { speed: speedText(shown.speed) }) };

  if (shown.kind === "brightness") return { icon: "i-material-symbols-light-mode", text: t("theater.status.brightness", { n: String(Math.round(shown.level * 100)) }) };

  return {
    icon: shown.level === 0 ? "i-material-symbols-volume-off" : shown.level < 0.5 ? "i-material-symbols-volume-down" : "i-material-symbols-volume-up",
    text: t("theater.status.volume", { n: String(Math.round(shown.level * 100)) })
  };
});

const trackItems = computed<DropdownMenuItem[]>(() => [
  { label: t("theater.label.subtitles_off"), type: "checkbox", checked: track.value === null, onSelect: () => chooseTrack(null) },
  ...props.tracks.map((one) => ({ label: one.label, type: "checkbox" as const, checked: track.value === one.id, onSelect: () => chooseTrack(one.id) }))
]);

const speedItems = computed<DropdownMenuItem[]>(() =>
  SPEEDS.map((one) => ({ label: one === 1 ? t("theater.label.speed_normal", { speed: speedText(one) }) : speedText(one), type: "checkbox" as const, checked: speed.value === one, onSelect: () => setSpeed(one) }))
);

watch(
  () => volume.heard.value,
  (heard) => {
    const element = videoRef.value;

    if (element) {
      element.volume = volume.loudness.value.level;
      element.muted = heard === 0;
    }
  }
);

watch(track, () => showTrack());

watch(controlsShown, () => placeCues());

watch(
  () => props.src,
  () => {
    started.value = false;
    failed.value = false;
    current.value = 0;
    length.value = null;
    brightness.value = 1;
    cue.value = null;
  }
);

function say(words: string): void {
  announced.value = words;
}

function showTrack(): void {
  const element = videoRef.value;

  if (!element) return;

  for (const node of element.querySelectorAll("track")) node.track.mode = Number(node.dataset.track) === track.value ? "showing" : "disabled";
}

// The subtitles stand above the controls while the controls stand, and at the foot of the picture once they go.
function placeCues(): void {
  const element = videoRef.value;

  if (!element) return;

  const line = controlsShown.value ? CUE_LINE_OVER_CONTROLS : "auto";

  for (const node of element.querySelectorAll("track")) for (const cue of Array.from(node.track.cues ?? [])) (cue as VTTCue).line = line;
}

function chooseTrack(id: number | null): void {
  if (track.value !== null) remembered.value = track.value;

  track.value = id;

  const chosen = props.tracks.find((one) => one.id === id);

  say(chosen ? t("theater.status.subtitles_on", { name: chosen.label }) : t("theater.status.subtitles_off"));
}

function setSpeed(value: number): void {
  speed.value = value;
  if (videoRef.value) videoRef.value.playbackRate = value;
  say(t("theater.status.speed", { speed: speedText(value) }));
}

let idle: ReturnType<typeof setTimeout> | null = null;
let hiddenAtPress = false;

function wake(): void {
  awake.value = true;

  if (idle !== null) clearTimeout(idle);

  idle = setTimeout(() => {
    idle = null;
    // The controls stay while a control holds the keyboard's focus.
    if (!rootRef.value?.querySelector(":focus-visible")) awake.value = false;
  }, CONTROLS_IDLE_MS);
}

function toggle(): void {
  const element = videoRef.value;

  if (!element) return;

  if (element.paused) void element.play().catch(() => undefined);
  else element.pause();
}

// A press is read before it brings the controls, so what it finds is whether they were standing. It is read as the
// press comes down through the player, ahead of the picture's gesture reader, which brings the controls itself.
function onPress(event: PointerEvent): void {
  hiddenAtPress = !controlsShown.value;

  if (event.pointerType === "touch") lastTouchAt = performance.now();

  wake();
}

// A press on the picture plays or pauses with a mouse. A finger's tap is read by the gesture reader, which shows
// or hides the controls once it knows the tap was not the first of a double tap.
function onVideoPress(event: MouseEvent): void {
  if ((event as PointerEvent).pointerType === "touch") return;

  toggle();
}

// A touch screen raises a double click of its own after two taps, which is the gesture's and not full screen's.
function onVideoDoubleClick(): void {
  if (performance.now() - lastTouchAt < 1_000) return;

  void toggleFullscreen();
}

// What a finished gesture says to a reader who cannot see the cue.
function announceGesture(settled: PlayerCue): void {
  if (settled.kind === "seek") {
    if (settled.by !== null) say(t(settled.by < 0 ? "theater.status.went_back" : "theater.status.went_forward", { n: String(Math.abs(settled.by)) }));
    else say(`${playClock(settled.seconds)} / ${playClock(settled.length ?? 0)}`);

    return;
  }

  if (settled.kind === "hold") {
    say(t("theater.status.speed", { speed: speedText(settled.speed) }));

    return;
  }

  say(t(settled.kind === "brightness" ? "theater.status.brightness" : "theater.status.volume", { n: String(Math.round(settled.level * 100)) }));
}

usePlayerGestures({
  surface: () => videoRef.value,
  length: () => length.value,
  speed: () => speed.value,
  volume: () => volume.heard.value,
  setVolume: (level) => volume.slide(level),
  brightness: () => brightness.value,
  setBrightness: (level) => {
    brightness.value = level;
  },
  onTime: (seconds) => {
    current.value = seconds;
  },
  onCue: (next) => {
    cue.value = next;
  },
  onSettled: announceGesture,
  onTap: () => {
    if (!hiddenAtPress) awake.value = false;
  },
  wake,
  blocked: () => overlayOpen(document)
});

function skip(by: number): void {
  const element = videoRef.value;

  if (!element) return;

  element.currentTime = skipTo(element.currentTime, by, length.value);
  current.value = element.currentTime;
  say(t(by < 0 ? "theater.status.went_back" : "theater.status.went_forward", { n: String(Math.abs(by)) }));
}

function seek(value: number | undefined): void {
  const element = videoRef.value;

  if (!element || value === undefined) return;

  element.currentTime = value;
  current.value = value;
}

// The picture's own quarter turns, and the player's size it is fitted to, where the screen's orientation cannot turn it.
const turned = ref(0);
const box = ref({ width: 0, height: 0 });
const turnedStyle = computed(() => turnedBox(turned.value, box.value.width, box.value.height));
// The picture as it stands: turned by its own quarter where the screen cannot turn it, and drawn at the brightness
// a finger on its left half last set.
const videoStyle = computed(() => ({ ...turnedStyle.value, filter: brightness.value === 1 ? undefined : `brightness(${brightness.value})` }));
let boxWatch: ResizeObserver | null = null;

function turnPicture(): void {
  const root = rootRef.value;

  if (root) box.value = { width: root.clientWidth, height: root.clientHeight };

  turned.value = nextTurn(turned.value);
  say(t("theater.status.turned", { degrees: turned.value }));
}

// On a phone the screen turns: full screen first, since a browser locks the orientation only there. A browser that
// offers no lock, or refuses it, turns the picture itself.
async function turn(): Promise<void> {
  // A picture turned by its own quarter comes back at the next press on a phone, where it only ever goes on its side.
  if (narrow.value && turned.value !== 0) {
    turned.value = 0;
    if (document.fullscreenElement) await document.exitFullscreen().catch(() => undefined);

    return;
  }

  const orientation = screen.orientation as (ScreenOrientation & { lock?: (to: string) => Promise<void> }) | undefined;

  if (narrow.value && orientation?.lock) {
    try {
      if (!document.fullscreenElement) await rootRef.value?.requestFullscreen?.();
      await orientation.lock(otherOrientation(orientation.type));
      turned.value = 0;

      return;
    } catch {
      // Falls to turning the picture.
    }
  }

  turnPicture();
}

async function toggleFullscreen(): Promise<void> {
  const root = rootRef.value;
  const element = videoRef.value as (HTMLVideoElement & { webkitEnterFullscreen?: () => void }) | null;

  if (!root) return;

  try {
    if (document.fullscreenElement) await document.exitFullscreen();
    else if (root.requestFullscreen) await root.requestFullscreen();
    else element?.webkitEnterFullscreen?.();
  } catch {
    // A browser that refuses full screen leaves the player where it stands.
  }
}

function onMetadata(): void {
  const element = videoRef.value;

  if (!element) return;

  length.value = knownLength(element.duration);
  element.playbackRate = speed.value;
  element.volume = volume.loudness.value.level;
  element.muted = volume.heard.value === 0;

  if (props.start > 0 && (length.value === null || props.start < length.value)) element.currentTime = props.start;

  current.value = element.currentTime;
  showTrack();

  if (props.autoplay) void element.play().catch(() => undefined);
}

function onTime(): void {
  current.value = videoRef.value?.currentTime ?? 0;
}

let saving: ReturnType<typeof setInterval> | null = null;

function stopSaving(): void {
  if (saving !== null) clearInterval(saving);

  saving = null;
}

function tell(): void {
  if (started.value) emit("position", props.episode, videoRef.value?.currentTime ?? current.value);
}

function onPlay(): void {
  started.value = true;
  playing.value = true;
  wake();
  say(t("common.action.play"));
  stopSaving();
  saving = setInterval(tell, SAVE_EVERY_MS);
}

function onPause(): void {
  playing.value = false;
  stopSaving();
  if (!videoRef.value?.ended) say(t("common.action.pause"));
  tell();
}

function onEnded(): void {
  playing.value = false;
  stopSaving();
  emit("ended", props.episode, length.value ?? current.value);
}

function onKey(event: KeyboardEvent): void {
  if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey) return;

  const target = event.target as HTMLElement | null;

  // A key meant for a field, a control holding it, or something open over the player is left to that.
  if (target?.closest("input, textarea, select, [contenteditable], [role='slider'], [role='menu'], [role='dialog'], [role='listbox']")) return;
  if (document.querySelector("[role='dialog'][data-state='open'], [role='menu'][data-state='open']")) return;
  if ((event.key === " " || event.key === "Enter") && target?.closest("button, a")) return;

  const action = keyAction(event.key);

  if (action === null) return;

  event.preventDefault();
  wake();

  switch (action) {
    case "toggle":
      toggle();
      break;
    case "back":
      skip(-SKIP_SECONDS);
      break;
    case "forward":
      skip(SKIP_SECONDS);
      break;
    case "louder":
    case "quieter": {
      const level = steppedVolume(volume.heard.value, action === "louder" ? VOLUME_STEP : -VOLUME_STEP);

      volume.slide(level);
      say(t("theater.status.volume", { n: String(Math.round(level * 100)) }));
      break;
    }
    case "mute":
      volume.toggleMute();
      say(t(volume.loudness.value.muted ? "theater.status.muted" : "theater.status.unmuted"));
      break;
    case "fullscreen":
      void toggleFullscreen();
      break;
    case "turn":
      void turn();
      break;
    case "captions":
      if (!props.tracks.length) break;

      chooseTrack(toggledTrack(track.value, remembered.value, props.tracks));
      break;
  }
}

function freeOrientation(): void {
  try {
    (screen.orientation as (ScreenOrientation & { unlock?: () => void }) | undefined)?.unlock?.();
  } catch {
    // A browser that locked nothing has nothing to give back.
  }
}

function onFullscreen(): void {
  fullscreen.value = document.fullscreenElement === rootRef.value;

  // Leaving full screen gives the orientation back.
  if (!document.fullscreenElement) {
    freeOrientation();
    turned.value = 0;
  }
}

function onLeave(): void {
  if (started.value) emit("leaving", props.episode, videoRef.value?.currentTime ?? current.value);
}

let release: (() => void) | null = null;

onMounted(() => {
  window.addEventListener("keydown", onKey);
  window.addEventListener("pagehide", onLeave);
  document.addEventListener("fullscreenchange", onFullscreen);
  release = holdThroughFullscreen(document);
  boxWatch = new ResizeObserver(() => {
    const root = rootRef.value;

    if (root) box.value = { width: root.clientWidth, height: root.clientHeight };
  });
  if (rootRef.value) boxWatch.observe(rootRef.value);
});

onBeforeUnmount(() => {
  tell();
  stopSaving();
  window.removeEventListener("keydown", onKey);
  window.removeEventListener("pagehide", onLeave);
  document.removeEventListener("fullscreenchange", onFullscreen);
  // Going back from a turned player leaves no lock and no full screen behind on the screen it goes to.
  freeOrientation();
  if (document.fullscreenElement === rootRef.value) void document.exitFullscreen().catch(() => undefined);
  release?.();
  boxWatch?.disconnect();

  if (idle !== null) clearTimeout(idle);
});
</script>

<style scoped>
/* The seek and volume bars stand on the player's own black ground: the done part in the accent and the rest in
   one dimmed light, meeting at a hard edge. */
.theater-seek :deep([data-slot="track"]) {
  background-color: rgb(255 255 255 / 0.3);
}

/* A finger on the picture belongs to the player's gestures: the page under it neither scrolls nor selects, and the
   phone's own menu does not come up while a finger rests. */
.theater-picture {
  touch-action: none;
  user-select: none;
  -webkit-user-select: none;
  -webkit-touch-callout: none;
}

/* The cue is a thing that comes and goes on its own, so it fades where it stands. */
.theater-cue-enter-active,
.theater-cue-leave-active {
  transition: opacity var(--bmo-motion-quick) var(--bmo-ease-move);
}

.theater-cue-enter-from,
.theater-cue-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .theater-cue-enter-active,
  .theater-cue-leave-active {
    transition: none;
  }
}

.theater-player video::cue {
  font-size: 1.1em;
}
</style>
