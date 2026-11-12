<template>
  <!-- The libraries, the administrator's alone (`docs/api.md`, 「Libraries」): each registered folder with what its
       last scan found, a scan asked for again, a rename and its removal; a folder registered from here. Anyone
       else is shown nothing of them. -->
  <TheaterFrame id="theater-libraries" nav :actions="actions">
    <div v-if="administrator" class="min-h-0 flex-1 overflow-y-auto">
      <p v-if="librariesFailed && !libraries.length" role="alert" class="badge-ink p-4 text-sm text-error">{{ $t("theater.error.libraries_failed") }}</p>

      <ContentSkeleton v-else-if="!librariesRead" list-class="flex flex-col py-2" :rows="2" :label="$t('theater.status.libraries_loading')">
        <div class="flex flex-col gap-1 px-4 py-2"><SkeletonLine size="base" width="w-1/3" /><SkeletonLine width="w-2/3" /><SkeletonLine width="w-1/4" /></div>
      </ContentSkeleton>

      <p v-else-if="!libraries.length" class="p-4 text-base text-muted">{{ $t("theater.message.no_libraries") }}</p>

      <MovingList v-else class="flex flex-col py-2" :aria-label="$t('theater.label.libraries')">
        <li v-for="library in libraries" :key="library.id" class="flex min-w-0 items-center gap-2 px-4 py-2">
          <UIcon :name="library.scanning ? 'i-material-symbols-progress-activity' : 'i-material-symbols-folder-outline'" class="size-6 shrink-0 text-muted" :class="library.scanning ? 'animate-spin motion-reduce:animate-none' : ''" aria-hidden="true" />

          <div class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-base text-highlighted">{{ library.name }}</span>
            <span class="font-mono text-sm wrap-anywhere text-muted">{{ library.path }}</span>
            <span class="truncate text-sm text-muted tabular-nums">
              {{ $t(library.realm === "mine" ? "theater.label.mine" : "theater.label.organization") }} · {{ $t("theater.label.titles_count", { n: countText(library.titleCount) }) }} · {{ $t("theater.label.episodes_count", { n: countText(library.episodeCount) }) }} ·
              {{ library.scanning ? $t("theater.status.scanning") : library.scannedAt ? $t("theater.label.scanned_at", { at: formatDateClock(Math.floor(library.scannedAt / 1000)) }) : $t("theater.label.never_scanned") }}
            </span>
          </div>

          <div class="flex shrink-0 items-center">
            <UTooltip :text="$t('theater.action.rescan')">
              <UButton color="neutral" variant="ghost" icon="i-material-symbols-sync" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('theater.action.rescan')" :loading="library.scanning" :disabled="library.scanning" @click="store.scanLibrary(library)" />
            </UTooltip>
            <UTooltip :text="$t('theater.action.rename')">
              <UButton color="neutral" variant="ghost" icon="i-material-symbols-edit-outline" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('theater.action.rename')" @click="openRename(library)" />
            </UTooltip>
            <UTooltip :text="$t('theater.action.delete')">
              <UButton color="neutral" variant="ghost" icon="i-material-symbols-delete-outline" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('theater.action.delete')" @click="remove(library)" />
            </UTooltip>
          </div>
        </li>
      </MovingList>
    </div>

    <LibraryDialog v-model:open="registering" />
    <NameDialog v-model:open="renaming" :title="$t('theater.heading.rename_library')" :action="$t('common.action.save')" :current="chosen?.name ?? ''" :send="rename" />
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { navigateTo } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import MovingList from "bmo-ui/app/components/MovingList.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { countText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";
import { formatDateClock } from "bmo-ui/app/domain/time";

import LibraryDialog from "../../components/theater/LibraryDialog.vue";
import NameDialog from "../../components/theater/NameDialog.vue";
import TheaterFrame from "../../components/theater/TheaterFrame.vue";
import { useTheater } from "../../composables/useTheater";
import { useTheaterEvents } from "../../composables/useTheaterEvents";

import type { BarAction } from "bmo-ui/app/domain/tabs";
import type { Library } from "../../../shared/types";

const { t } = useI18n();
const store = useTheater();
const { administrator, libraries, librariesRead, librariesFailed, loadLibraries, checkAdministrator } = store;

const chosen = ref<Library | null>(null);
const registering = ref(false);
const renaming = ref(false);
const refreshing = ref(false);

// Registering and scanning are the administrator's alone, so nobody else is offered them.
const actions = computed<BarAction[]>(() =>
  administrator.value
    ? [
        { id: "register", label: t("theater.action.register"), icon: "i-material-symbols-create-new-folder-outline", onSelect: () => (registering.value = true) },
        { id: "refresh", label: t("theater.action.refresh"), icon: "i-material-symbols-refresh", loading: refreshing.value, onSelect: () => void refresh() }
      ]
    : []
);

// A reader who does not administer the theater is taken back to the library: this screen is not theirs.
watch(
  administrator,
  (administers) => {
    if (administers === false) void navigateTo("/theater", { replace: true });
    else if (administers && !librariesRead.value) void loadLibraries();
  },
  { immediate: true }
);

void checkAdministrator();

function openRename(library: Library): void {
  chosen.value = library;
  renaming.value = true;
}

async function rename(name: string): Promise<void> {
  if (chosen.value) await store.renameLibrary(chosen.value, name);
}

async function remove(library: Library): Promise<void> {
  const sure = await askQuestion({ title: t("theater.question.delete_library"), item: library.name, effect: t("theater.question.delete_library_effect"), confirm: t("theater.action.delete") });

  if (sure) await store.deleteLibrary(library);
}

async function refresh(): Promise<void> {
  refreshing.value = true;

  try {
    await loadLibraries();
  } finally {
    refreshing.value = false;
  }
}

useTheaterEvents(() => void loadLibraries());
</script>
