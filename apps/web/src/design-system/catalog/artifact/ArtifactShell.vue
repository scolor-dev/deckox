<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import DesignSystemCatalog from "../DesignSystemCatalog.vue";
import tokensSource from "../../tokens.css?raw";

const componentCount = Object.keys(import.meta.glob("../../components/*/*.vue")).length;
const tokenCount = new Set(tokensSource.match(/--[\w-]+(?=\s*:)/g)).size;

type Mode = "auto" | "light" | "dark";
const modes: { key: Mode; label: string }[] = [
  { key: "auto", label: "Auto" },
  { key: "light", label: "Light" },
  { key: "dark", label: "Dark" },
];
const stamped = document.documentElement.dataset.theme;
const mode = ref<Mode>(stamped === "light" || stamped === "dark" ? stamped : "auto");
function setMode(next: Mode) {
  mode.value = next;
  if (next === "auto") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = next;
}

const main = ref<HTMLElement | null>(null);
const sections = ref<{ id: string; title: string }[]>([]);
const active = ref("");
let observer: IntersectionObserver | undefined;

onMounted(() => {
  const found = [...(main.value?.querySelectorAll<HTMLElement>("section.catalog-section") ?? [])];
  sections.value = found.map((el, index) => {
    el.id = `section-${String(index)}`;
    return { id: el.id, title: el.querySelector("h2")?.textContent ?? el.id };
  });
  active.value = sections.value[0]?.id ?? "";
  observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) if (entry.isIntersecting) active.value = entry.target.id;
    },
    { rootMargin: "-80px 0px -70% 0px" },
  );
  for (const el of found) observer.observe(el);
});
onBeforeUnmount(() => observer?.disconnect());
</script>

<template>
  <div class="shell">
    <header class="shell-top">
      <div class="shell-title">
        <h1>Deckox Design System</h1>
        <p>{{ componentCount }} components · {{ tokenCount }} tokens · light / dark</p>
      </div>
      <div
        class="shell-modes"
        role="radiogroup"
        aria-label="Theme"
      >
        <button
          v-for="item in modes"
          :key="item.key"
          type="button"
          role="radio"
          :aria-checked="mode === item.key"
          :class="{ on: mode === item.key }"
          @click="setMode(item.key)"
        >
          {{ item.label }}
        </button>
      </div>
    </header>
    <div class="shell-body">
      <nav
        class="shell-nav"
        aria-label="Sections"
      >
        <a
          v-for="item in sections"
          :key="item.id"
          :href="`#${item.id}`"
          :class="{ on: active === item.id }"
        >{{ item.title }}</a>
      </nav>
      <main ref="main">
        <DesignSystemCatalog />
      </main>
    </div>
  </div>
</template>

<style>
*, *::before, *::after { box-sizing: border-box; }
:root { color-scheme: light; }
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) { color-scheme: dark; } }
:root[data-theme="dark"] { color-scheme: dark; }
body {
  margin: 0;
  color: var(--text-base);
  background: var(--surface-page);
  font: 14px/1.5 -apple-system, BlinkMacSystemFont, "Hiragino Sans", "Yu Gothic UI", "Yu Gothic", "Meiryo", "Segoe UI", sans-serif;
}
.shell-top {
  position: sticky;
  top: env(safe-area-inset-top, 0px);
  z-index: 30;
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border-default);
  background: var(--surface-elevated);
}
.shell-title h1 { margin: 0; font-size: 17px; line-height: 1.3; color: var(--text-heading); }
.shell-title p { margin: 0; font-size: 12px; color: var(--text-muted); }
.shell-modes { display: inline-flex; padding: 2px; border: 1px solid var(--border-strong); border-radius: 6px; background: var(--surface-page); }
.shell-modes button {
  padding: 4px 12px;
  border: 0;
  border-radius: 4px;
  color: var(--text-secondary);
  background: transparent;
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.shell-modes button.on { color: var(--text-inverse); background: var(--brand-primary); }
.shell-modes button:focus-visible, .shell-nav a:focus-visible { outline: none; box-shadow: var(--shadow-focus); }
.shell-body { display: grid; grid-template-columns: 220px minmax(0, 1fr); align-items: start; }
.shell-nav {
  position: sticky;
  top: calc(env(safe-area-inset-top, 0px) + 58px);
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: calc(100vh - 58px);
  overflow-y: auto;
  padding: 16px 8px 24px 16px;
}
.shell-nav a {
  padding: 4px 8px;
  border-radius: 4px;
  color: var(--text-secondary);
  font-size: 12px;
  text-decoration: none;
}
.shell-nav a:hover { background: var(--surface-hover); color: var(--text-heading); }
.shell-nav a.on { color: var(--brand-primary); background: var(--brand-focus-ring); font-weight: 600; }
.shell main > .catalog { max-width: 1000px; margin: 0; padding: 24px 24px 96px; background: transparent; }
.shell main .catalog .catalog-header { display: none; }
.shell main section.catalog-section { scroll-margin-top: 72px; }
@media (max-width: 900px) {
  .shell-body { display: block; }
  .shell-nav { position: static; flex-direction: row; max-height: none; overflow-x: auto; padding: 8px 16px; border-bottom: 1px solid var(--border-default); }
  .shell-nav a { flex: none; white-space: nowrap; }
  .shell main > .catalog { padding: 16px 16px 64px; }
}
@media (max-width: 520px) {
  .shell-top { padding: 8px 16px; }
  .shell-title p { display: none; }
}
</style>
