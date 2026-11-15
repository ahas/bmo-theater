<template>
  <!-- One episode in a list: its poster, its code and name, what it is and how far the viewer has come as a bar
       along the poster's foot, a mark where it is watched, and its length at the row's end. On a phone the length
       stands on the line under the name, so the name keeps the row's width; where the row carries controls of its
       own the poster gives way too. The row plays it; its menu holds what else is done with it. -->
  <li class="flex min-w-0 items-center gap-2 px-2 sm:px-4" :aria-current="props.current ? 'true' : undefined">
    <slot name="lead" />

    <UPageCard
      :to="props.to"
      variant="ghost"
      :aria-label="name"
      class="min-w-0 flex-1"
      :class="props.current ? 'bg-elevated/60' : ''"
      :ui="ROW_UI"
    >
      <template #header>
        <span class="block w-24 sm:w-36" :class="props.compact ? 'hidden sm:block' : ''">
          <ArtworkFrame
            :src="props.poster"
            :size="posterSize(props.episode)"
            :shape="LANDSCAPE"
            :icon="props.episode.kind === 'clip' ? 'i-material-symbols-video-file-outline' : 'i-material-symbols-movie-outline'"
            :name="props.episode.name"
            class="rounded-md"
          >
            <UProgress
              v-if="watched.fraction > 0 && !watched.done"
              :model-value="percent(watched.fraction)"
              size="xs"
              class="absolute inset-x-0 bottom-0"
              :ui="{ base: 'rounded-none bg-black/50', indicator: 'rounded-none' }"
              aria-hidden="true"
            />
          </ArtworkFrame>
        </span>
      </template>

      <template #title>
        <span class="flex min-w-0 items-center gap-2">
          <span v-if="code" class="shrink-0 text-sm font-normal text-muted tabular-nums">{{ code }}</span>
          <span class="truncate">{{ props.episode.name }}</span>
        </span>
      </template>

      <template v-if="secondary" #description>
        <span class="flex min-w-0 items-center gap-2">
          <UIcon v-if="!props.episode.available" name="i-material-symbols-link-off" class="size-4 shrink-0 text-warning sm:hidden" aria-hidden="true" />
          <UIcon v-else-if="watched.done" name="i-material-symbols-check-circle" class="size-4 shrink-0 text-primary sm:hidden" aria-hidden="true" />
          <span class="truncate">{{ secondary }}</span>
        </span>
      </template>

      <span class="hidden shrink-0 items-center gap-2 text-sm text-muted tabular-nums sm:flex">
        <UIcon v-if="!props.episode.available" name="i-material-symbols-link-off" class="size-5 text-warning" aria-hidden="true" />
        <UIcon v-else-if="watched.done" name="i-material-symbols-check-circle" class="size-5 text-primary" aria-hidden="true" />
        <span>{{ lengthClock(props.episode.durationMs) }}</span>
      </span>
    </UPageCard>

    <slot name="trail">
      <UDropdownMenu v-if="props.menu.length" :modal="false" :ui="{ item: 'pointer-coarse:min-h-11' }" :items="props.menu" :content="{ align: 'end' }">
        <IconButton icon="i-material-symbols-more-vert" class="pointer-coarse:size-11 pointer-coarse:justify-center" :label="$t('common.action.more')" />
      </UDropdownMenu>
    </slot>
  </li>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ArtworkFrame from "bmo-ui/app/components/ArtworkFrame.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import { LANDSCAPE } from "bmo-ui/app/domain/artwork";
import { useNarrow } from "bmo-ui/app/domain/screen";

import { episodeLabel, episodeName, percent } from "../../domain/a11y";
import { posterSize } from "../../domain/cards";
import { episodeCode, lengthClock } from "../../domain/episodes";

import type { DropdownMenuItem } from "@nuxt/ui";
import type { RouteLocationRaw } from "vue-router";
import type { Watched } from "../../domain/episodes";
import type { Episode } from "../../../shared/types";

interface Props {
  episode: Episode;
  watched: Watched;
  to: RouteLocationRaw;
  /** The poster's address, or none. */
  poster: string | null;
  menu?: DropdownMenuItem[] | DropdownMenuItem[][];
  /** The episode playing now, in a list beside the player. */
  current?: boolean;
  /** Whether the poster gives way on a phone, for a row carrying controls of its own. */
  compact?: boolean;
}

const props = withDefaults(defineProps<Props>(), { menu: () => [], current: false, compact: false });

// The component library's page card laid out as a row: the poster, then the name and the line under it, then the
// length at the end; a row a finger presses is at least as tall as a finger.
const ROW_UI = {
  root: "rounded-md select-none active:bg-elevated",
  container: "flex min-h-11 flex-row items-center gap-3 p-2 sm:p-2 lg:flex",
  wrapper: "min-w-0 flex-1 flex-row items-center gap-3",
  header: "mb-0 shrink-0",
  body: "flex min-w-0 flex-1 flex-col gap-1",
  title: "min-w-0 text-base font-normal text-nowrap",
  description: "mt-0 min-w-0 text-sm text-muted tabular-nums text-nowrap"
} as const;

const { t } = useI18n();
const narrow = useNarrow();

const code = computed(() => episodeCode(props.episode));
// The line under the name: on a phone the length leads it and the watched mark is named beside its icon, since the
// row's end, where both stand on a wider screen, has no room for them.
const secondary = computed(() =>
  [narrow.value ? lengthClock(props.episode.durationMs) : "", episodeLabel(t, props.episode), props.episode.available ? "" : t("theater.label.unavailable"), narrow.value && props.watched.done ? t("theater.label.watched") : ""]
    .filter(Boolean)
    .join(" · ")
);
const watched = computed(() => props.watched);
const name = computed(() => episodeName(t, props.episode, watched.value));
</script>
