<script setup lang="ts">
// A standard design-system primitive not yet used anywhere in Deckox
// (existing on/off settings use AppCheckbox) — kept visually consistent
// with the rest of the control set (brand-primary "on", same focus ring)
// so it's ready when a screen wants the switch affordance specifically.
withDefaults(
  defineProps<{
    modelValue: boolean;
    label?: string | null;
    disabled?: boolean;
  }>(),
  {
    label: null,
    disabled: false,
  },
);

defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <label class="ds-switch">
    <input
      type="checkbox"
      role="switch"
      :aria-checked="modelValue"
      :checked="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
    >
    <span class="ds-switch-track"><span class="ds-switch-thumb" /></span>
    <span
      v-if="label"
      class="ds-switch-label"
    >{{ label }}</span>
  </label>
</template>

<style scoped>
.ds-switch { display: inline-flex; align-items: center; gap: var(--space-2); cursor: pointer; }
.ds-switch input { position: absolute; width: 1px; height: 1px; overflow: hidden; opacity: 0; }
.ds-switch-track {
  position: relative;
  width: 34px;
  height: 18px;
  flex: 0 0 auto;
  border-radius: 9px;
  background: var(--border-strong);
  transition: background var(--duration-base) var(--ease-standard);
}
.ds-switch-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--surface-elevated);
  box-shadow: var(--shadow-thumb);
  transition: transform var(--duration-base) var(--ease-standard);
}
.ds-switch input:checked + .ds-switch-track { background: var(--brand-primary); }
.ds-switch input:checked + .ds-switch-track .ds-switch-thumb { transform: translateX(16px); }
.ds-switch input:focus-visible + .ds-switch-track { outline: 2px solid var(--brand-primary); outline-offset: 2px; }
.ds-switch input:disabled ~ * { opacity: .5; }
.ds-switch:has(input:disabled) { cursor: not-allowed; }
.ds-switch-label { color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }

@media (prefers-reduced-motion: reduce) {
  .ds-switch-track, .ds-switch-thumb { transition: none; }
}
</style>
