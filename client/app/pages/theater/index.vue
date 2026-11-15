<template>
  <!-- The library (`docs/design.md`, 「The console」): the realm's rows — continue watching, next up, recently added,
       one row for each kind of title the realm holds, and the playlists — each standing only while it holds
       something, in the service's own order. A realm holding nothing says so with the one action that fills it. -->
  <TheaterFrame id="theater-library" nav realms :actions="actions">
    <template #create="{ cell }">
      <CreateButton :class="cell" :label="$t('theater.action.upload_video')" @click="uploading = true" />
    </template>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto pb-(--bmo-create-room)" :class="CARD_MEASURE">
      <p v-if="titlesFailed[realm] && !titles[realm].length" role="alert" class="badge-ink p-4 text-sm text-error">{{ $t("theater.error.titles_failed") }}</p>

      <ContentSkeleton v-else-if="!titlesRead[realm]" list-class="flex flex-col gap-8 py-4" :rows="2" :label="$t('theater.status.titles_loading')">
        <div class="flex flex-col gap-2">
          <div class="flex h-9 items-center px-4"><SkeletonLine size="lg" width="w-40" /></div>
          <div class="flex gap-4 overflow-hidden px-4">
            <div v-for="n in 8" :key="n" class="flex shrink-0 flex-col gap-2 p-2" :style="{ width: cardWidth(LANDSCAPE) }">
              <SkeletonBar class="aspect-video w-full rounded-md" />
              <SkeletonLine size="base" width="w-3/4" />
              <SkeletonLine width="w-1/2" />
            </div>
          </div>
        </div>
      </ContentSkeleton>

      <div v-else-if="empty" class="flex flex-wrap items-center gap-x-4 gap-y-2 p-4">
        <p class="text-base text-muted">{{ $t(realm === "mine" ? "theater.message.no_titles_mine" : "theater.message.no_titles_organization") }}</p>
        <UButton color="neutral" variant="outline" icon="i-material-symbols-upload" :label="$t('theater.action.upload_video')" @click="uploading = true" />
      </div>

      <div v-else class="flex flex-col gap-8 py-4">
        <CardRow v-if="watchingRow.length" v-slot="{ item }" :title="$t('theater.label.continue_watching')" :items="watchingRow">
          <EpisodeCard
            :episode="item.episode"
            :title="store.titleOf(item.episode.titleId) ?? null"
            :to="`/theater/watch/${item.episode.id}`"
            :shape="watchingShape"
            :width="cardWidth(watchingShape)"
            set-aside
            @set-aside="void store.setAside(item.episode)"
          />
        </CardRow>

        <CardRow v-if="nextUpRow.length" v-slot="{ item }" :title="$t('theater.label.next_up')" :items="nextUpRow">
          <EpisodeCard :episode="item.episode" :title="store.titleOf(item.episode.titleId) ?? null" :to="`/theater/watch/${item.episode.id}`" :shape="nextUpShape" :width="cardWidth(nextUpShape)" />
        </CardRow>

        <CardRow v-for="row in titleRows" :key="row.key" v-slot="{ item }" :title="row.title" :items="row.items" :all="row.all">
          <TitleCard :title="item.title" :shape="row.shape" :width="cardWidth(row.shape)" />
        </CardRow>

        <CardRow v-if="playlistRow.length" v-slot="{ item }" :title="$t('theater.label.playlists')" :items="playlistRow" :all="{ path: '/theater/playlists', query: realmQuery }">
          <PlaylistCard :playlist="item.playlist" :shape="SQUARE" :width="cardWidth(SQUARE)" />
        </CardRow>
      </div>
    </div>

    <UploadDialog v-model:open="uploading" :realm="realm" @filed="(episode) => void navigateTo(`/theater/titles/${episode.titleId}`)" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";
import CardRow from "bmo-ui/app/components/CardRow.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { LANDSCAPE, SQUARE } from "bmo-ui/app/domain/artwork";

import EpisodeCard from "../../components/theater/EpisodeCard.vue";
import PlaylistCard from "../../components/theater/PlaylistCard.vue";
import TheaterFrame from "../../components/theater/TheaterFrame.vue";
import TitleCard from "../../components/theater/TitleCard.vue";
import UploadDialog from "../../components/theater/UploadDialog.vue";
import { CARD_MEASURE, cardWidth, episodesShape, titlesShape } from "../../domain/cards";
import { kindRow } from "../../domain/rows";
import { useKeptScroll } from "../../composables/useKeptScroll";
import { useRealm } from "../../composables/useRealm";
import { useTheater } from "../../composables/useTheater";
import { useTheaterEvents } from "../../composables/useTheaterEvents";

import type { ArtworkShape } from "bmo-ui/app/domain/artwork";
import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Episode, Playlist, Title } from "../../../shared/types";

// How many cards a row holds; its all link opens the rest.
const ROW_CAP = 20;

interface TitleRow {
  key: string;
  title: string;
  items: { title: Title }[];
  shape: ArtworkShape;
  all: Record<string, unknown>;
}

const { t } = useI18n();
const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { realm } = useRealm();
const store = useTheater();
const { titles, titlesRead, titlesFailed, playlists, watching, nextUp } = store;

const uploading = ref(false);
const refreshing = ref(false);

const realmQuery = computed(() => (realm.value === "organization" ? { realm: "organization" } : {}));
const empty = computed(() => !titles.value[realm.value].length && !playlists.value[realm.value].length);

// The carousel takes objects, so each card's member is handed in one.
const watchingRow = computed(() => (watching.value[realm.value] ?? []).slice(0, ROW_CAP).map((episode: Episode) => ({ episode })));
const nextUpRow = computed(() => (nextUp.value[realm.value] ?? []).slice(0, ROW_CAP).map((episode: Episode) => ({ episode })));
const watchingShape = computed(() => episodesShape(watchingRow.value.map((one) => one.episode)));
const nextUpShape = computed(() => episodesShape(nextUpRow.value.map((one) => one.episode)));
const playlistRow = computed(() => playlists.value[realm.value].slice(0, ROW_CAP).map((playlist: Playlist) => ({ playlist })));

// Recently added, then one row for each kind the realm holds, each in the service's order of the one added to last
// first.
const titleRows = computed<TitleRow[]>(() => {
  const held = titles.value[realm.value];
  const recent = held.slice(0, ROW_CAP);
  const rows: TitleRow[] = [
    { key: "recent", title: t("theater.label.recently_added"), items: recent.map((title) => ({ title })), shape: titlesShape(recent), all: { path: "/theater/browse", query: { ...realmQuery.value } } }
  ];

  for (const row of kindRow(held)) {
    const shown = row.titles.slice(0, ROW_CAP);

    rows.push({ key: row.kind, title: t(row.name), items: shown.map((title) => ({ title })), shape: titlesShape(shown), all: { path: "/theater/browse", query: { ...realmQuery.value, kind: row.kind } } });
  }

  return held.length ? rows : [];
});

const actions = computed<BarAction[]>(() => [
  { id: "refresh", label: t("theater.action.refresh"), icon: "i-material-symbols-refresh", loading: refreshing.value, onSelect: () => void refresh() }
]);

function load(asked: typeof realm.value): Promise<unknown> {
  return Promise.all([store.loadTitles(asked), store.loadPlaylists(asked), store.loadWatching(asked), store.loadNextUp(asked)]);
}

watch(
  realm,
  (asked) => {
    if (!titlesRead.value[asked]) void load(asked);
    else void Promise.all([store.loadWatching(asked), store.loadNextUp(asked)]);
  },
  { immediate: true }
);

// What the viewer watched elsewhere moves continue watching and next up; they are read again, one read at a time.
let rereading: Promise<unknown> | null = null;
let again = false;

watch(store.rowsChanged, () => {
  if (rereading) {
    again = true;

    return;
  }

  const reread = (): void => {
    rereading = Promise.all([store.loadWatching(realm.value), store.loadNextUp(realm.value)]).finally(() => {
      rereading = null;

      if (again) {
        again = false;
        reread();
      }
    });
  };

  reread();
});

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await load(realm.value);
  } finally {
    refreshing.value = false;
  }
}

useKeptScroll(scrollerRef, () => titlesRead.value[realm.value]);

useTheaterEvents(() => void load(realm.value));
</script>
