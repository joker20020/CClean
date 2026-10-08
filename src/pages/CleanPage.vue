<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import {
  clean,
  getBrewStatus,
  getRules,
  getSystemOverview,
  openPath,
  runBrewCleanup,
  scan,
  type BrewInfo,
  type CleanOutcome,
  type RuleInfo,
  type ScanResult,
} from "../api";
import { CATEGORY_NAMES, formatBytes, RISK_NAMES } from "../utils/format";

const CATEGORY_ORDER = ["system", "browser", "ai", "dev", "apps", "gpu"];
const PREVIEW_COUNT = 6;

const rules = ref<RuleInfo[]>([]);
const results = ref<ScanResult[]>([]);
const selected = reactive<Record<string, boolean>>({});
const expanded = reactive<Record<string, boolean>>({});
const collapsed = reactive<Record<string, boolean>>({});
const showAll = reactive<Record<string, boolean>>({});
const scanning = ref(false);
const cleaning = ref(false);
const outcomes = ref<CleanOutcome[]>([]);
const errorMsg = ref("");

const groups = computed(() => {
  const map: Record<string, RuleInfo[]> = {};
  for (const r of rules.value) {
    (map[r.category] ??= []).push(r);
  }
  return CATEGORY_ORDER.filter((c) => map[c]?.length).map((c) => ({
    category: c,
    name: CATEGORY_NAMES[c] ?? c,
    items: map[c],
  }));
});

const resultMap = computed(() => {
  const m: Record<string, ScanResult> = {};
  for (const r of results.value) m[r.id] = r;
  return m;
});

const totalScannable = computed(() =>
  results.value
    .filter((r) => r.action !== "report")
    .reduce((sum, r) => sum + r.sizeBytes, 0),
);
const totalReport = computed(() =>
  results.value
    .filter((r) => r.action === "report")
    .reduce((sum, r) => sum + r.sizeBytes, 0),
);
const totalFreed = computed(() => outcomes.value.reduce((s, o) => s + o.freedBytes, 0));

function groupScannableSize(category: string) {
  return results.value
    .filter((r) => r.category === category && r.action !== "report")
    .reduce((s, r) => s + r.sizeBytes, 0);
}
/** 扫描结果为 0 个文件的可清理项：不可勾选（显示为未勾选的禁用态） */
function isEmptyResult(id: string): boolean {
  const res = resultMap.value[id];
  return !!res && res.sizeBytes === 0 && res.fileCount === 0;
}
function groupChecked(category: string) {
  return groups.value
    .find((g) => g.category === category)
    ?.items.every(
      (r) =>
        r.action === "report" ||
        !r.available ||
        isEmptyResult(r.id) ||
        selected[r.id],
    );
}
function visibleItems(g: { items: RuleInfo[] }) {
  return showAll[g.items[0]?.category ?? ""] ? g.items : g.items.slice(0, PREVIEW_COUNT);
}

const brew = ref<BrewInfo | null>(null);
const brewing = ref(false);
const brewResult = ref("");

onMounted(async () => {
  try {
    rules.value = await getRules();
    for (const r of rules.value) {
      // 不可用（本机未检测到）的项初始即为未勾选且禁用
      if (r.action !== "report") selected[r.id] = r.available && r.defaultSelected;
    }
    const overview = await getSystemOverview();
    if (overview.platform === "macos") {
      brew.value = await getBrewStatus();
    }
  } catch (e) {
    errorMsg.value = String(e);
  }
});

async function doBrewCleanup() {
  const ok = window.confirm(
    "将执行 brew cleanup -s 清理 Homebrew 旧版本与下载缓存，是否继续？",
  );
  if (!ok) return;
  brewing.value = true;
  try {
    brewResult.value = await runBrewCleanup();
    brew.value = await getBrewStatus();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    brewing.value = false;
  }
}

async function doScan() {
  scanning.value = true;
  errorMsg.value = "";
  outcomes.value = [];
  try {
    const selectedIds = rules.value
      .filter((r) => r.action !== "report" && selected[r.id])
      .map((r) => r.id);
    const reportIds = rules.value.filter((r) => r.action === "report").map((r) => r.id);
    results.value = await scan([...selectedIds, ...reportIds]);
    // 扫描结果为 0 个文件的项自动取消勾选，且不可再手动勾选
    for (const r of results.value) {
      if (r.action !== "report" && r.sizeBytes === 0 && r.fileCount === 0) {
        selected[r.id] = false;
      }
    }
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    scanning.value = false;
  }
}

async function doClean() {
  const targets = results.value.filter(
    (r) => r.action !== "report" && selected[r.id] && r.sizeBytes > 0,
  );
  if (targets.length === 0) return;
  const total = targets.reduce((s, r) => s + r.sizeBytes, 0);
  const ok = window.confirm(
    `将永久删除 ${targets.length} 项缓存，预计释放 ${formatBytes(total)}。\n` +
      "缓存可再生，正在被占用的文件会自动跳过。是否继续？",
  );
  if (!ok) return;
  cleaning.value = true;
  errorMsg.value = "";
  try {
    outcomes.value = await clean(targets.map((r) => ({ ruleId: r.id })));
    await doScan();
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    cleaning.value = false;
  }
}

function toggleGroup(category: string, val: boolean) {
  for (const r of rules.value) {
    if (
      r.category === category &&
      r.action !== "report" &&
      r.available &&
      !isEmptyResult(r.id)
    ) {
      selected[r.id] = val;
    }
  }
}

async function openDir(path: string) {
  try {
    await openPath(path);
  } catch (e) {
    errorMsg.value = String(e);
  }
}

function outcomeOf(ruleId: string): CleanOutcome | undefined {
  return outcomes.value.find((o) => o.ruleId === ruleId);
}
</script>

<template>
  <div class="panel toolbar">
    <div class="toolbar-actions">
      <button class="btn btn-primary" :disabled="scanning" @click="doScan">
        <svg v-if="scanning" class="spin" viewBox="0 0 20 20" width="14" height="14">
          <circle cx="10" cy="10" r="7" fill="none" stroke="currentColor" stroke-width="2.4"
            stroke-dasharray="32 44" stroke-linecap="round" />
        </svg>
        {{ scanning ? "正在扫描…" : "扫描所选项目" }}
      </button>
      <button
        class="btn btn-ghost"
        :disabled="cleaning || scanning || totalScannable === 0"
        @click="doClean"
      >
        {{ cleaning ? "正在清理…" : "清理勾选缓存" }}
      </button>
    </div>
    <div v-if="results.length" class="summary">
      <span class="chip chip-primary">可清理 <b>{{ formatBytes(totalScannable) }}</b></span>
      <span class="chip">只报告 {{ formatBytes(totalReport) }}</span>
      <span v-if="totalFreed > 0" class="chip chip-green">
        本次释放 {{ formatBytes(totalFreed) }}
      </span>
    </div>
  </div>

  <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

  <div v-if="brew?.available" class="panel brew-card">
    <div class="brew-head">
      <div>
        <b>Homebrew 缓存维护</b>
        <div class="muted brew-version">{{ brew.version }}</div>
      </div>
      <button class="btn btn-ghost btn-sm" :disabled="brewing" @click="doBrewCleanup">
        {{ brewing ? "正在清理…" : "执行 brew cleanup -s" }}
      </button>
    </div>
    <pre class="brew-pre">{{ brewResult || brew.dryRun }}</pre>
  </div>

  <div v-for="g in groups" :key="g.category" class="collapse-card" :class="{ open: !collapsed[g.category] }">
    <button class="collapse-head" @click="collapsed[g.category] = !collapsed[g.category]">
      <svg class="collapse-chevron" viewBox="0 0 20 20" width="14" height="14">
        <path d="M8 5l5 5-5 5" fill="none" stroke="currentColor" stroke-width="1.8"
          stroke-linecap="round" stroke-linejoin="round" />
      </svg>
      <span class="group-name">{{ g.name }}</span>
      <span class="group-count">{{ g.items.length }} 项</span>
      <span v-if="groupScannableSize(g.category) > 0" class="group-size">
        {{ formatBytes(groupScannableSize(g.category)) }}
      </span>
      <label class="group-check" @click.stop>
        <input
          type="checkbox"
          :checked="groupChecked(g.category)"
          @change="toggleGroup(g.category, ($event.target as HTMLInputElement).checked)"
        />
        全选
      </label>
    </button>

    <div v-show="!collapsed[g.category]" class="collapse-body">
      <div v-for="r in visibleItems(g)" :key="r.id" class="rule-row">
        <div class="rule-main">
          <input
            v-if="r.action !== 'report'"
            type="checkbox"
            v-model="selected[r.id]"
            :disabled="!r.available || isEmptyResult(r.id)"
          />
          <span v-else class="tag tag-L0">只报告</span>
          <div class="rule-info" @click="expanded[r.id] = !expanded[r.id]">
            <div class="rule-name">
              {{ r.name }}
              <span class="tag" :class="`tag-${r.risk}`">{{ RISK_NAMES[r.risk] ?? r.risk }}</span>
              <svg v-if="resultMap[r.id]" class="rule-chevron" :class="{ up: expanded[r.id] }"
                viewBox="0 0 20 20" width="12" height="12">
                <path d="M5 8l5 5 5-5" fill="none" stroke="currentColor" stroke-width="1.8"
                  stroke-linecap="round" stroke-linejoin="round" />
              </svg>
            </div>
            <div class="rule-desc muted">{{ r.description }}</div>
          </div>
          <div class="rule-size">
            <template v-if="resultMap[r.id]">
              <b :class="{ reportSize: r.action === 'report' }">
                {{ formatBytes(resultMap[r.id].sizeBytes) }}
              </b>
              <span class="muted files">{{ resultMap[r.id].fileCount }} 个文件</span>
              <span v-if="resultMap[r.id].errorCount > 0" class="skipped">
                {{ resultMap[r.id].errorCount }} 项跳过
              </span>
            </template>
            <span v-else class="muted">{{ r.available ? "未扫描" : "未检测到" }}</span>
          </div>
        </div>
        <div v-if="outcomeOf(r.id)" class="outcome">
          已释放 {{ formatBytes(outcomeOf(r.id)!.freedBytes) }} ·
          删除 {{ outcomeOf(r.id)!.deletedFiles }} 个文件
          <span v-if="outcomeOf(r.id)!.errorCount > 0" class="outcome-skip">
            · {{ outcomeOf(r.id)!.errorCount }} 项被占用或无权限已跳过
          </span>
        </div>
        <div v-if="expanded[r.id] && resultMap[r.id]" class="targets">
          <div
            v-for="t in resultMap[r.id].targets.filter((x) => x.exists).slice(0, 20)"
            :key="t.path"
            class="target"
          >
            <span class="target-path" :title="t.path">{{ t.path }}</span>
            <span class="muted target-size">{{ formatBytes(t.sizeBytes) }}</span>
            <button class="link-btn" @click="openDir(t.path)">打开目录</button>
          </div>
          <div
            v-if="resultMap[r.id].targets.filter((x) => x.exists).length === 0"
            class="muted"
          >
            未发现对应目录
          </div>
        </div>
      </div>

      <button
        v-if="g.items.length > PREVIEW_COUNT"
        class="show-more"
        @click="showAll[g.category] = !showAll[g.category]"
      >
        {{ showAll[g.category]
          ? "收起"
          : `展开剩余 ${g.items.length - PREVIEW_COUNT} 项（共 ${g.items.length} 项）` }}
      </button>
    </div>
  </div>

  <div v-if="!groups.length && !errorMsg" class="panel loading-panel">正在加载规则…</div>
</template>

<style scoped>
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
  position: sticky;
  top: 0;
  z-index: 5;
}
.toolbar-actions {
  display: flex;
  gap: 10px;
}
.summary {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.chip {
  font-size: 12.5px;
  padding: 5px 12px;
  border-radius: 20px;
  background: var(--panel-2);
  border: 1px solid var(--border);
  color: var(--text-2);
}
.chip b {
  color: var(--text);
}
.chip-primary {
  background: var(--primary-soft);
  border-color: transparent;
  color: var(--primary-soft-text);
}
.chip-primary b {
  color: var(--primary-soft-text);
}
.chip-green {
  background: var(--green-soft);
  border-color: transparent;
  color: var(--green-text);
}
.error-panel {
  color: var(--red-text);
}
.spin {
  animation: spin 1s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

/* brew */
.brew-card {
  background: var(--purple-soft);
  border-color: transparent;
}
.brew-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.brew-version {
  font-size: 12px;
  margin-top: 2px;
}
.brew-pre {
  margin: 10px 0 0;
  background: var(--code-bg);
  color: var(--code-text);
  border-radius: 9px;
  padding: 10px 12px;
  font-size: 12px;
  font-family: Consolas, monospace;
  white-space: pre-wrap;
  max-height: 180px;
  overflow: auto;
}

/* 分组折叠卡片 */
.collapse-head {
  gap: 10px;
}
.group-name {
  font-size: 14px;
}
.group-count {
  font-size: 12px;
  color: var(--muted);
  font-weight: 500;
  background: var(--panel-2);
  border-radius: 20px;
  padding: 1px 10px;
}
.group-size {
  font-size: 13px;
  font-weight: 700;
  color: var(--primary);
}
.group-check {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--muted);
  font-weight: 500;
}
.collapse-chevron {
  margin-left: 0;
}
.collapse-card.open .collapse-chevron {
  transform: rotate(90deg);
}

/* 规则行 */
.rule-row {
  border-top: 1px solid var(--border);
  padding: 11px 2px;
}
.rule-main {
  display: flex;
  align-items: center;
  gap: 11px;
}
.rule-info {
  flex: 1;
  cursor: pointer;
  min-width: 0;
}
.rule-name {
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13.5px;
}
.rule-chevron {
  color: var(--muted);
  transition: transform 0.18s ease;
}
.rule-chevron.up {
  transform: rotate(180deg);
}
.rule-desc {
  font-size: 12px;
  margin-top: 2px;
  line-height: 1.5;
}
.rule-size {
  min-width: 175px;
  text-align: right;
  font-size: 13px;
  flex-shrink: 0;
}
.reportSize {
  color: var(--muted);
}
.files {
  font-size: 12px;
  margin-left: 8px;
}
.skipped {
  font-size: 12px;
  color: var(--amber-text);
  margin-left: 8px;
}
.outcome {
  margin: 7px 0 0 28px;
  font-size: 12.5px;
  color: var(--green-text);
}
.outcome-skip {
  color: var(--amber-text);
}
.targets {
  margin: 8px 0 2px 28px;
  border-left: 2px solid var(--track);
  padding-left: 12px;
}
.target {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 12px;
  padding: 3px 0;
}
.target-path {
  flex: 1;
  color: var(--text-2);
  word-break: break-all;
}
.target-size {
  white-space: nowrap;
}
.loading-panel {
  color: var(--muted);
}
</style>
