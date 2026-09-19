<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { AppToast, ToastRegion } from "../design-system/components";
import { dismissNotification, notificationItems } from "../notifications";

const { t } = useI18n();
</script>

<template>
  <ToastRegion>
    <AppToast
      v-for="item in notificationItems"
      :key="item.id"
      :tone="item.kind"
      :dismiss-label="t('common.dismiss')"
      @dismiss="dismissNotification(item.id)"
    >
      {{ item.message }}
    </AppToast>
  </ToastRegion>
</template>
