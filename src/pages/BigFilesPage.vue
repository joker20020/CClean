<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import * as echarts from "echarts";
import {
  defaultRoots,
  listBigFiles,
  listDirSizes,
  openPath,
  revealItem,
  type BigFile,
  type DirEntry,
} from "../api";
import { formatBytes } from "../utils/format";
import { chartColors } from "../utils/theme";

const { t, locale } = useI18n();
const PAGE_SIZE = 30;
const tab = ref<"files" | "dirs">("files");

// 大文件
const thresholds = [
  { label: "≥ 100 MB", bytes: 100 * 1024 * 1024 },
  { label: "≥ 500 MB", bytes: 500 * 1024 * 1024 },
  { label: "≥ 1 GB", bytes: 1024 * 1024 * 1024 },
];
const thresholdIdx = ref(1);
const roots = ref<string[]>([]);
const files = ref<BigFile[]>([]);
const fileShown = ref(PAGE_SIZE);
const scanningFiles = ref(false);

// 目录占用
const presets = ref<{ label: string; path: string }[]>([]);
const currentRoot = ref("");
const dirEntries = ref<DirEntry[]>([]);
const dirShown = ref(PAGE_SIZE);
const scanningDirs = ref(false);
const treemapEl = ref<HTMLDivElement | null>(null);
let chart: echarts.ECharts | null = null;

const errorMsg = ref("");

async function scanFiles() {
  scanningFiles.value = true;
  errorMsg.value = "";
  fileShown.value = PAGE_SIZE;
  try {
    files.value = await listBigFiles(null, thresholds[thresholdIdx.value].bytes, 200);
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    scanningFiles.value = false;
  }
}

async function scanDirs(path: string) {
  currentRoot.value = path;
  scanningDirs.value = true;
  errorMsg.value = "";
  dirShown.value = PAGE_SIZE;
  try {
    dirEntries.value = (await listDirSizes(path)).sort((a, b) => b.sizeBytes - a.sizeBytes);
    renderTreemap();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    scanningDirs.value = false;
  }
}

function renderTreemap() {
  if (!treemapEl.value) return;
  if (!chart) chart = echarts.init(treemapEl.value);
  const c = chartColors();
  const data = dirEntries.value
    .filter((d) => d.sizeBytes > 0)
    .map((d) => ({ name: d.name, value: d.sizeBytes, path: d.path }));
  chart.setOption(
    {
      tooltip: {
        formatter: (info: { data: { name: string; value: number } }) =>
          `${info.data.name}<br/>${formatBytes(info.data.value)}`,
      },
      series: [
        {
          type: "treemap",
          roam: false,
          nodeClick: false,
          breadcrumb: { show: false },
          label: {
            formatter: (p: { name: string; value: number }) =>
              `${p.name}\n${formatBytes(p.value ?? 0)}`,
            fontSize: 11,
            color: "#fff",
          },
          upperLabel: { show: false },
          itemStyle: { borderColor: c.dark ? "#141d33" : "#fff", borderWidth: 2, gapWidth: 2 },
          color: c.palette,
          data,
        },
      ],
    },
    { notMerge: true },
  );
}

function fmtDate(ts: number | null): string {
  if (!ts) return "-";
  return new Date(ts * 1000).toLocaleDateString();
}

async function openDir(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function locateFile(path: string) {
  try {
    await revealItem(path);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function onThemeChange() {
  if (tab.value === "dirs" && dirEntries.value.length) renderTreemap();
}
function onResize() {
  chart?.resize();
}

watch(tab, (v) => {
  if (v === "dirs" && dirEntries.value.length === 0 && presets.value[1]) {
    scanDirs(presets.value[1].path);
  }
});

onMounted(async () => {
  roots.value = await defaultRoots();
  const buildPresets = () => {
    if (!roots.value.length) return;
    const desktop = roots.value[0];
    const home = desktop.replace(/[\\/][^\\/]+$/, "");
    const isWin = home.includes("\\") || /^[A-Za-z]:/.test(home);
    presets.value = [
      { label: t("bigfiles.preset.userHome"), path: home },
      {
        label: t(isWin ? "bigfiles.preset.cRootSlow" : "bigfiles.preset.rootSlow"),
        path: isWin ? "C:\\" : "/",
      },
      {
        label: t(isWin ? "bigfiles.preset.appdataLocal" : "bigfiles.preset.library"),
        path: isWin ? `${home}\\AppData\\Local` : `${home}/Library`,
      },
    ];
  };
  buildPresets();
  watch(locale, buildPresets);
  await scanFiles();
  window.addEventListener("resize", onResize);
  window.addEventListener("theme-change", onThemeChange);
});
</script>

<template>
  <div class="panel tabs-panel">
    <div class="seg">
      <button :class="{ active: tab === 'files' }" @click="tab = 'files'">{{ t("bigfiles.tab.files") }}</button>
      <button :class="{ active: tab === 'dirs' }" @click="tab = 'dirs'">{{ t("bigfiles.tab.dirs") }}</button>
    </div>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

  <!-- 大文件 -->
  <div v-if="tab === 'files'" class="panel">
    <div class="toolbar-row">
      <div class="seg">
        <button
          v-for="(th, i) in thresholds"
          :key="th.label"
          :class="{ active: thresholdIdx === i }"
          @click="thresholdIdx = i; scanFiles()"
        >
          {{ th.label }}
        </button>
      </div>
      <button class="btn btn-primary btn-sm" :disabled="scanningFiles" @click="scanFiles">
        {{ scanningFiles ? t("bigfiles.scanning") : t("bigfiles.rescan") }}
      </button>
    </div>
    <div class="muted scan-roots">
      {{ t("bigfiles.scanRange", { roots: roots.join("；") || t("bigfiles.scanRangeDefault") }) }}
    </div>

    <table class="data-table">
      <thead>
        <tr>
          <th>{{ t("bigfiles.table.file") }}</th>
          <th style="width: 110px">{{ t("bigfiles.table.size") }}</th>
          <th style="width: 110px">{{ t("bigfiles.table.modified") }}</th>
          <th style="width: 80px"></th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="f in files.slice(0, fileShown)" :key="f.path">
          <td class="file-name" :title="f.path">
            {{ f.name }}
            <div class="muted path">{{ f.path }}</div>
          </td>
          <td class="num">{{ formatBytes(f.sizeBytes) }}</td>
          <td class="num">{{ fmtDate(f.modifiedTs) }}</td>
          <td class="num">
            <button class="link-btn" @click="locateFile(f.path)">{{ t("common.locate") }}</button>
          </td>
        </tr>
      </tbody>
    </table>
    <button
      v-if="files.length > fileShown"
      class="show-more"
      @click="fileShown += PAGE_SIZE"
    >
      {{ t("common.showMore", { n: files.length - fileShown }) }}
    </button>
    <div v-if="!scanningFiles && files.length === 0" class="muted empty">
      {{ t("bigfiles.empty") }}
    </div>
  </div>

  <!-- 目录占用 -->
  <template v-else>
    <div class="panel">
      <div class="toolbar-row">
        <div class="seg">
          <button
            v-for="p in presets"
            :key="p.path"
            :class="{ active: currentRoot === p.path }"
            @click="scanDirs(p.path)"
          >
            {{ p.label }}
          </button>
        </div>
        <span class="muted root-path">{{ currentRoot }}</span>
      </div>
      <div v-if="scanningDirs" class="muted scanning-hint">
        {{ t("bigfiles.scanningDirs") }}
      </div>
      <div v-show="!scanningDirs" ref="treemapEl" class="treemap"></div>
    </div>

    <div class="panel">
      <table class="data-table">
        <thead>
          <tr>
            <th>{{ t("bigfiles.table.name") }}</th>
            <th style="width: 120px">{{ t("bigfiles.table.size") }}</th>
            <th style="width: 110px">{{ t("bigfiles.table.fileCount") }}</th>
            <th style="width: 80px"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in dirEntries.slice(0, dirShown)" :key="d.path">
            <td class="file-name" :title="d.path">
              {{ d.name }}
              <span class="type-tag">{{ d.isDir ? t("bigfiles.table.typeDir") : t("bigfiles.table.typeFile") }}</span>
            </td>
            <td class="num">{{ formatBytes(d.sizeBytes) }}</td>
            <td class="num">{{ d.fileCount.toLocaleString() }}</td>
            <td class="num">
              <button class="link-btn" @click="openDir(d.path)">{{ t("common.open") }}</button>
            </td>
          </tr>
        </tbody>
      </table>
      <button
        v-if="dirEntries.length > dirShown"
        class="show-more"
        @click="dirShown += PAGE_SIZE"
      >
        {{ t("common.showMore", { n: dirEntries.length - dirShown }) }}
      </button>
    </div>
  </template>
</template>

<style scoped>
.tabs-panel {
  padding: 10px 12px;
}
.toolbar-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}
.scan-roots {
  font-size: 12px;
  margin: 12px 0;
  word-break: break-all;
}
.root-path {
  font-size: 12px;
  word-break: break-all;
  text-align: right;
}
.num {
  text-align: right;
  white-space: nowrap;
}
.file-name {
  word-break: break-all;
}
.path {
  font-size: 11px;
  margin-top: 1px;
}
.type-tag {
  font-size: 10px;
  background: var(--panel-2);
  color: var(--muted);
  border: 1px solid var(--border);
  border-radius: 7px;
  padding: 0 7px;
  margin-left: 6px;
}
.empty {
  padding: 18px 0 4px;
  text-align: center;
}
.scanning-hint {
  padding: 16px 0;
  text-align: center;
}
.treemap {
  height: 380px;
  margin-top: 12px;
}
.error-panel {
  color: var(--red-text);
}
</style>
