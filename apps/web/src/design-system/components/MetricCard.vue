<script setup lang="ts">
// The card shell (header/value/footer/warning state) for Overview's metric
// tiles. Deliberately doesn't render a chart itself — sparkline drawing is
// real per-metric logic (scale, path generation), not shell markup, so it
// stays the consumer's job via the default slot. TablePanel draws the same
// line: own the repeated chrome, not the arbitrary content inside it.
withDefaults(
  defineProps<{
    label: string;
    /** Right-aligned header meta, e.g. "4 cores". */
    meta?: string | null;
    value: string;
    footer?: string | null;
    warning?: boolean;
    warningText?: string | null;
  }>(),
  {
    meta: null,
    footer: null,
    warning: false,
    warningText: null,
  },
);
</script>

<template>
  <div :class="['ds-metric-card', { 'ds-metric-card--warning': warning }]">
    <div class="ds-metric-head">
      <span>{{ label }}</span>
      <small v-if="meta">{{ meta }}</small>
    </div>
    <strong>{{ value }}</strong>
    <slot />
    <p
      v-if="warning && warningText"
      class="ds-metric-warning"
    >
      {{ warningText }}
    </p>
    <p
      v-if="footer"
      class="ds-metric-foot"
    >
      {{ footer }}
    </p>
  </div>
</template>

<style scoped>
.ds-metric-card {
  min-width: 0;
  padding: var(--space-4) var(--space-5);
  border: 1px solid var(--border-default);
  border-radius: var(--radius-md);
  background: var(--surface-elevated);
}
.ds-metric-card--warning { border-color: var(--metric-warning-border); background: var(--warning-bg); }
.ds-metric-head { display: flex; justify-content: space-between; gap: var(--space-2); color: var(--text-label); font-size: var(--font-xs); }
.ds-metric-head small { min-width: 0; overflow-wrap: anywhere; color: var(--text-faint-alt); text-align: right; }
.ds-metric-card strong {
  display: block;
  min-height: 30px;
  margin: var(--space-2) 0 var(--space-2);
  color: var(--text-emphasis);
  font-size: var(--font-2xl);
  font-weight: 600;
}
.ds-metric-foot { margin: var(--space-2) 0 0; color: var(--text-faint-alt); font-size: var(--font-2xs); }
.ds-metric-warning { margin: var(--space-2) 0 0; color: var(--metric-warning-text); font-size: var(--font-2xs); }
</style>
