<script setup lang="ts">
// Multi-root on purpose: a bare <dt>/<dd> pair, not wrapped in an extra
// element, so a <DetailList> full of these stays a spec-valid <dl>.
defineProps<{
  term: string;
}>();
</script>

<template>
  <dt>{{ term }}</dt>
  <dd><slot /></dd>
</template>
