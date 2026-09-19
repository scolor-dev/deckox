<script setup lang="ts">
// The "nothing here yet" message — TablePanel already renders this inline
// for its own empty/loading state; use this directly for an empty area
// that isn't a table (e.g. a card with no content yet).
defineProps<{
  message: string;
}>();
</script>

<template>
  <div class="ds-empty-state">
    <p>{{ message }}</p>
    <slot />
  </div>
</template>

<style scoped>
.ds-empty-state { padding: var(--space-10) var(--space-5); color: var(--text-faint); text-align: center; }
.ds-empty-state p { margin: 0; }
.ds-empty-state:has(> :not(p)) p { margin-bottom: var(--space-3); }
</style>
