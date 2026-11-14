<template>
  <!-- The player's screen (`docs/design.md`, 「The console」): the episode playing over its stream, from where the
       viewer stopped, its position set every few seconds; under it what it is, what is done with it, and what
       comes next — the playlist's episodes where it was opened from one, which then follow on their own. -->
  <TheaterFrame id="theater-watch">
    <ToolBar :label="$t('theater.aria.watch_bar')">
      <div>
        <UTooltip :text="backWords">
          <UButton color="neutral" variant="ghost" size="sm" square icon="i-material-symbols-arrow-back" :aria-label="backWords" :to="backPath" />
        </UTooltip>
      </div>
      <div class="min-w-0 flex-1 shrink!">
        <span class="truncate text-muted">{{ playlist?.name ?? title?.name ?? "" }}</span>
      </div>
      <ActionGroup v-if="actions.length > 0" :actions="actions" :narrow-fold="false" class="ms-auto" />
    </ToolBar>

    <PlayerStage
      :key="`${episodeId}:${at}`"
      v-model:track="track"
      class="theater-stage"
      :failed="failed"
      :player="player"
      @position="keep"
      @leaving="keepLeaving"
      @ended="ended"
      @next="goNext(true)"
    />

    <div class="min-h-0 flex-1 overflow-y-auto">
      <header class="flex flex-col gap-1 px-4 py-4">
        <h1 v-if="episode" class="text-xl font-semibold text-highlighted">{{ episodeTitle(episode) }}</h1>
        <SkeletonLine v-else size="lg" width="w-1/2" />
        <p v-if="episode && title" class="text-sm text-muted">
          <RouterLink :to="`/theater/titles/${title.id}`" class="rounded-sm underline-offset-4 hover:text-highlighted hover:underline active:text-default pointer-coarse:inline-flex pointer-coarse:min-h-11 pointer-coarse:items-center">{{ title.name }}</RouterLink>
          <template v-if="episode.label"> · {{ episodeLabel(t, episode) }}</template>
        </p>
      </header>

      <section v-if="playlist" :aria-label="playlist.name" class="pb-4">
        <h2 class="flex items-center gap-3 px-4 pb-2 text-lg font-medium text-highlighted">
          <span class="truncate">{{ playlist.name }}</span>
          <span class="shrink-0 text-sm font-normal text-muted tabular-nums">{{ countText((at ?? 0) + 1) }}/{{ countText(playlist.episodeIds.length) }}</span>
        </h2>
        <ul class="flex flex-col">
          <template v-for="(id, index) in playlist.episodeIds" :key="`${id}:${index}`">
            <EpisodeRow
              v-if="store.episodeOf(id)"
              :episode="store.episodeOf(id)!"
              :watched="watchedEpisode(store.episodeOf(id)!)"
              :to="{ path: `/theater/watch/${id}`, query: { playlist: String(playlist.id), at: String(index) } }"
              :poster="store.episodeOf(id)!.hasPoster ? theaterAddress(paths.episodePoster(id)) : null"
              :current="index === at"
            >
              <template #trail><span class="w-0" /></template>
            </EpisodeRow>
          </template>
        </ul>
      </section>

      <section v-else-if="nextEpisode" :aria-label="$t('theater.label.next_episode')" class="pb-4">
        <h2 class="px-4 pb-2 text-lg font-medium text-highlighted">{{ $t("theater.label.next_episode") }}</h2>
        <ul class="flex flex-col">
          <EpisodeRow
            :episode="nextEpisode"
            :watched="watchedEpisode(nextEpisode)"
            :to="`/theater/watch/${nextEpisode.id}`"
            :poster="nextEpisode.hasPoster ? theaterAddress(paths.episodePoster(nextEpisode.id)) : null"
          >
            <template #trail><span class="w-0" /></template>
          </EpisodeRow>
        </ul>
      </section>
    </div>

    <SubtitleDialog v-model:open="subtitlesOpen" :episode="episode" />
    <AddToPlaylistDialog v-if="episode" v-model:open="addingOpen" :episode-ids="[episode.id]" :realm="episode.realm" :subject="episode.name" />
    <ShareDialog v-if="episode" v-model:open="sharingOpen" :road="road" :spans="spans" :span="FIRST_SPAN" :members="members" :subject="episode.name" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, inject, ref, watch } from "vue";
import { RouterLink } from "vue-router";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ActionGroup from "bmo-ui/app/components/ActionGroup.vue";
import ShareDialog from "bmo-ui/app/components/ShareDialog.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { countText } from "bmo-ui/app/domain/numbers";
import { relayCall } from "bmo-ui/app/domain/relay";

import AddToPlaylistDialog from "../../../components/theater/AddToPlaylistDialog.vue";
import EpisodeRow from "../../../components/theater/EpisodeRow.vue";
import PlayerStage from "../../../components/theater/PlayerStage.vue";
import SubtitleDialog from "../../../components/theater/SubtitleDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import { episodeLabel } from "../../../domain/a11y";
import { paths } from "../../../domain/api";
import { episodeAfter, episodeTitle, placeAfter, placeOf, watchedEpisode } from "../../../domain/episodes";
import { firstTrack, positionMoved, startAt, trackName } from "../../../domain/player";
import { CONSOLE_PEOPLE, FIRST_SPAN, SHARE_SPANS, memberRoad, shareRoad } from "../../../domain/share";
import { refusalWord } from "../../../domain/store";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { SERVICE, theaterAddress, useTheater } from "../../../composables/useTheater";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { PlayerTrack } from "bmo-ui/app/domain/video";
import type { StagePlayer } from "../../../components/theater/PlayerStage.vue";
import type { ConsolePeople } from "../../../domain/share";
import type { Episode, Playlist } from "../../../../shared/types";

const route = useRoute();
const { t, locale } = useI18n();
const store = useTheater();

const playlist = ref<Playlist | null>(null);
const track = ref<number | null>(null);
const failed = ref("");
const tracksRead = ref(false);
const subtitlesOpen = ref(false);
const addingOpen = ref(false);
const sharingOpen = ref(false);

// The last position set, and whose: a player being left tells of its own episode after the next has opened.
let lastKept: { episode: number; seconds: number } | null = null;

const episodeId = computed(() => Number(route.params.id));
const episode = computed<Episode | null>(() => store.episodeOf(episodeId.value) ?? null);
// An episode watched by a grant opens nothing but watching (`docs/design.md`, 「Grants」), so no action stands.
const granted = computed(() => episode.value?.granted === true);
const actions = computed<BarAction[]>(() => (granted.value ? [] : ownActions.value));
const ownActions = computed<BarAction[]>(() => [
  { id: "subtitles", label: t("theater.action.subtitles"), icon: "i-material-symbols-subtitles-outline", disabled: !episode.value, onSelect: () => (subtitlesOpen.value = true) },
  { id: "add-to-playlist", label: t("theater.action.add_to_playlist"), icon: "i-material-symbols-playlist-add", disabled: !episode.value, onSelect: () => (addingOpen.value = true) },
  { id: "share", label: t("theater.action.share"), icon: "i-material-symbols-share-outline", disabled: !episode.value, onSelect: () => (sharingOpen.value = true) }
]);
const title = computed(() => (episode.value ? (store.titleOf(episode.value.titleId) ?? null) : null));
const askedPlace = computed(() => (route.query.at === undefined ? null : Number(route.query.at)));
const at = computed(() => (playlist.value ? placeOf(playlist.value.episodeIds, episodeId.value, askedPlace.value) : null));
const autoplay = computed(() => route.query.play === "1");

const tracks = computed<PlayerTrack[]>(() =>
  (store.subtitles.value[episodeId.value] ?? []).map((one) => ({ id: one.id, src: theaterAddress(paths.vtt(one.id)), label: trackName(one, locale.value), language: one.language }))
);

// Where it starts: the second the address names, else where the viewer stopped.
const start = computed(() => {
  const asked = Number(route.query.t);

  if (Number.isFinite(asked) && asked > 0) return asked;

  return startAt(episode.value?.position ?? null, episode.value?.durationMs ? episode.value.durationMs / 1000 : null);
});

const nextPlace = computed(() => (playlist.value && at.value !== null ? placeAfter(playlist.value.episodeIds, at.value) : null));
const nextEpisode = computed<Episode | null>(() => {
  if (playlist.value) return nextPlace.value === null ? null : (store.episodeOf(playlist.value.episodeIds[nextPlace.value]!) ?? null);

  const list = episode.value ? store.episodes.value[episode.value.titleId] : undefined;
  const after = list ? episodeAfter(list, episodeId.value) : null;

  return after === null ? null : (store.episodeOf(after) ?? null);
});
const nextWords = computed(() => (nextEpisode.value ? t("theater.action.next_named", { name: episodeTitle(nextEpisode.value) }) : null));

// The episode on the stage, once it and its subtitles are read.
const player = computed<StagePlayer | null>(() => {
  const shown = episode.value;

  if (!shown || !tracksRead.value) return null;

  return {
    episode: shown.id,
    src: theaterAddress(paths.stream(shown.id)),
    poster: shown.hasPoster ? theaterAddress(paths.episodePoster(shown.id)) : null,
    title: episodeTitle(shown),
    tracks: tracks.value,
    start: start.value,
    autoplay: autoplay.value,
    next: nextWords.value
  };
});

const backPath = computed(() => {
  if (playlist.value) return `/theater/playlists/${playlist.value.id}`;

  return episode.value && !granted.value ? `/theater/titles/${episode.value.titleId}` : "/theater";
});
const backWords = computed(() => {
  if (playlist.value) return t("theater.label.playlist");

  return granted.value ? t("theater.label.library") : t("theater.label.title");
});

const road = computed(() => shareRoad(store.shareRoad({ episodeId: episodeId.value })));
// People are shared with only where the console offers its directory, and only an episode of the reader's own.
const people = inject<ConsolePeople | null>(CONSOLE_PEOPLE, null);
const members = computed(() => (people && episode.value?.realm === "mine" && !granted.value ? memberRoad(people, store.grantRoad(episodeId.value)) : undefined));
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

watch([episodeId, () => route.query.playlist], () => void load(), { immediate: true });

async function load(): Promise<void> {
  failed.value = "";
  tracksRead.value = false;

  try {
    const read = await store.loadEpisode(episodeId.value);
    const playlistId = Number(route.query.playlist);

    await Promise.all([
      // An episode watched by a grant opens nothing of its title, so its title is not asked for.
      read.granted || store.titleOf(read.titleId) ? Promise.resolve() : store.loadTitle(read.titleId).then(() => undefined),
      store.loadSubtitles(read.id).catch(() => []),
      Number.isSafeInteger(playlistId) && playlistId > 0 ? loadPlaylist(playlistId) : Promise.resolve((playlist.value = null))
    ]);

    track.value = firstTrack(store.subtitles.value[read.id] ?? [], locale.value);
    tracksRead.value = true;

    if (!playlist.value && !read.granted && !store.episodes.value[read.titleId]) await store.loadEpisodes(read.titleId).catch(() => undefined);
  } catch (error) {
    failed.value = t(refusalWord(error));
  }
}

async function loadPlaylist(id: number): Promise<void> {
  try {
    playlist.value = store.playlistOf(id) ?? (await store.loadPlaylist(id));
    await store.loadEpisodesByIds(playlist.value.episodeIds);
  } catch {
    playlist.value = null;
  }
}

function keep(episodeOf: number, seconds: number): void {
  if (lastKept?.episode === episodeOf && !positionMoved(lastKept.seconds, seconds)) return;

  lastKept = { episode: episodeOf, seconds };
  void store.setPosition(episodeOf, seconds);
}

// A position set as the page goes away, by the one call that outlives the page.
function keepLeaving(episodeOf: number, seconds: number): void {
  void relayCall(SERVICE, paths.position(episodeOf), { method: "PUT", keepalive: true, body: { seconds } }).catch(() => undefined);
}

// The end counts the episode watched; a playlist goes on to its next by itself.
function ended(episodeOf: number, seconds: number): void {
  lastKept = { episode: episodeOf, seconds };
  void store.setPosition(episodeOf, seconds);

  if (playlist.value && nextEpisode.value) goNext(true);
}

function goNext(play: boolean): void {
  const next = nextEpisode.value;

  if (!next) return;

  const query: Record<string, string> = play ? { play: "1" } : {};

  if (playlist.value && nextPlace.value !== null) Object.assign(query, { playlist: String(playlist.value.id), at: String(nextPlace.value) });

  void navigateTo({ path: `/theater/watch/${next.id}`, query });
}

// A stream opened afresh reads the episode again, leaving the player playing.
// The share dialog is the layer's; where the focus goes as it comes and goes is said here.
useDialogFocus(sharingOpen);

useTheaterEvents(() => void store.loadEpisode(episodeId.value).catch(() => undefined));

// The playlist kept as the stream tells it changed.
watch(
  () => (playlist.value ? store.playlistOf(playlist.value.id) : null),
  (changed) => {
    if (changed) {
      playlist.value = changed;
      void store.loadEpisodesByIds(changed.episodeIds);
    }
  }
);
</script>

<style scoped>
/* The player is as tall as its picture at the screen's width, leaving room under it; a phone on its side gives
   it the screen less the bars above it. */
.theater-stage {
  height: min(56.25vw, calc(100dvh - 12rem));
}

@media (orientation: landscape) and (max-height: 32rem) {
  .theater-stage {
    height: calc(100dvh - 5.5rem);
  }
}
</style>
