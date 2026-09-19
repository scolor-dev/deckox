<script setup lang="ts">
import { ref } from "vue";
import { useEscapeToClose } from "../../composables/useEscapeToClose";
import { useFocusTrap } from "../../composables/useFocusTrap";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
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

const dialogRef = ref<HTMLElement | null>(null);
useFocusTrap(dialogRef, () => props.open);
useEscapeToClose(() => props.open, () => { emit("close"); }, () => props.closeOnEscape);
</script>

<template>
  <div
    v-if="open"
    class="ds-dialog-backdrop"
    @click.self="closeOnBackdrop ? emit('close') : undefined"
  >
    <section
      ref="dialogRef"
      tabindex="-1"
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
  z-index: var(--z-overlay);
  inset: 0;
  display: grid;
  padding: var(--space-6);
  background: var(--overlay-color);
  place-items: center;
}
.ds-dialog {
  width: min(420px, 100%);
  padding: var(--space-5);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  box-shadow: var(--shadow-overlay);
}
.ds-dialog h2 { margin: 0 0 var(--space-1); font-size: var(--font-lg); color: var(--text-heading); }
.ds-dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-3);
}

@media (max-width: 700px) {
  .ds-dialog-backdrop { padding: var(--space-3); }
}
.ds-dialog:focus { outline: none; }
</style>
