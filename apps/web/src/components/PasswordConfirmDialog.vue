<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { AppButton, AppStack, ConfirmDialog, InfoNote, NoticeBanner, TextField } from "../design-system/components";

const { t } = useI18n();

const props = withDefaults(
  defineProps<{
    open: boolean;
    title: string;
    description?: string | null;
    confirmLabel: string;
    pendingLabel: string;
    submitting?: boolean;
    errorMessage?: string | null;
    danger?: boolean;
  }>(),
  {
    description: null,
    submitting: false,
    errorMessage: null,
    danger: true,
  },
);

const emit = defineEmits<{ confirm: [password: string]; close: [] }>();

const password = ref("");

watch(
  () => props.open,
  (isOpen) => {
    if (!isOpen) password.value = "";
  },
);

function submit() {
  if (!password.value) return;
  emit("confirm", password.value);
}
</script>

<template>
  <ConfirmDialog
    :open="open"
    :title="title"
    @close="emit('close')"
  >
    <AppStack gap="3">
      <InfoNote v-if="description">
        {{ description }}
      </InfoNote>
      <AppStack
        id="password-confirm-form"
        as="form"
        gap="3"
        @submit.prevent="submit"
      >
        <TextField
          id="password-confirm-dialog-input"
          v-model="password"
          :label="t('common.adminPasswordLabel')"
          type="password"
          autocomplete="current-password"
          required
          data-autofocus
        />
        <NoticeBanner
          v-if="errorMessage"
          tone="error"
        >
          {{ errorMessage }}
        </NoticeBanner>
      </AppStack>
    </AppStack>
    <template #actions>
      <AppButton
        :disabled="submitting"
        @click="emit('close')"
      >
        {{ t("common.close") }}
      </AppButton>
      <AppButton
        variant="primary"
        type="submit"
        form="password-confirm-form"
        :danger="danger"
        :disabled="submitting"
      >
        {{ submitting ? pendingLabel : confirmLabel }}
      </AppButton>
    </template>
  </ConfirmDialog>
</template>
