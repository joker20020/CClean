<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
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
  if (roots.value.length) {
    const desktop = roots.value[0];
    const home = desktop.replace(/[\\/][^\\/]+$/, "");
    const isWin = home.includes("\\") || /^[A-Za-z]:/.test(home);
    presets.value = [
      { label: "用户目录", path: home },
      { label: isWin ? "C 盘根目录（较慢）" : "根目录（较慢）", path: isWin ? "C:\\" : "/" },
      {
        label: isWin ? "AppData\\Local" : "~/Library",
        path: isWin ? `${home}\\AppData\\Local` : `${home}/Library`,
      },
    ];
  }
  await scanFiles();
  window.addEventListener("resize", onResize);
  window.addEventListener("theme-change", onThemeChange);
});
</script>

<template>
  <div class="panel tabs-panel">
    <div class="seg">
      <button :class="{ active: tab === 'files' }" @click="tab = 'files'">大文件</button>
      <button :class="{ active: tab === 'dirs' }" @click="tab = 'dirs'">目录占用</button>
    </div>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

  <!-- 大文件 -->
  <div v-if="tab === 'files'" class="panel">
    <div class="toolbar-row">
      <div class="seg">
        <button
          v-for="(t, i) in thresholds"
          :key="t.label"
          :class="{ active: thresholdIdx === i }"
          @click="thresholdIdx = i; scanFiles()"
        >
          {{ t.label }}
        </button>
      </div>
      <button class="btn btn-primary btn-sm" :disabled="scanningFiles" @click="scanFiles">
        {{ scanningFiles ? "正在扫描…" : "重新扫描" }}
      </button>
    </div>
    <div class="muted scan-roots">
      扫描范围：{{ roots.join("；") || "默认用户目录" }}
    </div>

    <table class="data-table">
      <thead>
        <tr>
          <th>文件</th>
          <th style="width: 110px">大小</th>
          <th style="width: 110px">修改时间</th>
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
            <button class="link-btn" @click="locateFile(f.path)">定位</button>
          </td>
        </tr>
      </tbody>
    </table>
    <button
      v-if="files.length > fileShown"
      class="show-more"
      @click="fileShown += PAGE_SIZE"
    >
      显示更多（还有 {{ files.length - fileShown }} 项）
    </button>
    <div v-if="!scanningFiles && files.length === 0" class="muted empty">
      未发现超过阈值的文件
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
        正在递归统计（系统盘根目录可能需要数分钟）…
      </div>
      <div v-show="!scanningDirs" ref="treemapEl" class="treemap"></div>
    </div>

    <div class="panel">
      <table class="data-table">
        <thead>
          <tr>
            <th>名称</th>
            <th style="width: 120px">占用</th>
            <th style="width: 110px">文件数</th>
            <th style="width: 80px"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in dirEntries.slice(0, dirShown)" :key="d.path">
            <td class="file-name" :title="d.path">
              {{ d.name }}
              <span class="type-tag">{{ d.isDir ? "目录" : "文件" }}</span>
            </td>
            <td class="num">{{ formatBytes(d.sizeBytes) }}</td>
            <td class="num">{{ d.fileCount.toLocaleString() }}</td>
            <td class="num">
              <button class="link-btn" @click="openDir(d.path)">打开</button>
            </td>
          </tr>
        </tbody>
      </table>
      <button
        v-if="dirEntries.length > dirShown"
        class="show-more"
        @click="dirShown += PAGE_SIZE"
      >
        显示更多（还有 {{ dirEntries.length - dirShown }} 项）
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
