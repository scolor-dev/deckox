<script setup lang="ts">
// The title row every screen opens with: heading and subtitle on the left,
// screen-level actions (Refresh, Save report, ...) on the right.
defineProps<{
  title: string;
  subtitle?: string | null;
}>();
</script>

<template>
  <header class="ds-page-header">
    <div>
      <h1>{{ title }}</h1>
      <p v-if="subtitle">
        {{ subtitle }}
      </p>
    </div>
    <div
      v-if="$slots.actions"
      class="ds-page-actions"
    >
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.ds-page-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-5);
  margin-bottom: var(--space-6);
}
.ds-page-header h1 {
  margin: 0;
  color: var(--text-heading);
  font-size: var(--font-3xl);
  line-height: 1.35;
  letter-spacing: .01em;
  overflow-wrap: anywhere;
}
.ds-page-header p { margin: var(--space-1) 0 0; color: var(--text-muted); font-size: var(--font-sm); overflow-wrap: anywhere; }
.ds-page-actions { display: flex; flex-wrap: wrap; align-items: center; justify-content: flex-end; gap: var(--space-2); }

@media (max-width: 700px) {
  .ds-page-header { align-items: flex-start; margin-bottom: var(--space-5); }
  .ds-page-header h1 { font-size: var(--font-2xl); }
}
</style>
