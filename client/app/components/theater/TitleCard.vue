<template>
  <!-- One title as a card (`docs/design.md`, 「The console」, the cards): its poster at the shape its row stands at,
       its name and what it is. The whole card opens it.
       It stands at the width its row hands it; a grid's cell gives it its own. -->
  <div class="min-w-0" :style="{ width: props.width }">
    <UPageCard :to="`/theater/titles/${props.title.id}`" variant="ghost" :title="props.title.name" :description="line" :aria-label="name" :ui="CARD_UI">
      <template #header>
        <ArtworkFrame
          :src="props.title.hasPoster ? theaterAddress(paths.titlePoster(props.title.id)) : null"
          :size="posterSize(props.title)"
          :shape="props.shape"
          :icon="KIND_ICONS[props.title.kind]"
          :name="props.title.name"
          class="rounded-md"
        />
      </template>
    </UPageCard>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import ArtworkFrame from "bmo-ui/app/components/ArtworkFrame.vue";
import { countText } from "bmo-ui/app/domain/numbers";

import { kindWord, titleName } from "../../domain/a11y";
import { paths } from "../../domain/api";
import { CARD_UI, posterSize } from "../../domain/cards";
import { KIND_ICONS } from "../../domain/icons";
import { theaterAddress } from "../../composables/useTheater";

import type { ArtworkShape } from "bmo-ui/app/domain/artwork";
import type { Title } from "../../../shared/types";

interface Props {
  title: Title;
  /** The shape its row or grid stands at. */
  shape: ArtworkShape;
  /** The width it stands at in a row; a grid's cell gives it its own. */
  width?: string;
}

const props = withDefaults(defineProps<Props>(), { width: undefined });

const { t } = useI18n();

const line = computed(() => `${t(kindWord(props.title.kind))} · ${t("theater.label.episodes_count", { n: countText(props.title.episodeCount) })}`);
const name = computed(() => titleName(t, props.title, countText));
</script>
