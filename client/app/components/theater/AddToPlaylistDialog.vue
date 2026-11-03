<template>
  <!-- Episodes put at the end of a playlist: one the reader reads that may hold them, or a new one made with
       them. An organization playlist holds organization episodes alone (`docs/design.md`, 「Two realms」). -->
  <SettingsDialog v-model:open="open" :title="$t('theater.heading.add_to_playlist')" :description="props.subject" :dirty="dirty">
    <UFormField :label="$t('theater.label.playlist')" name="playlist">
      <USelect :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="target" :items="items" value-key="value" class="w-full" :disabled="busy" :aria-label="$t('theater.label.playlist')" />
    </UFormField>

    <UFormField v-if="target === NEW" :label="$t('theater.label.new_playlist_name')" name="playlistName" required>
      <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="name" class="w-full" :disabled="busy" />
    </UFormField>

    <UAlert v-if="failed" color="error" variant="subtle" :description="failed" />

    <template #footer="{ close }">
      <UButton class="pointer-coarse:min-h-11" :label="$t('common.action.cancel')" color="neutral" variant="ghost" @click="close" />
      <UButton class="badge-ink pointer-coarse:min-h-11" :label="$t('theater.action.add')" :loading="busy" :disabled="!ready" @click="add" />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";

import { nameOk } from "../../domain/forms";
import { refusalWord } from "../../domain/store";
import { useDialogFocus } from "../../composables/useDialogFocus";
import { useTheater } from "../../composables/useTheater";

import type { SelectItem } from "@nuxt/ui";
import type { Realm } from "../../../shared/types";

interface Props {
  episodeIds: number[];
  /** The realm the episodes stand in, which decides the playlists that may hold them. */
  realm: Realm;
  subject?: string;
}

const props = withDefaults(defineProps<Props>(), { subject: undefined });

const open = defineModel<boolean>("open", { default: false });

const NEW = "new";

const { t } = useI18n();
const store = useTheater();

const target = ref(NEW);
const name = ref("");
const failed = ref("");
const busy = ref(false);

// A playlist of either realm holds an organization episode; only the reader's own holds one of theirs.
const realms = computed<Realm[]>(() => (props.realm === "organization" ? ["mine", "organization"] : ["mine"]));
const items = computed<SelectItem[]>(() => [
  { label: t("theater.label.new_playlist"), value: NEW },
  ...realms.value.flatMap((realm) => store.playlists.value[realm].map((playlist) => ({ label: `${playlist.name} · ${t(realm === "mine" ? "theater.label.mine" : "theater.label.organization")}`, value: String(playlist.id) })))
]);
const dirty = computed(() => target.value !== NEW || name.value.trim() !== "");
const ready = computed(() => !busy.value && (target.value !== NEW || nameOk(name.value)));

useDialogFocus(open);

watch(open, (opened) => {
  if (!opened) return;

  target.value = NEW;
  name.value = "";
  failed.value = "";

  for (const realm of realms.value) if (!store.playlistsRead.value[realm]) void store.loadPlaylists(realm);
});

async function add(): Promise<void> {
  busy.value = true;
  failed.value = "";

  try {
    const playlist = target.value === NEW ? await store.makePlaylist(props.realm, name.value.trim(), props.episodeIds) : await store.addToPlaylist(Number(target.value), props.episodeIds);

    store.island.say({ tone: "success", title: t("theater.message.added_to_playlist", { name: playlist.name }) });
    target.value = NEW;
    name.value = "";
    open.value = false;
  } catch (error) {
    failed.value = t(refusalWord(error));
  } finally {
    busy.value = false;
  }
}
</script>
