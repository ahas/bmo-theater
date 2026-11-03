<template>
  <!-- A folder registered as a library: its path on the service's host, the realm its titles stand in, and its
       name where the folder's own should not be used. Registering it starts its first scan
       (`docs/api.md`, 「Libraries」). -->
  <SettingsDialog v-model:open="open" :title="$t('theater.heading.new_library')" :dirty="dirty">
    <UFormField :label="$t('theater.label.folder')" name="path" required>
      <UInput v-model="path" class="w-full" :ui="{ base: 'font-mono pointer-coarse:min-h-11' }" :disabled="busy" />
    </UFormField>

    <UFormField :label="$t('theater.label.realm')" name="realm">
      <USelect :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="realm" :items="realmItems" value-key="value" class="w-full" :disabled="busy" :aria-label="$t('theater.label.realm')" />
    </UFormField>

    <UFormField :label="$t('theater.label.name')" name="name" :hint="$t('theater.label.optional')">
      <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="name" class="w-full" :disabled="busy" />
    </UFormField>

    <UAlert v-if="failed" color="error" variant="subtle" :description="failed" />

    <template #footer="{ close }">
      <UButton class="pointer-coarse:min-h-11" :label="$t('common.action.cancel')" color="neutral" variant="ghost" @click="close" />
      <UButton class="badge-ink pointer-coarse:min-h-11" :label="$t('theater.action.register')" :loading="busy" :disabled="!path.trim().startsWith('/') || busy" @click="register" />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";

import { refusalWord } from "../../domain/store";
import { useDialogFocus } from "../../composables/useDialogFocus";
import { useTheater } from "../../composables/useTheater";

import type { SelectItem } from "@nuxt/ui";
import type { Realm } from "../../../shared/types";

const open = defineModel<boolean>("open", { default: false });

const { t } = useI18n();
const store = useTheater();

const realm = ref<Realm>("mine");
const path = ref("");
const name = ref("");
const failed = ref("");
const busy = ref(false);

const realmItems = computed<SelectItem[]>(() => [
  { label: t("theater.label.mine"), value: "mine" },
  { label: t("theater.label.organization"), value: "organization" }
]);
const dirty = computed(() => path.value.trim() !== "" || name.value.trim() !== "" || realm.value !== "mine");

useDialogFocus(open);

watch(open, (opened) => {
  if (!opened) return;

  realm.value = "mine";
  path.value = "";
  name.value = "";
  failed.value = "";
});

async function register(): Promise<void> {
  busy.value = true;
  failed.value = "";

  try {
    const made = await store.makeLibrary(realm.value, path.value.trim(), name.value.trim() || null);

    store.island.say({ tone: "success", title: t("theater.message.library_registered", { name: made.name }) });
    path.value = "";
    name.value = "";
    realm.value = "mine";
    open.value = false;
  } catch (error) {
    failed.value = t(refusalWord(error));
  } finally {
    busy.value = false;
  }
}
</script>
