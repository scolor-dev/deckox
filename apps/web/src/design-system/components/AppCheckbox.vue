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
  gap: 8px;
  cursor: pointer;
  color: var(--text-strong);
  font-size: 12px;
  font-weight: 600;
}
.ds-checkbox-field input { width: 16px; height: 16px; margin: 0; }
.ds-checkbox-field input:disabled { cursor: not-allowed; }
.ds-checkbox-field small { display: block; margin: 4px 0 0 24px; color: var(--text-muted); font-size: 11px; }
</style>
