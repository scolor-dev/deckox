<script setup lang="ts">
const props = defineProps<{
  tabs: { key: string; label: string }[];
  modelValue: string;
  label: string;
}>();

const emit = defineEmits<{
  "update:modelValue": [key: string];
}>();

const MOVES: Partial<Record<string, (index: number, count: number) => number>> = {
  ArrowRight: (index, count) => (index + 1) % count,
  ArrowLeft: (index, count) => (index - 1 + count) % count,
  Home: () => 0,
  End: (_index, count) => count - 1,
};

function handleKeydown(event: KeyboardEvent, index: number) {
  const move = MOVES[event.key];
  if (!move) return;
  event.preventDefault();
  const target = props.tabs[move(index, props.tabs.length)];
  emit("update:modelValue", target.key);
  const buttons = (event.currentTarget as HTMLElement).parentElement?.querySelectorAll<HTMLElement>("[role='tab']");
  buttons?.[props.tabs.indexOf(target)]?.focus();
}
</script>

<template>
  <div
    class="ds-tab-bar"
    role="tablist"
    :aria-label="label"
  >
    <button
      v-for="(tab, index) in tabs"
      :key="tab.key"
      type="button"
      role="tab"
      :class="['ds-tab', { 'ds-tab--active': tab.key === modelValue }]"
      :aria-selected="tab.key === modelValue"
      :tabindex="tab.key === modelValue ? 0 : -1"
      @click="emit('update:modelValue', tab.key)"
      @keydown="handleKeydown($event, index)"
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
.ds-tab:focus-visible { outline: 2px solid var(--brand-primary); outline-offset: -2px; }
</style>
