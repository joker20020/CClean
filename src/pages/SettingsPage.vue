<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { check } from "@tauri-apps/plugin-updater";
import { getVersion } from "@tauri-apps/api/app";
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
import { changeLanguage, type LanguagePref } from "../i18n";

const { t } = useI18n();

const settings = ref<Settings | null>(null);
const saving = ref(false);
const savedTip = ref(false);
const errorMsg = ref("");
const appVersion = ref("");

const privilege = ref<PrivilegeStatus | null>(null);
const elevating = ref(false);
const elevateMsg = ref("");

async function elevate() {
  elevating.value = true;
  elevateMsg.value = t("settings.privilege.requesting");
  try {
    await relaunchElevated();
  } catch (e) {
    elevateMsg.value = t("settings.privilege.failed", { msg: String(e) });
    elevating.value = false;
  }
}

const intervals = [6, 12, 24, 48];
const thresholds = [5, 10, 15, 20];

async function load() {
  settings.value = await getSettings();
  privilege.value = await getPrivilegeStatus();
  try {
    appVersion.value = await getVersion();
  } catch {
    // 非 Tauri 环境下忽略
  }
}

async function pickTheme(mode: ThemeMode) {
  if (!settings.value) return;
  settings.value.theme = mode;
  await changeTheme(mode);
}

async function pickLanguage(pref: LanguagePref) {
  if (!settings.value) return;
  settings.value.language = pref;
  await changeLanguage(pref);
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
  updateMsg.value = t("settings.about.checking");
  try {
    const update = await check();
    if (!update) {
      updateMsg.value = t("settings.about.upToDate");
      return;
    }
    updateMsg.value = t("settings.about.found", { version: update.version });
    await update.downloadAndInstall((event) => {
      if (event.event === "Started") {
        updateMsg.value = t("settings.about.downloading", { version: update.version });
      } else if (event.event === "Progress") {
        const mb = (event.data.chunkLength / (1024 * 1024)).toFixed(1);
        updateMsg.value = t("settings.about.downloadingProgress", {
          version: update.version,
          mb,
        });
      } else if (event.event === "Finished") {
        updateMsg.value = t("settings.about.installing");
      }
    });
    updateMsg.value = t("settings.about.installed");
  } catch (e) {
    updateMsg.value = t("settings.about.failed", { msg: String(e) });
  } finally {
    updating.value = false;
  }
}

async function save(updates: Record<string, string>) {
  saving.value = true;
  savedTip.value = false;
  errorMsg.value = "";
  try {
    settings.value = await setSettings(updates);
    savedTip.value = true;
    setTimeout(() => (savedTip.value = false), 2000);
  } catch (e) {
    errorMsg.value = String(e);
  } finally {
    saving.value = false;
  }
}

onMounted(load);
</script>

<template>
  <div v-if="!settings" class="panel loading-panel">{{ t("settings.loading") }}</div>
  <template v-else>
    <div v-if="errorMsg" class="panel error-panel">{{ errorMsg }}</div>

    <!-- 权限 -->
    <div class="panel">
      <div class="panel-title">{{ t("settings.privilege.title") }}</div>
      <div v-if="privilege" class="row">
        <div class="label">
          <div class="row-head">
            {{ t("settings.privilege.current") }}
            <span :class="privilege.elevated ? 'badge badge-admin' : 'badge badge-user'">
              {{ privilege.elevated
                ? t("settings.privilege.admin")
                : t("settings.privilege.user") }}
            </span>
          </div>
          <div class="muted row-desc">
            {{ t("settings.privilege.desc") }}
          </div>
        </div>
        <button
          v-if="privilege.canElevate && !privilege.elevated"
          class="btn btn-primary"
          :disabled="elevating"
          @click="elevate"
        >
          {{ elevating ? t("settings.privilege.waiting") : t("settings.privilege.elevate") }}
        </button>
        <span v-else-if="privilege.elevated" class="saved-text">{{ t("settings.privilege.fullPerms") }}</span>
      </div>
      <div v-if="elevateMsg" class="muted tip">{{ elevateMsg }}</div>
    </div>

    <!-- 外观 -->
    <div class="panel">
      <div class="panel-title">{{ t("settings.appearance.title") }}</div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.appearance.theme") }}</div>
          <div class="muted row-desc">{{ t("settings.appearance.themeDesc") }}</div>
        </div>
        <div class="seg theme-seg">
          <button :class="{ active: settings.theme === 'light' }" @click="pickTheme('light')">{{ t("settings.appearance.light") }}</button>
          <button :class="{ active: settings.theme === 'dark' }" @click="pickTheme('dark')">{{ t("settings.appearance.dark") }}</button>
          <button :class="{ active: settings.theme === 'system' }" @click="pickTheme('system')">
            {{ t("settings.appearance.system") }}
          </button>
        </div>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.appearance.language") }}</div>
          <div class="muted row-desc">{{ t("settings.appearance.languageDesc") }}</div>
        </div>
        <div class="seg theme-seg">
          <button :class="{ active: settings.language === 'zh-CN' }" @click="pickLanguage('zh-CN')">
            {{ t("settings.appearance.langZh") }}
          </button>
          <button :class="{ active: settings.language === 'en-US' }" @click="pickLanguage('en-US')">
            {{ t("settings.appearance.langEn") }}
          </button>
          <button :class="{ active: settings.language === 'system' }" @click="pickLanguage('system')">
            {{ t("settings.appearance.langSystem") }}
          </button>
        </div>
      </div>
    </div>

    <!-- 后台与监控 -->
    <div class="panel">
      <div class="panel-title">{{ t("settings.background.title") }}</div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.background.resident") }}</div>
          <div class="muted row-desc">
            {{ t("settings.background.residentDesc") }}
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
          <div class="row-head">{{ t("settings.background.autostart") }}</div>
          <div class="muted row-desc">{{ t("settings.background.autostartDesc") }}</div>
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
          <div class="row-head">{{ t("settings.background.interval") }}</div>
          <div class="muted row-desc">{{ t("settings.background.intervalDesc") }}</div>
        </div>
        <select
          :value="settings.snapshotIntervalHours"
          @change="save({ snapshot_interval_hours: String(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="h in intervals" :key="h" :value="h">{{ t("settings.background.everyHours", { h }) }}</option>
        </select>
      </div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.background.threshold") }}</div>
          <div class="muted row-desc">
            {{ t("settings.background.thresholdDesc") }}
          </div>
        </div>
        <select
          :value="settings.lowSpaceThresholdPct"
          @change="save({ low_space_threshold_pct: String(($event.target as HTMLSelectElement).value) })"
        >
          <option v-for="p in thresholds" :key="p" :value="p">{{ t("settings.background.thresholdOption", { p }) }}</option>
        </select>
      </div>
    </div>

    <!-- 桌面浮窗 -->
    <div class="panel">
      <div class="panel-title">{{ t("settings.float.title") }}</div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.float.enabled") }}</div>
          <div class="muted row-desc">
            {{ t("settings.float.enabledDesc") }}
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
          <div class="row-head">{{ t("settings.float.through") }}</div>
          <div class="muted row-desc">
            {{ t("settings.float.throughDesc") }}
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
          <div class="row-head">{{ t("settings.float.deleteMode") }}</div>
          <div class="muted row-desc">{{ t("settings.float.deleteModeDesc") }}</div>
        </div>
        <select
          :value="settings.defaultDeleteMode"
          @change="save({ default_delete_mode: ($event.target as HTMLSelectElement).value })"
        >
          <option value="permanent">{{ t("settings.float.permanent") }}</option>
          <option value="recycle">{{ t("settings.float.recycle") }}</option>
        </select>
      </div>
    </div>

    <!-- 关于 -->
    <div class="panel">
      <div class="panel-title">{{ t("settings.about.title") }}</div>
      <div class="row">
        <div class="label">
          <div class="row-head">{{ t("settings.about.version") }}</div>
          <div class="muted row-desc">
            {{ t("settings.about.versionDesc", { version: appVersion }) }}
          </div>
        </div>
        <button class="btn btn-primary" :disabled="updating" @click="checkUpdate">
          {{ updating ? t("settings.about.processing") : t("settings.about.check") }}
        </button>
      </div>
      <div v-if="updateMsg" class="muted tip">{{ updateMsg }}</div>
      <div v-if="saving" class="muted tip">{{ t("common.saving") }}</div>
      <div v-else-if="savedTip" class="saved-text tip">✓ {{ t("common.saved") }}</div>
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
