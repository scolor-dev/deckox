<script setup lang="ts">
withDefaults(
  defineProps<{
    spacing?: "sm" | "md" | "lg";
  }>(),
  {
    spacing: "md",
  },
);
</script>

<template>
  <hr :class="['ds-divider', `ds-divider--${spacing}`]">
</template>

<style scoped>
.ds-divider { border: 0; border-top: 1px solid var(--border-default); }
.ds-divider--sm { margin: 10px 0; }
.ds-divider--md { margin: 18px 0; }
.ds-divider--lg { margin: 32px 0; }
</style>
