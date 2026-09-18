<script setup lang="ts">
import { useEscapeToClose } from "../composables/useEscapeToClose";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    ariaLabel?: string | null;
    /** small matches ConfirmDialog's footprint; large matches a
     * content-heavy view like a log viewer. */
    size?: "small" | "medium" | "large";
    closeOnBackdrop?: boolean;
    closeOnEscape?: boolean;
  }>(),
  {
    ariaLabel: null,
    size: "medium",
    closeOnBackdrop: true,
    closeOnEscape: true,
  },
);

const emit = defineEmits<{
  close: [];
}>();

useEscapeToClose(() => props.open, () => { emit("close"); }, () => props.closeOnEscape);
</script>

<template>
  <div
    v-if="open"
    class="ds-modal-backdrop"
    @click.self="closeOnBackdrop ? emit('close') : undefined"
  >
    <section
      :class="['ds-modal', `ds-modal--${size}`]"
      role="dialog"
      aria-modal="true"
      :aria-label="ariaLabel ?? title"
    >
      <header class="ds-modal-header">
        <h2>{{ title }}</h2>
        <div class="ds-modal-header-actions">
          <slot name="header-actions" />
          <button
            type="button"
            class="ds-modal-close"
            aria-label="Close"
            @click="emit('close')"
          >
            ×
          </button>
        </div>
      </header>
      <div class="ds-modal-body">
        <slot />
      </div>
      <footer
        v-if="$slots.footer"
        class="ds-modal-footer"
      >
        <slot name="footer" />
      </footer>
    </section>
  </div>
</template>

<style scoped>
.ds-modal-backdrop {
  position: fixed;
  z-index: 60;
  inset: 0;
  display: grid;
  padding: 24px;
  background: var(--overlay-color);
  place-items: center;
}
.ds-modal {
  display: flex;
  max-height: min(760px, calc(100vh - 48px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--surface-elevated);
  box-shadow: 0 14px 40px var(--shadow-color);
}
.ds-modal--small { width: min(420px, 100%); }
.ds-modal--medium { width: min(600px, 100%); }
.ds-modal--large { width: min(900px, 100%); }

.ds-modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
  padding: 16px 18px;
  border-bottom: 1px solid var(--border-default);
}
.ds-modal-header h2 { margin: 0; color: var(--text-heading); font-size: 15px; }
.ds-modal-header-actions { display: flex; flex-shrink: 0; align-items: center; gap: 8px; }
.ds-modal-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  border: 0;
  border-radius: 4px;
  color: var(--text-muted);
  background: transparent;
  cursor: pointer;
  font-size: 18px;
  line-height: 1;
}
.ds-modal-close:hover { color: var(--text-primary); background: var(--surface-hover); }
.ds-modal-body { overflow: auto; padding: 16px 18px; }
.ds-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 14px 18px;
  border-top: 1px solid var(--border-default);
}

@media (max-width: 700px) {
  .ds-modal-backdrop { padding: 10px; }
}
</style>
