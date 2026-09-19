<script setup lang="ts">
defineProps<{
  tabs: { key: string; label: string }[];
  modelValue: string;
  label: string;
}>();

defineEmits<{
  "update:modelValue": [key: string];
}>();
</script>

<template>
  <div
    class="ds-tab-bar"
    role="tablist"
    :aria-label="label"
  >
    <button
      v-for="tab in tabs"
      :key="tab.key"
      type="button"
      role="tab"
      :class="['ds-tab', { 'ds-tab--active': tab.key === modelValue }]"
      :aria-selected="tab.key === modelValue"
      @click="$emit('update:modelValue', tab.key)"
    >
      {{ tab.label }}
    </button>
  </div>
</template>

<style scoped>
.ds-tab-bar {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-0-5);
  margin-bottom: var(--space-2);
  border-bottom: 1px solid var(--border-default);
}
.ds-tab {
  padding: var(--space-2) var(--space-4);
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font: inherit;
  font-size: var(--font-md);
  font-weight: 600;
}
.ds-tab:hover:not(.ds-tab--active) { color: var(--text-primary); background: var(--surface-hover); }
.ds-tab--active { color: var(--link); border-bottom-color: var(--brand-primary); }
</style>
