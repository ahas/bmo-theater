<template>
  <!-- Episodes put at the end of a playlist: one the reader reads that may hold them, or a new one made with
       them. An organization playlist holds organization episodes alone (`docs/design.md`, 「Two realms」). -->
  <SettingsDialog v-model:open="open" :title="$t('theater.heading.add_to_playlist')" :description="props.subject" :dirty="dirty">
    <UFormField :label="$t('theater.label.playlist')" name="playlist">
      <ChoiceSelect :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="target" :items="items" value-key="value" class="w-full" :disabled="busy" :aria-label="$t('theater.label.playlist')" />
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
import ChoiceSelect from "bmo-ui/app/components/ChoiceSelect.vue";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";

import { NEW_PLAYLIST as NEW, nameOk, playlistChoices, playlistRealms } from "../../domain/forms";
import { refusalWord } from "../../domain/store";
import { useDialogFocus } from "../../composables/useDialogFocus";
import { useTheater } from "../../composables/useTheater";

import type { Realm } from "../../../shared/types";

interface Props {
  episodeIds: number[];
  /** The realm the episodes stand in, which decides the playlists that may hold them. */
  realm: Realm;
  subject?: string;
}

const props = withDefaults(defineProps<Props>(), { subject: undefined });

const open = defineModel<boolean>("open", { default: false });

const { t } = useI18n();
const store = useTheater();

const target = ref(NEW);
const name = ref("");
const failed = ref("");
const busy = ref(false);

// The playlists of the realms that may hold the episodes (`playlistChoices`).
const realms = computed(() => playlistRealms(props.realm));
const items = computed(() => playlistChoices(t, realms.value, store.playlists.value));
const dirty = computed(() => target.value !== NEW || name.value.trim() !== "");
const ready = computed(() => !busy.value && (target.value !== NEW || nameOk(name.value)));

useDialogFocus(open);

// A screen may make the dialog in the same moment it opens it, so an open it is made with counts as an opening.
watch(
  open,
  (opened) => {
    if (!opened) return;

    target.value = NEW;
    name.value = "";
    failed.value = "";

    for (const realm of realms.value) if (!store.playlistsRead.value[realm]) void store.loadPlaylists(realm);
  },
  { immediate: true }
);

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
