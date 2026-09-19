<script setup lang="ts">
import AppIconButton from "./AppIconButton.vue";

// One transient message. The tone is a left accent (same success/warning/
// danger colors as NoticeBanner), not a fill — a toast floats over
// arbitrary page content, so it keeps the neutral elevated surface.
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
      ×
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
  box-shadow: 0 4px 14px var(--shadow-soft);
  color: var(--text-strong);
  font-size: var(--font-sm);
}
.ds-toast p { margin: 0; }
.ds-toast--success { border-left-color: var(--success-strong); }
.ds-toast--warning { border-left-color: var(--warning-strong); }
.ds-toast--error { border-left-color: var(--danger-accent); }
</style>
