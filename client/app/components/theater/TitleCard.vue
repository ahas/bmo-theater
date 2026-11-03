<template>
  <!-- One title in the library's grid: its poster, its name and what it is. The whole card opens it. -->
  <RouterLink
    :to="`/theater/titles/${props.title.id}`"
    :aria-label="name"
    class="group flex min-w-0 flex-col gap-2 rounded-lg p-2 transition-colors select-none hover:bg-elevated/60 active:bg-elevated"
  >
    <TheaterPoster :src="props.title.hasPoster ? theaterAddress(paths.titlePoster(props.title.id)) : null" :icon="KIND_ICONS[props.title.kind]" />
    <span class="flex min-w-0 flex-col">
      <span class="truncate text-base font-medium text-highlighted">{{ props.title.name }}</span>
      <span class="truncate text-sm text-muted tabular-nums">{{ $t(kindWord(props.title.kind)) }} · {{ $t("theater.label.episodes_count", { n: countText(props.title.episodeCount) }) }}</span>
    </span>
  </RouterLink>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { RouterLink } from "vue-router";
import { useI18n } from "vue-i18n";
import { countText } from "bmo-ui/app/domain/numbers";

import TheaterPoster from "./TheaterPoster.vue";
import { kindWord, titleName } from "../../domain/a11y";
import { paths } from "../../domain/api";
import { KIND_ICONS } from "../../domain/icons";
import { theaterAddress } from "../../composables/useTheater";

import type { Title } from "../../../shared/types";

const props = defineProps<{ title: Title }>();

const { t } = useI18n();

const name = computed(() => titleName(t, props.title, countText));
</script>
