<script setup lang="ts">
import { onMounted, ref } from "vue";
import { check } from "@tauri-apps/plugin-updater";
import {
  getPrivilegeStatus,
  getSettings,
  relaunchElevated,
  setFloatClickThrough,
  setFloatEnabled,
  setSettings,
  type PrivilegeStatus,
  type Settings,
} from "../api";
import { changeTheme, type ThemeMode } from "../utils/theme";

const settings = ref<Settings | null>(null);
const saving = ref(false);
const savedTip = ref("");
const errorMsg = ref("");

const privilege = ref<PrivilegeStatus | null>(null);
const elevating = ref(false);
const elevateMsg = ref("");

async function elevate() {
  elevating.value = true;
  elevateMsg.value = "正在请求管理员权限，请在系统弹窗中点击“是”…";
  try {
    await relaunchElevated();
  } catch (e) {
    elevateMsg.value = `提权未完成：${String(e)}`;
    elevating.value = false;
  }
}

const intervals = [6, 12, 24, 48];
const thresholds = [5, 10, 15, 20];

async function load() {
  settings.value = await getSettings();
  privilege.value = await getPrivilegeStatus();
}

async function pickTheme(mode: ThemeMode) {
  if (!settings.value) return;
  settings.value.theme = mode;
  await changeTheme(mode);
}

async function toggleFloat(enabled: boolean) {
  try {
    const v = await setFloatEnabled(enabled);
    if (settings.value) settings.value.floatEnabled = v;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function toggleThrough(enabled: boolean) {
  try {
    const v = await setFloatClickThrough(enabled);
    if (settings.value) settings.value.floatClickThrough = v;
  } catch (e) {
    errorMsg.value = String(e);
  }
}

const updateMsg = ref("");
const updating = ref(false);

async function checkUpdate() {
  updating.value = true;
  updateMsg.value = "正在检查更新…";
  try {
    const update = await check();
    if (!update) {
      updateMsg.value = "当前已是最新版本";
      return;
    }
    updateMsg.value = `发现新版本 ${update.version}，正在下载安装…`;
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        updateMsg.value = `正在下载新版本 ${update.version}…`;
      } else if (event.event === "Progress") {
        const mb = (event.data.chunkLength / (1024 * 1024)).toFixed(1);
        updateMsg.value = `正在下载新版本 ${update.version}… 已下载 ${mb} MB`;
      } else if (event.event === "Finished") {
        updateMsg.value = "下载完成，正在安装…";
      }
    });
    updateMsg.value = "更新已安装，请退出并重新启动 CClean";
  } catch (e) {
    updateMsg.value = `检查更新失败：${String(e)}（发布地址未配置或无网络时会出现）`;
  } finally {
    updating.value = false;
  }
}

async function save(updates: Record<string, string>) {
  saving.value = true;
  savedTip.value = "";
  errorMsg.value = "";
  try {
    settings.value = await setSettings(updates);
    savedTip.value = "已保存";
    setTimeout(() => (savedTip.value = ""), 2000);
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    saving.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div v-if="!settings" class="panel loading-panel">加载中…</div>
  <template v-else>
    <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

    <!-- 权限 -->
    <div class="panel">
      <div class="panel-title">权限</div>
      <div v-if="privilege" class="row">
        <div class="label">
          <div class="row-head">
            当前权限
            <span :class="privilege.elevated ? 'badge badge-admin' : 'badge badge-user'">
              {{ privilege.elevated ? "管理员权限运行中" : "标准权限运行中" }}
            </span>
          </div>
          <div class="muted row-desc">
            标准权限下会自动跳过 C:\Windows\Temp、Prefetch、Windows 更新缓存等受保护位置；
            提权后可清理这些位置。提权仅在本次运行有效，重启应用后恢复标准权限
          </div>
        </div>
        <button
          v-if="privilege.canElevate && !privilege.elevated"
          class="btn btn-primary"
          :disabled="elevating"
          @click="elevate"
        >
          {{ elevating ? "等待授权…" : "以管理员身份重启" }}
        </button>
        <span v-else-if="privilege.elevated" class="saved-text">已具备完整清理权限</span>
      </div>
      <div v-if="elevateMsg" class="muted tip">{{ elevateMsg }}</div>
    </div>

    <!-- 外观 -->
    <div class="panel">
      <div class="panel-title">外观</div>
      <div class="row">
        <div class="label">
          <div class="row-head">主题</div>
          <div class="muted row-desc">浅色、深色或跟随系统外观自动切换</div>
        </div>
        <div class="seg theme-seg">
          <button :class="{ active: settings.theme === 'light' }" @click="pickTheme('light')">浅色</button>
          <button :class="{ active: settings.theme === 'dark' }" @click="pickTheme('dark')">深色</button>
          <button :class="{ active: settings.theme === 'system' }" @click="pickTheme('system')">
            跟随系统
          </button>
        </div>
      </div>
    </div>

    <!-- 后台与监控 -->
    <div class="panel">
      <div class="panel-title">后台与监控</div>
      <div class="row">
        <div class="label">
          <div class="row-head">常驻后台（系统托盘）</div>
          <div class="muted row-desc">
            关闭主窗口后最小化到托盘，继续定时快照与低空间监控
          </div>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            :checked="settings.residentEnabled"
            @change="save({ resident_enabled: String(($event.target as HTMLInputElement).checked) })"
          />
          <span></span>
        </label>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">开机自启</div>
          <div class="muted row-desc">登录系统后自动启动 CClean（建议与常驻后台同时开启）</div>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            :checked="settings.autostartEnabled"
            @change="save({ autostart_enabled: String(($event.target as HTMLInputElement).checked) })"
          />
          <span></span>
        </label>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">自动快照间隔</div>
          <div class="muted row-desc">后台静默扫描全部规则并记录占用，用于趋势对比</div>
        </div>
        <select
          :value="settings.snapshotIntervalHours"
          @change="save({ snapshot_interval_hours: String(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="h in intervals" :key="h" :value="h">每 {{ h }} 小时</option>
        </select>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">低空间告警阈值</div>
          <div class="muted row-desc">
            系统盘剩余比例低于该值时发送系统通知（默认每小时最多一次，可在仪表盘提示条暂缓 12 小时）
          </div>
        </div>
        <select
          :value="settings.lowSpaceThresholdPct"
          @change="save({ low_space_threshold_pct: String(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="p in thresholds" :key="p" :value="p">剩余 {{ p }}%</option>
        </select>
      </div>
    </div>

    <!-- 桌面浮窗 -->
    <div class="panel">
      <div class="panel-title">桌面浮窗</div>
      <div class="row">
        <div class="label">
          <div class="row-head">显示浮窗</div>
          <div class="muted row-desc">
            桌面置顶的迷你窗口，实时显示系统盘剩余空间；可拖动，靠近屏幕左右边缘自动吸附
          </div>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            :checked="settings.floatEnabled"
            @change="toggleFloat(($event.target as HTMLInputElement).checked)"
          />
          <span></span>
        </label>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">浮窗鼠标穿透</div>
          <div class="muted row-desc">
            开启后鼠标点击会穿过浮窗（适合只看不动）；恢复请用托盘菜单或本开关
          </div>
        </div>
        <label class="switch">
          <input
            type="checkbox"
            :checked="settings.floatClickThrough"
            @change="toggleThrough(($event.target as HTMLInputElement).checked)"
          />
          <span></span>
        </label>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">默认删除方式</div>
          <div class="muted row-desc">永久删除不可恢复；移入回收站可反悔（清理页仍可逐项切换）</div>
        </div>
        <select
          :value="settings.defaultDeleteMode"
          @change="save({ default_delete_mode: ($event.target as HTMLSelectElement).value })"
        >
          <option value="permanent">永久删除</option>
          <option value="recycle">移入回收站</option>
        </select>
      </div>
    </div>

    <!-- 关于 -->
    <div class="panel">
      <div class="panel-title">关于与更新</div>
      <div class="row">
        <div class="label">
          <div class="row-head">版本更新</div>
          <div class="muted row-desc">
            当前 v0.1.0。更新源在 tauri.conf.json 的 plugins.updater.endpoints 配置，
            发布新版本后可在此一键下载安装
          </div>
        </div>
        <button class="btn btn-primary" :disabled="updating" @click="checkUpdate">
          {{ updating ? "处理中…" : "检查更新" }}
        </button>
      </div>
      <div v-if="updateMsg" class="muted tip">{{ updateMsg }}</div>
      <div v-if="saving" class="muted tip">保存中…</div>
      <div v-else-if="savedTip" class="saved-text tip">✓ {{ savedTip }}</div>
    </div>
  </template>
</template>

<style scoped>
.loading-panel {
  color: var(--muted);
}
.error-panel {
  color: var(--red-text);
}
.row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 20px;
  padding: 14px 0;
  border-bottom: 1px solid var(--border);
}
.row:last-child {
  border-bottom: 0;
}
.row-head {
  font-size: 13.5px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 10px;
}
.row-desc {
  font-size: 12.5px;
  margin-top: 4px;
  line-height: 1.6;
  max-width: 620px;
}
.theme-seg {
  flex-shrink: 0;
}
.saved-text {
  color: var(--green-text);
  font-size: 13px;
}

/* 开关 */
.switch {
  position: relative;
  display: inline-block;
  width: 42px;
  height: 24px;
  flex-shrink: 0;
}
.switch input {
  opacity: 0;
  width: 0;
  height: 0;
}
.switch span {
  position: absolute;
  inset: 0;
  background: var(--border-strong);
  border-radius: 24px;
  transition: 0.2s;
  cursor: pointer;
}
.switch span::before {
  content: "";
  position: absolute;
  width: 18px;
  height: 18px;
  left: 3px;
  top: 3px;
  background: #fff;
  border-radius: 50%;
  transition: 0.2s;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.25);
}
.switch input:checked + span {
  background: var(--primary);
}
.switch input:checked + span::before {
  transform: translateX(18px);
}
.tip {
  padding-top: 12px;
  font-size: 13px;
}

/* 徽章 */
.badge {
  display: inline-block;
  padding: 2px 11px;
  border-radius: 999px;
  font-size: 12px;
  font-weight: 600;
}
.badge-user {
  background: var(--panel-2);
  color: var(--text-2);
  border: 1px solid var(--border);
}
.badge-admin {
  background: var(--green-soft);
  color: var(--green-text);
}
</style>
