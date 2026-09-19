<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: boolean;
    label: string;
    help?: string | null;
    disabled?: boolean;
  }>(),
  {
    help: null,
    disabled: false,
  },
);

defineEmits<{
  "update:modelValue": [value: boolean];
}>();
</script>

<template>
  <div class="ds-checkbox-field">
    <label>
      <input
        type="checkbox"
        :checked="modelValue"
        :disabled="disabled"
        @change="$emit('update:modelValue', ($event.target as HTMLInputElement).checked)"
      >
      <span>{{ label }}</span>
    </label>
    <small v-if="help">{{ help }}</small>
  </div>
</template>

<style scoped>
.ds-checkbox-field label {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  cursor: pointer;
  color: var(--text-strong);
  font-size: var(--font-sm);
  font-weight: 600;
}
.ds-checkbox-field input { width: 16px; height: 16px; margin: 0; }
.ds-checkbox-field input:disabled { cursor: not-allowed; }
.ds-checkbox-field small { display: block; margin: var(--space-1) 0 0 var(--space-6); color: var(--text-muted); font-size: var(--font-xs); }
.ds-checkbox-field input:focus-visible { outline: 2px solid var(--brand-primary); outline-offset: 2px; }
</style>
