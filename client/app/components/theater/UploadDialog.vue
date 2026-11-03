<template>
  <!-- Filing a video from the console: the title it goes under — one of the realm's, or a new one made for it —,
       the file, and its name; the bytes are sent as they are read, with how far they have come
       (`docs/api.md`, 「Titles」, filing a video). -->
  <SettingsDialog v-model:open="open" :title="$t('theater.heading.upload')" :dirty="dirty">
    <UFormField v-if="props.titleId === null" :label="$t('theater.label.title')" name="title">
      <USelect :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="target" :items="targetItems" value-key="value" class="w-full" :disabled="sending" :aria-label="$t('theater.label.title')" />
    </UFormField>

    <UFormField v-if="props.titleId === null && target === NEW" :label="$t('theater.label.new_title_name')" name="titleName" required>
      <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="titleName" class="w-full" :disabled="sending" />
    </UFormField>

    <UFormField :label="$t('theater.label.video_file')" name="file" required>
      <UFileUpload v-model="file" accept="video/*,.mkv,.avi,.ogv,.m4v,.mov" :label="$t('theater.label.drop_video')" :disabled="sending" layout="list" class="w-full" />
    </UFormField>

    <UFormField :label="$t('theater.label.episode_name')" name="name">
      <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="name" class="w-full" :placeholder="file ? bareName(file.name) : undefined" :disabled="sending" />
    </UFormField>

    <div v-if="sending" role="status" class="flex items-center gap-3 text-sm text-muted tabular-nums">
      <span class="shrink-0">{{ sentWords }}</span>
      <UProgress :model-value="share" size="sm" class="min-w-0 flex-1" :aria-label="$t('theater.aria.upload_progress')" />
    </div>

    <UAlert v-if="failed" color="error" variant="subtle" :description="failed" />

    <template #footer="{ close }">
      <UButton class="pointer-coarse:min-h-11" :label="$t('common.action.cancel')" color="neutral" variant="ghost" :disabled="sending" @click="close" />
      <UButton class="badge-ink pointer-coarse:min-h-11" :label="$t('theater.action.upload')" :loading="sending" :disabled="!ready" @click="send" />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";
import { countLocale } from "bmo-ui/app/domain/numbers";

import { bareName, fileForm, fileTargets, nameOk, sentText } from "../../domain/forms";
import { refusalWord } from "../../domain/store";
import { useDialogFocus } from "../../composables/useDialogFocus";
import { useTheater } from "../../composables/useTheater";

import type { SelectItem } from "@nuxt/ui";
import type { Episode, Realm } from "../../../shared/types";

interface Props {
  realm: Realm;
  /** The title the video goes under, where the screen already names one. */
  titleId?: number | null;
}

const props = withDefaults(defineProps<Props>(), { titleId: null });

const emit = defineEmits<{ filed: [episode: Episode] }>();

const open = defineModel<boolean>("open", { default: false });

const NEW = "new";

const { t } = useI18n();
const { titles, file: fileVideo, island } = useTheater();

const file = ref<File | null | undefined>(null);
const target = ref<string>(NEW);
const titleName = ref("");
const name = ref("");
const failed = ref("");
const sent = ref(0);
const total = ref(0);
const sending = ref(false);

const targetItems = computed<SelectItem[]>(() => [
  { label: t("theater.label.new_title"), value: NEW },
  ...fileTargets(titles.value[props.realm]).map((title) => ({ label: title.name, value: String(title.id) }))
]);

const dirty = computed(() => !!file.value || titleName.value.trim() !== "" || name.value.trim() !== "");
const ready = computed(() => !!file.value && (props.titleId !== null || target.value !== NEW || nameOk(titleName.value)) && !sending.value);
const share = computed(() => (total.value > 0 ? Math.floor((sent.value / total.value) * 100) : 0));
const sentWords = computed(() => sentText(sent.value, total.value || (file.value?.size ?? 0), countLocale()));

useDialogFocus(open);

watch(open, (opened) => {
  if (!opened) return;

  file.value = null;
  target.value = NEW;
  titleName.value = "";
  name.value = "";
  failed.value = "";
  sent.value = 0;
  total.value = 0;
});

async function send(): Promise<void> {
  const chosen = file.value;

  if (!chosen) return;

  sending.value = true;
  failed.value = "";
  sent.value = 0;
  total.value = chosen.size;

  try {
    const where = props.titleId !== null ? { titleId: props.titleId } : target.value === NEW ? { realm: props.realm, name: titleName.value.trim() } : { titleId: Number(target.value) };
    const episode = await fileVideo(where, fileForm(chosen, { name: name.value }), (done, all) => {
      sent.value = done;
      if (all > 0) total.value = all;
    });

    island.say({ tone: "success", title: t("theater.message.uploaded", { name: episode.name }) });
    emit("filed", episode);
    // What was sent is no change any more, so the dialog closes asking nothing.
    file.value = null;
    titleName.value = "";
    name.value = "";
    open.value = false;
  } catch (error) {
    failed.value = t(refusalWord(error));
  } finally {
    sending.value = false;
  }
}
</script>
