<template>
  <!-- The player's screen (`docs/design.md`, 「The console」): the episode playing over its stream, from where the
       viewer stopped, its position set every few seconds; under it what it is and what is done with it; and the
       list — the playlist's episodes where it was opened from one, else the next episode and the part of the title
       the one playing stands in. On a wide screen the list stands in a column of its own beside the player, or under
       it in theater mode; on a narrow one it stands under the player across the whole width. The player stays
       standing from one episode to the next, so its mode, full screen included, goes on with it. -->
  <TheaterFrame id="theater-watch">
    <ToolBar :label="$t('theater.aria.watch_bar')">
      <div>
        <IconButton size="sm" icon="i-material-symbols-arrow-back" :to="backPath" :label="backWords" />
      </div>
      <div class="min-w-0 flex-1 shrink!">
        <span class="truncate text-muted">{{ playlist?.name ?? title?.name ?? "" }}</span>
      </div>
      <ActionGroup v-if="actions.length > 0" :actions="actions" :narrow-fold="false" class="ms-auto" />
    </ToolBar>

    <DefineList>
      <section v-if="playlist" :aria-label="playlist.name" class="pb-4" data-watch-section>
        <h2 class="flex h-11 items-center gap-3 px-4 text-lg font-medium text-highlighted">
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

      <template v-else-if="section">
        <section v-if="nextEpisode" :aria-label="$t('theater.label.next_episode')" class="pb-4" data-watch-next>
          <h2 class="flex h-11 items-center px-4 text-lg font-medium text-highlighted">{{ $t("theater.label.next_episode") }}</h2>
          <ul class="flex flex-col">
            <EpisodeRow :episode="nextEpisode" :watched="watchedEpisode(nextEpisode)" :to="`/theater/watch/${nextEpisode.id}`" :poster="posterOf(nextEpisode)">
              <template #trail><span class="w-0" /></template>
            </EpisodeRow>
          </ul>
        </section>

        <section :aria-label="section.name(t)" class="pb-4" data-watch-section>
          <h2 class="flex h-11 items-center gap-3 px-4 text-lg font-medium text-highlighted">
            <span class="truncate">{{ section.name(t) }}</span>
            <span class="shrink-0 text-sm font-normal text-muted tabular-nums">{{ countText(sectionPlace + 1) }}/{{ countText(section.episodes.length) }}</span>
          </h2>
          <ul class="flex flex-col">
            <EpisodeRow v-for="one in section.episodes" :key="one.id" :episode="one" :watched="watchedEpisode(one)" :to="`/theater/watch/${one.id}`" :poster="posterOf(one)" :current="one.id === episodeId">
              <template #trail><span class="w-0" /></template>
            </EpisodeRow>
          </ul>
        </section>
      </template>

      <ContentSkeleton v-else-if="listReading" list-class="flex flex-col" :rows="4" :label="$t('theater.status.episodes_loading')">
        <div class="flex items-center gap-3 px-4 py-2">
          <SkeletonBar class="aspect-video w-24 shrink-0 rounded-md sm:w-36" />
          <div class="flex flex-1 flex-col gap-1"><SkeletonLine size="base" width="w-2/3" /><SkeletonLine width="w-1/3" /></div>
        </div>
      </ContentSkeleton>
    </DefineList>

    <!-- One splitter at every width and in every mode, the player always the first thing in its main region, so
         neither the width crossing nor a change of mode draws the player again. -->
    <div ref="room" class="min-h-0 flex-1" :data-watch-layout="layout">
      <USplitter id="theater-watch" auto-save-id="theater-watch" :items="regions" :ui="{ handle: SPLIT_HANDLE }" class="h-full min-h-0">
        <template #main>
          <div ref="main" class="watch-main flex h-full min-h-0 w-full min-w-0 flex-col" :class="layout === 'theater' ? 'overflow-y-auto' : 'overflow-hidden'" data-watch-main>
            <PlayerStage v-model:track="track" class="theater-stage" :failed="failed" :player="player" :controls="controls" @position="keep" @leaving="keepLeaving" @ended="ended" @next="goNext(true)" />

            <div ref="rest" class="min-w-0" :class="layout === 'theater' ? '' : 'min-h-0 flex-1 overflow-y-auto'" data-watch-rest>
              <header class="flex flex-col gap-1 px-4 py-4">
                <h1 v-if="episode" class="text-xl font-semibold text-highlighted">{{ episodeTitle(episode) }}</h1>
                <SkeletonLine v-else size="lg" width="w-1/2" />
                <p v-if="episode && title" class="text-sm text-muted">
                  <RouterLink :to="`/theater/titles/${title.id}`" class="rounded-sm underline-offset-4 hover:text-highlighted hover:underline active:text-default pointer-coarse:inline-flex pointer-coarse:min-h-11 pointer-coarse:items-center">{{ title.name }}</RouterLink>
                  <template v-if="episode.label"> · {{ episodeLabel(t, episode) }}</template>
                </p>
              </header>

              <UseList v-if="layout !== 'normal'" />
            </div>
          </div>
        </template>

        <template #list>
          <div ref="column" class="h-full min-h-0 w-full min-w-0 overflow-y-auto pt-2" data-watch-list>
            <UseList />
          </div>
        </template>
      </USplitter>
    </div>

    <SubtitleDialog v-model:open="subtitlesOpen" :episode="episode" />
    <AddToPlaylistDialog v-if="episode" v-model:open="addingOpen" :episode-ids="[episode.id]" :realm="episode.realm" :subject="episode.name" />
    <ShareDialog v-if="episode" v-model:open="sharingOpen" :road="road" :spans="spans" :span="FIRST_SPAN" :members="members" :subject="episode.name" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, inject, nextTick, ref, useTemplateRef, watch } from "vue";
import { RouterLink } from "vue-router";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import { createReusableTemplate } from "@vueuse/core";
import ActionGroup from "bmo-ui/app/components/ActionGroup.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import ShareDialog from "bmo-ui/app/components/ShareDialog.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { countText } from "bmo-ui/app/domain/numbers";
import { relayCall } from "bmo-ui/app/domain/relay";
import { useWidth } from "bmo-ui/app/domain/screen";
import { SPLIT_HANDLE } from "bmo-ui/app/domain/side-parts";

import AddToPlaylistDialog from "../../../components/theater/AddToPlaylistDialog.vue";
import EpisodeRow from "../../../components/theater/EpisodeRow.vue";
import PlayerStage from "../../../components/theater/PlayerStage.vue";
import SubtitleDialog from "../../../components/theater/SubtitleDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import { episodeLabel } from "../../../domain/a11y";
import { paths } from "../../../domain/api";
import { episodeTitle, followingOf, placeAfter, placeOf, sectionOf, sectionsOf, watchedEpisode } from "../../../domain/episodes";
import { firstTrack, positionMoved, startAt, trackName } from "../../../domain/player";
import { CONSOLE_PEOPLE, FIRST_SPAN, SHARE_SPANS, memberRoad, shareRoad } from "../../../domain/share";
import { refusalWord } from "../../../domain/store";
import { useAutoplay } from "../../../composables/useAutoplay";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { usePlayMode } from "../../../composables/usePlayMode";
import { SERVICE, theaterAddress, useTheater } from "../../../composables/useTheater";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { SplitterItem } from "@nuxt/ui";
import type { PlayerControl, PlayerTrack } from "bmo-ui/app/domain/video";
import type { StagePlayer } from "../../../components/theater/PlayerStage.vue";
import type { ConsolePeople } from "../../../domain/share";
import type { Episode, Playlist } from "../../../../shared/types";

// The screen stays standing as the address moves from one episode to the next, so its player goes on with it.
definePageMeta({ key: "theater-watch" });

// The room a wide screen needs to stand the player beside the list (`docs/design.md`, 「The console」): 60rem.
const WIDE_PX = 960;

// The list's region beside the player, in shares of the screen's width: about a third, from a quarter to a half.
const LIST_REGION = { minSize: 25, defaultSize: 34, maxSize: 50 } as const;

const route = useRoute();
const { t, locale } = useI18n();
const store = useTheater();
const { autoplay, setAutoplay } = useAutoplay();
const { mode, setMode } = usePlayMode();
// The list is drawn in one of two places, its own column or under what plays, from one template.
const [DefineList, UseList] = createReusableTemplate();

const roomRef = useTemplateRef<HTMLElement>("room");
const mainRef = useTemplateRef<HTMLElement>("main");
const restRef = useTemplateRef<HTMLElement>("rest");
const columnRef = useTemplateRef<HTMLElement>("column");
// The screen's own width, never the window's; read as wide until it is measured, so a desktop's first frame
// stands as it will stay.
const roomWidth = useWidth(() => roomRef.value, Number.POSITIVE_INFINITY);

const playlist = ref<Playlist | null>(null);
const track = ref<number | null>(null);
const failed = ref("");
const tracksRead = ref(false);
// Whether the episode and what lists around it are still being read.
const reading = ref(true);
const subtitlesOpen = ref(false);
const addingOpen = ref(false);
const sharingOpen = ref(false);

// The last position set, and whose: a player being left tells of its own episode after the next has opened.
let lastKept: { episode: number; seconds: number } | null = null;

const episodeId = computed(() => Number(route.params.id));
const episode = computed<Episode | null>(() => store.episodeOf(episodeId.value) ?? null);
// An episode watched by a grant opens nothing but watching (`docs/design.md`, 「Grants」), so no action stands.
const granted = computed(() => episode.value?.granted === true);
const actions = computed<BarAction[]>(() => [autoplayAction.value, ...(granted.value ? [] : ownActions.value)]);
// Whether the next episode follows on its own: the viewer's own switch, standing pressed while on.
const autoplayAction = computed<BarAction>(() => ({ id: "autoplay", label: t("theater.action.autoplay"), icon: "i-material-symbols-autoplay", pressed: autoplay.value, onSelect: () => setAutoplay(!autoplay.value) }));
const ownActions = computed<BarAction[]>(() => [
  { id: "subtitles", label: t("theater.action.subtitles"), icon: "i-material-symbols-subtitles-outline", disabled: !episode.value, onSelect: () => (subtitlesOpen.value = true) },
  { id: "add-to-playlist", label: t("theater.action.add_to_playlist"), icon: "i-material-symbols-playlist-add", disabled: !episode.value, onSelect: () => (addingOpen.value = true) },
  { id: "share", label: t("theater.action.share"), icon: "i-material-symbols-share-outline", disabled: !episode.value, onSelect: () => (sharingOpen.value = true) }
]);
const title = computed(() => (episode.value ? (store.titleOf(episode.value.titleId) ?? null) : null));
const askedPlace = computed(() => (route.query.at === undefined ? null : Number(route.query.at)));
const at = computed(() => (playlist.value ? placeOf(playlist.value.episodeIds, episodeId.value, askedPlace.value) : null));
const startsPlaying = computed(() => route.query.play === "1");

const tracks = computed<PlayerTrack[]>(() =>
  (store.subtitles.value[episodeId.value] ?? []).map((one) => ({ id: one.id, src: theaterAddress(paths.vtt(one.id)), label: trackName(one, locale.value), language: one.language }))
);

// Where it starts: from the start where the title's page asked for it, the second the address names, else where
// the viewer stopped.
const start = computed(() => {
  if (route.query.from === "start") return 0;

  const asked = Number(route.query.t);

  if (Number.isFinite(asked) && asked > 0) return asked;

  return startAt(episode.value?.position ?? null, episode.value?.durationMs ? episode.value.durationMs / 1000 : null);
});

const nextPlace = computed(() => (playlist.value && at.value !== null ? placeAfter(playlist.value.episodeIds, at.value) : null));
const nextEpisode = computed<Episode | null>(() => {
  if (playlist.value) return nextPlace.value === null ? null : (store.episodeOf(playlist.value.episodeIds[nextPlace.value]!) ?? null);

  // A show goes on along its main line, specials one after another, extras to nothing.
  const list = episode.value ? store.episodes.value[episode.value.titleId] : undefined;

  return list && title.value ? followingOf(sectionsOf(title.value.kind, list), episodeId.value) : null;
});
const nextWords = computed(() => (nextEpisode.value ? t("theater.action.next_named", { name: episodeTitle(nextEpisode.value) }) : null));

// The part of the title the episode playing stands in, where it was not opened from a playlist.
const section = computed(() => {
  if (playlist.value || !episode.value || !title.value) return null;

  const list = store.episodes.value[episode.value.titleId];

  return list ? sectionOf(sectionsOf(title.value.kind, list), episode.value.id) : null;
});
const sectionPlace = computed(() => section.value?.episodes.findIndex((one) => one.id === episodeId.value) ?? -1);
// An episode watched by a grant has no list, since its title is not read.
const listed = computed(() => playlist.value !== null || !granted.value);
const listReading = computed(() => listed.value && reading.value);

const wide = computed(() => roomWidth.value >= WIDE_PX);
// How the screen stands: the player beside the list, the player across the whole width, or a phone's.
const layout = computed<"normal" | "theater" | "narrow">(() => {
  if (!wide.value) return "narrow";

  return mode.value === "theater" || !listed.value ? "theater" : "normal";
});
const regions = computed<SplitterItem[]>(() =>
  layout.value === "normal"
    ? [
        { id: "theater-watch-main", slot: "main", order: 1, minSize: 100 - LIST_REGION.maxSize },
        { id: "theater-watch-list", slot: "list", order: 2, ...LIST_REGION }
      ]
    : [{ id: "theater-watch-main", slot: "main", order: 1, minSize: 100, defaultSize: 100 }]
);
// The theater mode's control in the player's bar, standing only where the two modes lay the screen out apart, and
// naming the mode a press goes to.
const controls = computed<PlayerControl[]>(() => {
  if (!wide.value || !listed.value) return [];

  const theater = mode.value === "theater";

  return [
    {
      id: "play-mode",
      label: t(theater ? "theater.action.normal_mode" : "theater.action.theater_mode"),
      icon: theater ? "i-material-symbols-width-normal-outline" : "i-material-symbols-width-wide-outline",
      key: "t",
      onSelect: () => setMode(theater ? "normal" : "theater")
    }
  ];
});

// The episode ready for the stage, once it and its subtitles are read.
const ready = computed<StagePlayer | null>(() => {
  const shown = episode.value;

  if (!shown || !tracksRead.value) return null;

  return {
    episode: shown.id,
    src: theaterAddress(paths.stream(shown.id)),
    poster: shown.hasPoster ? theaterAddress(paths.episodePoster(shown.id)) : null,
    title: episodeTitle(shown),
    tracks: tracks.value,
    start: start.value,
    autoplay: startsPlaying.value,
    next: nextWords.value,
    autoNext: autoplay.value
  };
});
// The episode on the stage. While the next one is read the one before keeps standing, so the player is never taken
// down between them and full screen goes on.
const player = ref<StagePlayer | null>(null);

watch(ready, (next) => {
  if (next !== null) player.value = next;
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
  reading.value = true;

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
  } finally {
    reading.value = false;
  }
}

function posterOf(shown: Episode): string | null {
  return shown.hasPoster ? theaterAddress(paths.episodePoster(shown.id)) : null;
}

// Another episode opens at the top of what it is, and the list beside it brings the one playing into view.
watch(episodeId, async () => {
  await nextTick();

  if (mainRef.value) mainRef.value.scrollTop = 0;
  if (restRef.value) restRef.value.scrollTop = 0;
});

watch([section, playlist, layout], async () => {
  await nextTick();

  columnRef.value?.querySelector("[aria-current='true']")?.scrollIntoView({ block: "nearest" });
});

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

// The end counts the episode watched; what plays next is asked for by the player where the viewer leaves autoplay
// on and did not cancel it.
function ended(episodeOf: number, seconds: number): void {
  lastKept = { episode: episodeOf, seconds };
  void store.setPosition(episodeOf, seconds);
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
/* The player is as tall as its picture at the width of the region it stands in, never so tall that less than a
   sixth of the region stays under it; a phone on its side gives it the whole region under the bars. */
.watch-main {
  container-type: inline-size;
}

.theater-stage {
  height: min(56.25cqw, 83.333%);
}

@media (orientation: landscape) and (max-height: 32rem) {
  .theater-stage {
    height: 100%;
  }
}
</style>
