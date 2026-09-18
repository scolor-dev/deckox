<script setup lang="ts">
// One clickable row inside a Popover used as a dropdown menu.
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
  padding: 7px 10px;
  border: 0;
  border-radius: 4px;
  color: var(--text-strong);
  background: transparent;
  cursor: pointer;
  font: inherit;
  font-size: 12px;
  text-align: left;
}
.ds-popover-item:hover { background: var(--surface-hover); }
.ds-popover-item--danger { color: var(--danger-strong-border); }
.ds-popover-item--danger:hover { background: var(--danger-bg); }
</style>
