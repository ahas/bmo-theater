<template>
  <!-- A poster frame at a video's own shape, sixteen by nine, or the kind's mark on a muted ground where the
       service holds none. -->
  <span class="relative block aspect-video w-full overflow-hidden bg-muted" :class="props.rounded ? 'rounded-md' : ''">
    <LoadingImage v-if="props.src" :src="props.src" alt="" fill image-class="size-full object-cover" />
    <span v-else class="flex size-full items-center justify-center text-muted" aria-hidden="true">
      <UIcon :name="props.icon" class="size-8" />
    </span>
  </span>
</template>

<script setup lang="ts">
import LoadingImage from "bmo-ui/app/components/LoadingImage.vue";

interface Props {
  /** The poster's address, or none where there is no poster. */
  src: string | null;
  icon?: string;
  rounded?: boolean;
}

const props = withDefaults(defineProps<Props>(), { icon: "i-material-symbols-movie-outline", rounded: true });
</script>
