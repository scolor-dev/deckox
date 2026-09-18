<script setup lang="ts">
import { useEscapeToClose } from "../composables/useEscapeToClose";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    /** Falls back to `title` — set explicitly only when the visible
     * heading isn't a good standalone label (rare). */
    ariaLabel?: string | null;
    closeOnBackdrop?: boolean;
    closeOnEscape?: boolean;
  }>(),
  {
    ariaLabel: null,
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
    class="ds-dialog-backdrop"
    @click.self="closeOnBackdrop ? emit('close') : undefined"
  >
    <section
      class="ds-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="ariaLabel ?? title"
    >
      <h2>{{ title }}</h2>
      <slot />
      <div
        v-if="$slots.actions"
        class="ds-dialog-actions"
      >
        <slot name="actions" />
      </div>
    </section>
  </div>
</template>

<style scoped>
.ds-dialog-backdrop {
  position: fixed;
  z-index: 60;
  inset: 0;
  display: grid;
  padding: 24px;
  background: var(--overlay-color);
  place-items: center;
}
.ds-dialog {
  width: min(420px, 100%);
  padding: 18px;
  border: 1px solid var(--border-strong);
  border-radius: 6px;
  background: var(--surface-elevated);
  box-shadow: 0 14px 40px var(--shadow-color);
}
.ds-dialog h2 { margin: 0 0 4px; font-size: 15px; color: var(--text-heading); }
.ds-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 10px;
}

@media (max-width: 700px) {
  .ds-dialog-backdrop { padding: 10px; }
}
</style>
