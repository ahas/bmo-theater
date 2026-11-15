<template>
  <!-- The app root a console stands its screens in: the component library's app holding the layer's tooltip setting,
       a dashboard group around the page, and the layer's page layers. -->
  <UApp :tooltip="TOOLTIP_PROVIDER" :toaster="null">
    <UDashboardGroup>
      <NuxtPage />
    </UDashboardGroup>
    <MessageIsland />
    <CutLineTip />
  </UApp>
</template>

<script setup lang="ts">
import { TOOLTIP_PROVIDER } from "bmo-ui/app/domain/tooltip";
</script>
