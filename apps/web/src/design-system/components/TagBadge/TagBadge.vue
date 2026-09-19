<script setup lang="ts">
defineProps<{
  category: "standard" | "deckox" | "product";
}>();
</script>

<template>
  <span :class="['ds-tag-badge', `ds-tag-badge--${category}`]">
    <slot />
  </span>
</template>

<style scoped>
.ds-tag-badge {
  display: inline-block;
  padding: var(--space-0-5) var(--space-2);
  border-radius: var(--radius-badge);
  font-size: var(--font-2xs);
  font-weight: 600;
  line-height: 1.6;
  white-space: nowrap;
}
.ds-tag-badge--standard { background: var(--tag-standard-bg); color: var(--tag-standard-text); }
.ds-tag-badge--deckox { background: var(--tag-deckox-bg); color: var(--tag-deckox-text); }
.ds-tag-badge--product { background: var(--tag-product-bg); color: var(--tag-product-text); }
</style>
