<template>
  <!-- One episode as a wide card in a row of continue watching or next up (`docs/design.md`, 「The console」): its
       poster with how far it was watched along its foot, the title's name, the episode's code and name and the time
       left. The whole card plays it; where the row offers it, a button over the picture sets it aside.
       It stands at the width its row hands it; a grid's cell gives it its own. -->
  <div class="min-w-0" :style="{ width: props.width }">
    <UPageCard :to="props.to" variant="ghost" :title="titleName" :description="line" :aria-label="name" :ui="CARD_UI">
      <template #header>
        <ArtworkFrame
          :src="props.episode.hasPoster ? theaterAddress(paths.episodePoster(props.episode.id)) : null"
          :size="posterSize(props.episode)"
          :shape="props.shape"
          :icon="props.episode.kind === 'clip' ? 'i-material-symbols-video-file-outline' : 'i-material-symbols-movie-outline'"
          :name="episodeTitle(props.episode)"
          class="rounded-md"
        >
          <UProgress
            v-if="watched.fraction > 0"
            :model-value="percent(watched.fraction)"
            size="xs"
            class="absolute inset-x-0 bottom-0"
            :ui="{ base: 'rounded-none bg-black/50', indicator: 'rounded-none' }"
            aria-hidden="true"
          />
        </ArtworkFrame>
      </template>

      <div v-if="props.setAside" class="absolute end-3 top-3 z-10">
        <IconButton icon="i-material-symbols-close" size="sm" color="neutral" variant="solid" :label="$t('theater.action.set_aside')" @click.prevent.stop="emit('setAside')" />
      </div>
    </UPageCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ArtworkFrame from "bmo-ui/app/components/ArtworkFrame.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";

import { episodeLabel, episodeName, percent } from "../../domain/a11y";
import { paths } from "../../domain/api";
import { CARD_UI, posterSize } from "../../domain/cards";
import { episodeCode, episodeTitle, minutesLeft, watchedEpisode } from "../../domain/episodes";
import { theaterAddress } from "../../composables/useTheater";

import type { ArtworkShape } from "bmo-ui/app/domain/artwork";
import type { RouteLocationRaw } from "vue-router";
import type { Episode, Title } from "../../../shared/types";

interface Props {
  episode: Episode;
  /** The title it stands under, where it is read. */
  title: Title | null;
  to: RouteLocationRaw;
  shape: ArtworkShape;
  width?: string;
  /** Whether the card offers to set the episode aside. */
  setAside?: boolean;
}

const props = withDefaults(defineProps<Props>(), { width: undefined, setAside: false });

const emit = defineEmits<{ setAside: [] }>();

const { t } = useI18n();

const watched = computed(() => watchedEpisode(props.episode));
const titleName = computed(() => props.title?.name ?? episodeTitle(props.episode));
// The line under the name: the episode's code and name where a title's name heads the card, then the time left.
const line = computed(() => {
  const left = minutesLeft(props.episode);

  return [props.title ? [episodeCode(props.episode), props.episode.name].filter(Boolean).join(" ") : episodeLabel(t, props.episode), left === null ? "" : t("common.label.left_minutes", { n: String(left) })]
    .filter(Boolean)
    .join(" · ");
});
const name = computed(() => [props.title?.name, episodeName(t, props.episode, watched.value)].filter(Boolean).join(", "));
</script>
