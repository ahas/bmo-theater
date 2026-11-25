<template>
  <!-- One title (`docs/design.md`, 「The console」, a title): a band of its poster dimmed under its poster, its name,
       one line of what it is, its actions — play or go on with its up next, from the start, add to a playlist,
       share — and its description; then a show's seasons, one chosen at a time with the specials last, and its
       extras apart after them, or the episodes of any other title holding more than one. -->
  <TheaterFrame id="theater-title">
    <ToolBar :label="$t('theater.aria.title_bar')">
      <div>
        <IconButton size="sm" icon="i-material-symbols-arrow-back" :label="$t('theater.label.library')" :to="backPath" />
      </div>

      <ActionGroup :actions="actions" :narrow-fold="false" class="ms-auto" />
      <CreateButton :label="$t('theater.action.upload_video')" :disabled="title === null" @click="uploading = true" />
    </ToolBar>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto pb-(--bmo-create-room)">
      <p v-if="failed" role="alert" class="badge-ink p-4 text-sm text-error">{{ failed }}</p>

      <template v-else>
        <!-- The band: the poster blurred across the screen's width under a veil of the page's own ground, so the
             words on it read in either theme. -->
        <section data-title-band class="relative isolate overflow-hidden bg-elevated" :aria-label="title?.name">
          <img v-if="poster" data-title-backdrop :src="poster" alt="" aria-hidden="true" class="absolute inset-0 -z-10 size-full scale-110 object-cover blur-2xl">
          <span class="absolute inset-0 -z-10 bg-default/80" aria-hidden="true" />

          <div class="flex flex-col gap-4 p-4 sm:flex-row sm:items-end sm:gap-6 sm:p-6">
            <div class="w-full shrink-0 sm:w-auto" :style="narrow ? undefined : { width: heroWidth }">
              <ArtworkFrame
                v-if="title"
                :src="poster"
                :size="posterSize(title)"
                :shape="heroShape"
                :icon="KIND_ICONS[title.kind]"
                :name="title.name"
                class="rounded-lg shadow-lg"
              />
              <SkeletonBar v-else class="aspect-video w-full rounded-lg" />
            </div>

            <div class="flex min-w-0 flex-1 flex-col gap-2">
              <h1 v-if="title" class="text-2xl font-semibold text-highlighted sm:text-3xl">{{ title.name }}</h1>
              <SkeletonLine v-else size="lg" width="w-1/2" />

              <p v-if="facts" class="text-sm text-toned tabular-nums">{{ facts }}</p>
              <SkeletonLine v-else width="w-1/3" />

              <div class="flex flex-wrap items-center gap-2 pt-2">
                <UButton
                  v-if="playing"
                  data-title-play
                  class="badge-ink max-w-full pointer-coarse:min-h-11"
                  icon="i-material-symbols-play-arrow"
                  :label="playing.label"
                  :aria-label="playing.name"
                  :to="watchPath(playing.episode.id, false)"
                />
                <div class="flex items-center gap-1">
                  <IconButton v-if="playing?.resumes" data-title-from-start icon="i-material-symbols-replay" class="pointer-coarse:size-11 pointer-coarse:justify-center" :label="$t('theater.action.from_start')" :to="watchPath(playing.episode.id, true)" />
                  <IconButton v-if="playing" icon="i-material-symbols-playlist-add" class="pointer-coarse:size-11 pointer-coarse:justify-center" :label="$t('theater.action.add_to_playlist')" @click="addToPlaylist(playing.episode)" />
                  <IconButton v-if="playing" icon="i-material-symbols-share-outline" class="pointer-coarse:size-11 pointer-coarse:justify-center" :label="$t('theater.action.share')" @click="share(playing.episode)" />
                </div>
              </div>
            </div>
          </div>
        </section>

        <p v-if="title?.description" class="max-w-3xl px-4 pt-4 text-base whitespace-pre-line text-default sm:px-6">{{ title.description }}</p>

        <ContentSkeleton v-if="episodeList === null" list-class="flex flex-col pt-4" :rows="Math.min(6, Math.max(1, title?.episodeCount ?? 3))" :label="$t('theater.status.episodes_loading')">
          <div class="flex items-center gap-3 px-4 py-2">
            <SkeletonBar class="aspect-video w-24 shrink-0 rounded-md sm:w-36" />
            <div class="flex flex-1 flex-col gap-1"><SkeletonLine size="base" width="w-2/3" /><SkeletonLine width="w-1/3" /></div>
          </div>
        </ContentSkeleton>

        <p v-else-if="!episodeList.length" class="p-4 text-base text-muted">{{ $t("theater.message.no_episodes") }}</p>

        <template v-else-if="listed">
          <section v-if="shown" :aria-label="shown.name($t)" class="pt-4">
            <div class="flex h-11 items-center px-4">
              <ChoiceSelect
                v-if="sections.chosen.length > 1"
                v-model="sectionKey"
                :items="sectionItems"
                value-key="value"
                variant="ghost"
                :aria-label="$t('theater.label.season')"
                :ui="{ base: 'text-lg font-semibold text-highlighted md:text-lg pointer-coarse:min-h-11' }"
              />
              <h2 v-else class="text-lg font-semibold text-highlighted">{{ shown.name($t) }}</h2>
            </div>
            <ul class="flex flex-col">
              <EpisodeRow v-for="episode in shown.episodes" :key="episode.id" :episode="episode" :watched="watchedOf(episode)" :to="watchPath(episode.id, false)" :poster="posterOf(episode)" :menu="menuOf(episode)" />
            </ul>
          </section>

          <section v-if="sections.extras" :aria-label="sections.extras.name($t)" class="pt-4 pb-4">
            <div class="flex h-11 items-center px-4">
              <h2 class="text-lg font-semibold text-highlighted">{{ sections.extras.name($t) }}</h2>
            </div>
            <ul class="flex flex-col">
              <EpisodeRow v-for="episode in sections.extras.episodes" :key="episode.id" :episode="episode" :watched="watchedOf(episode)" :to="watchPath(episode.id, false)" :poster="posterOf(episode)" :menu="menuOf(episode)" />
            </ul>
          </section>
        </template>
      </template>
    </div>

    <UploadDialog v-if="title" v-model:open="uploading" :realm="title.realm" :title-id="title.id" />
    <SubtitleDialog v-model:open="subtitlesOpen" :episode="chosen" />
    <AddToPlaylistDialog v-if="chosen" v-model:open="addingOpen" :episode-ids="[chosen.id]" :realm="chosen.realm" :subject="chosen.name" />
    <ShareDialog v-if="chosen" v-model:open="sharingOpen" :road="road" :spans="spans" :span="FIRST_SPAN" :subject="chosen.name" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ActionGroup from "bmo-ui/app/components/ActionGroup.vue";
import ArtworkFrame from "bmo-ui/app/components/ArtworkFrame.vue";
import ChoiceSelect from "bmo-ui/app/components/ChoiceSelect.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import ShareDialog from "bmo-ui/app/components/ShareDialog.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { LANDSCAPE, shapeOf } from "bmo-ui/app/domain/artwork";
import { countText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";
import { useNarrow } from "bmo-ui/app/domain/screen";

import AddToPlaylistDialog from "../../../components/theater/AddToPlaylistDialog.vue";
import EpisodeRow from "../../../components/theater/EpisodeRow.vue";
import SubtitleDialog from "../../../components/theater/SubtitleDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import UploadDialog from "../../../components/theater/UploadDialog.vue";
import { kindWord } from "../../../domain/a11y";
import { paths } from "../../../domain/api";
import { posterSize } from "../../../domain/cards";
import { episodeCode, episodeTitle, factsOf, sectionOf, sectionsOf, watchedById, watchedEpisode } from "../../../domain/episodes";
import { KIND_ICONS } from "../../../domain/icons";
import { FIRST_SPAN, SHARE_SPANS, shareRoad } from "../../../domain/share";
import { refusalWord } from "../../../domain/store";
import { runningTime } from "../../../domain/time";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { theaterAddress, useTheater } from "../../../composables/useTheater";
import { useKeptScroll } from "../../../composables/useKeptScroll";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { DropdownMenuItem, SelectMenuItem } from "@nuxt/ui";
import type { Ref } from "vue";
import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Episode } from "../../../../shared/types";

const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const route = useRoute();
const { t } = useI18n();
const store = useTheater();
const narrow = useNarrow();

const chosen = ref<Episode | null>(null);
const failed = ref("");
const uploading = ref(false);
const refreshing = ref(false);
const subtitlesOpen = ref(false);
const addingOpen = ref(false);
const sharingOpen = ref(false);

const titleId = computed(() => Number(route.params.id));
const title = computed(() => store.titleOf(titleId.value) ?? null);
const episodeList = computed<Episode[] | null>(() => store.episodes.value[titleId.value] ?? null);
const progress = computed(() => store.progressOf("title", titleId.value));
const watched = computed(() => watchedById(progress.value));
const sections = computed(() => sectionsOf(title.value?.kind ?? "clip", episodeList.value ?? []));
const poster = computed(() => (title.value?.hasPoster ? theaterAddress(paths.titlePoster(title.value.id)) : null));
// The poster stands at its own shape, the frame's width read so a picture of any shape covers about the same room.
const heroShape = computed(() => shapeOf(title.value ? posterSize(title.value) : null) ?? LANDSCAPE);
const heroWidth = computed(() => `calc(15rem * ${Math.sqrt(heroShape.value.ratio).toFixed(3)})`);
// A film or a clip title of one episode lists no episodes: its page plays it.
const listed = computed(() => (episodeList.value?.length ?? 0) > 1 || title.value?.kind === "show");
const backPath = computed(() => ({ path: "/theater", query: title.value?.realm === "organization" ? { realm: "organization" } : {} }));

// One line of what the title is: its kind, a show's seasons, its episodes and how long they all run.
const facts = computed(() => {
  if (!title.value || episodeList.value === null) return null;

  const read = factsOf(sections.value, episodeList.value);
  const parts = [t(kindWord(title.value.kind))];

  if (title.value.kind === "show" && read.seasons > 0) parts.push(t("theater.label.seasons_count", { n: countText(read.seasons) }));
  if (read.episodes > 1) parts.push(t("theater.label.episodes_count", { n: countText(read.episodes) }));
  if (read.durationMs > 0) parts.push(runningTime(t, read.durationMs));

  return parts.join(" · ");
});

// What the play button plays: the title's up next, from where the viewer stopped, else its first main-line episode.
const playing = computed(() => {
  const list = episodeList.value;

  if (!list?.length) return null;

  const upNext = progress.value?.upNext ? list.find((one) => one.id === progress.value?.upNext) : undefined;
  const episode = upNext ?? sections.value.chosen[0]?.episodes[0] ?? list[0]!;
  const resumes = upNext !== undefined && (progress.value?.upNextSeconds ?? 0) > 0;
  const code = title.value?.kind === "show" ? episodeCode(episode) : "";
  const label = code ? t(resumes ? "theater.action.continue_named" : "theater.action.play_named", { name: code }) : t(resumes ? "theater.action.continue" : "theater.action.play");

  return { episode, resumes, label, name: `${t(resumes ? "theater.action.continue" : "theater.action.play")}: ${episodeTitle(episode)}` };
});

// The section shown: the one the address names, else the one the up next stands in, else the first.
const sectionKey = computed<string>({
  get: () => {
    const asked = typeof route.query.season === "string" ? route.query.season : null;

    if (asked && sections.value.chosen.some((one) => one.key === asked)) return asked;

    const holding = progress.value?.upNext ? sectionOf(sections.value, progress.value.upNext) : null;

    return holding && holding !== sections.value.extras ? holding.key : (sections.value.chosen[0]?.key ?? "");
  },
  set: (value) => void navigateTo({ path: route.path, query: { ...route.query, season: value } })
});
const shown = computed(() => sections.value.chosen.find((one) => one.key === sectionKey.value) ?? sections.value.chosen[0] ?? null);
const sectionItems = computed<SelectMenuItem[]>(() => sections.value.chosen.map((one) => ({ label: one.name(t), value: one.key })));

// What acts on the title as a whole beside the band's actions: reading it again, and, for a title of one episode,
// that episode's subtitles and, for a clip, deleting it.
const actions = computed<BarAction[]>(() => {
  const only = !listed.value ? (episodeList.value?.[0] ?? null) : null;

  return [
    { id: "refresh", label: t("theater.action.refresh"), icon: "i-material-symbols-refresh", loading: refreshing.value, onSelect: () => void refresh() },
    ...(only ? [{ id: "subtitles", label: t("theater.action.subtitles"), icon: "i-material-symbols-subtitles-outline", onSelect: () => openFor(only, subtitlesOpen) }] : []),
    ...(only?.kind === "clip" ? [{ id: "delete", label: t("theater.action.delete"), icon: "i-material-symbols-delete-outline", onSelect: () => void remove(only) }] : [])
  ];
});

const road = computed(() => shareRoad(store.shareRoad({ episodeId: chosen.value?.id ?? 0 })));
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

watch(titleId, () => void load(), { immediate: true });

useKeptScroll(scrollerRef, () => episodeList.value !== null);

function watchedOf(episode: Episode) {
  return watched.value.get(episode.id) ?? watchedEpisode(episode);
}

function posterOf(episode: Episode): string | null {
  return episode.hasPoster ? theaterAddress(paths.episodePoster(episode.id)) : null;
}

// Where an episode plays: from where the viewer stopped, which the player's screen reads itself, or from the start.
function watchPath(episodeId: number, fromStart: boolean) {
  return { path: `/theater/watch/${episodeId}`, query: fromStart ? { from: "start" } : {} };
}

function openFor(episode: Episode, which: Ref<boolean>): void {
  chosen.value = episode;
  which.value = true;
}

function addToPlaylist(episode: Episode): void {
  openFor(episode, addingOpen);
}

function share(episode: Episode): void {
  openFor(episode, sharingOpen);
}

function menuOf(episode: Episode): DropdownMenuItem[][] {
  return [
    [
      { label: t("theater.action.add_to_playlist"), icon: "i-material-symbols-playlist-add", onSelect: () => openFor(episode, addingOpen) },
      { label: t("theater.action.subtitles"), icon: "i-material-symbols-subtitles-outline", onSelect: () => openFor(episode, subtitlesOpen) },
      { label: t("theater.action.share"), icon: "i-material-symbols-share-outline", onSelect: () => openFor(episode, sharingOpen) }
    ],
    // A library's episode goes with its file, so only a filed or burned clip is deleted here.
    ...(episode.kind === "clip" ? [[{ label: t("theater.action.delete"), icon: "i-material-symbols-delete-outline", color: "error" as const, onSelect: () => void remove(episode) }]] : [])
  ];
}

async function remove(episode: Episode): Promise<void> {
  const sure = await askQuestion({ title: t("theater.question.delete_episode"), item: episode.name, confirm: t("theater.action.delete") });

  if (sure && (await store.deleteEpisode(episode))) void store.loadProgress("title", titleId.value);
}

// The title and its episodes are read again every time the screen opens the title, so what stands here was
// read for the reader standing here (`docs/design.md`, 「The console」).
async function load(): Promise<void> {
  failed.value = "";

  try {
    await Promise.all([store.openTitle(titleId.value), store.loadProgress("title", titleId.value)]);
  } catch (error) {
    failed.value = t(refusalWord(error));
  }
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await load();
  } finally {
    refreshing.value = false;
  }
}

// The share dialog is the layer's; where the focus goes as it comes and goes is said here.
useDialogFocus(sharingOpen);

useTheaterEvents(
  () => void load(),
  (event) => {
    const data = event.data as { titleId?: number };

    if ((event.kind === "episode.created" || event.kind === "episode.deleted") && (data.titleId === undefined || data.titleId === titleId.value)) void store.loadProgress("title", titleId.value);
  }
);
</script>
