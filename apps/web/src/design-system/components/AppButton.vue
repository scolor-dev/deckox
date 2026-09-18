<script setup lang="ts">
withDefaults(
  defineProps<{
    /** Which of the app's five button roles this is. See the design
     * system catalog for when to use each. */
    variant?: "default" | "primary" | "action" | "menu" | "logout";
    /** Only meaningful for default/primary/action — menu and logout have
     * no destructive form. */
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
  border-radius: 4px;
  background: var(--surface-elevated);
  color: var(--text-strong);
  cursor: pointer;
  font: inherit;
}
.ds-button:disabled { opacity: .48; cursor: not-allowed; }

/* default: the general-weight action (refresh, close, cancel). */
.ds-button--default {
  min-width: 68px;
  padding: 7px 14px;
  font-size: 13px;
}
.ds-button--default:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }

/* action: a small inline action inside a table row. */
.ds-button--action { padding: 5px 8px; font-size: 11px; }
.ds-button--action:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }
.ds-button--action.ds-button--danger { border-color: var(--danger-border); color: var(--danger-strong-border); }
/* The non-danger action hover leans on a border-color jump (border-strong
 * -> border-hover) for its contrast, not the background tint alone — danger
 * needs the same two-part cue, or the hover reads as almost no change at
 * all (surface-elevated -> danger-bg is a very close pale tint, especially
 * in light mode: #fff -> #fff5f5). */
.ds-button--action.ds-button--danger:hover:not(:disabled) { border-color: var(--danger-strong-border); background: var(--danger-bg); }

/* primary: the single filled call-to-action in a confirm dialog. */
.ds-button--primary {
  padding: 9px 14px;
  border-color: var(--brand-primary-border);
  color: var(--text-inverse);
  background: var(--brand-primary);
  font-weight: 600;
}
.ds-button--primary:hover:not(:disabled) { background: var(--brand-primary-hover); }
.ds-button--primary.ds-button--danger { border-color: var(--danger-strong-border); background: var(--danger-strong); }
.ds-button--primary.ds-button--danger:hover:not(:disabled) { background: var(--danger-strong-hover); }

/* menu: the mobile hamburger toggle only — not for general use. */
.ds-button--menu { padding: 5px 9px; font-size: 12px; }
.ds-button--menu:hover:not(:disabled) { border-color: var(--border-hover); background: var(--surface-hover); }

/* logout: bare-text link-style button, unique to the sidebar footer. */
.ds-button--logout {
  width: fit-content;
  padding: 0;
  border: 0;
  color: var(--text-label);
  background: transparent;
  font-size: 11px;
}
.ds-button--logout:hover:not(:disabled) { color: var(--link); text-decoration: underline; }
</style>
