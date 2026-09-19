<script setup lang="ts">
defineOptions({ inheritAttrs: false });

withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    id: string;
    type?: "text" | "search" | "password" | "email" | "url" | "time";
    placeholder?: string | null;
    help?: string | null;
    disabled?: boolean;
    required?: boolean;
    labelHidden?: boolean;
  }>(),
  {
    type: "text",
    placeholder: null,
    help: null,
    disabled: false,
    required: false,
    labelHidden: false,
  },
);

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="ds-field">
    <label
      :for="id"
      :class="{ 'ds-field-label--hidden': labelHidden }"
    >{{ label }}</label>
    <input
      v-bind="$attrs"
      :id="id"
      :type="type"
      :value="modelValue"
      :placeholder="placeholder ?? undefined"
      :disabled="disabled"
      :required="required"
      @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    >
    <small v-if="help">{{ help }}</small>
  </div>
</template>

<style scoped>
.ds-field { display: grid; gap: var(--space-1); }
.ds-field label { color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }
.ds-field input {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-base);
  background: var(--surface-elevated);
  font: inherit;
}
.ds-field input:focus { border-color: var(--brand-focus); box-shadow: var(--shadow-focus); }
.ds-field input:disabled { opacity: .6; cursor: not-allowed; }
.ds-field small { color: var(--text-muted); font-size: var(--font-xs); }
.ds-field .ds-field-label--hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
