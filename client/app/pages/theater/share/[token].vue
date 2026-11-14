<template>
  <!-- What a share link opens, to whoever holds it with no sign-in (`docs/api.md`, 「Share links」): the one episode,
       or the playlist's episodes one after another, in the same player as the console's. It reads only what the
       token opens and sets no position: a reader here is nobody's account. No control leads anywhere else in
       the console. -->
  <ScreenFrame class="bg-default">
    <section class="flex min-h-0 flex-1 flex-col" :aria-label="heading || $t('theater.title')">
      <PlayerStage
        :key="`${episode?.id}:${at}`"
        v-model:track="track"
        class="theater-share-stage"
        :failed="failed"
        :player="player"
        @ended="ended"
        @next="go(at + 1, true)"
      />

      <div class="min-h-0 flex-1 overflow-y-auto">
        <header v-if="!failed" class="mx-auto flex max-w-3xl flex-col gap-1 px-4 py-4">
          <h1 v-if="episode" class="text-xl font-semibold text-highlighted">{{ episodeTitle(episode) }}</h1>
          <SkeletonLine v-else size="lg" width="w-1/2" />
          <p v-if="playlist" class="text-sm text-muted tabular-nums">{{ playlist.name }} · {{ countText(at + 1) }}/{{ countText(episodes.length) }}</p>
        </header>

        <ul v-if="playlist && episodes.length > 1" class="mx-auto flex max-w-3xl flex-col pb-4" :aria-label="playlist.name">
          <li v-for="(one, index) in episodes" :key="`${one.id}:${index}`" class="px-2 sm:px-4">
            <button
              type="button"
              class="flex min-h-11 w-full min-w-0 items-center gap-3 rounded-md p-2 text-start transition-colors select-none hover:bg-elevated/60 active:bg-elevated focus-inset"
              :class="index === at ? 'bg-elevated/60' : ''"
              :aria-current="index === at ? 'true' : undefined"
              :aria-label="episodeName($t, one, null)"
              @click="go(index, true)"
            >
              <span class="w-24 shrink-0 sm:w-36">
                <TheaterPoster :src="one.hasPoster ? theaterAddress(paths.shared.poster(token, one.id)) : null" />
              </span>
              <span class="min-w-0 flex-1 truncate text-base text-highlighted">{{ episodeTitle(one) }}</span>
              <span class="shrink-0 text-sm text-muted tabular-nums">{{ lengthClock(one.durationMs) }}</span>
            </button>
          </li>
        </ul>
      </div>
    </section>
  </ScreenFrame>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useHead, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ScreenFrame from "bmo-ui/app/components/ScreenFrame.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { countText } from "bmo-ui/app/domain/numbers";

import TheaterPoster from "../../../components/theater/TheaterPoster.vue";
import PlayerStage from "../../../components/theater/PlayerStage.vue";
import { episodeName } from "../../../domain/a11y";
import { paths } from "../../../domain/api";
import { episodeTitle, lengthClock, placeAfter } from "../../../domain/episodes";
import { readShared, sharedFailure } from "../../../domain/shared";
import { firstTrack, trackName } from "../../../domain/player";
import { theaterAddress, theaterApi } from "../../../composables/useTheater";

import type { PlayerTrack } from "bmo-ui/app/domain/video";
import type { StagePlayer } from "../../../components/theater/PlayerStage.vue";
import type { SharedOpening } from "../../../domain/shared";
import type { Subtitle } from "../../../../shared/types";

// Standing alone, with no frame of the console's around it.
definePageMeta({ layout: false });

const route = useRoute();
const { t, locale } = useI18n();
const api = theaterApi();

const opening = ref<SharedOpening | null>(null);
const subtitles = ref<Subtitle[]>([]);
const track = ref<number | null>(null);
const failed = ref("");
const at = ref(0);
const tracksRead = ref(false);
const autoplay = ref(false);

const token = computed(() => String(route.params.token ?? ""));
const playlist = computed(() => opening.value?.playlist ?? null);
const episodes = computed(() => opening.value?.episodes ?? []);
const episode = computed(() => episodes.value[at.value] ?? null);
const heading = computed(() => playlist.value?.name ?? (episode.value ? episodeTitle(episode.value) : ""));
const tracks = computed<PlayerTrack[]>(() => subtitles.value.map((one) => ({ id: one.id, src: theaterAddress(paths.shared.vtt(token.value, one.id)), label: trackName(one, locale.value), language: one.language })));
const nextPlace = computed(() => placeAfter(episodes.value.map((one) => one.id), at.value));
const nextWords = computed(() => {
  const next = nextPlace.value === null ? null : episodes.value[nextPlace.value];

  return next ? t("theater.action.next_named", { name: episodeTitle(next) }) : null;
});

// The episode on the stage, once it and its subtitles are read. A reader here starts each episode at its start.
const player = computed<StagePlayer | null>(() => {
  const shown = episode.value;

  if (!shown || !tracksRead.value) return null;

  return {
    episode: shown.id,
    src: theaterAddress(paths.shared.stream(token.value, shown.id)),
    poster: shown.hasPoster ? theaterAddress(paths.shared.poster(token.value, shown.id)) : null,
    title: episodeTitle(shown),
    tracks: tracks.value,
    autoplay: autoplay.value,
    next: nextWords.value
  };
});

useHead({ title: computed(() => heading.value || t("theater.title")) });

watch(token, () => void open(), { immediate: true });

watch(episode, async (shown) => {
  tracksRead.value = false;

  if (!shown) return;

  try {
    subtitles.value = await api.shared.subtitles(token.value, shown.id);
  } catch {
    subtitles.value = [];
  }

  track.value = firstTrack(subtitles.value, locale.value);
  tracksRead.value = true;
});

async function open(): Promise<void> {
  failed.value = "";
  at.value = 0;

  try {
    opening.value = await readShared(api.shared, token.value);

    if (!opening.value.episodes.length) failed.value = t("theater.error.share_empty");
  } catch (error) {
    failed.value = t(sharedFailure(error));
  }
}

function go(place: number, play: boolean): void {
  if (place < 0 || place >= episodes.value.length) return;

  autoplay.value = play;
  at.value = place;
}

// A playlist goes on to its next episode by itself; nothing is set for the reader.
function ended(): void {
  if (nextPlace.value !== null) go(nextPlace.value, true);
}
</script>

<style scoped>
.theater-share-stage {
  height: min(56.25vw, calc(100dvh - 8rem));
}

@media (orientation: landscape) and (max-height: 32rem) {
  .theater-share-stage {
    height: 100dvh;
  }
}
</style>
