<script setup lang="ts">
defineProps<{
  label: string;
}>();
</script>

<template>
  <fieldset class="ds-tag-toggles">
    <legend class="ds-tag-toggles-legend">
      {{ label }}
    </legend>
    <slot />
  </fieldset>
</template>

<style scoped>
.ds-tag-toggles { display: flex; flex-wrap: wrap; gap: var(--space-2); padding: 0; border: 0; margin: 0; }
.ds-tag-toggles-legend {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
