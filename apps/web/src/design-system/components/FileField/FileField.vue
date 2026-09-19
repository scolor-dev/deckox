<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    id: string;
    accept?: string | null;
    help?: string | null;
    disabled?: boolean;
  }>(),
  {
    accept: null,
    help: null,
    disabled: false,
  },
);

defineEmits<{
  select: [file: File | null];
}>();
</script>

<template>
  <div class="ds-field">
    <label :for="id">{{ label }}</label>
    <input
      :id="id"
      type="file"
      :accept="accept ?? undefined"
      :disabled="disabled"
      @change="$emit('select', ($event.target as HTMLInputElement).files?.[0] ?? null)"
    >
    <small v-if="help">{{ help }}</small>
  </div>
</template>

<style scoped>
.ds-field { display: grid; gap: var(--space-1); }
.ds-field label { color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }
.ds-field input[type="file"] {
  width: 100%;
  padding: var(--space-2);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  outline: none;
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font: inherit;
  font-size: var(--font-sm);
}
.ds-field input[type="file"]:focus-visible { border-color: var(--brand-focus); box-shadow: var(--shadow-focus); }
.ds-field input[type="file"]::file-selector-button {
  margin-right: var(--space-3);
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  color: var(--text-strong);
  background: var(--surface-subtle);
  cursor: pointer;
  font: inherit;
  font-size: var(--font-sm);
}
.ds-field input[type="file"]:not(:disabled)::file-selector-button:hover {
  border-color: var(--border-hover);
  background: var(--surface-hover);
}
.ds-field input[type="file"]:disabled { opacity: .6; cursor: not-allowed; }
.ds-field small { color: var(--text-muted); font-size: var(--font-xs); }
</style>
