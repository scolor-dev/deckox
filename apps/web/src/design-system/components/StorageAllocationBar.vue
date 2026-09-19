<script setup lang="ts">
// A multi-segment stacked bar plus legend — ProgressBar is one value
// against a track; this is several shares of one whole (Storage's
// per-mount breakdown). Which segments exist and what color each gets is
// the caller's decision (so a segment keeps its color across refreshes);
// this component only draws them to scale.
defineProps<{
  segments: { key: string; label: string; percent: number; color: string; value: string }[];
  /** Screen-reader summary of the whole bar; the legend already carries
   * each segment's own label/value as visible text. */
  label: string;
}>();
</script>

<template>
  <div class="ds-allocation">
    <div
      class="ds-allocation-bar"
      role="img"
      :aria-label="label"
    >
      <span
        v-for="segment in segments"
        :key="segment.key"
        :style="{ width: segment.percent + '%', background: segment.color }"
      />
    </div>
    <ul class="ds-allocation-legend">
      <li
        v-for="segment in segments"
        :key="segment.key"
      >
        <span
          class="ds-allocation-swatch"
          :style="{ background: segment.color }"
        />
        <span class="ds-allocation-label">{{ segment.label }}</span>
        <span class="ds-allocation-value">{{ segment.value }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.ds-allocation-bar {
  display: flex;
  overflow: hidden;
  height: var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--border-track);
}
.ds-allocation-bar span { display: block; height: 100%; }
.ds-allocation-bar span:not(:last-child) { margin-right: 2px; }
.ds-allocation-legend {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-4);
  margin: var(--space-3) 0 0;
  padding: 0;
  list-style: none;
}
.ds-allocation-legend li {
  display: inline-flex;
  min-width: 0;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--font-xs);
}
.ds-allocation-swatch { width: 9px; height: 9px; flex-shrink: 0; border-radius: 2px; }
.ds-allocation-label { overflow: hidden; color: var(--text-strong); text-overflow: ellipsis; white-space: nowrap; }
.ds-allocation-value { flex-shrink: 0; color: var(--text-faint-alt); }
</style>
