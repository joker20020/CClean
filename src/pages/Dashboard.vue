<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import * as echarts from "echarts";
import { getSettings, getSystemOverview, snoozeLowSpace, type Overview } from "../api";
import { formatBytes } from "../utils/format";
import { chartColors } from "../utils/theme";

const emit = defineEmits<{ (e: "go-clean"): void }>();

const { t, locale } = useI18n();

const overview = ref<Overview | null>(null);
const errorMsg = ref("");
const gaugeEl = ref<HTMLDivElement | null>(null);
const thresholdPct = ref(10);
const snoozed = ref(false);
let chart: echarts.ECharts | null = null;

const sysDisk = computed(() => (overview.value ? systemDisk(overview.value) : null));
const freePct = computed(() => {
  const d = sysDisk.value;
  return d && d.totalBytes ? (d.availableBytes / d.totalBytes) * 100 : 100;
});
const lowSpace = computed(
  () => !snoozed.value && sysDisk.value !== null && freePct.value < thresholdPct.value,
);

async function snooze12h() {
  try {
    await snoozeLowSpace(12);
    snoozed.value = true;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function systemDisk(o: Overview) {
  if (o.platform === "windows") {
    return o.disks.find((d) => d.mountName.toUpperCase().startsWith("C")) ?? o.disks[0];
  }
  return o.disks.find((d) => d.mountName === "/") ?? o.disks[0];
}

function renderGauge(o: Overview) {
  if (!gaugeEl.value) return;
  const disk = systemDisk(o);
  if (!disk) return;
  if (!chart) chart = echarts.init(gaugeEl.value);
  const c = chartColors();
  const usedPct = disk.totalBytes ? (disk.usedBytes / disk.totalBytes) * 100 : 0;
  chart.setOption(
    {
      series: [
        {
          type: "gauge",
          startAngle: 210,
          endAngle: -30,
          min: 0,
          max: 100,
          radius: "94%",
          progress: { show: true, width: 14, roundCap: true },
          axisLine: { lineStyle: { width: 14, color: [[1, c.track]] } },
          axisTick: { show: false },
          splitLine: { show: false },
          axisLabel: { show: false },
          pointer: { show: false },
          anchor: { show: false },
          itemStyle: { color: c.usedColor(usedPct) },
          title: { show: false },
          detail: {
            valueAnimation: true,
            offsetCenter: ["0", "-5%"],
            formatter: () => `{v|${formatBytes(disk.availableBytes)}}\n{l|${t("common.available")}}`,
            rich: {
              v: { fontSize: 26, fontWeight: 700, color: c.text, lineHeight: 36 },
              l: { fontSize: 13, color: c.muted },
            },
          },
          data: [{ value: Number(usedPct.toFixed(1)) }],
        },
      ],
    },
    { notMerge: true },
  );
}

function usedPct(d: { usedBytes: number; totalBytes: number }) {
  return d.totalBytes ? (d.usedBytes / d.totalBytes) * 100 : 0;
}

function onThemeChange() {
  if (overview.value) renderGauge(overview.value);
}
function onResize() {
  chart?.resize();
}
// 语言切换后重绘仪表盘（中心文字包含本地化文案）
watch(locale, () => {
  if (overview.value) renderGauge(overview.value);
});

onMounted(async () => {
  try {
    overview.value = await getSystemOverview();
    try {
      const s = await getSettings();
      thresholdPct.value = s.lowSpaceThresholdPct;
    } catch {
      // 设置读取失败时使用默认阈值
    }
    renderGauge(overview.value);
    window.addEventListener("resize", onResize);
    window.addEventListener("theme-change", onThemeChange);
  } catch (e) {
    errorMsg.value = String(e);
  }
});
onUnmounted(() => {
  window.removeEventListener("resize", onResize);
  window.removeEventListener("theme-change", onThemeChange);
  chart?.dispose();
});
</script>

<template>
  <div v-if="errorMsg" class="panel">{{ t("dashboard.errorRead", { msg: errorMsg }) }}</div>
  <template v-else>
    <div v-if="lowSpace" class="low-space-banner">
      <div class="banner-text">
        <svg viewBox="0 0 20 20" width="18" height="18" class="banner-icon">
          <path d="M10 2.5L17.8 16H2.2z" fill="none" stroke="currentColor"
            stroke-width="1.6" stroke-linejoin="round" />
          <path d="M10 8v3.6" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
          <circle cx="10" cy="14" r="1" fill="currentColor" />
        </svg>
        <div>
          <b>{{ t("dashboard.banner.title") }}</b>
          {{ t("dashboard.banner.text", {
            mount: sysDisk?.mountName,
            bytes: formatBytes(sysDisk?.availableBytes ?? 0),
            pct: freePct.toFixed(1),
          }) }}
        </div>
      </div>
      <div class="banner-actions">
        <button class="btn btn-primary btn-sm" @click="emit('go-clean')">{{ t("dashboard.banner.goClean") }}</button>
        <button class="btn btn-ghost btn-sm" @click="snooze12h">{{ t("dashboard.banner.snooze") }}</button>
      </div>
    </div>

    <template v-if="overview">
      <div class="top-grid">
        <div class="panel gauge-panel">
          <div class="panel-title">{{ t("dashboard.gauge.title") }}</div>
          <div ref="gaugeEl" class="gauge"></div>
          <div class="gauge-sub muted">
            {{ t("dashboard.gauge.usedInfo", {
              mount: systemDisk(overview)?.mountName,
              used: formatBytes(systemDisk(overview)?.usedBytes ?? 0),
              total: formatBytes(systemDisk(overview)?.totalBytes ?? 0),
            }) }}
          </div>
        </div>

        <div class="panel disks-panel">
          <div class="panel-title">{{ t("dashboard.disks.title") }}</div>
          <div v-for="d in overview.disks" :key="d.mountName" class="disk-row">
            <div class="disk-head">
              <span class="disk-name">
                <span class="disk-dot" :style="{ background: `var(--${usedPct(d) > 85 ? 'red' : usedPct(d) > 70 ? 'amber' : 'primary'})` }"></span>
                {{ d.mountName }}
              </span>
              <span class="muted">{{ t("dashboard.disks.available", { bytes: formatBytes(d.availableBytes) }) }}</span>
            </div>
            <div class="bar">
              <div
                class="bar-fill"
                :style="{
                  width: usedPct(d).toFixed(1) + '%',
                  background:
                    usedPct(d) > 85 ? 'var(--red)' : usedPct(d) > 70 ? 'var(--amber)' : 'var(--primary)',
                }"
              ></div>
            </div>
            <div class="muted disk-sub">
              {{ t("dashboard.disks.usage", {
                used: formatBytes(d.usedBytes),
                total: formatBytes(d.totalBytes),
                fs: d.fileSystem,
              }) }}
            </div>
          </div>
          <div class="panel-footer">
            <span class="muted os-text">
              {{ overview.os }}<span v-if="overview.elevated" class="elevated">{{ t("dashboard.disks.elevated") }}</span>
            </span>
            <button class="btn btn-primary" @click="emit('go-clean')">{{ t("dashboard.disks.startScan") }}</button>
          </div>
        </div>
      </div>

      <div class="panel">
        <div class="panel-title">{{ t("dashboard.promise.title") }}</div>
        <div class="promise-grid">
          <div class="promise-item">
            <span class="promise-icon ic-green">
              <svg viewBox="0 0 20 20" width="16" height="16"><path d="M5 10.5l3.2 3.2L15 7" fill="none"
                stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" /></svg>
            </span>
            <div>{{ t("dashboard.promise.p1") }}</div>
          </div>
          <div class="promise-item">
            <span class="promise-icon ic-purple">
              <svg viewBox="0 0 20 20" width="16" height="16">
                <path d="M10 3l5.5 2v4c0 3.6-2.4 6.3-5.5 7.4C6.9 15.3 4.5 12.6 4.5 9V5z"
                  fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
              </svg>
            </span>
            <div>{{ t("dashboard.promise.p2") }}</div>
          </div>
          <div class="promise-item">
            <span class="promise-icon ic-blue">
              <svg viewBox="0 0 20 20" width="16" height="16">
                <path d="M4 10l3.2-5h5.6L16 10v4.5a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 4 14.5z"
                  fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
                <circle cx="10" cy="12" r="1.6" fill="currentColor" />
              </svg>
            </span>
            <div>{{ t("dashboard.promise.p3") }}</div>
          </div>
          <div class="promise-item">
            <span class="promise-icon ic-amber">
              <svg viewBox="0 0 20 20" width="16" height="16">
                <path d="M4 8h12v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2z" fill="none"
                  stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
                <path d="M7 8V6a3 3 0 0 1 6 0v2" fill="none" stroke="currentColor"
                  stroke-width="1.4" stroke-linecap="round" />
              </svg>
            </span>
            <div>{{ t("dashboard.promise.p4") }}</div>
          </div>
        </div>
      </div>
    </template>
    <div v-else class="panel loading-panel">{{ t("dashboard.loading") }}</div>
  </template>
</template>

<style scoped>
.low-space-banner {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  background: var(--red-soft);
  border: 1px solid transparent;
  color: var(--red-text);
  border-radius: 12px;
  padding: 12px 16px;
  margin-bottom: 16px;
}
.banner-text {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13.5px;
  min-width: 0;
  flex: 1;
}
.banner-text > div {
  min-width: 0;
}
.banner-icon {
  flex-shrink: 0;
}
.banner-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.top-grid {
  display: grid;
  grid-template-columns: 340px 1fr;
  gap: 16px;
}
.gauge-panel {
  display: flex;
  flex-direction: column;
}
.gauge {
  height: 210px;
}
.gauge-sub {
  text-align: center;
  font-size: 12.5px;
}
.disk-row {
  margin-bottom: 16px;
}
.disk-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}
.disk-name {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 7px;
}
.disk-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}
.disk-sub {
  font-size: 12px;
  margin-top: 5px;
}
.panel-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 14px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}
.os-text {
  font-size: 12.5px;
}
.elevated {
  color: var(--green-text);
}
.promise-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px 20px;
}
.promise-item {
  display: flex;
  align-items: flex-start;
  gap: 11px;
  color: var(--text-2);
  line-height: 1.6;
  font-size: 13px;
}
.promise-icon {
  width: 30px;
  height: 30px;
  border-radius: 9px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.ic-green {
  background: var(--green-soft);
  color: var(--green-text);
}
.ic-purple {
  background: var(--purple-soft);
  color: var(--purple-text);
}
.ic-blue {
  background: var(--primary-soft);
  color: var(--primary-soft-text);
}
.ic-amber {
  background: var(--amber-soft);
  color: var(--amber-text);
}
.loading-panel {
  color: var(--muted);
}
</style>
