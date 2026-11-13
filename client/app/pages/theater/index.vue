<template>
  <!-- The library (`docs/design.md`, 「The console」): the posters of the realm's titles in a grid, the reader's own
       in one tab and the organization's in the other. A title opens its seasons and episodes. -->
  <TheaterFrame id="theater-library" nav realms :actions="actions">
    <template #create="{ cell }">
      <CreateButton :class="cell" :label="$t('theater.action.upload_video')" @click="uploading = true" />
    </template>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto pb-(--bmo-create-room)">
      <p v-if="titlesFailed[realm] && !shown.length" role="alert" class="badge-ink p-4 text-sm text-error">{{ $t("theater.error.titles_failed") }}</p>

      <ContentSkeleton v-else-if="!titlesRead[realm]" :list-class="GRID" :rows="8" :label="$t('theater.status.titles_loading')">
        <div class="flex flex-col gap-2 p-2">
          <SkeletonBar class="aspect-video w-full rounded-md" />
          <SkeletonLine size="base" width="w-3/4" />
          <SkeletonLine width="w-1/2" />
        </div>
      </ContentSkeleton>

      <p v-else-if="!shown.length" class="p-4 text-base text-muted">{{ $t(realm === "mine" ? "theater.message.no_titles_mine" : "theater.message.no_titles_organization") }}</p>

      <ul v-else :class="GRID" :aria-label="$t(realm === 'mine' ? 'theater.label.mine' : 'theater.label.organization')">
        <li v-for="title in shown" :key="title.id" class="min-w-0">
          <TitleCard :title="title" />
        </li>
      </ul>
    </div>

    <UploadDialog v-model:open="uploading" :realm="realm" @filed="(episode) => void navigateTo(`/theater/titles/${episode.titleId}`)" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import CreateButton from "bmo-ui/app/components/CreateButton.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";

import TheaterFrame from "../../components/theater/TheaterFrame.vue";
import TitleCard from "../../components/theater/TitleCard.vue";
import UploadDialog from "../../components/theater/UploadDialog.vue";
import { useKeptScroll } from "../../composables/useKeptScroll";
import { useRealm } from "../../composables/useRealm";
import { useTheater } from "../../composables/useTheater";
import { useTheaterEvents } from "../../composables/useTheaterEvents";

import type { BarAction } from "bmo-ui/app/domain/tabs";

// The cards stand as many to a row as fit at a width a poster reads at, filling the screen.
const GRID = "grid grid-cols-[repeat(auto-fill,minmax(11rem,1fr))] gap-2 p-2 sm:gap-4 sm:p-4";

const { t } = useI18n();
const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { realm } = useRealm();
const { titles, titlesRead, titlesFailed, loadTitles } = useTheater();

const uploading = ref(false);
const refreshing = ref(false);

const shown = computed(() => [...titles.value[realm.value]].sort((a, b) => a.name.localeCompare(b.name)));
const actions = computed<BarAction[]>(() => [
  { id: "refresh", label: t("theater.action.refresh"), icon: "i-material-symbols-refresh", loading: refreshing.value, onSelect: () => void refresh() }
]);

watch(
  realm,
  (asked) => {
    if (!titlesRead.value[asked]) void loadTitles(asked);
  },
  { immediate: true }
);

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await loadTitles(realm.value);
  } finally {
    refreshing.value = false;
  }
}

useKeptScroll(scrollerRef, () => titlesRead.value[realm.value]);

useTheaterEvents(() => void loadTitles(realm.value));
</script>
