<template>
  <!-- A row of the library in full (`docs/design.md`, 「The console」, a row in full): the realm's titles, or those
       of one kind, as a grid of cards at the one shape most of them have, in the order the screen's row chooses —
       added last first, by name, or watched last first. The realm, the kind and the order stand in the address. -->
  <TheaterFrame id="theater-browse">
    <ToolBar :label="$t('theater.aria.browse_bar')">
      <div>
        <IconButton size="sm" icon="i-material-symbols-arrow-back" :label="$t('theater.label.library')" :to="{ path: '/theater', query: realmQuery }" />
      </div>
      <div class="min-w-0 flex-1 shrink!">
        <span class="truncate text-highlighted">{{ heading }}</span>
      </div>
      <div data-slot="order">
        <USelect v-model="order" :items="orderItems" value-key="value" variant="ghost" size="sm" :aria-label="$t('theater.label.order')" />
      </div>
    </ToolBar>

    <div ref="scroller" class="min-h-0 flex-1 overflow-y-auto" :class="CARD_MEASURE" :style="{ '--theater-cell': cardWidth(shape) }">
      <p v-if="failed" role="alert" class="badge-ink p-4 text-sm text-error">{{ $t("theater.error.titles_failed") }}</p>

      <ContentSkeleton v-else-if="shown === null" :list-class="GRID" :rows="12" :label="$t('theater.status.titles_loading')">
        <div class="flex flex-col gap-2 p-2">
          <SkeletonBar class="aspect-video w-full rounded-md" />
          <SkeletonLine size="base" width="w-3/4" />
          <SkeletonLine width="w-1/2" />
        </div>
      </ContentSkeleton>

      <p v-else-if="!shown.length" class="p-4 text-base text-muted">{{ $t(realm === "mine" ? "theater.message.no_titles_mine" : "theater.message.no_titles_organization") }}</p>

      <ul v-else :class="GRID" :aria-label="heading">
        <li v-for="title in shown" :key="title.id" class="min-w-0">
          <TitleCard :title="title" :shape="shape" />
        </li>
      </ul>
    </div>
  </TheaterFrame>
</template>

<script setup lang="ts">
import { computed, ref, useTemplateRef, watch } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import IconButton from "bmo-ui/app/components/IconButton.vue";
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import ToolBar from "bmo-ui/app/components/ToolBar.vue";
import { LANDSCAPE } from "bmo-ui/app/domain/artwork";

import TheaterFrame from "../../components/theater/TheaterFrame.vue";
import TitleCard from "../../components/theater/TitleCard.vue";
import { kindWord } from "../../domain/a11y";
import { CARD_MEASURE, cardWidth, titlesShape } from "../../domain/cards";
import { ROW_KINDS } from "../../domain/rows";
import { useKeptScroll } from "../../composables/useKeptScroll";
import { useRealm } from "../../composables/useRealm";
import { theaterApi } from "../../composables/useTheater";
import { useTheaterEvents } from "../../composables/useTheaterEvents";

import type { SelectItem } from "@nuxt/ui";
import type { Title, TitleKind, TitleOrder } from "../../../shared/types";

type Order = Exclude<TitleOrder, "id">;

// The orders a row in full offers, each with the key of the words it is chosen by, the first where the address
// names none.
const ORDERS: readonly { value: Order; words: string }[] = [
  { value: "added", words: "theater.label.order_added" },
  { value: "name", words: "theater.label.order_name" },
  { value: "watched", words: "theater.label.order_watched" }
];

// The cards stand as many to a row as fit at a little under their own shape's width in a row, filling the screen,
// so a phone holds two of them side by side.
const GRID = "grid grid-cols-[repeat(auto-fill,minmax(calc(var(--theater-cell)*0.8),1fr))] gap-2 p-2 sm:gap-4 sm:p-4";

const { t } = useI18n();
const route = useRoute();
const api = theaterApi();
const scrollerRef = useTemplateRef<HTMLElement>("scroller");

const { realm } = useRealm();

const read = ref<Title[] | null>(null);
const failed = ref(false);

const realmQuery = computed(() => (realm.value === "organization" ? { realm: "organization" } : {}));
const kind = computed<TitleKind | null>(() => ROW_KINDS.find((one) => one === route.query.kind) ?? null);
const order = computed<Order>({
  get: () => ORDERS.find((one) => one.value === route.query.order)?.value ?? "added",
  set: (value) => void navigateTo({ path: route.path, query: { ...route.query, order: value === "added" ? undefined : value } })
});
const orderItems = computed<SelectItem[]>(() => ORDERS.map((one) => ({ value: one.value, label: t(one.words) })));
const heading = computed(() => (kind.value ? t(kindWord(kind.value)) : t("theater.label.recently_added")));
const shown = computed(() => (read.value === null ? null : read.value.filter((title) => kind.value === null || title.kind === kind.value)));
const shape = computed(() => (shown.value ? titlesShape(shown.value) : LANDSCAPE));

async function load(): Promise<void> {
  try {
    read.value = await api.titles(realm.value, order.value);
    failed.value = false;
  } catch {
    failed.value = read.value === null;
  }
}

watch([realm, order], () => {
  read.value = null;
  void load();
}, { immediate: true });

useKeptScroll(scrollerRef, () => read.value !== null);

useTheaterEvents(
  () => void load(),
  (event) => {
    if (event.kind.startsWith("title.") || event.kind === "episode.created" || (order.value === "watched" && event.kind === "position.changed")) void load();
  }
);
</script>
