<script setup lang="ts">
import { ref } from "vue";
import { useEscapeToClose } from "../../composables/useEscapeToClose";
import { useFocusTrap } from "../../composables/useFocusTrap";
import AppIcon from "../AppIcon/AppIcon.vue";
import AppIconButton from "../AppIconButton/AppIconButton.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    ariaLabel?: string | null;
    closeLabel?: string;
    size?: "small" | "medium" | "large";
    closeOnBackdrop?: boolean;
    closeOnEscape?: boolean;
  }>(),
  {
    ariaLabel: null,
    closeLabel: "Close",
    size: "medium",
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
    class="ds-modal-backdrop"
    @click.self="closeOnBackdrop ? emit('close') : undefined"
  >
    <section
      ref="dialogRef"
      tabindex="-1"
      :class="['ds-modal', `ds-modal--${size}`]"
      role="dialog"
      aria-modal="true"
      :aria-label="ariaLabel ?? title"
    >
      <header class="ds-modal-header">
        <h2>{{ title }}</h2>
        <div class="ds-modal-header-actions">
          <slot name="header-actions" />
          <AppIconButton
            :label="closeLabel"
            @click="emit('close')"
          >
            <AppIcon name="close" />
          </AppIconButton>
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
  z-index: var(--z-overlay);
  inset: 0;
  display: grid;
  padding: var(--space-6);
  background: var(--overlay-color);
  place-items: center;
}
.ds-modal {
  display: flex;
  max-height: min(760px, calc(100vh - 48px));
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  box-shadow: var(--shadow-overlay);
}
.ds-modal--small { width: min(420px, 100%); }
.ds-modal--medium { width: min(600px, 100%); }
.ds-modal--large { width: min(900px, 100%); }

.ds-modal-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-5);
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--border-default);
}
.ds-modal-header h2 { margin: 0; color: var(--text-heading); font-size: var(--font-lg); }
.ds-modal-header-actions { display: flex; flex-shrink: 0; align-items: center; gap: var(--space-2); }
.ds-modal-body { overflow: auto; padding: var(--space-4) var(--space-5); }
.ds-modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  padding: var(--space-4) var(--space-5);
  border-top: 1px solid var(--border-default);
}

@media (max-width: 700px) {
  .ds-modal-backdrop { padding: var(--space-3); }
}
.ds-modal:focus { outline: none; }
</style>
