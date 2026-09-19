<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from "vue";
import { useEscapeToClose } from "../composables/useEscapeToClose";

const props = withDefaults(
  defineProps<{
    open: boolean;
    /** Which side of the trigger the panel hangs from. */
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

function handleOutsideClick(event: MouseEvent) {
  if (rootRef.value && !rootRef.value.contains(event.target as Node)) emit("close");
}

useEscapeToClose(() => props.open, () => { emit("close"); });

watch(
  () => props.open,
  (isOpen) => {
    // Capture phase + next tick would be ideal to skip the opening click,
    // but a plain listener registered after the current click has already
    // finished dispatching is enough in practice here.
    if (isOpen) window.addEventListener("click", handleOutsideClick);
    else window.removeEventListener("click", handleOutsideClick);
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
        :class="['ds-popover-panel', `ds-popover-panel--${align}`]"
        role="menu"
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
</style>
