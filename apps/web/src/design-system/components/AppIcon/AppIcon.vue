<script setup lang="ts">
import { ICON_PATHS, type IconName } from "./icons";

withDefaults(
  defineProps<{
    name: IconName;
    size?: "sm" | "md" | "lg";
    label?: string | null;
  }>(),
  {
    size: "md",
    label: null,
  },
);
</script>

<template>
  <svg
    :class="['ds-icon', `ds-icon--${size}`]"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="2"
    stroke-linecap="round"
    stroke-linejoin="round"
    :role="label ? 'img' : undefined"
    :aria-label="label ?? undefined"
    :aria-hidden="label ? undefined : 'true'"
    focusable="false"
  >
    <path
      v-for="path in ICON_PATHS[name]"
      :key="path"
      :d="path"
    />
  </svg>
</template>

<style scoped>
.ds-icon { display: inline-block; flex-shrink: 0; vertical-align: middle; }
.ds-icon--sm { width: var(--space-3); height: var(--space-3); }
.ds-icon--md { width: var(--space-4); height: var(--space-4); }
.ds-icon--lg { width: var(--space-5); height: var(--space-5); }
</style>
