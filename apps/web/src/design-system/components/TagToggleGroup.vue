<script setup lang="ts">
// Accessible fieldset wrapper around a row of TagToggle pills. `label` is
// the fieldset's legend — visually hidden (the toolbar it sits in already
// makes its purpose clear), but announced by screen readers.
defineProps<{
  label: string;
}>();
</script>

<template>
  <fieldset class="ds-tag-toggles">
    <legend class="sr-only">
      {{ label }}
    </legend>
    <slot />
  </fieldset>
</template>

<style scoped>
.ds-tag-toggles { display: flex; flex-wrap: wrap; gap: var(--space-2); padding: 0; border: 0; margin: 0; }
</style>
