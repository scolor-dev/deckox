<script setup lang="ts">
import { useI18n } from "vue-i18n";
import { jobFinished, type Job } from "../../../api/client";
import { useJobs } from "../../../data/jobs";
import { AppButton, AppEmptyState, AppStack, StateBadge } from "../../../design-system/components";

defineOptions({ inheritAttrs: false });

const i18n = useI18n();
const { t, locale } = i18n;
const { jobs, refresh } = useJobs();

function kindLabel(job: Job) {
  const key = `jobs.kind.${job.kind}`;
  return i18n.te(key) ? t(key) : job.kind;
}

function stateLabel(job: Job) {
  return t(`jobs.state.${job.state}`);
}

function badge(job: Job): "active" | "inactive" | "failed" {
  if (job.state === "failed") return "failed";
  return jobFinished(job) ? "active" : "inactive";
}

function when(job: Job) {
  return new Intl.DateTimeFormat(locale.value, { hour: "2-digit", minute: "2-digit", second: "2-digit" })
    .format(job.finished_ms ?? job.started_ms ?? job.created_ms);
}
</script>

<template>
  <AppStack gap="3">
    <AppEmptyState
      v-if="jobs.length === 0"
      :message="t('jobs.empty')"
    />
    <ul
      v-else
      class="job-list"
    >
      <li
        v-for="job in jobs"
        :key="job.id"
        class="job-item"
      >
        <StateBadge :state="badge(job)">
          {{ stateLabel(job) }}
        </StateBadge>
        <div class="job-text">
          <strong>{{ kindLabel(job) }}</strong>
          <span>{{ job.subject }}</span>
          <small v-if="job.message && job.state === 'failed'">{{ job.message }}</small>
        </div>
        <small>{{ when(job) }}</small>
      </li>
    </ul>
    <AppButton
      variant="action"
      @click="refresh"
    >
      {{ t("common.refresh") }}
    </AppButton>
  </AppStack>
</template>
