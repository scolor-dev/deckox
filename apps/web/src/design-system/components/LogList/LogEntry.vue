<script setup lang="ts">
withDefaults(
  defineProps<{
    priority: "error" | "warning" | "info";
    priorityLabel: string;
    timestamp: string;
    isoTimestamp?: string | null;
    process?: string | null;
    pid?: number | null;
    message: string;
  }>(),
  {
    isoTimestamp: null,
    process: null,
    pid: null,
  },
);
</script>

<template>
  <li :class="['ds-log-entry', `ds-log-entry--${priority}`]">
    <div class="ds-log-meta">
      <time :datetime="isoTimestamp ?? undefined">{{ timestamp }}</time>
      <span>{{ priorityLabel }}</span>
      <span v-if="process">{{ process }}<template v-if="pid !== null">[{{ pid }}]</template></span>
    </div>
    <pre>{{ message }}</pre>
  </li>
</template>

<style scoped>
.ds-log-entry {
  padding: var(--space-2) var(--space-4);
  border-bottom: 1px solid var(--border-subtle);
  border-left: 3px solid var(--log-accent-neutral);
  background: var(--surface-elevated);
}
.ds-log-entry:last-child { border-bottom: 0; }
.ds-log-entry--error { border-left-color: var(--danger-accent); }
.ds-log-entry--warning { border-left-color: var(--warning-strong); }
.ds-log-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1) var(--space-3);
  color: var(--text-muted);
  font-size: var(--font-2xs);
}
.ds-log-meta span:last-child { min-width: 0; overflow-wrap: anywhere; }
.ds-log-entry pre {
  margin: var(--space-1) 0 0;
  overflow-wrap: anywhere;
  color: var(--text-primary);
  font: var(--font-2xs) / 1.6 ui-monospace, SFMono-Regular, Consolas, monospace;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
