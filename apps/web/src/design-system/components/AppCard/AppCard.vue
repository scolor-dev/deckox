<script setup lang="ts">
import AppHeading from "../AppHeading/AppHeading.vue";

withDefaults(
  defineProps<{
    padding?: "sm" | "md";
    title?: string;
  }>(),
  {
    padding: "md",
    title: "",
  },
);
</script>

<template>
  <div :class="['ds-card', `ds-card--${padding}`]">
    <header
      v-if="title || $slots.actions"
      class="ds-card-header"
    >
      <AppHeading
        v-if="title"
        size="md"
        truncate
      >
        {{ title }}
      </AppHeading>
      <div
        v-if="$slots.actions"
        class="ds-card-actions"
      >
        <slot name="actions" />
      </div>
    </header>
    <slot />
  </div>
</template>

<style scoped>
.ds-card {
  min-width: 0;
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
}
.ds-card--md { padding: var(--space-4) var(--space-5); }
.ds-card--sm { padding: var(--space-3) var(--space-4); }
.ds-card-header { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--space-2); margin-bottom: var(--space-3); }
.ds-card-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); margin-left: auto; }
</style>
