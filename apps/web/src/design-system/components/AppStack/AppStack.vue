<script setup lang="ts">
withDefaults(
  defineProps<{
    as?: string;
    direction?: "column" | "row";
    gap?: "1" | "2" | "3" | "4" | "5" | "6" | "8";
    align?: "start" | "center" | "end" | "stretch";
    justify?: "start" | "center" | "end" | "between";
    wrap?: boolean;
  }>(),
  {
    as: "div",
    direction: "column",
    gap: "3",
    align: "stretch",
    justify: "start",
    wrap: false,
  },
);
</script>

<template>
  <component
    :is="as"
    :class="['ds-stack', `ds-stack--${direction}`, `ds-stack--gap-${gap}`, `ds-stack--align-${align}`, `ds-stack--justify-${justify}`, { 'ds-stack--wrap': wrap }]"
  >
    <slot />
  </component>
</template>

<style scoped>
.ds-stack { display: flex; min-width: 0; }
.ds-stack--column { flex-direction: column; }
.ds-stack--row { flex-direction: row; }
.ds-stack--wrap { flex-wrap: wrap; }
.ds-stack--gap-1 { gap: var(--space-1); }
.ds-stack--gap-2 { gap: var(--space-2); }
.ds-stack--gap-3 { gap: var(--space-3); }
.ds-stack--gap-4 { gap: var(--space-4); }
.ds-stack--gap-5 { gap: var(--space-5); }
.ds-stack--gap-6 { gap: var(--space-6); }
.ds-stack--gap-8 { gap: var(--space-8); }
.ds-stack--align-start { align-items: flex-start; }
.ds-stack--align-center { align-items: center; }
.ds-stack--align-end { align-items: flex-end; }
.ds-stack--align-stretch { align-items: stretch; }
.ds-stack--justify-start { justify-content: flex-start; }
.ds-stack--justify-center { justify-content: center; }
.ds-stack--justify-end { justify-content: flex-end; }
.ds-stack--justify-between { justify-content: space-between; }
</style>
