<script setup lang="ts">
defineProps<{
  term: string;
}>();
</script>

<template>
  <dt>{{ term }}</dt>
  <dd><slot /></dd>
</template>
