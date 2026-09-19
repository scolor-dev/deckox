<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    id: string;
    rows?: number;
    placeholder?: string | null;
    help?: string | null;
    disabled?: boolean;
    required?: boolean;
  }>(),
  {
    rows: 4,
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
    <textarea
      :id="id"
      :value="modelValue"
      :rows="rows"
      :placeholder="placeholder ?? undefined"
      :disabled="disabled"
      :required="required"
      @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    />
    <small v-if="help">{{ help }}</small>
  </div>
</template>

<style scoped>
.ds-field { display: grid; gap: var(--space-1); }
.ds-field label { color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }
.ds-field textarea {
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-base);
  background: var(--surface-elevated);
  font: inherit;
  line-height: 1.5;
  resize: vertical;
}
.ds-field textarea:focus { border-color: var(--brand-focus); box-shadow: var(--shadow-focus); }
.ds-field textarea:disabled { opacity: .6; cursor: not-allowed; resize: none; }
.ds-field small { color: var(--text-muted); font-size: var(--font-xs); }
</style>
