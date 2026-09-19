<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    id: string;
    options: { value: string; label: string }[];
    help?: string | null;
    disabled?: boolean;
  }>(),
  {
    help: null,
    disabled: false,
  },
);

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="ds-field">
    <label :for="id">{{ label }}</label>
    <select
      :id="id"
      :value="modelValue"
      :disabled="disabled"
      @change="$emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
    >
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
      >
        {{ option.label }}
      </option>
    </select>
    <small v-if="help">{{ help }}</small>
  </div>
</template>

<style scoped>
.ds-field { display: grid; gap: var(--space-1); }
.ds-field label { color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }
.ds-field select {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-base);
  background: var(--surface-elevated);
  font: inherit;
}
.ds-field select:focus { border-color: var(--brand-focus); box-shadow: var(--shadow-focus); }
.ds-field select:disabled { color: var(--text-faint); background: var(--surface-muted); cursor: not-allowed; }
.ds-field small { color: var(--text-muted); font-size: var(--font-xs); }
</style>
