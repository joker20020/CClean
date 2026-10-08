<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";
import { LogicalPosition } from "@tauri-apps/api/dpi";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import {
  clean,
  getRules,
  getSystemOverview,
  scan,
  setSettings,
  showMainWindow,
  type Overview,
} from "./api";
import { formatBytes } from "./utils/format";

const win = getCurrentWindow();
const overview = ref<Overview | null>(null);

// idle：常态；confirm：等待清理确认；running：清理中；result：展示结果
const phase = ref<"idle" | "confirm" | "running" | "result">("idle");
const resultText = ref("");

const SNAP_GAP = 24; // 距屏幕边缘多少像素内触发吸附
let moveTimer: number | undefined;
let unlistenMoved: UnlistenFn | null = null;
let refreshTimer: number | undefined;
let phaseTimer: number | undefined;

function systemDisk(o: Overview) {
  if (o.platform === "windows") {
    return o.disks.find((d) => d.mountName.toUpperCase().startsWith("C")) ?? o.disks[0];
  }
  return o.disks.find((d) => d.mountName === "/") ?? o.disks[0];
}

async function refresh() {
  try {
    overview.value = await getSystemOverview();
  } catch {
    // 浮窗静默
  }
}

// 拖动停止（300ms 内无新 move 事件）后吸附到最近的左右边缘，并持久化位置
async function onMoved() {
  window.clearTimeout(moveTimer);
  moveTimer = window.setTimeout(async () => {
    try {
      const monitor = await currentMonitor();
      const pos = await win.outerPosition();
      const size = await win.outerSize();
      if (!monitor) return;
      const { width: mw, height: mh } = monitor.size;
      const { x: mox, y: moy } = monitor.position;
      const scale = monitor.scaleFactor;
      // 物理像素 → 逻辑像素
      const x = pos.x / scale;
      const y = pos.y / scale;
      const w = size.width / scale;
      const h = size.height / scale;
      const top = moy / scale;
      const left = mox / scale;
      const right = (mox + mw) / scale;
      const bottom = (moy + mh) / scale;
      let nx = x;
      const ny = Math.max(top + 8, Math.min(y, bottom - h - 8));
      if (x - left < SNAP_GAP) nx = left + 8;
      else if (right - (x + w) < SNAP_GAP) nx = right - w - 8;
      if (nx !== x || ny !== y) {
        await win.setPosition(new LogicalPosition(nx, ny));
      }
      const p = await win.outerPosition();
      await setSettings({ float_x: String(Math.round(p.x / scale)), float_y: String(Math.round(p.y / scale)) });
    } catch {
      // 忽略
    }
  }, 300);
}

function backToIdle(delay = 0) {
  window.clearTimeout(phaseTimer);
  phaseTimer = window.setTimeout(() => {
    phase.value = "idle";
    resultText.value = "";
  }, delay);
}

// 点击扫帚：进入二次确认（4 秒不确认自动取消，防止误触）
function askQuickClean() {
  if (phase.value !== "idle") return;
  phase.value = "confirm";
  backToIdle(4000);
}

function cancelQuickClean() {
  window.clearTimeout(phaseTimer);
  phase.value = "idle";
}

// 执行一键清理：扫描所有默认可安全清理的规则并直接清理
async function doQuickClean() {
  window.clearTimeout(phaseTimer);
  phase.value = "running";
  try {
    const rules = await getRules();
    const ids = rules
      .filter((r) => r.available && r.defaultSelected && r.action !== "report")
      .map((r) => r.id);
    if (ids.length === 0) {
      resultText.value = "暂无可清理项";
      phase.value = "result";
      backToIdle(3000);
      return;
    }
    const scanned = await scan(ids);
    const found = scanned.reduce((s, r) => s + r.sizeBytes, 0);
    if (found <= 0) {
      resultText.value = "已很干净";
      phase.value = "result";
      backToIdle(3000);
      return;
    }
    const outcomes = await clean(ids.map((id) => ({ ruleId: id })));
    const freed = outcomes.reduce((s, o) => s + o.freedBytes, 0);
    resultText.value = freed > 0 ? `已释放 ${formatBytes(freed)}` : "清理完成";
    phase.value = "result";
    await refresh();
    backToIdle(3500);
  } catch (e) {
    resultText.value = "清理失败，请在主窗口查看";
    phase.value = "result";
    backToIdle(3500);
  }
}

async function openMain() {
  try {
    await showMainWindow();
  } catch {
    // 忽略
  }
}

async function hideFloat() {
  await win.hide();
  await setSettings({ float_enabled: "false" });
}

onMounted(async () => {
  await refresh();
  refreshTimer = window.setInterval(refresh, 60_000);
  unlistenMoved = await listen("tauri://move", onMoved);
});
onUnmounted(() => {
  window.clearInterval(refreshTimer);
  window.clearTimeout(moveTimer);
  window.clearTimeout(phaseTimer);
  unlistenMoved?.();
});
</script>

<template>
  <div class="float">
    <div class="drag" data-tauri-drag-region>
      <template v-if="phase === 'idle' && overview">
        <div class="free" data-tauri-drag-region>
          {{ formatBytes(systemDisk(overview)?.availableBytes ?? 0) }}
          <span class="free-label">可用</span>
        </div>
        <div class="bar" data-tauri-drag-region>
          <div
            class="fill"
            data-tauri-drag-region
            :style="{
              width:
                ((systemDisk(overview)?.usedBytes ?? 0) /
                  (systemDisk(overview)?.totalBytes ?? 1)) *
                  100 +
                '%',
            }"
          ></div>
        </div>
      </template>
      <div v-else-if="phase === 'confirm'" class="msg msg-warn">清理推荐缓存？</div>
      <div v-else-if="phase === 'running'" class="msg">
        <span class="spin"></span>清理中…
      </div>
      <div v-else-if="phase === 'result'" class="msg msg-ok">{{ resultText }}</div>
      <div v-else class="free">—</div>
    </div>

    <div class="actions">
      <template v-if="phase === 'confirm'">
        <button class="ab ab-ok" title="确认清理" @click="doQuickClean">
          <svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3,8.5 6.5,12 13,4.5" /></svg>
        </button>
        <button class="ab" title="取消" @click="cancelQuickClean">
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="4" y1="4" x2="12" y2="12" /><line x1="12" y1="4" x2="4" y2="12" /></svg>
        </button>
      </template>
      <template v-else>
        <button class="ab" title="一键清理推荐缓存" :disabled="phase !== 'idle'" @click="askQuickClean">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M19 4l-7 7" /><path d="M14 4h5v5" /><path d="M12 11l-3.5 6.5a2.5 2.5 0 0 1-3.4 1l-.6-.3a1 1 0 0 1-.4-1.4L8.5 9" /><path d="M5 15.5l4-2.8 2.3 2.3-2.8 4" /></svg>
        </button>
        <button class="ab" title="打开主窗口" @click="openMain">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="13" rx="2" /><line x1="8" y1="21" x2="16" y2="21" /><line x1="12" y1="17" x2="12" y2="21" /></svg>
        </button>
        <button class="ab" title="隐藏浮窗（可在设置或托盘菜单中重新开启）" @click="hideFloat">
          <svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="4" y1="4" x2="12" y2="12" /><line x1="12" y1="4" x2="4" y2="12" /></svg>
        </button>
      </template>
    </div>
  </div>
</template>

<style scoped>
.float {
  display: flex;
  align-items: center;
  gap: 4px;
  height: 56px;
  padding: 0 8px 0 10px;
  border-radius: 28px;
  background: var(--float-bg);
  border: 1px solid var(--float-border);
  color: var(--float-text);
  backdrop-filter: blur(16px) saturate(1.3);
  -webkit-backdrop-filter: blur(16px) saturate(1.3);
  /* 投影范围必须小于窗口与胶囊之间的留白（窗口 200×64，胶囊约 180×56），
     否则被窗口裁切掉的阴影会在桌面形成灰边 */
  box-shadow: var(--float-shadow);
  user-select: none;
  -webkit-user-select: none;
  transition: background-color 0.25s ease, color 0.25s ease;
}
.drag {
  cursor: grab;
  min-width: 82px;
  display: flex;
  flex-direction: column;
  justify-content: center;
}
.drag:active {
  cursor: grabbing;
}
.free {
  font-size: 15px;
  font-weight: 700;
  white-space: nowrap;
}
.free-label {
  font-size: 10px;
  font-weight: 400;
  color: var(--float-muted);
  margin-left: 2px;
}
.bar {
  margin-top: 5px;
  height: 4px;
  width: 82px;
  border-radius: 2px;
  background: var(--float-track);
  overflow: hidden;
}
.fill {
  height: 4px;
  border-radius: 2px;
  background: linear-gradient(90deg, #22c55e, #f59e0b, #ef4444);
}
.msg {
  font-size: 12px;
  font-weight: 600;
  white-space: nowrap;
  display: flex;
  align-items: center;
  gap: 6px;
}
.msg-warn {
  color: #f59e0b;
}
.msg-ok {
  color: #22c55e;
}
.spin {
  width: 11px;
  height: 11px;
  border: 2px solid var(--float-track);
  border-top-color: var(--primary);
  border-radius: 50%;
  display: inline-block;
  animation: spin 0.7s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
.actions {
  display: flex;
  gap: 2px;
  padding-left: 4px;
  border-left: 1px solid var(--float-border);
  margin-left: 2px;
}
.ab {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 0;
  background: var(--float-btn);
  color: var(--float-text);
  cursor: pointer;
  font-size: 12px;
  line-height: 1;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s ease;
}
.ab:hover {
  background: var(--float-btn-hover);
}
.ab:disabled {
  opacity: 0.5;
  cursor: default;
}
.ab-ok {
  background: rgba(34, 197, 94, 0.22);
  color: #16a34a;
}
:root[data-theme="dark"] .ab-ok {
  color: #6ee7b7;
}
.ab-ok:hover {
  background: rgba(34, 197, 94, 0.36);
}
</style>
