<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  tone: "error" | "warning" | "success";
}>();

const role = computed(() => (props.tone === "error" ? "alert" : "status"));
</script>

<template>
  <div
    :class="['ds-notice', `ds-notice--${tone}`]"
    :role="role"
  >
    <slot />
  </div>
</template>

<style scoped>
.ds-notice {
  margin-bottom: var(--space-4);
  padding: var(--space-3) var(--space-3);
  border: 1px solid;
  border-radius: var(--radius-sm);
  font-size: var(--font-md);
}
.ds-notice--error { border-color: var(--danger-border); color: var(--danger-text); background: var(--danger-bg); }
.ds-notice--warning { border-color: var(--warning-border); color: var(--warning-text); background: var(--warning-bg); }
.ds-notice--success { border-color: var(--success-border); color: var(--success-text); background: var(--success-bg); }
</style>
