<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    /** 0–100. Out-of-range values are clamped, not rejected — a caller
     * computing a percentage from live metrics shouldn't have to guard
     * against float rounding pushing it to 100.4. */
    value: number;
    critical?: boolean;
    label?: string | null;
  }>(),
  {
    critical: false,
    label: null,
  },
);

const clamped = computed(() => Math.min(100, Math.max(0, props.value)));
</script>

<template>
  <div
    class="ds-progress"
    role="progressbar"
    :aria-valuenow="clamped"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-label="label ?? undefined"
  >
    <span
      :class="{ 'ds-progress-fill--critical': critical }"
      class="ds-progress-fill"
      :style="{ width: clamped + '%' }"
    />
  </div>
</template>

<style scoped>
.ds-progress { overflow: hidden; height: 5px; border-radius: 3px; background: var(--border-track); }
.ds-progress-fill { display: block; height: 100%; border-radius: inherit; background: var(--brand-focus); transition: width .2s ease; }
.ds-progress-fill--critical { background: var(--danger-accent); }

@media (prefers-reduced-motion: reduce) {
  .ds-progress-fill { transition: none; }
}
</style>
