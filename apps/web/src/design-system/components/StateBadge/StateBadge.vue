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
  gap: var(--space-2);
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
.ds-state-badge--failed { color: var(--danger-accent); font-weight: 700; }
.ds-state-badge--failed::before { background: var(--danger-accent); }
</style>
