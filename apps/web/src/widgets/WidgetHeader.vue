<script setup lang="ts">
defineProps<{ title: string }>();
</script>

<template>
  <header class="widget-header">
    <h2>{{ title }}</h2>
    <div
      v-if="$slots.default"
      class="widget-header-actions"
    >
      <slot />
    </div>
  </header>
</template>

<style scoped>
.widget-header { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: var(--space-2); }
.widget-header h2 { margin: 0; font-size: var(--font-lg); }
.widget-header-actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }
</style>
