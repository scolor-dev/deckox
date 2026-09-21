<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { AppButton, AppModal, AppStack, RadioGroup, SelectField, TextField } from "../design-system/components";
import type { PageDraft } from "./pageDraft";
import { MAX_SCREEN_ROWS, MIN_SCREEN_ROWS, type PageKind } from "./types";

const props = defineProps<{
  open: boolean;
  /** `null` while making a new page. */
  initial: PageDraft | null;
  canDelete: boolean;
}>();
const emit = defineEmits<{ close: []; save: [draft: PageDraft]; delete: [] }>();

const { t } = useI18n();
const title = ref("");
const kind = ref<PageKind>("scroll");
const rows = ref(8);

watch(() => props.open, (open) => {
  if (!open) return;
  title.value = props.initial?.title ?? "";
  kind.value = props.initial?.kind ?? "scroll";
  rows.value = props.initial?.rows ?? 8;
});

const kindOptions = computed(() => [
  { value: "scroll", label: t("layout.kindScroll") },
  { value: "screen", label: t("layout.kindScreen") },
]);
const rowOptions = Array.from({ length: MAX_SCREEN_ROWS - MIN_SCREEN_ROWS + 1 }, (_, index) => {
  const value = String(MIN_SCREEN_ROWS + index);
  return { value, label: value };
});

function save() {
  emit("save", { title: title.value, kind: kind.value, rows: rows.value });
}
</script>

<template>
  <AppModal
    :open="open"
    :title="initial ? t('layout.pageSettings') : t('layout.newPage')"
    :close-label="t('common.close')"
    @close="emit('close')"
  >
    <AppStack gap="4">
      <TextField
        id="page-title"
        v-model="title"
        :label="t('layout.pageName')"
        required
      />
      <RadioGroup
        :model-value="kind"
        name="page-kind"
        :label="t('layout.pageKind')"
        :options="kindOptions"
        :help="t('layout.pageKindHelp')"
        @update:model-value="kind = $event as PageKind"
      />
      <SelectField
        v-if="kind === 'screen'"
        id="page-rows"
        :model-value="String(rows)"
        :label="t('layout.screenRows')"
        :options="rowOptions"
        :help="t('layout.screenRowsHelp')"
        @update:model-value="rows = Number($event)"
      />
    </AppStack>
    <template #footer>
      <AppButton
        v-if="initial && canDelete"
        danger
        @click="emit('delete')"
      >
        {{ t("layout.deletePage") }}
      </AppButton>
      <AppButton
        variant="primary"
        :disabled="title.trim() === ''"
        @click="save"
      >
        {{ t("layout.save") }}
      </AppButton>
    </template>
  </AppModal>
</template>
