<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    meta?: string | null;
    value?: string | null;
    pairs?: { label: string; value: string }[] | null;
    footer?: string | null;
    warning?: boolean;
    warningText?: string | null;
  }>(),
  {
    meta: null,
    value: null,
    pairs: null,
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
    <strong v-if="value !== null">{{ value }}</strong>
    <dl
      v-if="pairs"
      class="ds-metric-pairs"
    >
      <div
        v-for="pair in pairs"
        :key="pair.label"
      >
        <dt>{{ pair.label }}</dt>
        <dd>{{ pair.value }}</dd>
      </div>
    </dl>
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
.ds-metric-pairs { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-3); margin: var(--space-2) 0; }
.ds-metric-pairs dt { color: var(--text-faint-alt); font-size: var(--font-2xs); }
.ds-metric-pairs dd { margin: var(--space-0-5) 0 0; color: var(--text-emphasis); font-size: var(--font-lg); font-weight: 600; overflow-wrap: anywhere; }
</style>
