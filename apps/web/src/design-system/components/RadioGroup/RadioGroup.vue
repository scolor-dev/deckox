<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    label: string;
    name: string;
    options: { value: string; label: string }[];
    help?: string | null;
  }>(),
  {
    help: null,
  },
);

defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <fieldset class="ds-radio-group">
    <legend>{{ label }}</legend>
    <label
      v-for="option in options"
      :key="option.value"
      class="ds-radio"
    >
      <input
        type="radio"
        :name="name"
        :value="option.value"
        :checked="modelValue === option.value"
        @change="$emit('update:modelValue', option.value)"
      >
      <span>{{ option.label }}</span>
    </label>
    <small v-if="help">{{ help }}</small>
  </fieldset>
</template>

<style scoped>
.ds-radio-group { display: grid; gap: var(--space-2); padding: 0; border: 0; margin: 0; }
.ds-radio-group legend { padding: 0; margin: 0 0 var(--space-0-5); color: var(--text-strong); font-size: var(--font-sm); font-weight: 600; }
.ds-radio { display: flex; align-items: center; gap: var(--space-2); cursor: pointer; color: var(--text-secondary); font-size: var(--font-md); }
.ds-radio input { width: 15px; height: 15px; margin: 0; accent-color: var(--brand-primary); }
.ds-radio-group small { color: var(--text-muted); font-size: var(--font-xs); }
</style>
