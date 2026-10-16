<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import Dashboard from "./pages/Dashboard.vue";
import CleanPage from "./pages/CleanPage.vue";
import AiPage from "./pages/AiPage.vue";
import BigFilesPage from "./pages/BigFilesPage.vue";
import TrendPage from "./pages/TrendPage.vue";
import RulesPage from "./pages/RulesPage.vue";
import SettingsPage from "./pages/SettingsPage.vue";
import { changeTheme, getThemeMode, type ThemeMode } from "./utils/theme";
import ConfirmModal from "./components/ConfirmModal.vue";
import { confirmDialog } from "./components/confirm";

const { t } = useI18n();

type PageKey = "dashboard" | "clean" | "ai" | "bigfiles" | "trend" | "rules" | "settings";

const page = ref<PageKey>("dashboard");
const themeMode = ref<ThemeMode>("system");
const appVersion = ref("");

const navItems: { key: PageKey; labelKey: string; icon: string }[] = [
  { key: "dashboard", labelKey: "app.nav.dashboard", icon: "dashboard" },
  { key: "clean", labelKey: "app.nav.clean", icon: "broom" },
  { key: "ai", labelKey: "app.nav.ai", icon: "sparkles" },
  { key: "bigfiles", labelKey: "app.nav.bigfiles", icon: "folder" },
  { key: "trend", labelKey: "app.nav.trend", icon: "chart" },
  { key: "rules", labelKey: "app.nav.rules", icon: "shield" },
  { key: "settings", labelKey: "app.nav.settings", icon: "gear" },
];

function goClean() {
  page.value = "clean";
}

async function pickTheme(mode: ThemeMode) {
  themeMode.value = mode;
  await changeTheme(mode);
}

onMounted(async () => {
  // 低空间通知（由后端常驻线程触发）
  await listen<{ availableBytes: number; thresholdPct: number }>("low-space-alert", async (e) => {
    const gb = (e.payload.availableBytes / 1024 ** 3).toFixed(1);
    const go = await confirmDialog({
      title: t("app.lowSpace.title"),
      message: t("app.lowSpace.confirm", { gb, pct: e.payload.thresholdPct }),
      confirmText: t("app.lowSpace.goClean"),
      cancelText: t("app.lowSpace.snooze"),
    });
    if (go) {
      page.value = "clean";
    } else {
      invoke("snooze_low_space", { hours: 12 }).catch(() => {});
    }
  });

  // 浮窗开关/穿透状态由后端 "float-state-changed" 事件广播，
  // 设置页直接监听该事件同步控件，无需在此中转。

  themeMode.value = getThemeMode();
  try {
    appVersion.value = await getVersion();
  } catch {
    // 非 Tauri 环境下忽略
  }
});
</script>

<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand">
        <span class="brand-logo" aria-hidden="true">
          <svg viewBox="0 0 32 32" width="22" height="22">
            <circle cx="16" cy="16" r="11" fill="none" stroke="#ffffff" stroke-width="3.4"
              stroke-linecap="round" stroke-dasharray="52 69" transform="rotate(-90 16 16)" />
            <circle cx="23.5" cy="22" r="3.4" fill="#34d399" />
          </svg>
        </span>
        <div class="brand-text">
          <div class="brand-name">CClean</div>
          <div class="brand-sub">{{ t("app.brand.subtitle") }}</div>
        </div>
      </div>

      <nav class="nav">
        <button
          v-for="item in navItems"
          :key="item.key"
          class="nav-item"
          :class="{ active: page === item.key }"
          @click="page = item.key"
        >
          <svg class="nav-icon" viewBox="0 0 20 20" width="17" height="17" aria-hidden="true">
            <!-- 仪表盘 -->
            <template v-if="item.icon === 'dashboard'">
              <path d="M3 11a7 7 0 1 1 14 0" fill="none" stroke="currentColor" stroke-width="1.6"
                stroke-linecap="round" />
              <path d="M10 11l3.2-3.2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
              <circle cx="10" cy="11" r="1.3" fill="currentColor" />
            </template>
            <!-- 扫帚 -->
            <template v-else-if="item.icon === 'broom'">
              <path d="M12.5 2.5l5 5" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
              <path d="M11.2 4.2l4.6 4.6-2.2 2.2-4.6-4.6z" fill="none" stroke="currentColor"
                stroke-width="1.4" />
              <path d="M9 6.4L4.6 10.8c-1 1-1.3 2.4-.9 3.7l.5 1.5-1.4 1.5 1.6 1.6 1.5-1.4 1.5.5c1.3.4 2.7.1 3.7-.9l4.4-4.4"
                fill="none" stroke="currentColor" stroke-width="1.4" stroke-linejoin="round" />
            </template>
            <!-- AI 闪光 -->
            <template v-else-if="item.icon === 'sparkles'">
              <path d="M10 3l1.6 4.4L16 9l-4.4 1.6L10 15l-1.6-4.4L4 9l4.4-1.6z" fill="currentColor" />
              <path d="M15 12l.8 2.2L18 15l-2.2.8L15 18l-.8-2.2L12 15l2.2-.8z" fill="currentColor" />
            </template>
            <!-- 文件夹 -->
            <template v-else-if="item.icon === 'folder'">
              <path d="M3 6.5A1.5 1.5 0 0 1 4.5 5h3.2l1.6 1.8h6.2A1.5 1.5 0 0 1 17 8.3v6.2a1.5 1.5 0 0 1-1.5 1.5h-11A1.5 1.5 0 0 1 3 14.5z"
                fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
            </template>
            <!-- 趋势 -->
            <template v-else-if="item.icon === 'chart'">
              <path d="M3 15l4-4 3 2.5L16 6" fill="none" stroke="currentColor" stroke-width="1.6"
                stroke-linecap="round" stroke-linejoin="round" />
              <path d="M13 6h3v3" fill="none" stroke="currentColor" stroke-width="1.6"
                stroke-linecap="round" stroke-linejoin="round" />
            </template>
            <!-- 盾牌 -->
            <template v-else-if="item.icon === 'shield'">
              <path d="M10 2.8l5.5 2v4c0 3.6-2.4 6.3-5.5 7.4-3.1-1.1-5.5-3.8-5.5-7.4v-4z"
                fill="none" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round" />
              <path d="M7.6 10l1.7 1.7 3.2-3.4" fill="none" stroke="currentColor" stroke-width="1.5"
                stroke-linecap="round" stroke-linejoin="round" />
            </template>
            <!-- 齿轮 -->
            <template v-else-if="item.icon === 'gear'">
              <circle cx="10" cy="10" r="2.6" fill="none" stroke="currentColor" stroke-width="1.5" />
              <path d="M10 3.5v1.8M10 14.7v1.8M15.4 4.6l-1.3 1.3M5.9 14.1l-1.3 1.3M16.5 10h-1.8M5.3 10H3.5M15.4 15.4l-1.3-1.3M5.9 5.9L4.6 4.6"
                stroke="currentColor" stroke-width="1.5" stroke-linecap="round" />
            </template>
          </svg>
          <span>{{ t(item.labelKey) }}</span>
        </button>
      </nav>

      <div class="sidebar-footer">
        <div class="theme-switch" role="group" :title="t('app.themeSwitch.title')">
          <button
            v-for="btn in [
              { k: 'light', tk: 'app.themeSwitch.light' },
              { k: 'dark', tk: 'app.themeSwitch.dark' },
              { k: 'system', tk: 'app.themeSwitch.system' },
            ]"
            :key="btn.k"
            :class="{ active: themeMode === btn.k }"
            :title="t(btn.tk)"
            @click="pickTheme(btn.k as ThemeMode)"
          >
            <svg v-if="btn.k === 'light'" viewBox="0 0 20 20" width="15" height="15">
              <circle cx="10" cy="10" r="3.4" fill="none" stroke="currentColor" stroke-width="1.6" />
              <path d="M10 2.5v1.8M10 15.7v1.8M17.5 10h-1.8M4.3 10H2.5M15.3 4.7l-1.3 1.3M6 14l-1.3 1.3M15.3 15.3L14 14M6 6L4.7 4.7"
                stroke="currentColor" stroke-width="1.6" stroke-linecap="round" />
            </svg>
            <svg v-else-if="btn.k === 'dark'" viewBox="0 0 20 20" width="15" height="15">
              <path d="M15.4 11.6A6.2 6.2 0 0 1 8.4 4.6a6.2 6.2 0 1 0 7 7z" fill="currentColor" />
            </svg>
            <svg v-else viewBox="0 0 20 20" width="15" height="15">
              <rect x="2.5" y="4" width="15" height="12" rx="2" fill="none"
                stroke="currentColor" stroke-width="1.5" />
              <path d="M10 4v12" stroke="currentColor" stroke-width="1.5" />
              <path d="M10 4h5.5a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H10z" fill="currentColor" />
            </svg>
          </button>
        </div>
        <div class="version">v{{ appVersion }}</div>
      </div>
    </aside>

    <main class="main">
      <div class="content">
        <Dashboard v-if="page === 'dashboard'" @go-clean="goClean" />
        <CleanPage v-else-if="page === 'clean'" />
        <AiPage v-else-if="page === 'ai'" />
        <BigFilesPage v-else-if="page === 'bigfiles'" />
        <TrendPage v-else-if="page === 'trend'" />
        <RulesPage v-else-if="page === 'rules'" />
        <SettingsPage v-else-if="page === 'settings'" />
      </div>
    </main>

    <!-- 全局模态确认框（替代浏览器原生 confirm） -->
    <ConfirmModal />
  </div>
</template>

<style scoped>
.app-shell {
  display: flex;
  height: 100%;
  overflow: hidden;
}

/* ---------- 侧边栏 ---------- */
.sidebar {
  width: 208px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--sidebar);
  border-right: 1px solid var(--sidebar-border);
  padding: 18px 14px 14px;
  transition: background-color 0.25s ease, border-color 0.25s ease;
}
.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 4px 8px 18px;
}
.brand-logo {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(145deg, #2f6bff, #1d4ed8);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 4px 12px -3px rgba(47, 107, 255, 0.55);
  flex-shrink: 0;
}
.brand-name {
  font-size: 17px;
  font-weight: 800;
  letter-spacing: 0.2px;
  color: var(--text);
}
.brand-sub {
  font-size: 11px;
  color: var(--muted);
  margin-top: 1px;
}
.nav {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
}
.nav-item {
  display: flex;
  align-items: center;
  gap: 11px;
  border: 0;
  background: transparent;
  color: var(--text-2);
  font-family: inherit;
  font-size: 13.5px;
  font-weight: 500;
  padding: 9px 12px;
  border-radius: 10px;
  cursor: pointer;
  text-align: left;
  transition: all 0.14s ease;
}
.nav-item:hover {
  background: var(--panel-hover);
  color: var(--text);
}
.nav-item.active {
  background: var(--primary-soft);
  color: var(--primary-soft-text);
  font-weight: 600;
}
.nav-icon {
  flex-shrink: 0;
  opacity: 0.85;
}
.nav-item.active .nav-icon {
  opacity: 1;
}

/* ---------- 底部主题切换 ---------- */
.sidebar-footer {
  border-top: 1px solid var(--border);
  padding-top: 12px;
}
.theme-switch {
  display: flex;
  gap: 2px;
  background: var(--bg-soft);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 3px;
}
.theme-switch button {
  flex: 1;
  border: 0;
  background: transparent;
  color: var(--muted);
  height: 30px;
  border-radius: 7px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.14s ease;
}
.theme-switch button:hover {
  color: var(--text);
}
.theme-switch button.active {
  background: var(--panel);
  color: var(--primary);
  box-shadow: 0 1px 3px rgba(16, 24, 40, 0.12);
}
:root[data-theme="dark"] .theme-switch button.active {
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.4);
}
.version {
  text-align: center;
  font-size: 11px;
  color: var(--muted);
  margin-top: 10px;
}

/* ---------- 主内容 ---------- */
.main {
  flex: 1;
  overflow: hidden;
  display: flex;
}
.content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 28px;
}
</style>
