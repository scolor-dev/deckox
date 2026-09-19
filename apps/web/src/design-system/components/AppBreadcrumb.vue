<script setup lang="ts">
// Where you are in a hierarchy. Deckox's own navigation is flat today, so
// nothing uses this yet; the last item is always the current page and is
// never a link, whatever `href` it carries.
defineProps<{
  items: { label: string; href?: string }[];
  /** The nav landmark's accessible name, e.g. "Breadcrumb". */
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
/* The app's global stylesheet styles every bare `nav` and `nav a` (sidebar
 * grid, block links with padding and hover fills). Reset exactly what those
 * rules set, or a breadcrumb inherits the sidebar's look when the two share
 * a page. */
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
