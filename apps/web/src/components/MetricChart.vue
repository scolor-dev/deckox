<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(defineProps<{
  values: number[];
  label: string;
  maximum?: number;
}>(), {
  maximum: 0,
});

const points = computed(() => {
  if (props.values.length === 0) return "";
  const maximum = props.maximum > 0
    ? props.maximum
    : Math.max(1, ...props.values);
  const denominator = Math.max(props.values.length - 1, 1);
  return props.values.map((value, index) => {
    const x = index / denominator * 300;
    const y = 76 - Math.min(Math.max(value / maximum, 0), 1) * 68;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
});
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
  </svg>
</template>
