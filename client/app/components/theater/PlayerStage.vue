<template>
  <!-- Where an episode plays (`docs/design.md`, 「The console」): the layer's video player over the episode's stream,
       filling the black stage its screen sizes, a skeleton while the episode is read, or why it cannot play. -->
  <div data-player-stage class="shrink-0 bg-black">
    <p v-if="props.failed" role="alert" class="flex size-full items-center justify-center p-4 text-center text-base text-white">{{ props.failed }}</p>
    <VideoPlayer
      v-else-if="props.player"
      v-model:track="track"
      :video="props.player.episode"
      :src="props.player.src"
      :poster="props.player.poster"
      :title="props.player.title"
      :tracks="props.player.tracks"
      :start="props.player.start"
      :autoplay="props.player.autoplay"
      :next="props.player.next"
      :auto-next="props.player.autoNext ?? false"
      @position="(video, seconds) => emit('position', Number(video), seconds)"
      @leaving="(video, seconds) => emit('leaving', Number(video), seconds)"
      @ended="(video, seconds) => emit('ended', Number(video), seconds)"
      @next="emit('next')"
    />
    <SkeletonStatus v-else as="div" class="flex size-full" :label="$t('common.status.video_loading')"><SkeletonBar class="size-full" /></SkeletonStatus>
  </div>
</template>

<script setup lang="ts">
import SkeletonBar from "bmo-ui/app/components/SkeletonBar.vue";
import SkeletonStatus from "bmo-ui/app/components/SkeletonStatus.vue";
import VideoPlayer from "bmo-ui/app/components/VideoPlayer.vue";

import type { PlayerTrack } from "bmo-ui/app/domain/video";

/** The episode the stage plays, once it and its subtitles are read. */
export interface StagePlayer {
  episode: number;
  src: string;
  poster: string | null;
  /** The episode's name, which the player is named by. */
  title: string;
  tracks: PlayerTrack[];
  /** Where it starts, in seconds. */
  start?: number;
  autoplay: boolean;
  /** The next episode's words, where there is one to go on to. */
  next: string | null;
  /** Whether the next episode follows on its own, offered near the end. */
  autoNext?: boolean;
}

const props = defineProps<{
  /** Why the episode cannot play, or nothing. */
  failed: string;
  /** The episode to play, or none while it is being read. */
  player: StagePlayer | null;
}>();

// What the player tells of an episode, named by the episode it stood for when it told it.
const emit = defineEmits<{
  position: [episode: number, seconds: number];
  leaving: [episode: number, seconds: number];
  ended: [episode: number, seconds: number];
  next: [];
}>();

/** The subtitle shown, or none. */
const track = defineModel<number | null>("track", { default: null });
</script>
