<script setup lang="ts">
// The small icon-only button shape shared by AppModal's close (×) and
// AppChip's remove (×) — previously two near-identical bespoke buttons.
withDefaults(
  defineProps<{
    label: string;
    size?: "sm" | "md";
    danger?: boolean;
  }>(),
  {
    size: "md",
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
    :class="['ds-icon-button', `ds-icon-button--${size}`, { 'ds-icon-button--danger': danger }]"
    :aria-label="label"
    @click="$emit('click')"
  >
    <slot />
  </button>
</template>

<style scoped>
.ds-icon-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: 0 0 auto;
  padding: 0;
  border: 0;
  color: var(--text-muted);
  background: transparent;
  cursor: pointer;
  line-height: 1;
}
.ds-icon-button--md { width: 24px; height: 24px; border-radius: var(--radius-sm); font-size: 18px; }
.ds-icon-button--sm { width: 14px; height: 14px; border-radius: 50%; font-size: 12px; color: var(--text-faint); }
.ds-icon-button:hover { color: var(--text-primary); background: var(--surface-hover); }
.ds-icon-button--danger:hover { color: var(--danger-accent); background: var(--danger-bg); }
</style>
