<script setup lang="ts">
withDefaults(
  defineProps<{
    message: string;
    compact?: boolean;
  }>(),
  {
    compact: false,
  },
);
</script>

<template>
  <div :class="['ds-empty-state', { 'ds-empty-state--compact': compact }]">
    <p>{{ message }}</p>
    <slot />
  </div>
</template>

<style scoped>
.ds-empty-state { padding: var(--space-10) var(--space-5); color: var(--text-faint); text-align: center; }
.ds-empty-state--compact { padding: var(--space-3); }
.ds-empty-state p { margin: 0; }
.ds-empty-state:has(> :not(p)) p { margin-bottom: var(--space-3); }
</style>
