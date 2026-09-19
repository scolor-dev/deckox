<script setup lang="ts">
import { nextTick, onBeforeUnmount, ref, watch } from "vue";
import { useEscapeToClose } from "../../composables/useEscapeToClose";

const props = withDefaults(
  defineProps<{
    open: boolean;
    align?: "start" | "end";
  }>(),
  {
    align: "start",
  },
);

const emit = defineEmits<{
  close: [];
}>();

const rootRef = ref<HTMLElement | null>(null);
const panelRef = ref<HTMLElement | null>(null);
let trigger: HTMLElement | null = null;

function items(): HTMLElement[] {
  return [...(panelRef.value?.querySelectorAll<HTMLElement>("[role='menuitem']") ?? [])];
}

function handlePanelKeydown(event: KeyboardEvent) {
  if (event.key === "Tab") {
    trigger?.focus();
    emit("close");
    return;
  }
  const list = items();
  const index = list.indexOf(document.activeElement as HTMLElement);
  const next =
    event.key === "ArrowDown" ? (index + 1) % list.length
    : event.key === "ArrowUp" ? (index - 1 + list.length) % list.length
    : event.key === "Home" ? 0
    : event.key === "End" ? list.length - 1
    : -1;
  if (next === -1 || list.length === 0) return;
  event.preventDefault();
  list[next].focus();
}

function handleOutsideClick(event: MouseEvent) {
  if (rootRef.value && !rootRef.value.contains(event.target as Node)) emit("close");
}

useEscapeToClose(() => props.open, () => { emit("close"); });

watch(
  () => props.open,
  async (isOpen) => {
    if (isOpen) {
      trigger = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      window.addEventListener("click", handleOutsideClick);
      await nextTick();
      items()[0]?.focus();
    } else {
      window.removeEventListener("click", handleOutsideClick);
      const focusInside = panelRef.value?.contains(document.activeElement) ?? false;
      if (focusInside || document.activeElement === document.body) trigger?.focus();
      trigger = null;
    }
  },
);

onBeforeUnmount(() => { window.removeEventListener("click", handleOutsideClick); });
</script>

<template>
  <div
    ref="rootRef"
    class="ds-popover-anchor"
  >
    <slot name="trigger" />
    <Transition name="ds-popover-fade">
      <div
        v-if="open"
        ref="panelRef"
        :class="['ds-popover-panel', `ds-popover-panel--${align}`]"
        role="menu"
        @keydown="handlePanelKeydown"
      >
        <slot />
      </div>
    </Transition>
  </div>
</template>

<style scoped>
.ds-popover-anchor { position: relative; display: inline-block; }
.ds-popover-panel {
  position: absolute;
  z-index: var(--z-popover);
  top: calc(100% + 6px);
  min-width: 160px;
  padding: var(--space-2);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
  box-shadow: var(--shadow-overlay);
}
.ds-popover-panel--start { left: 0; }
.ds-popover-panel--end { right: 0; }

.ds-popover-fade-enter-active,
.ds-popover-fade-leave-active { transition: opacity var(--duration-fast) var(--ease-standard), transform var(--duration-fast) var(--ease-standard); }
.ds-popover-fade-enter-from,
.ds-popover-fade-leave-to { opacity: 0; transform: translateY(-4px); }

@media (prefers-reduced-motion: reduce) {
  .ds-popover-fade-enter-active,
  .ds-popover-fade-leave-active { transition: none; }
}
</style>
