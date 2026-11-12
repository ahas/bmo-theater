<template>
  <!-- The frame every theater screen stands in (`docs/design.md`, 「The console」): the console's title bar where
       it offers one, the screen's own otherwise, holding the theater's name and mark alone. What acts on the
       screen stands in the screen's own row directly under that bar: on the screens the theater is switched
       between, the frame draws that row, with their tabs and the realm they show, and the screen's actions at
       its right end; every other screen draws its toolbar there itself. -->
  <UDashboardPanel :id="props.id" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <template v-if="!barOffered" #header>
      <UDashboardNavbar :title="$t('theater.title')" icon="i-material-symbols-theaters-outline" />
    </template>

    <template #body>
      <nav v-if="props.nav" :aria-label="$t('theater.aria.screens')" class="flex h-11 shrink-0 items-stretch" :class="TAB_ROW_LINE">
        <UTabs v-model="screen" :items="screens" :content="false" variant="link" :ui="TAB_ROW_UI" class="min-w-0 flex-1">
          <!-- On a narrow screen a tab is named by its mark alone, so the screen's actions keep standing beside
               the row; its name stays as its accessible name and shows on hover anywhere over the tab, through a
               layer laid over the whole tab that a press passes through to it. -->
          <template #leading="{ item, ui }">
            <UIcon :name="item.icon!" data-slot="leadingIcon" :class="ui.leadingIcon()" />
            <UTooltip v-if="narrow" :text="item.label">
              <span class="absolute inset-0" aria-hidden="true" />
            </UTooltip>
          </template>
        </UTabs>

        <!-- The realm switch and the screen's actions each stand as a group of small buttons centred in the
             row, parted from the tabs by a line, as bmo-ui's own tab row stands its switch and actions. -->
        <div v-if="props.realms" class="flex shrink-0 items-center border-l border-default px-1">
          <TabSwitch v-model="realm" :items="realmItems" :label="$t('theater.aria.realm')" :compact="narrow" />
        </div>

        <div v-if="props.actions.length" class="flex shrink-0 items-center border-l border-default px-1">
          <ActionGroup :actions="props.actions" :narrow-fold="false" />
        </div>
      </nav>

      <slot />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { computed, inject, onMounted } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import ActionGroup from "bmo-ui/app/components/ActionGroup.vue";
import TabSwitch from "bmo-ui/app/components/TabSwitch.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { TAB_ROW_LINE, TAB_ROW_UI } from "bmo-ui/app/domain/tabs";

import { useRealm } from "../../composables/useRealm";
import { useTheater } from "../../composables/useTheater";

import type { TabsItem } from "@nuxt/ui";
import type { BarAction, SwitchItem } from "bmo-ui/app/domain/tabs";
import type { Realm } from "../../../shared/types";

type Screen = "library" | "playlists" | "libraries";

interface Props {
  id: string;
  /** Whether the row of the theater's screens stands under the bar. */
  nav?: boolean;
  /** Whether the realm switch stands in that row. */
  realms?: boolean;
  /** What acts on the screen as a whole, at the right end of that row. */
  actions?: readonly BarAction[];
}

const props = withDefaults(defineProps<Props>(), { nav: false, realms: false, actions: () => [] });

const SCREEN_PATHS: Record<Screen, string> = { library: "/theater", playlists: "/theater/playlists", libraries: "/theater/libraries" };

const { t } = useI18n();
const route = useRoute();
const narrow = useNarrow();
// Whether a console draws its title bar over the screen, under the key bmo-ui's bundle contract names. Nothing of
// the screen is laid into that bar, so the offer is read for whether it stands and nothing else.
const barOffered = inject<object | null>("bmo-console-title-bar", null) !== null;
const { realm } = useRealm();
const { administrator, checkAdministrator } = useTheater();

const screens = computed<TabsItem[]>(() =>
  [
    { label: t("theater.label.library"), value: "library", icon: "i-material-symbols-video-library-outline" },
    { label: t("theater.label.playlists"), value: "playlists", icon: "i-material-symbols-playlist-play" },
    // The libraries are the administrator's alone; nothing of them is shown to anyone else.
    ...(administrator.value ? [{ label: t("theater.label.libraries"), value: "libraries", icon: "i-material-symbols-folder-managed-outline" }] : [])
  ].map((item) => (narrow.value ? { ...item, ui: { label: "sr-only" } } : item))
);

const screen = computed<Screen>({
  get: () => (route.path.startsWith(SCREEN_PATHS.libraries) ? "libraries" : route.path.startsWith(SCREEN_PATHS.playlists) ? "playlists" : "library"),
  set: (value) => void navigateTo({ path: SCREEN_PATHS[value], query: value === "libraries" ? {} : { realm: route.query.realm } })
});

const realmItems = computed<SwitchItem<Realm>[]>(() => [
  { value: "mine", label: t("theater.label.mine"), icon: "i-material-symbols-person-outline" },
  { value: "organization", label: t("theater.label.organization"), icon: "i-material-symbols-groups-outline" }
]);

onMounted(() => void checkAdministrator());
</script>
