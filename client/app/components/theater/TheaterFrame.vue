<template>
  <!-- The frame every theater screen stands in (`docs/design.md`, 「The console」): the console's title bar where
       it offers one, the screen's own otherwise, with what acts on the screen as a whole at its right end; under
       it, on the screens the theater is switched between, a row of their tabs and the realm they show. -->
  <UDashboardPanel :id="props.id" :ui="{ body: 'p-0 sm:p-0 gap-0 sm:gap-0 overflow-y-hidden [&>*]:max-w-none' }">
    <template v-if="barOffer === null" #header>
      <UDashboardNavbar :title="$t('theater.title')" icon="i-material-symbols-theaters-outline">
        <template #right>
          <div class="flex items-center gap-2">
            <slot name="actions" />
          </div>
        </template>
      </UDashboardNavbar>
    </template>

    <template #body>
      <Teleport v-if="barOffer !== null" defer :to="barOffer.screenActions">
        <div class="flex items-center gap-2">
          <slot name="actions" />
        </div>
      </Teleport>

      <nav v-if="props.nav" :aria-label="$t('theater.aria.screens')" class="flex h-11 shrink-0 items-stretch" :class="TAB_ROW_LINE">
        <UTabs v-model="screen" :items="screens" :content="false" variant="link" :ui="TAB_ROW_UI" class="min-w-0 flex-1" />

        <TabSwitch v-if="props.realms" v-model="realm" :items="realmItems" :label="$t('theater.aria.realm')" :compact="narrow" class="[&>button]:min-w-11" />
      </nav>

      <slot />
    </template>
  </UDashboardPanel>
</template>

<script setup lang="ts">
import { computed, inject, onMounted } from "vue";
import { navigateTo, useRoute } from "nuxt/app";
import { useI18n } from "vue-i18n";
import TabSwitch from "bmo-ui/app/components/TabSwitch.vue";
import { useNarrow } from "bmo-ui/app/domain/screen";
import { TAB_ROW_LINE, TAB_ROW_UI } from "bmo-ui/app/domain/tabs";

import { useRealm } from "../../composables/useRealm";
import { useTheater } from "../../composables/useTheater";

import type { TabsItem } from "@nuxt/ui";
import type { SwitchItem } from "bmo-ui/app/domain/tabs";
import type { Realm } from "../../../shared/types";

type Screen = "library" | "playlists" | "libraries";

interface Props {
  id: string;
  /** Whether the row of the theater's screens stands under the bar. */
  nav?: boolean;
  /** Whether the realm switch stands in that row. */
  realms?: boolean;
}

const props = withDefaults(defineProps<Props>(), { nav: false, realms: false });

const SCREEN_PATHS: Record<Screen, string> = { library: "/theater", playlists: "/theater/playlists", libraries: "/theater/libraries" };

const { t } = useI18n();
const route = useRoute();
const narrow = useNarrow();
// Whether a console offers its title bar, under the key bmo-ui's bundle contract names.
const barOffer = inject<{ screenActions: string } | null>("bmo-console-title-bar", null);
const { realm } = useRealm();
const { administrator, checkAdministrator } = useTheater();

const screens = computed<TabsItem[]>(() => [
  { label: t("theater.label.library"), value: "library", icon: "i-material-symbols-video-library-outline" },
  { label: t("theater.label.playlists"), value: "playlists", icon: "i-material-symbols-playlist-play" },
  // The libraries are the administrator's alone; nothing of them is shown to anyone else.
  ...(administrator.value ? [{ label: t("theater.label.libraries"), value: "libraries", icon: "i-material-symbols-folder-managed-outline" }] : [])
]);

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
