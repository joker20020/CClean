import { computed, watch } from "vue";
import { createI18n } from "vue-i18n";
import type { getCurrentWindow } from "@tauri-apps/api/window";
import { emit, listen } from "@tauri-apps/api/event";
import { getSettings, setSettings } from "../api";
import zhCN from "./locales/zh-CN";
import enUS from "./locales/en-US";

export type Language = "zh-CN" | "en-US";
export type LanguagePref = Language | "system";
export const SUPPORTED_LANGUAGES: Language[] = ["zh-CN", "en-US"];

/** 跨窗口语言同步事件名（主窗口切换后通知浮窗窗口） */
export const LANG_EVENT = "cclean://lang-changed";

/**
 * 可扩展 i18n：新增语种时在 locales/ 下新增符合 MessageSchema 的词典，
 * 加入 SUPPORTED_LANGUAGES 与 messages 即可。
 */
export const i18n = createI18n({
  legacy: false,
  globalInjection: true,
  locale: "zh-CN",
  fallbackLocale: "zh-CN",
  messages: {
    "zh-CN": zhCN,
    "en-US": enUS,
  },
});

function resolveLanguage(pref: LanguagePref): Language {
  if (pref === "system") {
    const nav = (navigator.language || "zh-CN").toLowerCase();
    return nav.startsWith("zh") ? "zh-CN" : "en-US";
  }
  return pref;
}

let currentPref: LanguagePref = "system";

function apply(pref: LanguagePref) {
  currentPref = pref;
  const lang = resolveLanguage(pref);
  i18n.global.locale.value = lang;
  document.documentElement.lang = lang;
}

/** 应用启动时调用：读取后端设置并应用语言；失败时回退跟随系统。 */
export async function initLanguage(): Promise<LanguagePref> {
  let pref: LanguagePref = "system";
  try {
    const s = await getSettings();
    pref = (s.language as LanguagePref) ?? "system";
  } catch {
    // 后端不可用时跟随系统
  }
  apply(pref);
  window.addEventListener("languagechange", () => {
    if (currentPref === "system") apply("system");
  });
  return pref;
}

export async function changeLanguage(pref: LanguagePref) {
  apply(pref);
  try {
    await setSettings({ language: pref });
    // 通知其他窗口（如桌面浮窗）同步语言
    await emit(LANG_EVENT, pref);
  } catch {
    // 持久化失败不影响本次切换
  }
}

export function getLanguagePref(): LanguagePref {
  return currentPref;
}

/** 供浮窗等独立窗口调用：监听主窗口的语言切换并即时同步。 */
export function watchLanguageChanges() {
  listen<LanguagePref>(LANG_EVENT, (e) => apply(e.payload));
}

/**
 * 让 Tauri 窗口标题随语言切换（设置初始标题并监听语言变化）。
 * 传入 getCurrentWindow() 返回的窗口对象。
 */
export function bindWindowTitle(win: ReturnType<typeof getCurrentWindow>) {
  const update = () => {
    win.setTitle(i18n.global.t("app.brand.windowTitle")).catch(() => {});
  };
  update();
  watch(i18n.global.locale, update);
}

/** 分类名（响应式）：用于 JS/图表等非模板场景，例如 ECharts 图例。 */
export function useCategoryNames() {
  return computed<Record<string, string>>(() => ({
    system: i18n.global.t("category.system"),
    browser: i18n.global.t("category.browser"),
    dev: i18n.global.t("category.dev"),
    ai: i18n.global.t("category.ai"),
    apps: i18n.global.t("category.apps"),
    gpu: i18n.global.t("category.gpu"),
  }));
}

/** 风险等级名（响应式）。 */
export function useRiskNames() {
  return computed<Record<string, string>>(() => ({
    L0: i18n.global.t("risk.L0"),
    L1: i18n.global.t("risk.L1"),
    L2: i18n.global.t("risk.L2"),
  }));
}

/**
 * 只报告规则（模型库等）的本地化覆盖。
 *
 * 内置规则 YAML 中的 name/description 与后端 guidance 为中文原文，
 * 前端按规则 id 在 `reportRules.<id 后缀>.<字段>` 下查找译文；
 * 词典缺失时回退到后端原文，因此后续扩展只需补词典。
 */
export function useReportRules() {
  const tr = (id: string, field: "name" | "description" | "guidance", fallback: string) => {
    const suffix = id.replace(/^report\./, "");
    const key = `reportRules.${suffix}.${field}`;
    return i18n.global.te(key) ? i18n.global.t(key) : fallback;
  };
  return {
    name: (id: string, fallback: string) => tr(id, "name", fallback),
    description: (id: string, fallback: string) => tr(id, "description", fallback),
    guidance: (id: string, fallback: string) => tr(id, "guidance", fallback),
  };
}
