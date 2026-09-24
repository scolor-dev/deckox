<script setup lang="ts">
withDefaults(
  defineProps<{
    level?: "2" | "3" | "4";
    size?: "sm" | "md" | "lg" | "xl";
    truncate?: boolean;
  }>(),
  {
    level: "2",
    size: "xl",
    truncate: false,
  },
);
</script>

<template>
  <component
    :is="`h${level}`"
    :class="['ds-heading', `ds-heading--${size}`, { 'ds-heading--truncate': truncate }]"
  >
    <slot />
  </component>
</template>

<style scoped>
.ds-heading { margin: 0; color: var(--text-primary); line-height: 1.4; overflow-wrap: anywhere; }
.ds-heading--sm { font-size: var(--font-sm); }
.ds-heading--md { font-size: var(--font-md); }
.ds-heading--lg { font-size: var(--font-lg); }
.ds-heading--xl { font-size: var(--font-xl); }
.ds-heading--truncate { flex: 1 1 6rem; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
