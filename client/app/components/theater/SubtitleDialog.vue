<template>
  <!-- An episode's subtitles: each one with its burn, and a file added to them (`docs/api.md`, 「Subtitles」 and
       「Burns」). A subtitle burned into the picture is filed as a clip under the same title once the burn ends. -->
  <SettingsDialog v-model:open="open" :title="$t('theater.heading.subtitles')" :description="props.episode?.name" :dirty="dirty">
    <section :aria-label="$t('theater.heading.subtitles')" class="flex flex-col gap-2">
      <ContentSkeleton v-if="loading" list-class="flex flex-col gap-2" :rows="Math.max(1, props.episode?.subtitleCount ?? 1)" :label="$t('theater.status.subtitles_loading')">
        <div class="flex h-11 items-center gap-3"><SkeletonLine size="base" width="w-1/2" /></div>
      </ContentSkeleton>

      <p v-else-if="!list.length" class="text-sm text-muted">{{ $t("theater.message.no_subtitles") }}</p>

      <ul v-else class="flex flex-col">
        <li v-for="subtitle in list" :key="subtitle.id" class="flex min-h-11 items-center gap-2">
          <span class="flex min-w-0 flex-1 flex-col">
            <span class="truncate text-base text-highlighted">{{ trackName(subtitle, locale) }}</span>
            <span class="truncate text-sm text-muted tabular-nums">{{ subtitle.format.toUpperCase() }} · {{ $t("theater.label.cues_count", { n: countText(subtitle.cueCount) }) }}</span>
          </span>

          <BurnState v-if="burnOf(subtitle.id)" :burn="burnOf(subtitle.id)!" />

          <UTooltip :text="$t('theater.action.burn')">
            <UButton
              color="neutral"
              variant="ghost"
              icon="i-material-symbols-local-fire-department-outline"
              class="pointer-coarse:size-11 pointer-coarse:justify-center"
              :aria-label="$t('theater.action.burn')"
              :loading="burnOf(subtitle.id)?.state === 'running'"
              :disabled="burnOf(subtitle.id)?.state === 'running'"
              @click="burnIn(subtitle)"
            />
          </UTooltip>
          <UTooltip :text="$t('theater.action.delete')">
            <UButton color="error" variant="ghost" icon="i-material-symbols-delete-outline" class="pointer-coarse:size-11 pointer-coarse:justify-center" :aria-label="$t('theater.action.delete')" @click="remove(subtitle)" />
          </UTooltip>
        </li>
      </ul>

      <p v-if="readFailed" role="alert" class="badge-ink text-sm text-error">{{ $t("theater.error.subtitles_failed") }}</p>
    </section>

    <USeparator />

    <section :aria-label="$t('theater.heading.add_subtitle')" class="flex flex-col gap-4">
      <h3 class="text-base font-medium text-highlighted">{{ $t("theater.heading.add_subtitle") }}</h3>

      <UFormField :label="$t('theater.label.subtitle_file')" name="subtitleFile" required>
        <UFileUpload v-model="file" accept=".srt,.ass,.smi,.vtt" :label="$t('theater.label.drop_subtitle')" layout="list" class="w-full" :disabled="adding" />
      </UFormField>

      <div class="grid gap-4 sm:grid-cols-2">
        <UFormField :label="$t('theater.label.language')" name="language">
          <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="language" class="w-full" :placeholder="primaryLanguage(locale)" :disabled="adding" />
        </UFormField>
        <UFormField :label="$t('theater.label.subtitle_label')" name="label">
          <UInput :ui="{ base: 'pointer-coarse:min-h-11' }" v-model="label" class="w-full" :disabled="adding" />
        </UFormField>
      </div>

      <UAlert v-if="failed" color="error" variant="subtle" :description="failed" />
    </section>

    <template #footer="{ close }">
      <UButton class="pointer-coarse:min-h-11" :label="$t('common.action.close')" color="neutral" variant="ghost" @click="close" />
      <UButton class="badge-ink pointer-coarse:min-h-11" :label="$t('theater.action.add')" :loading="adding" :disabled="!file || adding" @click="add" />
    </template>
  </SettingsDialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import ContentSkeleton from "bmo-ui/app/components/ContentSkeleton.vue";
import SettingsDialog from "bmo-ui/app/components/SettingsDialog.vue";
import SkeletonLine from "bmo-ui/app/components/SkeletonLine.vue";
import { countText } from "bmo-ui/app/domain/numbers";
import { askQuestion } from "bmo-ui/app/domain/question";

import BurnState from "./BurnState.vue";
import { fileForm } from "../../domain/forms";
import { primaryLanguage, trackName } from "../../domain/player";
import { refusalWord } from "../../domain/store";
import { useDialogFocus } from "../../composables/useDialogFocus";
import { useTheater } from "../../composables/useTheater";

import type { Episode, Subtitle } from "../../../shared/types";

const props = defineProps<{ episode: Episode | null }>();

const open = defineModel<boolean>("open", { default: false });

const { t, locale } = useI18n();
const store = useTheater();

const file = ref<File | null | undefined>(null);
const language = ref("");
const label = ref("");
const failed = ref("");
const loading = ref(false);
const readFailed = ref(false);
const adding = ref(false);

const list = computed<Subtitle[]>(() => (props.episode ? (store.subtitles.value[props.episode.id] ?? []) : []));
const dirty = computed(() => !!file.value || language.value.trim() !== "" || label.value.trim() !== "");

useDialogFocus(open);

watch(open, async (opened) => {
  if (!opened || !props.episode) return;

  file.value = null;
  language.value = "";
  label.value = "";
  failed.value = "";
  readFailed.value = false;
  loading.value = !store.subtitles.value[props.episode.id];

  try {
    await Promise.all([store.loadSubtitles(props.episode.id), store.loadBurns(props.episode.id)]);
  } catch {
    readFailed.value = true;
  } finally {
    loading.value = false;
  }
});

function burnOf(subtitleId: number) {
  return props.episode ? store.burnOf(props.episode.id, subtitleId) : null;
}

async function add(): Promise<void> {
  const chosen = file.value;

  if (!chosen || !props.episode) return;

  adding.value = true;
  failed.value = "";

  try {
    await store.addSubtitle(props.episode.id, fileForm(chosen, { language: language.value || primaryLanguage(locale.value), label: label.value }));
    file.value = null;
    language.value = "";
    label.value = "";
  } catch (error) {
    failed.value = t(refusalWord(error));
  } finally {
    adding.value = false;
  }
}

async function burnIn(subtitle: Subtitle): Promise<void> {
  if (!props.episode) return;

  const started = await store.burn(props.episode.id, subtitle.id);

  if (started) store.island.say({ tone: "info", title: t("theater.message.burn_started", { name: trackName(subtitle, locale.value) }) });
}

async function remove(subtitle: Subtitle): Promise<void> {
  const name = trackName(subtitle, locale.value);
  const sure = await askQuestion({ title: t("theater.question.delete_subtitle"), item: name, confirm: t("theater.action.delete") });

  if (sure) await store.deleteSubtitle(subtitle);
}
</script>
