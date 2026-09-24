<script setup lang="ts">
import { computed } from "vue";
import { AppCard, AppText } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();
const title = computed(() => (typeof props.config.title === "string" ? props.config.title : ""));
const body = computed(() => (typeof props.config.body === "string" ? props.config.body : ""));
</script>

<template>
  <AppCard :title="title">
    <AppText
      as="p"
      preserve
    >
      {{ body }}
    </AppText>
  </AppCard>
</template>
