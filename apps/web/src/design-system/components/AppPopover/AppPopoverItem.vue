<script setup lang="ts">
withDefaults(
  defineProps<{
    danger?: boolean;
  }>(),
  {
    danger: false,
  },
);

defineEmits<{
  click: [];
}>();
</script>

<template>
  <button
    type="button"
    role="menuitem"
    tabindex="-1"
    :class="['ds-popover-item', { 'ds-popover-item--danger': danger }]"
    @click="$emit('click')"
  >
    <slot />
  </button>
</template>

<style scoped>
.ds-popover-item {
  display: block;
  width: 100%;
  padding: var(--space-2) var(--space-3);
  border: 0;
  border-radius: var(--radius-sm);
  color: var(--text-strong);
  background: transparent;
  cursor: pointer;
  font: inherit;
  font-size: var(--font-sm);
  text-align: left;
}
.ds-popover-item:hover { background: var(--surface-hover); }
.ds-popover-item--danger { color: var(--danger-strong-border); }
.ds-popover-item--danger:hover { background: var(--danger-bg); }
.ds-popover-item:focus-visible { outline: 2px solid var(--brand-primary); outline-offset: -2px; }
</style>
