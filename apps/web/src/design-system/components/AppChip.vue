<script setup lang="ts">
// A generic display pill for a discrete item (e.g. a search filter the
// user typed, a selected value) — distinct from TagBadge/TagToggle, which
// are specifically for the app's fixed standard/deckox/product categories.
import AppIconButton from "./AppIconButton.vue";

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
    <AppIconButton
      v-if="removable"
      size="sm"
      danger
      :label="removeLabel"
      @click="$emit('remove')"
    >
      ×
    </AppIconButton>
  </span>
</template>

<style scoped>
.ds-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font-size: var(--font-xs);
}
</style>
