<script setup lang="ts">
import { computed } from "vue";
import { NoticeBanner } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();
const tone = computed(() => {
  const value = props.config.tone;
  return value === "success" || value === "error" ? value : "warning";
});
const body = computed(() => (typeof props.config.body === "string" ? props.config.body : ""));
</script>

<template>
  <NoticeBanner :tone="tone">
    {{ body }}
  </NoticeBanner>
</template>
