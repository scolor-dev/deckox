<script setup lang="ts">
import AppIcon from "../AppIcon/AppIcon.vue";
import AppIconButton from "../AppIconButton/AppIconButton.vue";

defineProps<{
  tone: "success" | "warning" | "error";
  dismissLabel: string;
}>();

defineEmits<{
  dismiss: [];
}>();
</script>

<template>
  <div
    :class="['ds-toast', `ds-toast--${tone}`]"
    :role="tone === 'error' ? 'alert' : 'status'"
  >
    <p><slot /></p>
    <AppIconButton
      size="sm"
      :label="dismissLabel"
      @click="$emit('dismiss')"
    >
      <AppIcon
        name="close"
        size="sm"
      />
    </AppIconButton>
  </div>
</template>

<style scoped>
.ds-toast {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-3);
  border: 1px solid var(--border-strong);
  border-left-width: 4px;
  border-radius: var(--radius-sm);
  background: var(--surface-elevated);
  box-shadow: var(--shadow-raised);
  color: var(--text-strong);
  font-size: var(--font-sm);
}
.ds-toast p { margin: 0; }
.ds-toast--success { border-left-color: var(--success-strong); }
.ds-toast--warning { border-left-color: var(--warning-strong); }
.ds-toast--error { border-left-color: var(--danger-accent); }
</style>
