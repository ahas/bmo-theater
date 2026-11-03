<template>
  <!-- How a burn stands, as a chip: running with its spinning mark, finished leading to the clip it filed, or
       failed with why in its tooltip. -->
  <HintTip v-if="props.burn.state === 'failed'" :text="props.burn.error ?? $t('theater.status.burn_failed')">
    <UBadge color="error" variant="subtle" icon="i-material-symbols-error-outline" :label="$t('theater.status.burn_failed')" class="badge-ink" />
  </HintTip>
  <UBadge v-else-if="props.burn.state === 'running'" color="neutral" variant="subtle" :label="$t('theater.status.burning')">
    <template #leading>
      <UIcon name="i-material-symbols-progress-activity" class="size-4 animate-spin motion-reduce:animate-none" />
    </template>
  </UBadge>
  <UButton
    v-else-if="props.burn.clipId !== null"
    :to="`/theater/watch/${props.burn.clipId}`"
    color="success"
    variant="subtle"
    size="sm"
    icon="i-material-symbols-play-arrow"
    class="badge-ink"
    :label="$t('theater.status.burned')"
  />
</template>

<script setup lang="ts">
import HintTip from "bmo-ui/app/components/HintTip.vue";

import type { Burn } from "../../../shared/types";

const props = defineProps<{ burn: Burn }>();
</script>
