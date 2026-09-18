<script setup lang="ts">
defineProps<{
  state: "active" | "inactive" | "failed";
}>();
</script>

<template>
  <span :class="['ds-state-badge', `ds-state-badge--${state}`]">
    <slot />
  </span>
</template>

<style scoped>
.ds-state-badge {
  display: inline-flex;
  gap: 6px;
  align-items: center;
  white-space: nowrap;
}
.ds-state-badge::before {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  content: "";
}
.ds-state-badge--active { color: var(--success-text-badge); }
.ds-state-badge--active::before { background: var(--success-strong); }
.ds-state-badge--inactive { color: var(--text-muted); }
.ds-state-badge--inactive::before { background: var(--neutral-strong); }
/* failed is deliberately the one state that also carries font-weight — it
 * is the state a viewer most needs to notice at a glance (design review
 * "high priority": failed was previously indistinguishable from stopped). */
.ds-state-badge--failed { color: var(--danger-accent); font-weight: 700; }
.ds-state-badge--failed::before { background: var(--danger-accent); }
</style>
