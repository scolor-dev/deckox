<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    id: string;
    type?: "text" | "search" | "password" | "email" | "url";
    placeholder?: string | null;
    help?: string | null;
    disabled?: boolean;
    required?: boolean;
  }>(),
  {
    type: "text",
    placeholder: null,
    help: null,
    disabled: false,
    required: false,
  },
);

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="ds-field">
    <label :for="id">{{ label }}</label>
    <input
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
.ds-field { display: grid; gap: 4px; }
.ds-field label { color: var(--text-strong); font-size: 12px; font-weight: 600; }
.ds-field input {
  width: 100%;
  padding: 9px 10px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-base);
  background: var(--surface-elevated);
  font: inherit;
}
.ds-field input:focus { border-color: var(--brand-focus); box-shadow: 0 0 0 2px var(--brand-focus-ring); }
.ds-field input:disabled { opacity: .6; cursor: not-allowed; }
.ds-field small { color: var(--text-muted); font-size: 11px; }
</style>
