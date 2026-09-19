<script setup lang="ts">
import AppIcon from "../AppIcon/AppIcon.vue";
import AppIconButton from "../AppIconButton/AppIconButton.vue";

withDefaults(
  defineProps<{
    removable?: boolean;
    removeLabel?: string;
  }>(),
  {
    removable: false,
    removeLabel: "Remove",
  },
);

defineEmits<{
  remove: [];
}>();
</script>

<template>
  <span class="ds-chip">
    <slot />
    <AppIconButton
      v-if="removable"
      size="sm"
      danger
      :label="removeLabel"
      @click="$emit('remove')"
    >
      <AppIcon
        name="close"
        size="sm"
      />
    </AppIconButton>
  </span>
</template>

<style scoped>
.ds-chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-3);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  color: var(--text-secondary);
  background: var(--surface-elevated);
  font-size: var(--font-xs);
}
</style>
