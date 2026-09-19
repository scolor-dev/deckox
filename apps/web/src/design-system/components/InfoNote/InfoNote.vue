<script setup lang="ts">
</script>

<template>
  <aside class="ds-info-note">
    <slot />
  </aside>
</template>

<style scoped>
.ds-info-note {
  padding: var(--space-3) var(--space-3);
  border: 1px solid var(--info-border);
  border-radius: var(--radius-sm);
  color: var(--info-text);
  background: var(--info-bg);
  font-size: var(--font-xs);
}
.ds-info-note :deep(code) { color: var(--info-code-text); }
</style>
