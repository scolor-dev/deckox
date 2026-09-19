<script setup lang="ts">
// Hover/focus-triggered label. Pure CSS (no JS timers or listeners) — the
// show/hide is a plain :hover/:focus-within transition, which is the more
// robust choice here than tracking open state in script.
withDefaults(
  defineProps<{
    text: string;
    placement?: "top" | "bottom";
  }>(),
  {
    placement: "top",
  },
);
</script>

<template>
  <span class="ds-tooltip-anchor">
    <slot />
    <span
      :class="['ds-tooltip-bubble', `ds-tooltip-bubble--${placement}`]"
      role="tooltip"
    >{{ text }}</span>
  </span>
</template>

<style scoped>
.ds-tooltip-anchor { position: relative; display: inline-flex; }
.ds-tooltip-bubble {
  position: absolute;
  left: 50%;
  z-index: var(--z-popover);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
  color: var(--surface-elevated);
  background: var(--text-heading-strong);
  font-size: var(--font-xs);
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--duration-base) var(--ease-standard), transform var(--duration-base) var(--ease-standard);
}
.ds-tooltip-bubble--top { bottom: calc(100% + 6px); transform: translateX(-50%) translateY(4px); }
.ds-tooltip-bubble--bottom { top: calc(100% + 6px); transform: translateX(-50%) translateY(-4px); }
.ds-tooltip-anchor:hover .ds-tooltip-bubble,
.ds-tooltip-anchor:focus-within .ds-tooltip-bubble {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}
</style>
