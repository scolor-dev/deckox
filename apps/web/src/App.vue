<script setup lang="ts">
import { computed, ref } from "vue";
import type { ServerStatus } from "./api/client";
import OverviewView from "./views/OverviewView.vue";
import ServicesView from "./views/ServicesView.vue";
import StorageView from "./views/StorageView.vue";

type Page = "overview" | "services" | "storage";

const activePage = ref<Page>("overview");
const status = ref<ServerStatus | null>(null);
const menuOpen = ref(false);

const activeComponent = computed(() => ({
  overview: OverviewView,
  services: ServicesView,
  storage: StorageView,
})[activePage.value]);

const pageTitle = computed(() => ({
  overview: "概要",
  services: "サービス",
  storage: "ストレージ",
})[activePage.value]);

function navigate(page: Page) {
  activePage.value = page;
  menuOpen.value = false;
  document.title = `${pageTitle.value} · Deckox`;
}
</script>

<template>
  <div class="shell">
    <header class="mobile-bar">
      <div class="brand"><span class="brand-mark">D</span><span>Deckox</span></div>
      <button class="menu-button" type="button" :aria-expanded="menuOpen" aria-label="メニューを開く" @click="menuOpen = !menuOpen">Menu</button>
    </header>

    <aside :class="['sidebar', { open: menuOpen }]">
      <div class="brand">
        <span class="brand-mark">D</span>
        <span>Deckox</span>
      </div>
      <nav aria-label="メインナビゲーション">
        <button type="button" :class="{ active: activePage === 'overview' }" @click="navigate('overview')"><span>⌂</span>概要</button>
        <button type="button" :class="{ active: activePage === 'services' }" @click="navigate('services')"><span>◫</span>サービス</button>
        <button type="button" :class="{ active: activePage === 'storage' }" @click="navigate('storage')"><span>◇</span>ストレージ</button>
      </nav>
      <div class="agent-state">
        <span :class="['status-dot', status?.agent ? 'online' : 'offline']"></span>
        <div><strong>{{ status?.agent ? "Agent online" : "Agent unknown" }}</strong><small>{{ status?.agent?.hostname ?? "状態未取得" }}</small></div>
      </div>
      <div class="sidebar-footer">Management OS · v{{ status?.version ?? "0.1.0" }}</div>
    </aside>

    <main class="main-content">
      <component :is="activeComponent" @status="status = $event" />
    </main>
  </div>
</template>
