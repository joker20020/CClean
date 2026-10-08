<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import * as echarts from "echarts";
import { listen } from "@tauri-apps/api/event";
import {
  getCleanLogs,
  getFsEvents,
  getTrend,
  takeSnapshot,
  type CleanLog,
  type FsEvent,
  type TrendData,
} from "../api";
import { CATEGORY_NAMES, formatBytes } from "../utils/format";
import { chartColors } from "../utils/theme";

const PAGE_SIZE = 20;

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
      name: CATEGORY_NAMES[c] ?? c,
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
        legend: { data: ["系统盘剩余", "系统盘已用"], top: 0, textStyle: { color: c.muted } },
        xAxis: { type: "category", data: xs, ...axisCommon, splitLine: { show: false } },
        yAxis: { type: "value", name: "GB", nameTextStyle: { color: c.muted }, ...axisCommon },
        series: [
          {
            name: "系统盘剩余",
            type: "line",
            smooth: true,
            areaStyle: { opacity: 0.12 },
            color: c.green,
            symbol: "circle",
            symbolSize: 5,
            data: trend.value.snapshots.map((s) => gb(s.disk_free)),
          },
          {
            name: "系统盘已用",
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

const statusText: Record<string, string> = {
  ok: "成功",
  partial: "部分成功",
  failed: "失败",
};
const kindText: Record<string, string> = {
  create: "新建",
  write: "写入",
  delete: "删除",
  rename: "重命名",
};

function resize() {
  diskChart?.resize();
  catChart?.resize();
}
function onThemeChange() {
  nextTick(renderCharts);
}

onMounted(async () => {
  await load();
  unlisten = await listen("snapshot-taken", () => load());
  window.addEventListener("resize", resize);
  window.addEventListener("theme-change", onThemeChange);
});
onUnmounted(() => {
  unlisten?.();
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
      <b>占用增长趋势</b>
      <div class="muted toolbar-sub">
        快照在每次清理页扫描后/定时任务中记录；至少两个快照才能看出趋势
      </div>
    </div>
    <button class="btn btn-primary" :disabled="taking" @click="snapshotNow">
      {{ taking ? "正在记录…" : "立即记录快照" }}
    </button>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>
  <div v-if="loading" class="panel loading-panel">加载中…</div>

  <div class="panel">
    <div class="chart-title">系统盘容量变化（GB）</div>
    <div ref="diskChartEl" class="chart"></div>
    <div v-if="trend.snapshots.length < 2" class="muted hint">
      当前仅有 {{ trend.snapshots.length }} 个快照，定时快照（默认每日）后自动形成曲线
    </div>
  </div>

  <div class="panel">
    <div class="chart-title">各类缓存占用变化（GB）</div>
    <div ref="catChartEl" class="chart"></div>
  </div>

  <!-- 清理记录（可折叠） -->
  <div class="collapse-card" :class="{ open: sectionsOpen.logs }">
    <button class="collapse-head" @click="sectionsOpen.logs = !sectionsOpen.logs">
      <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
        <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
          stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      清理记录
      <span class="section-count">{{ logs.length }}</span>
    </button>
    <div v-show="sectionsOpen.logs" class="collapse-body">
      <table class="data-table">
        <thead>
          <tr>
            <th>时间</th>
            <th>规则</th>
            <th style="width: 110px">释放空间</th>
            <th style="width: 90px">状态</th>
            <th>备注</th>
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
          </tr>
        </tbody>
      </table>
      <button v-if="logs.length > logShown" class="show-more" @click="logShown += PAGE_SIZE">
        显示更多（还有 {{ logs.length - logShown }} 条）
      </button>
      <div v-if="logs.length === 0" class="muted hint">暂无清理记录</div>
    </div>
  </div>

  <!-- 实时事件（默认折叠） -->
  <div class="collapse-card" :class="{ open: sectionsOpen.events }">
    <button class="collapse-head" @click="sectionsOpen.events = !sectionsOpen.events">
      <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
        <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
          stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      缓存目录实时事件
      <span class="section-count">{{ events.length }}</span>
      <span class="muted section-note">最近 100 条</span>
    </button>
    <div v-show="sectionsOpen.events" class="collapse-body">
      <table class="data-table">
        <thead>
          <tr>
            <th>时间</th>
            <th style="width: 70px">类型</th>
            <th>规则</th>
            <th>路径</th>
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
          </tr>
        </tbody>
      </table>
      <button v-if="events.length > eventShown" class="show-more" @click="eventShown += PAGE_SIZE">
        显示更多（还有 {{ events.length - eventShown }} 条）
      </button>
      <div v-if="events.length === 0" class="muted hint">
        暂无事件（仅监听缓存目录内的新建/写入/删除，2 秒聚合）
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
