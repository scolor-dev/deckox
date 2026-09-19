<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { api, type InstalledSoftware, type SoftwarePackage } from "../api/client";
import { apiErrorKey } from "../api/errors";
import PasswordConfirmDialog from "../components/PasswordConfirmDialog.vue";
import {
  AppButton,
  AppCard,
  AppIcon,
  AppIconButton,
  AppStack,
  InfoNote,
  NoticeBanner,
  PageHeader,
  StateBadge,
  TableToolbar,
  TablePanel,
  TagBadge,
  TagToggle,
  TagToggleGroup,
  TextField,
} from "../design-system/components";
import { notify } from "../notifications";
import { preferences, type SoftwareTagFilterKey } from "../preferences";
import { buildSoftwareRows } from "../softwareGroups";

const { t } = useI18n();

const TAG_FILTER_KEYS: SoftwareTagFilterKey[] = ["installed", "not_installed"];

const packages = ref<SoftwarePackage[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const pending = ref<string | null>(null);

const newPackageName = ref("");
const adding = ref(false);
const addError = ref<string | null>(null);

type ManageAction = "install" | "remove" | "upgrade";

const CONFIRM_KEYS: Record<ManageAction, string> = {
  install: "software.confirmInstall",
  remove: "software.confirmRemove",
  upgrade: "software.confirmUpgrade",
};

const PENDING_LABEL_KEYS: Record<ManageAction, string> = {
  install: "software.installing",
  remove: "software.removing",
  upgrade: "software.upgrading",
};

const armed = ref<{ pkg: SoftwarePackage; action: ManageAction } | null>(null);
const actionError = ref<string | null>(null);
const submitting = ref(false);

function packageTag(pkg: SoftwarePackage): SoftwareTagFilterKey {
  return pkg.installed ? "installed" : "not_installed";
}

function tagLabel(tag: SoftwareTagFilterKey) {
  return tag === "installed" ? t("software.installed") : t("software.notInstalled");
}

function isTagHidden(tag: SoftwareTagFilterKey) {
  return preferences.hiddenSoftwareTags.includes(tag);
}

function toggleTag(tag: SoftwareTagFilterKey) {
  preferences.hiddenSoftwareTags = isTagHidden(tag)
    ? preferences.hiddenSoftwareTags.filter((hidden) => hidden !== tag)
    : [...preferences.hiddenSoftwareTags, tag];
}

const expandedBundles = ref(new Set<string>());

function toggleBundle(name: string) {
  const next = new Set(expandedBundles.value);
  if (next.has(name)) next.delete(name);
  else next.add(name);
  expandedBundles.value = next;
}

const rows = computed(() =>
  buildSoftwareRows(packages.value, expandedBundles.value, (pkg) => !isTagHidden(packageTag(pkg))),
);
const topLevelCount = computed(() => rows.value.filter((row) => row.depth === 0).length);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    packages.value = await api.software();
    const managedNames = new Set(packages.value.map((pkg) => pkg.name));
    for (const pkg of installedPackages.value) pkg.managed = managedNames.has(pkg.name);
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.software"));
  } finally {
    loading.value = false;
  }
}

const INSTALLED_LIMIT = 100;
const installedLoading = ref(false);
const installedError = ref<string | null>(null);
const installedPackages = ref<InstalledSoftware[]>([]);
const installedQuery = ref("");
const managing = ref<string | null>(null);

const installedMatches = computed(() => {
  const query = installedQuery.value.trim().toLowerCase();
  return query === ""
    ? installedPackages.value
    : installedPackages.value.filter((pkg) => pkg.name.toLowerCase().includes(query));
});
const installedShown = computed(() => installedMatches.value.slice(0, INSTALLED_LIMIT));

async function loadInstalled() {
  installedLoading.value = true;
  installedError.value = null;
  try {
    installedPackages.value = await api.installedSoftware();
  } catch (cause) {
    installedError.value = t(apiErrorKey(cause, "errors.software"));
  } finally {
    installedLoading.value = false;
  }
}

async function manageInstalled(pkg: InstalledSoftware) {
  managing.value = pkg.name;
  installedError.value = null;
  try {
    await api.softwareAllowlist(pkg.name, "allow");
    pkg.managed = true;
    notify("success", t("software.completed", { name: pkg.name }));
    await refresh();
  } catch (cause) {
    installedError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    managing.value = null;
  }
}

async function addPackage() {
  const name = newPackageName.value.trim();
  if (!name) return;

  adding.value = true;
  addError.value = null;
  try {
    await api.softwareAllowlist(name, "allow");
    newPackageName.value = "";
    notify("success", t("software.completed", { name }));
    await refresh();
  } catch (cause) {
    addError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    adding.value = false;
  }
}

async function disallow(pkg: SoftwarePackage) {
  if (!window.confirm(t("software.confirmDisallow", { name: pkg.name }))) return;

  pending.value = `${pkg.name}:disallow`;
  error.value = null;
  try {
    await api.softwareAllowlist(pkg.name, "disallow");
    notify("success", t("software.completed", { name: pkg.name }));
    await refresh();
  } catch (cause) {
    error.value = t(apiErrorKey(cause, "errors.softwareAction"));
    notify("error", error.value);
  } finally {
    pending.value = null;
  }
}

function openAction(pkg: SoftwarePackage, action: ManageAction) {
  armed.value = { pkg, action };
  actionError.value = null;
}

function closeAction() {
  armed.value = null;
  actionError.value = null;
}

async function confirmAction(password: string) {
  if (!armed.value || submitting.value) return;
  const { pkg, action } = armed.value;

  submitting.value = true;
  pending.value = `${pkg.name}:${action}`;
  actionError.value = null;
  try {
    await api.softwareAction(pkg.name, action, password);
    notify("success", t("software.completed", { name: pkg.name }));
    closeAction();
    await refresh();
  } catch (cause) {
    actionError.value = t(apiErrorKey(cause, "errors.softwareAction"));
  } finally {
    submitting.value = false;
    pending.value = null;
  }
}

onMounted(() => {
  void loadInstalled();
  void refresh();
});
</script>

<template>
  <div class="view">
    <PageHeader
      :title="t('software.title')"
      :subtitle="t('software.subtitle')"
    >
      <template #actions>
        <AppButton
          :disabled="loading"
          @click="refresh"
        >
          {{ loading ? t("common.loading") : t("common.refresh") }}
        </AppButton>
      </template>
    </PageHeader>

    <NoticeBanner
      v-if="error"
      tone="error"
    >
      {{ error }}
    </NoticeBanner>

    <TablePanel
      :loading="loading && packages.length === 0"
      :empty="rows.length === 0"
      :empty-message="t('software.empty')"
    >
      <template #toolbar>
        <TableToolbar :count="t('software.count', { count: topLevelCount })">
          <template #filters>
            <TagToggleGroup :label="t('software.tagVisibility')">
              <TagToggle
                v-for="tag in TAG_FILTER_KEYS"
                :key="tag"
                :category="tag === 'installed' ? 'standard' : 'other'"
                :checked="!isTagHidden(tag)"
                @update:checked="toggleTag(tag)"
              >
                {{ tagLabel(tag) }}
              </TagToggle>
            </TagToggleGroup>
          </template>
        </TableToolbar>
      </template>
      <template #loading>
        {{ t("software.loading") }}
      </template>
      <table>
        <thead>
          <tr>
            <th>{{ t("software.name") }}</th>
            <th>{{ t("software.status") }}</th>
            <th>{{ t("software.installedVersion") }}</th>
            <th>{{ t("software.availableVersion") }}</th>
            <th>{{ t("software.actions") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="row in rows"
            :key="row.pkg.name"
            :class="{ 'software-child': row.depth === 1 }"
          >
            <td>
              <AppStack
                direction="row"
                gap="2"
                align="center"
                wrap
              >
                <AppIconButton
                  v-if="row.childCount > 0"
                  :label="t(row.expanded ? 'software.collapseBundle' : 'software.expandBundle', { name: row.pkg.name })"
                  :aria-expanded="row.expanded"
                  @click="toggleBundle(row.pkg.name)"
                >
                  <AppIcon :name="row.expanded ? 'chevron-down' : 'chevron-right'" />
                </AppIconButton>
                <strong class="service-name">{{ row.pkg.name }}</strong>
                <TagBadge
                  v-if="row.childCount > 0"
                  category="standard"
                >
                  {{ t("software.bundleCount", { count: row.childCount }) }}
                </TagBadge>
                <TagBadge
                  v-if="row.pkg.auto && row.depth === 0"
                  category="deckox"
                >
                  {{ t("software.autoDetected") }}
                </TagBadge>
              </AppStack>
            </td>
            <td>
              <StateBadge :state="row.pkg.installed ? 'active' : 'inactive'">
                {{ row.pkg.installed ? t("software.installed") : t("software.notInstalled") }}
              </StateBadge>
              <small v-if="row.pkg.upgradable">{{ t("software.upgradable") }}</small>
            </td>
            <td>{{ row.pkg.installed_version ?? t("common.none") }}</td>
            <td>{{ row.pkg.available_version ?? t("common.none") }}</td>
            <td>
              <AppStack
                class="row-actions"
                direction="row"
                gap="2"
              >
                <AppButton
                  v-if="!row.pkg.installed"
                  variant="action"
                  :disabled="pending !== null"
                  @click="openAction(row.pkg, 'install')"
                >
                  {{ t("software.install") }}
                </AppButton>
                <template v-else>
                  <AppButton
                    v-if="row.pkg.upgradable"
                    variant="action"
                    :disabled="pending !== null"
                    @click="openAction(row.pkg, 'upgrade')"
                  >
                    {{ t("software.upgrade") }}
                  </AppButton>
                  <AppButton
                    variant="action"
                    danger
                    :disabled="pending !== null"
                    @click="openAction(row.pkg, 'remove')"
                  >
                    {{ t("software.remove") }}
                  </AppButton>
                </template>
                <AppButton
                  variant="action"
                  danger
                  :disabled="pending !== null"
                  @click="disallow(row.pkg)"
                >
                  {{ t("software.disallow") }}
                </AppButton>
              </AppStack>
            </td>
          </tr>
        </tbody>
      </table>
    </TablePanel>

    <AppCard>
      <AppStack gap="3">
        <h2>
          {{ t("software.addTitle") }}
        </h2>
        <NoticeBanner
          v-if="addError"
          tone="error"
        >
          {{ addError }}
        </NoticeBanner>
        <AppStack
          as="form"
          direction="row"
          gap="2"
          align="end"
          wrap
          @submit.prevent="addPackage"
        >
          <TextField
            id="software-package-name"
            v-model="newPackageName"
            :label="t('software.addLabel')"
            :placeholder="t('software.addPlaceholder')"
            required
          />
          <AppButton
            type="submit"
            :disabled="adding"
          >
            {{ adding ? t("software.adding") : t("software.add") }}
          </AppButton>
        </AppStack>
        <InfoNote>{{ t("software.addHelp") }}</InfoNote>
      </AppStack>
    </AppCard>

    <AppStack gap="3">
      <h2>{{ t("software.installedTitle") }}</h2>
      <InfoNote>{{ t("software.installedHelp") }}</InfoNote>
      <NoticeBanner
        v-if="installedError"
        tone="error"
      >
        {{ installedError }}
      </NoticeBanner>
      <TablePanel
        :loading="installedLoading"
        :empty="installedShown.length === 0"
        :empty-message="t('software.installedEmpty')"
      >
        <template #toolbar>
          <TableToolbar :count="t('software.installedCount', { shown: installedShown.length, total: installedMatches.length })">
            <template #search>
              <TextField
                id="software-installed-search"
                v-model="installedQuery"
                :label="t('software.installedSearch')"
                type="search"
                :placeholder="t('software.installedSearchPlaceholder')"
                label-hidden
              />
            </template>
          </TableToolbar>
        </template>
        <template #loading>
          {{ t("software.installedLoading") }}
        </template>
        <table>
          <thead>
            <tr>
              <th>{{ t("software.name") }}</th>
              <th>{{ t("software.installedVersion") }}</th>
              <th>{{ t("software.actions") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="pkg in installedShown"
              :key="pkg.name"
            >
              <td><strong class="service-name">{{ pkg.name }}</strong></td>
              <td>{{ pkg.version }}</td>
              <td>
                <StateBadge
                  v-if="pkg.managed"
                  state="active"
                >
                  {{ t("software.managed") }}
                </StateBadge>
                <AppButton
                  v-else
                  variant="action"
                  :disabled="managing !== null"
                  @click="manageInstalled(pkg)"
                >
                  {{ t("software.manage") }}
                </AppButton>
              </td>
            </tr>
          </tbody>
        </table>
        <template
          v-if="installedMatches.length > INSTALLED_LIMIT"
          #footer
        >
          <InfoNote>{{ t("software.installedLimited", { limit: INSTALLED_LIMIT }) }}</InfoNote>
        </template>
      </TablePanel>
    </AppStack>

    <InfoNote>{{ t("software.platformNote") }}</InfoNote>

    <PasswordConfirmDialog
      :open="armed !== null"
      :title="armed ? `${t(`software.${armed.action}`)}: ${armed.pkg.name}` : ''"
      :description="armed ? t(CONFIRM_KEYS[armed.action], { name: armed.pkg.name }) : null"
      :confirm-label="armed ? t(`software.${armed.action}`) : ''"
      :pending-label="armed ? t(PENDING_LABEL_KEYS[armed.action]) : ''"
      :submitting="submitting"
      :error-message="actionError"
      @confirm="confirmAction"
      @close="closeAction"
    />
  </div>
</template>
