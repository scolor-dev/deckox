<script setup lang="ts">
import { computed } from "vue";
import { AppCard } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();
const title = computed(() => (typeof props.config.title === "string" ? props.config.title : ""));
const body = computed(() => (typeof props.config.body === "string" ? props.config.body : ""));
</script>

<template>
  <AppCard>
    <h2
      v-if="title"
      class="text-block-title"
    >
      {{ title }}
    </h2>
    <p class="text-block-body">
      {{ body }}
    </p>
  </AppCard>
</template>

<style scoped>
.text-block-title { margin: 0 0 var(--space-2); font-size: var(--font-lg); }
.text-block-body { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; }
</style>
