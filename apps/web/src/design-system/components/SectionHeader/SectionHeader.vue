<script setup lang="ts">
import AppHeading from "../AppHeading/AppHeading.vue";

withDefaults(
  defineProps<{
    title: string;
    level?: "2" | "3" | "4";
    size?: "sm" | "md" | "lg" | "xl";
  }>(),
  {
    level: "2",
    size: "lg",
  },
);
</script>

<template>
  <header class="ds-section-header">
    <AppHeading
      :level="level"
      :size="size"
    >
      {{ title }}
    </AppHeading>
    <div
      v-if="$slots.actions"
      class="ds-section-actions"
    >
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.ds-section-header { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--space-2); }
.ds-section-actions { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2); }
</style>
