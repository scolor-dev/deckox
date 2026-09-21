<script setup lang="ts">
import { computed } from "vue";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();
const text = computed(() => (typeof props.config.text === "string" ? props.config.text : ""));
</script>

<template>
  <h2 class="heading-block">
    {{ text }}
  </h2>
</template>

<style scoped>
.heading-block { margin: 0; font-size: var(--font-xl); }
</style>
