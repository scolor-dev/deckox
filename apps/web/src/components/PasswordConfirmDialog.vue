<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";

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
  <div
    v-if="open"
    class="dialog-backdrop"
    @click.self="emit('close')"
  >
    <section
      class="confirm-dialog"
      role="dialog"
      aria-modal="true"
      :aria-label="title"
    >
      <h2>{{ title }}</h2>
      <p
        v-if="description"
        class="settings-help"
      >
        {{ description }}
      </p>
      <form
        class="settings-form"
        @submit.prevent="submit"
      >
        <label for="password-confirm-dialog-input">{{ t("common.adminPasswordLabel") }}</label>
        <input
          id="password-confirm-dialog-input"
          v-model="password"
          type="password"
          autocomplete="current-password"
          required
        >
        <p
          v-if="errorMessage"
          class="notice error"
          role="alert"
        >
          {{ errorMessage }}
        </p>
        <div class="dialog-actions">
          <button
            class="button"
            type="button"
            :disabled="submitting"
            @click="emit('close')"
          >
            {{ t("common.close") }}
          </button>
          <button
            :class="['primary-button', { 'danger-button': danger }]"
            type="submit"
            :disabled="submitting"
          >
            {{ submitting ? pendingLabel : confirmLabel }}
          </button>
        </div>
      </form>
    </section>
  </div>
</template>
