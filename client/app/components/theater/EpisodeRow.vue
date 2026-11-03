<template>
  <!-- One episode in a list: its poster, its name, its code, what it is and how far the viewer has come as a
       bar, and its length at the row's end. On a phone the code and the length stand on the line under the name,
       so the name keeps the row's width; where the row carries controls of its own the poster gives way too.
       The row plays it; its menu holds what else is done with it. -->
  <li class="flex min-w-0 items-center gap-2 px-2 sm:px-4" :aria-current="props.current ? 'true' : undefined">
    <slot name="lead" />

    <RouterLink
      :to="props.to"
      :aria-label="name"
      class="flex min-h-11 min-w-0 flex-1 items-center gap-3 rounded-md p-2 transition-colors select-none hover:bg-elevated/60 active:bg-elevated focus-inset"
      :class="props.current ? 'bg-elevated/60' : ''"
    >
      <span class="w-24 shrink-0 sm:w-36" :class="props.compact ? 'hidden sm:block' : ''">
        <TheaterPoster :src="poster" :icon="props.episode.kind === 'clip' ? 'i-material-symbols-video-file-outline' : 'i-material-symbols-movie-outline'" />
      </span>

      <span class="flex min-w-0 flex-1 flex-col gap-1">
        <span class="flex min-w-0 items-center gap-2">
          <span v-if="code" class="hidden shrink-0 text-sm text-muted tabular-nums sm:inline">{{ code }}</span>
          <span class="truncate text-base text-highlighted">{{ props.episode.name }}</span>
        </span>
        <span v-if="secondary" class="flex min-w-0 items-center gap-2 text-sm text-muted tabular-nums">
          <UIcon v-if="!props.episode.available" name="i-material-symbols-link-off" class="size-4 shrink-0 text-warning sm:hidden" aria-hidden="true" />
          <UIcon v-else-if="watched.done" name="i-material-symbols-check-circle-outline" class="size-4 shrink-0 text-primary sm:hidden" aria-hidden="true" />
          <span class="truncate">{{ secondary }}</span>
        </span>
        <UProgress v-if="watched.fraction > 0" :model-value="percent(watched.fraction)" size="xs" :aria-hidden="true" />
      </span>

      <span class="hidden shrink-0 items-center gap-2 text-sm text-muted tabular-nums sm:flex">
        <UIcon v-if="!props.episode.available" name="i-material-symbols-link-off" class="size-5 text-warning" aria-hidden="true" />
        <UIcon v-else-if="watched.done" name="i-material-symbols-check-circle-outline" class="size-5 text-primary" aria-hidden="true" />
        <span>{{ lengthClock(props.episode.durationMs) }}</span>
      </span>
    </RouterLink>

    <slot name="trail">
      <UDropdownMenu :modal="false" :ui="{ item: 'pointer-coarse:min-h-11' }" v-if="props.menu.length" :items="props.menu" :content="{ align: 'end' }">
        <UTooltip :text="$t('common.action.more')">
          <UButton color="neutral" variant="ghost" icon="i-material-symbols-more-vert" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('common.action.more')" />
        </UTooltip>
      </UDropdownMenu>
    </slot>
  </li>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { useI18n } from "vue-i18n";
import { useNarrow } from "bmo-ui/app/domain/screen";

import TheaterPoster from "./TheaterPoster.vue";
import { episodeLabel, episodeName, percent } from "../../domain/a11y";
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

const { t } = useI18n();

const code = computed(() => episodeCode(props.episode));
const poster = computed(() => props.poster);
// The line under the name: on a phone the code and the length lead it, since the row's end has no room for them.
const narrow = useNarrow();
const secondary = computed(() =>
  [narrow.value ? code.value : "", narrow.value ? lengthClock(props.episode.durationMs) : "", episodeLabel(t, props.episode), props.episode.available ? "" : t("theater.label.unavailable")].filter(Boolean).join(" · ")
);
const watched = computed(() => props.watched);
const name = computed(() => episodeName(t, props.episode, watched.value));
</script>
