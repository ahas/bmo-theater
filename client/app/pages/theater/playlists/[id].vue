<template>
  <!-- One playlist (`docs/design.md`, 「The console」): how far the viewer has come through it and the button that
       goes on from there, then its episodes in order, each with its own progress. The order is changed by the
       handle at an item's left — dragged with a pointer, or moved with the arrow keys — and on a phone by each
       item's up and down buttons; every change sends the whole order. -->
  <TheaterFrame id="theater-playlist">
    <ToolBar :label="$t('theater.aria.playlist_bar')">
      <div>
        <IconButton size="sm" icon="i-material-symbols-arrow-back" :label="$t('theater.label.playlists')" :to="backPath" />
      </div>

      <ActionGroup :actions="actions" :narrow-fold="false" class="ms-auto" />
    </ToolBar>

    <div class="min-h-0 flex-1 overflow-y-auto">
      <p v-if="failed" role="alert" class="badge-ink p-4 text-sm text-error">{{ failed }}</p>

      <template v-else>
        <header class="flex flex-col gap-2 px-4 pt-4 pb-2">
          <h1 v-if="playlist" class="text-2xl font-semibold text-highlighted">{{ playlist.name }}</h1>
          <SkeletonLine v-else size="lg" width="w-1/2" />
        </header>

        <div class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 pb-4">
          <div v-if="progress && progress.items.length" class="flex min-w-48 flex-1 items-center gap-3 text-sm text-muted tabular-nums">
            <RollingNumber class="shrink-0" :value="progress.doneCount" :text="progressText(progress.doneCount, progress.items.length)" />
            <UProgress :model-value="Math.floor((progress.doneCount / progress.items.length) * 100)" size="sm" class="min-w-0 flex-1" :aria-label="$t('theater.aria.playlist_progress')" />
          </div>
          <UButton v-if="nextPath" class="badge-ink ms-auto pointer-coarse:min-h-11" icon="i-material-symbols-play-arrow" :label="$t(progress?.nextSeconds ? 'theater.action.continue' : 'theater.action.play')" :to="nextPath" />
        </div>

        <ContentSkeleton v-if="!playlist || !itemsRead" list-class="flex flex-col" :rows="Math.min(6, Math.max(1, playlist?.episodeIds.length ?? 3))" :label="$t('theater.status.episodes_loading')">
          <div class="flex items-center gap-3 px-4 py-2">
            <SkeletonBar class="aspect-video w-20 shrink-0 rounded-md sm:w-36" />
            <div class="flex flex-1 flex-col gap-1"><SkeletonLine size="base" width="w-2/3" /><SkeletonLine width="w-1/3" /></div>
          </div>
        </ContentSkeleton>

        <p v-else-if="!items.length" class="px-4 pb-4 text-base text-muted">{{ $t("theater.message.playlist_empty") }}</p>

        <MovingList v-else class="flex flex-col pb-4" :aria-label="$t('theater.label.episodes')">
          <EpisodeRow
            v-for="item in items"
            :key="item.key"
            :episode="item.episode"
            :watched="progress ? watchedOf(progress.items[item.at]) : watchedEpisode(item.episode)"
            :to="{ path: `/theater/watch/${item.episode.id}`, query: { playlist: String(playlistId), at: String(item.at) } }"
            :poster="item.episode.hasPoster ? theaterAddress(paths.episodePoster(item.episode.id)) : null"
            compact
            :class="dragging === item.at ? 'opacity-60' : over === item.at ? 'bg-elevated/40' : ''"
            @dragover.prevent="over = item.at"
            @drop.prevent="drop(item.at)"
          >
            <template v-if="!narrow" #lead>
              <IconButton
                data-reorder
                draggable="true"
                icon="i-material-symbols-drag-indicator"
                class="cursor-grab text-muted active:cursor-grabbing"
                :label="$t('theater.aria.reorder_named', { name: item.episode.name })"
                :aria-describedby="hintId"
                @dragstart="start(item.at, $event)"
                @dragend="finish"
                @keydown="moveByKey(item.at, $event)"
              />
            </template>

            <template #trail>
              <div class="flex shrink-0 items-center">
                <template v-if="narrow">
                  <IconButton icon="i-material-symbols-arrow-upward" class="size-11 justify-center" :label="$t('theater.action.move_up')" :disabled="item.at === 0" @click="move(item.at, item.at - 1)" />
                  <IconButton icon="i-material-symbols-arrow-downward" class="size-11 justify-center" :label="$t('theater.action.move_down')" :disabled="item.at === lastPlace" @click="move(item.at, item.at + 1)" />
                </template>
                <IconButton icon="i-material-symbols-close" class="pointer-coarse:size-11 pointer-coarse:justify-center" :label="$t('theater.action.remove')" @click="removeAt(item.at)" />
              </div>
            </template>
          </EpisodeRow>
        </MovingList>

        <p :id="hintId" class="sr-only">{{ $t("theater.aria.reorder_hint") }}</p>
      </template>
    </div>

    <!-- What a move changed, said once it has landed. -->
    <p class="sr-only" aria-live="polite" aria-atomic="true">{{ announced }}</p>

    <NameDialog v-model:open="renaming" :title="$t('theater.heading.rename_playlist')" :action="$t('common.action.save')" :current="playlist?.name ?? ''" :send="rename" />
    <ShareDialog v-if="playlist" v-model:open="sharing" :road="road" :spans="spans" :span="FIRST_SPAN" :subject="playlist.name" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ActionGroup from "bmo-ui/app/components/ActionGroup.vue";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import MovingList from "bmo-ui/app/components/MovingList.vue";
import ShareDialog from "bmo-ui/app/components/ShareDialog.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import RollingNumber from "bmo-ui/app/components/RollingNumber.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { countText, progressText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";
import { NARROW_QUERY, useNarrow } from "bmo-ui/app/domain/screen";

import EpisodeRow from "../../../components/theater/EpisodeRow.vue";
import NameDialog from "../../../components/theater/NameDialog.vue";
import TheaterFrame from "../../../components/theater/TheaterFrame.vue";
import { paths } from "../../../domain/api";
import { watchedEpisode, watchedOf } from "../../../domain/episodes";
import { itemKeys } from "../../../domain/order";
import { FIRST_SPAN, SHARE_SPANS, shareRoad } from "../../../domain/share";
import { refusalWord } from "../../../domain/store";
import { useDialogFocus } from "../../../composables/useDialogFocus";
import { theaterAddress, useTheater } from "../../../composables/useTheater";
import { useTheaterEvents } from "../../../composables/useTheaterEvents";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Episode } from "../../../../shared/types";

const route = useRoute();
const { t } = useI18n();
// A drag is read from a pointer alone, so a finger of any width moves an item by its buttons.
const narrow = useNarrow(`${NARROW_QUERY}, (pointer: coarse)`);
const store = useTheater();

const failed = ref("");
const announced = ref("");
const hintId = "theater-reorder-hint";
const dragging = ref<number | null>(null);
const over = ref<number | null>(null);
const itemsRead = ref(false);
const renaming = ref(false);
const sharing = ref(false);

const playlistId = computed(() => Number(route.params.id));
const playlist = computed(() => store.playlistOf(playlistId.value) ?? null);
const actions = computed<BarAction[]>(() => [
  { id: "rename", label: t("theater.action.rename"), icon: "i-material-symbols-edit-outline", disabled: !playlist.value, onSelect: () => (renaming.value = true) },
  { id: "share", label: t("theater.action.share"), icon: "i-material-symbols-share-outline", disabled: !playlist.value, onSelect: () => (sharing.value = true) },
  { id: "delete", label: t("theater.action.delete"), icon: "i-material-symbols-delete-outline", disabled: !playlist.value, onSelect: () => void remove() }
]);
const progress = computed(() => store.progressOf("playlist", playlistId.value));
const backPath = computed(() => ({ path: "/theater/playlists", query: playlist.value?.realm === "organization" ? { realm: "organization" } : {} }));

// The items in order, each with the key it keeps while the order changes; an episode not read is left out.
const items = computed<{ key: string; at: number; episode: Episode }[]>(() => {
  const order = playlist.value?.episodeIds ?? [];
  const keys = itemKeys(order);

  return order.flatMap((id, index) => {
    const episode = store.episodeOf(id);

    return episode ? [{ key: keys[index]!, at: index, episode }] : [];
  });
});

const lastPlace = computed(() => (playlist.value?.episodeIds.length ?? 1) - 1);

const nextPath = computed(() => {
  const next = progress.value?.next;

  if (!next || !playlist.value) return null;

  const at = playlist.value.episodeIds.indexOf(next);
  const query: Record<string, string> = { playlist: String(playlistId.value), at: String(at) };

  if (progress.value?.nextSeconds) query.t = String(Math.floor(progress.value.nextSeconds));

  return { path: `/theater/watch/${next}`, query };
});

const road = computed(() => shareRoad(store.shareRoad({ playlistId: playlistId.value })));
const spans = computed(() => SHARE_SPANS.map((span) => ({ value: span.value, label: t(span.label) })));

watch(playlistId, () => void load(), { immediate: true });

watch(
  () => playlist.value?.episodeIds,
  (order) => {
    if (order) void store.loadEpisodesByIds(order);
  }
);

async function load(): Promise<void> {
  failed.value = "";
  itemsRead.value = false;

  try {
    const read = await store.loadPlaylist(playlistId.value);

    await Promise.all([store.loadEpisodesByIds(read.episodeIds), store.loadProgress("playlist", read.id)]);
    itemsRead.value = true;
  } catch (error) {
    failed.value = t(refusalWord(error));
  }
}

async function move(from: number, to: number): Promise<void> {
  if (!playlist.value || to < 0 || to >= playlist.value.episodeIds.length) return;

  const name = items.value.find((item) => item.at === from)?.episode.name ?? "";

  if (await store.reorderPlaylist(playlist.value, from, to)) announced.value = t("theater.status.moved", { name, n: countText(to + 1), all: countText(playlist.value.episodeIds.length) });
}

// The arrow keys on an item's handle move it a place, and the focus stays on its handle where it lands.
async function moveByKey(index: number, event: KeyboardEvent): Promise<void> {
  const to = event.key === "ArrowUp" ? index - 1 : event.key === "ArrowDown" ? index + 1 : null;

  if (to === null) return;

  event.preventDefault();
  await move(index, to);
  await nextTick();
  document.querySelectorAll<HTMLElement>("[data-reorder]")[Math.max(0, Math.min(to, lastPlace.value))]?.focus();
}

function start(index: number, event: DragEvent): void {
  dragging.value = index;
  event.dataTransfer?.setData("text/plain", String(index));
  if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
}

function finish(): void {
  dragging.value = null;
  over.value = null;
}

function drop(index: number): void {
  const from = dragging.value;

  finish();

  if (from !== null && from !== index) void move(from, index);
}

async function removeAt(index: number): Promise<void> {
  if (!playlist.value) return;

  const name = items.value.find((item) => item.at === index)?.episode.name ?? "";

  if (await store.removeFromPlaylist(playlist.value, index)) announced.value = t("theater.status.removed", { name });
}

async function rename(name: string): Promise<void> {
  if (playlist.value) await store.renamePlaylist(playlist.value, name);
}

async function remove(): Promise<void> {
  const standing = playlist.value;

  if (!standing) return;

  if (await askQuestion({ title: t("theater.question.delete_playlist"), item: standing.name, confirm: t("theater.action.delete") })) {
    if (await store.deletePlaylist(standing)) void navigateTo(backPath.value);
  }
}

// The share dialog is the layer's; where the focus goes as it comes and goes is said here.
useDialogFocus(sharing);

useTheaterEvents(
  () => void load(),
  (event) => {
    if (event.kind === "playlist.changed" && (event.data as { id?: number }).id === playlistId.value) void store.loadProgress("playlist", playlistId.value);
  }
);
</script>
