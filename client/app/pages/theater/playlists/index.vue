<template>
  <!-- The playlists of a realm (`docs/design.md`, 「The console」): each by its name, how many episodes it holds and
       how long they run. A playlist opens its episodes, how far the viewer has come and the way to go on. -->
  <TheaterFrame id="theater-playlists" nav realms>
    <template #actions>
      <UTooltip :text="$t('theater.action.new_playlist')">
        <UButton color="neutral" variant="ghost" class="pointer-coarse:size-11 pointer-coarse:justify-center" icon="i-material-symbols-playlist-add" :aria-label="$t('theater.action.new_playlist')" @click="making = true" />
      </UTooltip>
      <UTooltip :text="$t('theater.action.refresh')">
        <UButton color="neutral" variant="ghost" class="pointer-coarse:size-11 pointer-coarse:justify-center" icon="i-material-symbols-refresh" :aria-label="$t('theater.action.refresh')" :loading="refreshing" @click="refresh" />
      </UTooltip>
    </template>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto">
      <p v-if="playlistsFailed[realm] && !shown.length" role="alert" class="badge-ink p-4 text-sm text-error">{{ $t("theater.error.playlists_failed") }}</p>

      <ContentSkeleton v-else-if="!playlistsRead[realm]" list-class="flex flex-col py-2" :rows="4" :label="$t('theater.status.playlists_loading')">
        <div class="flex h-15 items-center gap-3 px-4">
          <SkeletonBar class="size-6 shrink-0 rounded-md" />
          <div class="flex flex-1 flex-col"><SkeletonLine size="base" width="w-1/3" /><SkeletonLine width="w-1/5" /></div>
        </div>
      </ContentSkeleton>

      <p v-else-if="!shown.length" class="p-4 text-base text-muted">{{ $t("theater.message.no_playlists") }}</p>

      <MovingList v-else class="flex flex-col py-2" :aria-label="$t('theater.label.playlists')">
        <li v-for="playlist in shown" :key="playlist.id" class="flex min-w-0 items-center gap-2 px-2 sm:px-4">
          <RouterLink
            :to="`/theater/playlists/${playlist.id}`"
            class="flex min-h-11 min-w-0 flex-1 items-center gap-3 rounded-md p-2 transition-colors select-none hover:bg-elevated/60 active:bg-elevated focus-inset"
          >
            <UIcon name="i-material-symbols-playlist-play" class="size-6 shrink-0 text-muted" aria-hidden="true" />
            <span class="flex min-w-0 flex-1 flex-col">
              <span class="truncate text-base text-highlighted">{{ playlist.name }}</span>
              <span class="truncate text-sm text-muted tabular-nums">{{ $t("theater.label.episodes_count", { n: countText(playlist.episodeIds.length) }) }}</span>
            </span>
            <span class="shrink-0 text-sm text-muted tabular-nums">{{ lengthClock(playlist.durationMs) }}</span>
          </RouterLink>

          <UDropdownMenu :modal="false" :ui="{ item: 'pointer-coarse:min-h-11' }" :items="menuOf(playlist)" :content="{ align: 'end' }">
            <UTooltip :text="$t('common.action.more')">
              <UButton color="neutral" variant="ghost" icon="i-material-symbols-more-vert" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('common.action.more')" />
            </UTooltip>
          </UDropdownMenu>
        </li>
      </MovingList>
    </div>

    <NameDialog v-model:open="making" :title="$t('theater.heading.new_playlist')" :action="$t('theater.action.make')" :send="make" />
    <NameDialog v-model:open="renaming" :title="$t('theater.heading.rename_playlist')" :action="$t('common.action.save')" :current="chosen?.name ?? ''" :send="rename" />
    <ShareLinkDialog v-if="chosen" v-model:open="sharing" :road="road" :spans="spans" :span="FIRST_SPAN" :subject="chosen.name" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { RouterLink } from "vue-router";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import MovingList from "bmo-ui/app/components/MovingList.vue";
import ShareLinkDialog from "bmo-ui/app/components/ShareLinkDialog.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { countText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";

import NameDialog from "../../../components/theater/NameDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import { lengthClock } from "../../../domain/episodes";
import { FIRST_SPAN, SHARE_SPANS, shareRoad } from "../../../domain/share";
import { useKeptScroll } from "../../../composables/useKeptScroll";
import { useRealm } from "../../../composables/useRealm";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { useTheater } from "../../../composables/useTheater";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { DropdownMenuItem } from "@nuxt/ui";
import type { Playlist } from "../../../../shared/types";

const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { t } = useI18n();
const { realm } = useRealm();
const store = useTheater();
const { playlists, playlistsRead, playlistsFailed, loadPlaylists } = store;

const chosen = ref<Playlist | null>(null);
const making = ref(false);
const renaming = ref(false);
const sharing = ref(false);
const refreshing = ref(false);

const shown = computed(() => [...playlists.value[realm.value]].sort((a, b) => a.name.localeCompare(b.name)));
const road = computed(() => shareRoad(store.shareRoad({ playlistId: chosen.value?.id ?? 0 })));
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

watch(
  realm,
  (asked) => {
    if (!playlistsRead.value[asked]) void loadPlaylists(asked);
  },
  { immediate: true }
);

function menuOf(playlist: Playlist): DropdownMenuItem[][] {
  const open = (which: { value: boolean }) => () => {
    chosen.value = playlist;
    which.value = true;
  };

  return [
    [
      { label: t("theater.action.rename"), icon: "i-material-symbols-edit-outline", onSelect: open(renaming) },
      { label: t("theater.action.share"), icon: "i-material-symbols-share-outline", onSelect: open(sharing) }
    ],
    [{ label: t("theater.action.delete"), icon: "i-material-symbols-delete-outline", color: "error" as const, onSelect: () => void remove(playlist) }]
  ];
}

async function make(name: string): Promise<void> {
  const made = await store.makePlaylist(realm.value, name);

  void navigateTo(`/theater/playlists/${made.id}`);
}

async function rename(name: string): Promise<void> {
  if (chosen.value) await store.renamePlaylist(chosen.value, name);
}

async function remove(playlist: Playlist): Promise<void> {
  if (await askQuestion({ title: t("theater.question.delete_playlist"), item: playlist.name, confirm: t("theater.action.delete") })) await store.deletePlaylist(playlist);
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await loadPlaylists(realm.value);
  } finally {
    refreshing.value = false;
  }
}

// The share dialog is the layer's; where the focus goes as it comes and goes is said here.
useDialogFocus(sharing);

useKeptScroll(scrollerRef, () => playlistsRead.value[realm.value]);

useTheaterEvents(() => void loadPlaylists(realm.value));
</script>
