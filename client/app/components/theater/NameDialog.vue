<template>
  <!-- One name written: a playlist made or renamed, a library renamed. The save is the one action, enabled only
       while the name differs from the one standing and reads as a name. -->
  <SettingsDialog v-model:open="open" :title="props.title" :dirty="dirty">
    <form class="flex flex-col gap-4" @submit.prevent="save">
      <UFormField :label="$t('theater.label.name')" name="name" required :error="failed || undefined">
        <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="name" class="w-full" :disabled="busy" />
      </UFormField>
    </form>

    <template #footer="{ close }">
      <UButton class="pointer-coarse:min-h-11" :label="$t('common.action.cancel')" color="neutral" variant="ghost" @click="close" />
      <UButton class="badge-ink pointer-coarse:min-h-11" :label="props.action" :loading="busy" :disabled="!dirty || !nameOk(name) || busy" @click="save" />
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

interface Props {
  title: string;
  /** The save's words: what the save does. */
  action: string;
  /** The name standing, empty for something new. */
  current?: string;
  /** Sends the name; it throws where the service refuses it. */
  send: (name: string) => Promise<unknown>;
}

const props = withDefaults(defineProps<Props>(), { current: "" });

const open = defineModel<boolean>("open", { default: false });

const { t } = useI18n();

const name = ref("");
const failed = ref("");
const busy = ref(false);

const dirty = computed(() => name.value.trim() !== props.current.trim());

useDialogFocus(open);

watch(open, (opened) => {
  if (!opened) return;

  name.value = props.current;
  failed.value = "";
});

async function save(): Promise<void> {
  if (!dirty.value || !nameOk(name.value) || busy.value) return;

  busy.value = true;
  failed.value = "";

  try {
    await props.send(name.value.trim());
    // What was saved is no change any more, so the dialog closes asking nothing.
    name.value = props.current;
    open.value = false;
  } catch (error) {
    failed.value = t(refusalWord(error));
  } finally {
    busy.value = false;
  }
}
</script>
