<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { dismissNotification, notificationItems } from "../notifications";

const { t } = useI18n();
</script>

<template>
  <div
    class="notification-region"
    aria-live="polite"
    aria-atomic="false"
  >
    <div
      v-for="item in notificationItems"
      :key="item.id"
      :class="['notification-item', item.kind]"
      role="status"
    >
      <span>{{ item.message }}</span>
      <button
        type="button"
        :aria-label="t('common.dismiss')"
        @click="dismissNotification(item.id)"
      >
        {{ t("common.close") }}
      </button>
    </div>
  </div>
</template>
