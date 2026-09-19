<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "default" | "primary" | "action" | "menu" | "logout";
    danger?: boolean;
    disabled?: boolean;
    type?: "button" | "submit";
  }>(),
  {
    variant: "default",
    danger: false,
    disabled: false,
    type: "button",
  },
);
</script>

<template>
  <button
    :type="type"
    :disabled="disabled"
    :class="['ds-button', `ds-button--${variant}`, { 'ds-button--danger': danger }]"
  >
    <slot />
  </button>
</template>

<style scoped>
.ds-button {
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--surface-elevated);
  color: var(--text-strong);
  cursor: pointer;
  font: inherit;
}
.ds-button:disabled { opacity: .48; cursor: not-allowed; }

.ds-button--default {
  min-width: 68px;
  padding: var(--space-2) var(--space-4);
  font-size: var(--font-md);
}
.ds-button--default:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }

.ds-button--action { padding: var(--space-1) var(--space-2); font-size: var(--font-xs); }
.ds-button--action:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }
.ds-button--action.ds-button--danger { border-color: var(--danger-border); color: var(--danger-strong-border); }
.ds-button--action.ds-button--danger:hover:not(:disabled) { border-color: var(--danger-strong-border); background: var(--danger-bg); }

.ds-button--primary {
  padding: var(--space-2) var(--space-4);
  border-color: var(--brand-primary-border);
  color: var(--text-inverse);
  background: var(--brand-primary);
  font-weight: 600;
}
.ds-button--primary:hover:not(:disabled) { background: var(--brand-primary-hover); }
.ds-button--primary.ds-button--danger { border-color: var(--danger-strong-border); background: var(--danger-strong); }
.ds-button--primary.ds-button--danger:hover:not(:disabled) { background: var(--danger-strong-hover); }

.ds-button--menu { padding: var(--space-1) var(--space-2); font-size: var(--font-sm); }
.ds-button--menu:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }

.ds-button--logout {
  width: fit-content;
  padding: 0;
  border: 0;
  color: var(--text-label);
  background: transparent;
  font-size: var(--font-xs);
}
.ds-button--logout:hover:not(:disabled) { color: var(--link); text-decoration: underline; }
</style>
