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
  z-index: 40;
  padding: 4px 8px;
  border-radius: 4px;
  color: var(--surface-elevated);
  background: var(--text-heading-strong);
  font-size: 11px;
  white-space: nowrap;
  opacity: 0;
  pointer-events: none;
  transition: opacity .15s ease, transform .15s ease;
}
.ds-tooltip-bubble--top { bottom: calc(100% + 6px); transform: translateX(-50%) translateY(4px); }
.ds-tooltip-bubble--bottom { top: calc(100% + 6px); transform: translateX(-50%) translateY(-4px); }
.ds-tooltip-anchor:hover .ds-tooltip-bubble,
.ds-tooltip-anchor:focus-within .ds-tooltip-bubble {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
}
</style>
