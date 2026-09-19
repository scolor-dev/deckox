<script setup lang="ts">
withDefaults(
  defineProps<{
    as?: string;
    min?: "sm" | "md" | "lg";
    gap?: "2" | "3" | "4" | "5" | "6" | "8";
  }>(),
  {
    as: "div",
    min: "md",
    gap: "4",
  },
);
</script>

<template>
  <component
    :is="as"
    :class="['ds-grid', `ds-grid--min-${min}`, `ds-grid--gap-${gap}`]"
  >
    <slot />
  </component>
</template>

<style scoped>
.ds-grid { display: grid; }
.ds-grid--min-sm { grid-template-columns: repeat(auto-fill, minmax(min(160px, 100%), 1fr)); }
.ds-grid--min-md { grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr)); }
.ds-grid--min-lg { grid-template-columns: repeat(auto-fill, minmax(min(320px, 100%), 1fr)); }
.ds-grid--gap-2 { gap: var(--space-2); }
.ds-grid--gap-3 { gap: var(--space-3); }
.ds-grid--gap-4 { gap: var(--space-4); }
.ds-grid--gap-5 { gap: var(--space-5); }
.ds-grid--gap-6 { gap: var(--space-6); }
.ds-grid--gap-8 { gap: var(--space-8); }
</style>
