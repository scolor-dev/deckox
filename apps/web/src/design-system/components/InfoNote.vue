<script setup lang="ts">
// Low-emphasis supplementary info below a form or table (e.g. "changes are
// available only for services listed in ..."). Not a notice/alert — no
// tone variants, no role, since it isn't reporting a state change.
</script>

<template>
  <aside class="ds-info-note">
    <slot />
  </aside>
</template>

<style scoped>
.ds-info-note {
  margin-top: 12px;
  padding: 10px 12px;
  border: 1px solid var(--info-border);
  border-radius: var(--radius-sm);
  color: var(--info-text);
  background: var(--info-bg);
  font-size: 11px;
}
.ds-info-note :deep(code) { color: var(--info-code-text); }
</style>
