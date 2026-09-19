<script setup lang="ts">
// One checkbox pill in a tag-visibility filter fieldset. Use inside
// TagToggleGroup.vue, which supplies the `role="group"`/fieldset wrapper.
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
  gap: 5px;
  align-items: center;
  padding: 4px 9px;
  border: 1px solid var(--tag-toggle-border);
  border-radius: var(--radius-pill);
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font-size: 11px;
  cursor: pointer;
  user-select: none;
}
.ds-tag-toggle input { margin: 0; }
.ds-tag-toggle:hover { background: var(--surface-hover); }
.ds-tag-toggle--off { color: var(--tag-off-text); background: var(--tag-off-bg); }
/* The "off" pill's own bg/border are both pale neutrals, close enough to
 * --surface-hover that a background-only hover barely reads (the same
 * problem AppButton's danger action variant had) — a checked pill's
 * category-colored border already gives it enough presence that a plain
 * bg tint on hover is fine, but "off" has no such cue, so it also gets a
 * border shift. */
.ds-tag-toggle--off:hover { border-color: var(--border-hover); background: var(--surface-hover); }
.ds-tag-toggle--standard:not(.ds-tag-toggle--off) { border-color: var(--tag-standard-border); }
.ds-tag-toggle--deckox:not(.ds-tag-toggle--off) { border-color: var(--tag-deckox-border); color: var(--tag-deckox-text); }
.ds-tag-toggle--product:not(.ds-tag-toggle--off) { border-color: var(--tag-product-border); color: var(--tag-product-text); }
</style>
