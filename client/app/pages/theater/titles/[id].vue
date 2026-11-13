<template>
  <!-- One title (`docs/design.md`, 「The console」): its name, how far the viewer has come through it and the
       episode to go on from, then its episodes by season, each with its own progress. -->
  <TheaterFrame id="theater-title">
    <ToolBar :label="$t('theater.aria.title_bar')">
      <div>
        <UTooltip :text="$t('theater.label.library')">
          <UButton color="neutral" variant="ghost" size="sm" square icon="i-material-symbols-arrow-back" :aria-label="$t('theater.label.library')" :to="backPath" />
        </UTooltip>
      </div>

      <div v-if="seasons.length > 1" data-slot="season">
        <USelect v-model="seasonKey" :items="seasonItems" value-key="value" variant="ghost" size="sm" class="min-w-32" :aria-label="$t('theater.label.season')" />
      </div>

      <ActionGroup :actions="actions" :narrow-fold="false" class="ms-auto" />
    </ToolBar>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto">
      <p v-if="failed" role="alert" class="badge-ink p-4 text-sm text-error">{{ failed }}</p>

      <template v-else>
        <header class="flex flex-col gap-2 px-4 pt-4 pb-2">
          <h1 v-if="title" class="text-2xl font-semibold text-highlighted">{{ title.name }}</h1>
          <SkeletonLine v-else size="lg" width="w-1/2" />
          <p v-if="title?.description" class="text-base text-muted">{{ title.description }}</p>
        </header>

        <div class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 pb-4">
          <div v-if="progress && progress.items.length" class="flex min-w-48 flex-1 items-center gap-3 text-sm text-muted tabular-nums">
            <RollingNumber class="shrink-0" :value="progress.doneCount" :text="progressText(progress.doneCount, progress.items.length)" />
            <UProgress :model-value="Math.floor((progress.doneCount / progress.items.length) * 100)" size="sm" class="min-w-0 flex-1" :aria-label="$t('theater.aria.title_progress')" />
          </div>
          <UButton v-if="nextEpisode" class="badge-ink ms-auto pointer-coarse:min-h-11" icon="i-material-symbols-play-arrow" :label="$t(progress?.nextSeconds ? 'theater.action.continue' : 'theater.action.play')" :to="watchPath(nextEpisode.id, progress?.nextSeconds ?? 0)" :aria-label="nextName" />
        </div>

        <ContentSkeleton v-if="episodeList === null" list-class="flex flex-col" :rows="Math.min(6, Math.max(1, title?.episodeCount ?? 3))" :label="$t('theater.status.episodes_loading')">
          <div class="flex items-center gap-3 px-4 py-2">
            <SkeletonBar class="aspect-video w-24 shrink-0 rounded-md sm:w-36" />
            <div class="flex flex-1 flex-col gap-1"><SkeletonLine size="base" width="w-2/3" /><SkeletonLine width="w-1/3" /></div>
          </div>
        </ContentSkeleton>

        <p v-else-if="!episodeList.length" class="px-4 pb-4 text-base text-muted">{{ $t("theater.message.no_episodes") }}</p>

        <section v-else :aria-label="seasonName(shownSeason?.season ?? null)" class="pb-4">
          <h2 class="px-4 pb-2 text-lg font-medium text-highlighted">{{ seasonName(shownSeason?.season ?? null) }}</h2>
          <ul class="flex flex-col">
            <EpisodeRow
              v-for="episode in shownSeason?.episodes ?? []"
              :key="episode.id"
              :episode="episode"
              :watched="watched.get(episode.id) ?? watchedEpisode(episode)"
              :to="watchPath(episode.id)"
              :poster="episode.hasPoster ? theaterAddress(paths.episodePoster(episode.id)) : null"
              :menu="menuOf(episode)"
            />
          </ul>
        </section>
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
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import ShareDialog from "bmo-ui/app/components/ShareDialog.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import RollingNumber from "bmo-ui/app/components/RollingNumber.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { progressText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";

import AddToPlaylistDialog from "../../../components/theater/AddToPlaylistDialog.vue";
import EpisodeRow from "../../../components/theater/EpisodeRow.vue";
import SubtitleDialog from "../../../components/theater/SubtitleDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import UploadDialog from "../../../components/theater/UploadDialog.vue";
import { paths } from "../../../domain/api";
import { episodeTitle, seasonsOf, watchedById, watchedEpisode } from "../../../domain/episodes";
import { FIRST_SPAN, SHARE_SPANS, shareRoad } from "../../../domain/share";
import { refusalWord } from "../../../domain/store";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { theaterAddress, useTheater } from "../../../composables/useTheater";
import { useKeptScroll } from "../../../composables/useKeptScroll";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { DropdownMenuItem, SelectItem } from "@nuxt/ui";
import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Episode } from "../../../../shared/types";

const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const route = useRoute();
const { t } = useI18n();
const store = useTheater();

const chosen = ref<Episode | null>(null);
const failed = ref("");
const uploading = ref(false);
const refreshing = ref(false);
const subtitlesOpen = ref(false);
const addingOpen = ref(false);
const sharingOpen = ref(false);

const titleId = computed(() => Number(route.params.id));
const title = computed(() => store.titleOf(titleId.value) ?? null);
const actions = computed<BarAction[]>(() => [
  { id: "upload", label: t("theater.action.upload"), icon: "i-material-symbols-upload", disabled: title.value === null, onSelect: () => (uploading.value = true) },
  { id: "refresh", label: t("theater.action.refresh"), icon: "i-material-symbols-refresh", loading: refreshing.value, onSelect: () => void refresh() }
]);
const episodeList = computed<Episode[] | null>(() => store.episodes.value[titleId.value] ?? null);
const progress = computed(() => store.progressOf("title", titleId.value));
const watched = computed(() => watchedById(progress.value));
const seasons = computed(() => seasonsOf(episodeList.value ?? []));
const nextEpisode = computed(() => (progress.value?.next ? (episodeList.value?.find((one) => one.id === progress.value?.next) ?? null) : null));
const nextName = computed(() => (nextEpisode.value ? `${t(progress.value?.nextSeconds ? "theater.action.continue" : "theater.action.play")}: ${episodeTitle(nextEpisode.value)}` : undefined));
const backPath = computed(() => ({ path: "/theater", query: title.value?.realm === "organization" ? { realm: "organization" } : {} }));

// The season shown: the one the address names, else the one the next episode stands in, else the first.
const seasonKey = computed<string>({
  get: () => (typeof route.query.season === "string" ? route.query.season : keyOf(nextEpisode.value?.season ?? seasons.value[0]?.season ?? null)),
  set: (value) => void navigateTo({ path: route.path, query: { ...route.query, season: value } })
});
const shownSeason = computed(() => seasons.value.find((one) => keyOf(one.season) === seasonKey.value) ?? seasons.value[0] ?? null);
const seasonItems = computed<SelectItem[]>(() => seasons.value.map((one) => ({ label: seasonName(one.season), value: keyOf(one.season) })));

const road = computed(() => shareRoad(store.shareRoad({ episodeId: chosen.value?.id ?? 0 })));
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

watch(titleId, () => void load(), { immediate: true });

useKeptScroll(scrollerRef, () => episodeList.value !== null);

function keyOf(season: number | null): string {
  return season === null ? "none" : String(season);
}

function seasonName(season: number | null): string {
  return season === null ? t("theater.label.season_none") : t("theater.label.season_number", { n: String(season) });
}

function watchPath(episodeId: number, seconds?: number) {
  return { path: `/theater/watch/${episodeId}`, query: seconds ? { t: String(Math.floor(seconds)) } : {} };
}

function menuOf(episode: Episode): DropdownMenuItem[][] {
  const open = (which: { value: boolean }) => () => {
    chosen.value = episode;
    which.value = true;
  };

  return [
    [
      { label: t("theater.action.add_to_playlist"), icon: "i-material-symbols-playlist-add", onSelect: open(addingOpen) },
      { label: t("theater.action.subtitles"), icon: "i-material-symbols-subtitles-outline", onSelect: open(subtitlesOpen) },
      { label: t("theater.action.share"), icon: "i-material-symbols-share-outline", onSelect: open(sharingOpen) }
    ],
    // A library's episode goes with its file, so only a filed or burned clip is deleted here.
    ...(episode.kind === "clip" ? [[{ label: t("theater.action.delete"), icon: "i-material-symbols-delete-outline", color: "error" as const, onSelect: () => void remove(episode) }]] : [])
  ];
}

async function remove(episode: Episode): Promise<void> {
  const sure = await askQuestion({ title: t("theater.question.delete_episode"), item: episode.name, confirm: t("theater.action.delete") });

  if (sure && (await store.deleteEpisode(episode))) void store.loadProgress("title", titleId.value);
}

async function load(): Promise<void> {
  failed.value = "";

  try {
    if (!store.titleOf(titleId.value)) await store.loadTitle(titleId.value);
    await Promise.all([store.loadEpisodes(titleId.value), store.loadProgress("title", titleId.value)]);
  } catch (error) {
    failed.value = t(refusalWord(error));
  }
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await store.loadTitle(titleId.value).catch(() => undefined);
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
