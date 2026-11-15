<template>
  <!-- One playlist as a card in the library's row of playlists: its mark over its name, how many episodes it holds
       and how long they run. The whole card opens it.
       It stands at the width its row hands it; a grid's cell gives it its own. -->
  <div class="min-w-0" :style="{ width: props.width }">
    <UPageCard :to="`/theater/playlists/${props.playlist.id}`" variant="ghost" :title="props.playlist.name" :description="line" :ui="CARD_UI">
      <template #header>
        <ArtworkFrame :src="null" :shape="props.shape" icon="i-material-symbols-playlist-play" :name="props.playlist.name" class="rounded-md" />
      </template>
    </UPageCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ArtworkFrame from "bmo-ui/app/components/ArtworkFrame.vue";
import { countText } from "bmo-ui/app/domain/numbers";

import { CARD_UI } from "../../domain/cards";
import { lengthClock } from "../../domain/episodes";

import type { ArtworkShape } from "bmo-ui/app/domain/artwork";
import type { Playlist } from "../../../shared/types";

const props = defineProps<{ playlist: Playlist; shape: ArtworkShape; width?: string }>();

const { t } = useI18n();

const line = computed(() => [t("theater.label.episodes_count", { n: countText(props.playlist.episodeIds.length) }), lengthClock(props.playlist.durationMs)].filter(Boolean).join(" · "));
</script>
