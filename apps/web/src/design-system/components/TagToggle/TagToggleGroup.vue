<script setup lang="ts">
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
