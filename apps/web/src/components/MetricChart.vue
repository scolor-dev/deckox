<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{
  values: number[];
  secondaryValues?: number[];
  label: string;
  maximum?: number;
  /** Total slots the chart width represents, so the period it covers stays
   * constant regardless of how many samples have arrived so far — a
   * partially-filled history occupies only the right portion of the width
   * (anchored to "now"), rather than being stretched to fill it. Must match
   * the history buffer's cap for the period to be meaningful. */
  limit?: number;
}>(), {
  maximum: 0,
  secondaryValues: () => [],
  limit: 120,
});

function makePoints(values: number[], maximum: number, limit: number) {
  if (values.length === 0) return "";
  const denominator = Math.max(limit - 1, 1);
  const offset = Math.max(limit - values.length, 0);
  return values.map((value, index) => {
    const x = (offset + index) / denominator * 300;
    const y = 76 - Math.min(Math.max(value / maximum, 0), 1) * 68;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
}

const chartMaximum = computed(() => {
  const maximum = props.maximum > 0
    ? props.maximum
    : Math.max(1, ...props.values, ...props.secondaryValues);
  return maximum;
});
const points = computed(() => makePoints(props.values, chartMaximum.value, props.limit));
const secondaryPoints = computed(() =>
  makePoints(props.secondaryValues, chartMaximum.value, props.limit));
</script>

<template>
  <svg
    class="metric-chart"
    viewBox="0 0 300 80"
    preserveAspectRatio="none"
    role="img"
    :aria-label="label"
  >
    <line
      x1="0"
      y1="76"
      x2="300"
      y2="76"
    />
    <polyline
      v-if="points"
      :points="points"
    />
    <polyline
      v-if="secondaryPoints"
      class="secondary"
      :points="secondaryPoints"
    />
  </svg>
</template>
