<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useI18n } from "vue-i18n";
import { getAiReport, openPath, type ModelGroup } from "../api";
import { formatBytes } from "../utils/format";
import { useReportRules } from "../i18n";

const { t } = useI18n();
const reportRules = useReportRules();
const PREVIEW_ENTRIES = 10;

const groups = ref<ModelGroup[]>([]);
const loading = ref(true);
const errorMsg = ref("");
const expanded = reactive<Record<string, boolean>>({});
const showAll = reactive<Record<string, boolean>>({});

const availableGroups = computed(() => groups.value.filter((g) => g.available));
const totalFootprint = computed(() =>
  availableGroups.value.reduce((s, g) => s + g.sizeBytes, 0),
);

async function load() {
  loading.value = true;
  errorMsg.value = "";
  try {
    groups.value = await getAiReport();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function openDir(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function visibleEntries(g: ModelGroup) {
  return showAll[g.ruleId] ? g.entries : g.entries.slice(0, PREVIEW_ENTRIES);
}

onMounted(load);
</script>

<template>
  <div class="panel toolbar">
    <div class="toolbar-info">
      <b>{{ t("ai.title") }}</b>
      <div class="muted toolbar-sub">
        {{ t("ai.subtitle") }}
      </div>
    </div>
    <div class="toolbar-right">
      <div class="total">{{ t("ai.total", { bytes: formatBytes(totalFootprint) }) }}</div>
      <button class="btn btn-ghost btn-sm" :disabled="loading" @click="load">
        {{ loading ? t("ai.stats") : t("common.refresh") }}
      </button>
    </div>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>
  <div v-if="loading" class="panel loading-panel">{{ t("ai.loading") }}</div>

  <div v-for="g in availableGroups" :key="g.ruleId" class="collapse-card" :class="{ open: expanded[g.ruleId] !== false }">
    <div class="group-head">
      <button class="collapse-head ai-head" @click="expanded[g.ruleId] = expanded[g.ruleId] === false ? true : false">
        <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
          <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        <span class="group-name">{{ reportRules.name(g.ruleId, g.name) }}</span>
        <span class="group-size">{{ formatBytes(g.sizeBytes) }}</span>
      </button>
      <button v-if="g.paths[0]" class="btn btn-ghost btn-sm" @click="openDir(g.paths[0])">
        {{ t("common.openDir") }}
      </button>
    </div>

    <div v-show="expanded[g.ruleId] !== false" class="collapse-body ai-body">
      <div class="muted desc">{{ reportRules.description(g.ruleId, g.description) }}</div>

      <div v-if="g.cliOutput" class="cli">
        <div class="cli-title">ollama list</div>
        <pre>{{ g.cliOutput }}</pre>
      </div>

      <table v-if="g.entries.length" class="data-table">
        <tbody>
          <tr v-for="e in visibleEntries(g)" :key="e.path">
            <td class="entry-name" :title="e.path">{{ e.name }}</td>
            <td class="entry-size">{{ formatBytes(e.sizeBytes) }}</td>
            <td class="entry-action">
              <button class="link-btn" @click="openDir(e.path)">{{ t("common.open") }}</button>
            </td>
          </tr>
        </tbody>
      </table>
      <button
        v-if="g.entries.length > PREVIEW_ENTRIES"
        class="show-more"
        @click="showAll[g.ruleId] = !showAll[g.ruleId]"
      >
        {{ showAll[g.ruleId]
          ? t("common.collapse")
          : t("common.expandMore", {
              rest: g.entries.length - PREVIEW_ENTRIES,
              total: g.entries.length,
            }) }}
      </button>

      <div class="guidance">
        <svg viewBox="0 0 20 20" width="14" height="14" class="guidance-icon">
          <circle cx="10" cy="10" r="7.2" fill="none" stroke="currentColor" stroke-width="1.5" />
          <path d="M10 9v4" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
          <circle cx="10" cy="6.8" r="0.9" fill="currentColor" />
        </svg>
        {{ reportRules.guidance(g.ruleId, g.guidance) }}
      </div>
    </div>
  </div>

  <div v-if="!loading && !errorMsg && availableGroups.length === 0" class="panel empty-panel">
    <div class="empty-illustration">🧊</div>
    {{ t("ai.empty") }}
  </div>
</template>

<style scoped>
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 12px;
}
.toolbar-sub {
  margin-top: 4px;
  font-size: 12.5px;
  max-width: 560px;
  line-height: 1.6;
}
.toolbar-right {
  text-align: right;
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 6px;
}
.total {
  font-size: 18px;
  font-weight: 700;
  color: var(--primary);
  white-space: nowrap;
}
.error-panel {
  color: var(--red-text);
}
.loading-panel,
.empty-panel {
  color: var(--muted);
}
.empty-panel {
  text-align: center;
  padding: 40px 20px;
}
.empty-illustration {
  font-size: 34px;
  margin-bottom: 10px;
}
.group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding-right: 14px;
  gap: 10px;
}
.ai-head {
  flex: 1;
  padding-right: 8px;
}
.group-name {
  font-weight: 700;
  font-size: 14.5px;
}
.group-size {
  margin-left: auto;
  color: var(--primary);
  font-weight: 700;
  font-size: 13.5px;
}
.ai-body {
  padding-top: 0;
}
.desc {
  font-size: 12.5px;
  margin: 0 0 12px;
  line-height: 1.6;
}
.cli {
  background: var(--code-bg);
  color: var(--code-text);
  border-radius: 9px;
  padding: 10px 12px;
  margin-bottom: 10px;
}
.cli-title {
  font-size: 11px;
  color: #64748b;
  margin-bottom: 4px;
}
.cli pre {
  margin: 0;
  font-size: 12px;
  white-space: pre-wrap;
  font-family: Consolas, monospace;
}
.entry-table,
.data-table {
  font-size: 13px;
}
.entry-name {
  word-break: break-all;
  color: var(--text-2);
}
.entry-size {
  width: 110px;
  text-align: right;
  white-space: nowrap;
}
.entry-action {
  width: 60px;
  text-align: right;
}
.guidance {
  margin-top: 12px;
  background: var(--amber-soft);
  color: var(--amber-text);
  border-radius: 9px;
  padding: 9px 12px;
  font-size: 12.5px;
  line-height: 1.6;
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.guidance-icon {
  flex-shrink: 0;
  margin-top: 2px;
}
</style>
