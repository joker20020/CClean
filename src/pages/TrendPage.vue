<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import * as echarts from "echarts";
import { listen } from "@tauri-apps/api/event";
import {
  clearCleanLogs,
  clearFsEvents,
  deleteCleanLog,
  deleteFsEvent,
  getCleanLogs,
  getFsEvents,
  getTrend,
  takeSnapshot,
  type CleanLog,
  type FsEvent,
  type TrendData,
} from "../api";
import { formatBytes } from "../utils/format";
import { chartColors } from "../utils/theme";
import { useCategoryNames } from "../i18n";
import { confirmDialog } from "../components/confirm";

const { t, locale } = useI18n();
const categoryNames = useCategoryNames();
const PAGE_SIZE = 20;
/** 内存中最多保留的实时事件条数（超出后丢弃最旧的） */
const MAX_LIVE_EVENTS = 300;

const trend = ref<TrendData>({ snapshots: [], items: [] });
const logs = ref<CleanLog[]>([]);
const events = ref<FsEvent[]>([]);
const loading = ref(false);
const taking = ref(false);
const errorMsg = ref("");
const sectionsOpen = reactive({ logs: true, events: false });
const logShown = ref(PAGE_SIZE);
const eventShown = ref(PAGE_SIZE);

const diskChartEl = ref<HTMLDivElement | null>(null);
const catChartEl = ref<HTMLDivElement | null>(null);
let diskChart: echarts.ECharts | null = null;
let catChart: echarts.ECharts | null = null;
let unlisten: (() => void) | null = null;
let unlistenFs: (() => void) | null = null;

const gb = (n: number) => +(n / (1024 * 1024 * 1024)).toFixed(2);
const fmtTime = (ts: number) => {
  const d = new Date(ts * 1000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
};

const categorySeries = computed(() => {
  const cats = ["system", "browser", "dev", "ai", "apps", "gpu"];
  const totals = new Map<string, Map<number, number>>();
  for (const it of trend.value.items) {
    const cat = it.rule_id.split(".")[0];
    if (!cats.includes(cat)) continue;
    if (!totals.has(cat)) totals.set(cat, new Map());
    const m = totals.get(cat)!;
    m.set(it.snapshot_id, (m.get(it.snapshot_id) ?? 0) + it.size_bytes);
  }
  return cats
    .filter((c) => totals.has(c))
    .map((c) => ({
      name: categoryNames.value[c] ?? c,
      type: "line",
      smooth: true,
      symbol: "circle",
      symbolSize: 5,
      data: trend.value.snapshots.map((s) => gb(totals.get(c)?.get(s.id) ?? 0)),
      connectNulls: true,
    }));
});

function renderCharts() {
  const c = chartColors();
  const xs = trend.value.snapshots.map((s) => fmtTime(s.created_at));
  const axisCommon = {
    axisLine: { lineStyle: { color: c.border } },
    axisLabel: { color: c.muted, fontSize: 11 },
    splitLine: { lineStyle: { color: c.border, type: "dashed" as const } },
  };

  if (diskChartEl.value) {
    diskChart ??= echarts.init(diskChartEl.value);
    diskChart.setOption(
      {
        tooltip: {
          trigger: "axis",
          valueFormatter: (v: unknown) => `${v} GB`,
          backgroundColor: c.dark ? "#1a2340" : "#ffffff",
          borderColor: c.border,
          textStyle: { color: c.text },
        },
        grid: { left: 50, right: 20, top: 36, bottom: 40 },
        legend: { data: [t("trend.diskFree"), t("trend.diskUsed")], top: 0, textStyle: { color: c.muted } },
        xAxis: { type: "category", data: xs, ...axisCommon, splitLine: { show: false } },
        yAxis: { type: "value", name: "GB", nameTextStyle: { color: c.muted }, ...axisCommon },
        series: [
          {
            name: t("trend.diskFree"),
            type: "line",
            smooth: true,
            areaStyle: { opacity: 0.12 },
            color: c.green,
            symbol: "circle",
            symbolSize: 5,
            data: trend.value.snapshots.map((s) => gb(s.disk_free)),
          },
          {
            name: t("trend.diskUsed"),
            type: "line",
            smooth: true,
            color: c.red,
            symbol: "circle",
            symbolSize: 5,
            data: trend.value.snapshots.map((s) => gb(s.disk_used)),
          },
        ],
      },
      { notMerge: true },
    );
  }

  if (catChartEl.value) {
    catChart ??= echarts.init(catChartEl.value);
    catChart.setOption(
      {
        tooltip: {
          trigger: "axis",
          valueFormatter: (v: unknown) => `${v} GB`,
          backgroundColor: c.dark ? "#1a2340" : "#ffffff",
          borderColor: c.border,
          textStyle: { color: c.text },
        },
        legend: { top: 0, textStyle: { color: c.muted } },
        grid: { left: 50, right: 20, top: 36, bottom: 40 },
        color: c.palette,
        xAxis: { type: "category", data: xs, ...axisCommon, splitLine: { show: false } },
        yAxis: { type: "value", name: "GB", nameTextStyle: { color: c.muted }, ...axisCommon },
        series: categorySeries.value,
      },
      { notMerge: true },
    );
  }
}

async function load() {
  loading.value = true;
  errorMsg.value = "";
  try {
    const [t, l, e] = await Promise.all([getTrend(180), getCleanLogs(100), getFsEvents(100)]);
    trend.value = t;
    logs.value = l;
    events.value = e;
    await nextTick();
    renderCharts();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function snapshotNow() {
  taking.value = true;
  try {
    await takeSnapshot();
    await load();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    taking.value = false;
  }
}

/** 删除单条清理记录（仅管理记录，不影响文件系统） */
async function removeLog(id: number) {
  try {
    await deleteCleanLog(id);
    logs.value = logs.value.filter((l) => l.id !== id);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function clearLogs() {
  if (logs.value.length === 0) return;
  const ok = await confirmDialog({
    title: t("trend.manage.clearLogsTitle"),
    message: t("trend.manage.clearLogsConfirm", { n: logs.value.length }),
    confirmText: t("trend.manage.clear"),
    cancelText: t("common.cancel"),
    danger: true,
  });
  if (!ok) return;
  try {
    await clearCleanLogs();
    logs.value = [];
    logShown.value = PAGE_SIZE;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

/** 删除单条实时事件 */
async function removeEvent(id: number) {
  try {
    await deleteFsEvent(id);
    events.value = events.value.filter((e) => e.id !== id);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function clearEvents() {
  if (events.value.length === 0) return;
  const ok = await confirmDialog({
    title: t("trend.manage.clearEventsTitle"),
    message: t("trend.manage.clearEventsConfirm", { n: events.value.length }),
    confirmText: t("trend.manage.clear"),
    cancelText: t("common.cancel"),
    danger: true,
  });
  if (!ok) return;
  try {
    await clearFsEvents();
    events.value = [];
    eventShown.value = PAGE_SIZE;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

/** 监听到的实时事件载荷（后端使用 camelCase） */
interface FsEventPayload {
  id: number;
  ts: number;
  kind: string;
  ruleId: string;
  path: string;
  sizeDelta: number | null;
}

function onFsEvent(payload: FsEventPayload) {
  const ev: FsEvent = {
    id: payload.id,
    ts: payload.ts,
    kind: payload.kind,
    rule_id: payload.ruleId,
    path: payload.path,
    size_delta: payload.sizeDelta,
  };
  // 去重（同一条事件可能已在初始加载中）
  if (events.value.some((e) => e.id === ev.id)) return;
  events.value = [ev, ...events.value].slice(0, MAX_LIVE_EVENTS);
}

const statusText = computed<Record<string, string>>(() => ({
  ok: t("trend.status.ok"),
  partial: t("trend.status.partial"),
  failed: t("trend.status.failed"),
}));
const kindText = computed<Record<string, string>>(() => ({
  create: t("trend.kind.create"),
  write: t("trend.kind.write"),
  delete: t("trend.kind.delete"),
  rename: t("trend.kind.rename"),
}));

function resize() {
  diskChart?.resize();
  catChart?.resize();
}
function onThemeChange() {
  nextTick(renderCharts);
}

// 语言切换后重绘图表（图例/分类名包含本地化文案）
watch(locale, () => nextTick(renderCharts));

onMounted(async () => {
  await load();
  unlisten = await listen("snapshot-taken", () => load());
  // 实时追加目录监听事件
  unlistenFs = await listen<FsEventPayload>("fs-event", (e) => onFsEvent(e.payload));
  window.addEventListener("resize", resize);
  window.addEventListener("theme-change", onThemeChange);
});
onUnmounted(() => {
  unlisten?.();
  unlistenFs?.();
  window.removeEventListener("resize", resize);
  window.removeEventListener("theme-change", onThemeChange);
  diskChart?.dispose();
  catChart?.dispose();
});
watch(trend, () => nextTick(renderCharts));
</script>

<template>
  <div class="panel toolbar">
    <div>
      <b>{{ t("trend.title") }}</b>
      <div class="muted toolbar-sub">
        {{ t("trend.subtitle") }}
      </div>
    </div>
    <button class="btn btn-primary" :disabled="taking" @click="snapshotNow">
      {{ taking ? t("trend.recording") : t("trend.record") }}
    </button>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>
  <div v-if="loading" class="panel loading-panel">{{ t("trend.loading") }}</div>

  <div class="panel">
    <div class="chart-title">{{ t("trend.diskChartTitle") }}</div>
    <div ref="diskChartEl" class="chart"></div>
    <div v-if="trend.snapshots.length < 2" class="muted hint">
      {{ t("trend.hint", { n: trend.snapshots.length }) }}
    </div>
  </div>

  <div class="panel">
    <div class="chart-title">{{ t("trend.catChartTitle") }}</div>
    <div ref="catChartEl" class="chart"></div>
  </div>

  <!-- 清理记录（可折叠） -->
  <div class="collapse-card" :class="{ open: sectionsOpen.logs }">
    <div class="section-head">
      <button class="collapse-head" @click="sectionsOpen.logs = !sectionsOpen.logs">
        <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
          <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        {{ t("trend.logsTitle") }}
        <span class="section-count">{{ logs.length }}</span>
      </button>
      <button
        v-if="sectionsOpen.logs && logs.length"
        class="btn btn-ghost btn-sm section-action"
        @click="clearLogs"
      >
        {{ t("trend.manage.clear") }}
      </button>
    </div>
    <div v-show="sectionsOpen.logs" class="collapse-body">
      <table class="data-table">
        <thead>
          <tr>
            <th>{{ t("trend.col.time") }}</th>
            <th>{{ t("trend.col.rule") }}</th>
            <th style="width: 110px">{{ t("trend.col.freed") }}</th>
            <th style="width: 90px">{{ t("trend.col.status") }}</th>
            <th>{{ t("trend.col.detail") }}</th>
            <th style="width: 56px"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="l in logs.slice(0, logShown)" :key="l.id">
            <td class="nowrap">{{ fmtTime(l.ts) }}</td>
            <td>{{ l.rule_id }}</td>
            <td class="num">{{ formatBytes(l.size_bytes) }}</td>
            <td>
              <span :class="['status-tag', l.status]">{{ statusText[l.status] ?? l.status }}</span>
            </td>
            <td class="muted detail">{{ l.detail }}</td>
            <td class="row-action">
              <button class="icon-btn" :title="t('trend.manage.deleteRow')" @click="removeLog(l.id)">
                <svg viewBox="0 0 20 20" width="14" height="14">
                  <path d="M6 6l8 8M14 6l-8 8" stroke="currentColor" stroke-width="1.7"
                    stroke-linecap="round" />
                </svg>
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <button v-if="logs.length > logShown" class="show-more" @click="logShown += PAGE_SIZE">
        {{ t("common.showMoreRows", { n: logs.length - logShown }) }}
      </button>
      <div v-if="logs.length === 0" class="muted hint">{{ t("trend.noLogs") }}</div>
    </div>
  </div>

  <!-- 实时事件（默认折叠） -->
  <div class="collapse-card" :class="{ open: sectionsOpen.events }">
    <div class="section-head">
      <button class="collapse-head" @click="sectionsOpen.events = !sectionsOpen.events">
        <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
          <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
            stroke-linecap="round" stroke-linejoin="round" />
        </svg>
        {{ t("trend.eventsTitle") }}
        <span class="section-count">{{ events.length }}</span>
        <span class="live-badge">
          <span class="live-dot"></span>{{ t("trend.manage.live") }}
        </span>
        <span class="muted section-note">{{ t("trend.eventsNote") }}</span>
      </button>
      <button
        v-if="sectionsOpen.events && events.length"
        class="btn btn-ghost btn-sm section-action"
        @click="clearEvents"
      >
        {{ t("trend.manage.clear") }}
      </button>
    </div>
    <div v-show="sectionsOpen.events" class="collapse-body">
      <table class="data-table">
        <thead>
          <tr>
            <th>{{ t("trend.col.time") }}</th>
            <th style="width: 70px">{{ t("trend.col.kind") }}</th>
            <th>{{ t("trend.col.rule") }}</th>
            <th>{{ t("trend.col.path") }}</th>
            <th style="width: 56px"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in events.slice(0, eventShown)" :key="e.id">
            <td class="nowrap">{{ fmtTime(e.ts) }}</td>
            <td>
              <span :class="['kind-tag', e.kind]">{{ kindText[e.kind] ?? e.kind }}</span>
            </td>
            <td class="nowrap">{{ e.rule_id }}</td>
            <td class="path" :title="e.path">{{ e.path }}</td>
            <td class="row-action">
              <button class="icon-btn" :title="t('trend.manage.deleteRow')" @click="removeEvent(e.id)">
                <svg viewBox="0 0 20 20" width="14" height="14">
                  <path d="M6 6l8 8M14 6l-8 8" stroke="currentColor" stroke-width="1.7"
                    stroke-linecap="round" />
                </svg>
              </button>
            </td>
          </tr>
        </tbody>
      </table>
      <button v-if="events.length > eventShown" class="show-more" @click="eventShown += PAGE_SIZE">
        {{ t("common.showMoreRows", { n: events.length - eventShown }) }}
      </button>
      <div v-if="events.length === 0" class="muted hint">
        {{ t("trend.noEvents") }}
      </div>
    </div>
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
}
.chart-title {
  font-weight: 700;
  font-size: 14px;
  margin-bottom: 8px;
}
.chart {
  height: 280px;
}
.hint {
  font-size: 12px;
  margin-top: 6px;
}
.section-count {
  font-size: 12px;
  color: var(--muted);
  font-weight: 500;
  background: var(--panel-2);
  border-radius: 20px;
  padding: 1px 10px;
}
.section-note {
  margin-left: auto;
  font-size: 12px;
  font-weight: 400;
}
.section-head {
  display: flex;
  align-items: center;
  gap: 8px;
}
.section-head .collapse-head {
  flex: 1;
}
.section-action {
  flex-shrink: 0;
  margin-right: 6px;
}
.row-action {
  text-align: right;
  width: 56px;
}
.icon-btn {
  border: none;
  background: transparent;
  color: var(--muted);
  border-radius: 7px;
  width: 26px;
  height: 26px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}
.icon-btn:hover {
  background: var(--red-soft);
  color: var(--red-text);
}
.live-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 11px;
  font-weight: 500;
  color: var(--green-text);
  background: var(--green-soft);
  border-radius: 20px;
  padding: 1px 9px;
  margin-left: 4px;
}
.live-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--green);
  animation: live-pulse 1.6s ease-in-out infinite;
}
@keyframes live-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.3;
  }
}
.collapse-card.open .collapse-chevron {
  transform: rotate(90deg);
}
.num {
  text-align: right;
  white-space: nowrap;
}
.nowrap {
  white-space: nowrap;
}
.path {
  word-break: break-all;
  color: var(--muted);
  font-size: 12px;
}
.detail {
  font-size: 12px;
  word-break: break-all;
}
.status-tag {
  border-radius: 7px;
  padding: 1px 9px;
  font-size: 11px;
  font-weight: 500;
}
.status-tag.ok {
  background: var(--green-soft);
  color: var(--green-text);
}
.status-tag.partial {
  background: var(--amber-soft);
  color: var(--amber-text);
}
.status-tag.failed {
  background: var(--red-soft);
  color: var(--red-text);
}
.kind-tag {
  border-radius: 7px;
  padding: 1px 8px;
  font-size: 11px;
  background: var(--panel-2);
  color: var(--text-2);
  border: 1px solid var(--border);
}
.kind-tag.delete {
  background: var(--red-soft);
  color: var(--red-text);
  border-color: transparent;
}
.kind-tag.create {
  background: var(--green-soft);
  color: var(--green-text);
  border-color: transparent;
}
.error-panel {
  color: var(--red-text);
}
.loading-panel {
  color: var(--muted);
}
</style>
