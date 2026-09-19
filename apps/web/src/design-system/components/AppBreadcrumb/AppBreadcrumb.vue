<script setup lang="ts">
defineProps<{
  items: { label: string; href?: string }[];
  label: string;
}>();
</script>

<template>
  <nav
    class="ds-breadcrumb-nav"
    :aria-label="label"
  >
    <ol class="ds-breadcrumb">
      <li
        v-for="(item, index) in items"
        :key="`${String(index)}-${item.label}`"
      >
        <a
          v-if="item.href && index < items.length - 1"
          :href="item.href"
        >{{ item.label }}</a>
        <span
          v-else
          :aria-current="index === items.length - 1 ? 'page' : undefined"
        >{{ item.label }}</span>
      </li>
    </ol>
  </nav>
</template>

<style scoped>
.ds-breadcrumb-nav { display: block; margin: 0; }
.ds-breadcrumb {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--font-sm);
}
.ds-breadcrumb li + li::before { content: "/"; margin-right: var(--space-2); color: var(--text-faint); }
.ds-breadcrumb a {
  display: inline;
  width: auto;
  padding: 0;
  border-radius: 0;
  color: var(--link);
  background: transparent;
  font-size: inherit;
  text-decoration: none;
}
.ds-breadcrumb a:hover { background: transparent; text-decoration: underline; }
.ds-breadcrumb span { color: var(--text-primary); font-weight: 600; }
</style>
