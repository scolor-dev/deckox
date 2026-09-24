<script setup lang="ts">
withDefaults(
  defineProps<{
    as?: string;
    tone?: "default" | "muted" | "faint";
    size?: "xs" | "sm" | "md";
    mono?: boolean;
    strong?: boolean;
    block?: boolean;
    preserve?: boolean;
  }>(),
  {
    as: "span",
    tone: "default",
    size: "md",
    mono: false,
    strong: false,
    block: false,
    preserve: false,
  },
);
</script>

<template>
  <component
    :is="as"
    :class="['ds-text', `ds-text--tone-${tone}`, `ds-text--${size}`, { 'ds-text--mono': mono, 'ds-text--strong': strong, 'ds-text--block': block, 'ds-text--preserve': preserve }]"
  >
    <slot />
  </component>
</template>

<style scoped>
.ds-text { min-width: 0; margin: 0; overflow-wrap: anywhere; }
.ds-text--tone-default { color: var(--text-primary); }
.ds-text--tone-muted { color: var(--text-muted); }
.ds-text--tone-faint { color: var(--text-faint); }
.ds-text--xs { font-size: var(--font-xs); }
.ds-text--sm { font-size: var(--font-sm); }
.ds-text--md { font-size: var(--font-md); }
.ds-text--mono { font-family: ui-monospace, SFMono-Regular, Consolas, monospace; }
.ds-text--strong { font-weight: 600; }
.ds-text--block { display: block; }
.ds-text--preserve { white-space: pre-wrap; }
</style>
