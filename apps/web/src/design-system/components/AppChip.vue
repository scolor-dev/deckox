<script setup lang="ts">
// A generic display pill for a discrete item (e.g. a search filter the
// user typed, a selected value) — distinct from TagBadge/TagToggle, which
// are specifically for the app's fixed standard/deckox/product categories.
withDefaults(
  defineProps<{
    removable?: boolean;
    removeLabel?: string;
  }>(),
  {
    removable: false,
    removeLabel: "Remove",
  },
);

defineEmits<{
  remove: [];
}>();
</script>

<template>
  <span class="ds-chip">
    <slot />
    <button
      v-if="removable"
      type="button"
      class="ds-chip-remove"
      :aria-label="removeLabel"
      @click="$emit('remove')"
    >
      ×
    </button>
  </span>
</template>

<style scoped>
.ds-chip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 10px;
  border: 1px solid var(--border-strong);
  border-radius: 12px;
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font-size: 11px;
}
.ds-chip-remove {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  color: var(--text-faint);
  background: transparent;
  cursor: pointer;
  font-size: 12px;
  line-height: 1;
}
.ds-chip-remove:hover { color: var(--danger-accent); background: var(--danger-bg); }
</style>
