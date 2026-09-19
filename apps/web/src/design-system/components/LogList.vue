<script setup lang="ts">
// Scroll container for LogEntry rows (the service-log viewer's body).
</script>

<template>
  <ol class="ds-log-list">
    <slot />
  </ol>
</template>

<style scoped>
.ds-log-list {
  margin: 0;
  padding: 0;
  overflow: auto;
  list-style: none;
  background: var(--surface-muted);
}
</style>
