<script setup lang="ts">
import { computed } from "vue";
import { AppHeading } from "../../../design-system/components";
import type { WidgetConfig } from "../../../widgets/types";

const props = defineProps<{ config: WidgetConfig }>();
const text = computed(() => (typeof props.config.text === "string" ? props.config.text : ""));
</script>

<template>
  <AppHeading>
    {{ text }}
  </AppHeading>
</template>
