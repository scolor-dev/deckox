<script setup lang="ts">
defineProps<{
  category: "standard" | "deckox" | "product" | "other";
  checked: boolean;
}>();

defineEmits<{
  "update:checked": [checked: boolean];
}>();
</script>

<template>
  <label :class="['ds-tag-toggle', `ds-tag-toggle--${category}`, { 'ds-tag-toggle--off': !checked }]">
    <input
      type="checkbox"
      :checked="checked"
      @change="$emit('update:checked', ($event.target as HTMLInputElement).checked)"
    >
    <slot />
  </label>
</template>

<style scoped>
.ds-tag-toggle {
  display: inline-flex;
  gap: var(--space-1);
  align-items: center;
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--tag-toggle-border);
  border-radius: var(--radius-pill);
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font-size: var(--font-xs);
  cursor: pointer;
  user-select: none;
}
.ds-tag-toggle input { margin: 0; }
.ds-tag-toggle:hover { background: var(--surface-hover); }
.ds-tag-toggle--off { color: var(--tag-off-text); background: var(--tag-off-bg); }
.ds-tag-toggle--off:hover { border-color: var(--border-hover); background: var(--surface-hover); }
.ds-tag-toggle--standard:not(.ds-tag-toggle--off) { border-color: var(--tag-standard-border); }
.ds-tag-toggle--deckox:not(.ds-tag-toggle--off) { border-color: var(--tag-deckox-border); color: var(--tag-deckox-text); }
.ds-tag-toggle--product:not(.ds-tag-toggle--off) { border-color: var(--tag-product-border); color: var(--tag-product-text); }
.ds-tag-toggle:has(input:focus-visible) { outline: 2px solid var(--brand-primary); outline-offset: 2px; }
</style>
