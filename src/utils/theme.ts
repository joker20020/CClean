import { emit, listen } from "@tauri-apps/api/event";
import { getSettings, setSettings } from "../api";

export type ThemeMode = "light" | "dark" | "system";

/** 跨窗口主题同步事件名（主窗口切换后通知浮窗窗口） */
export const THEME_EVENT = "cclean://theme-changed";

const mql = window.matchMedia("(prefers-color-scheme: dark)");

let currentMode: ThemeMode = "system";

function resolveDark(mode: ThemeMode): boolean {
  return mode === "dark" || (mode === "system" && mql.matches);
}

function apply(mode: ThemeMode) {
  const dark = resolveDark(mode);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  window.dispatchEvent(new CustomEvent("theme-change", { detail: { dark } }));
}

/** 应用启动时调用：读取后端设置并应用主题；失败时回退跟随系统。 */
export async function initTheme(): Promise<ThemeMode> {
  let mode: ThemeMode = "system";
  try {
    const s = await getSettings();
    mode = s.theme ?? "system";
  } catch {
    // 后端不可用时跟随系统
  }
  currentMode = mode;
  apply(mode);
  mql.addEventListener("change", () => {
    // 仅在跟随系统模式下响应系统切换
    if (currentMode === "system") apply("system");
  });
  return mode;
}

export async function changeTheme(mode: ThemeMode) {
  currentMode = mode;
  apply(mode);
  try {
    await setSettings({ theme: mode });
    // 通知其他窗口（如桌面浮窗）同步主题
    await emit(THEME_EVENT, mode);
  } catch {
    // 持久化失败不影响本次切换
  }
}

export function getThemeMode(): ThemeMode {
  return currentMode;
}

/** 供浮窗等独立窗口调用：监听主窗口的主题切换并即时同步。 */
export function watchThemeChanges() {
  listen<ThemeMode>(THEME_EVENT, (e) => {
    currentMode = e.payload;
    apply(e.payload);
  });
}

/** ECharts 配色：随当前主题变化，图表在 theme-change 事件中重绘。 */
export function chartColors() {
  const dark = document.documentElement.dataset.theme === "dark";
  const styles = getComputedStyle(document.documentElement);
  const v = (name: string) => styles.getPropertyValue(name).trim();
  return {
    dark,
    text: v("--text"),
    muted: v("--muted"),
    border: v("--border"),
    track: v("--track"),
    primary: v("--primary"),
    green: v("--green"),
    amber: v("--amber"),
    red: v("--red"),
    purple: v("--purple"),
    palette: [
      v("--primary"),
      v("--green"),
      v("--amber"),
      v("--purple"),
      "#0d9488",
      v("--red"),
    ],
    usedColor: (pct: number) =>
      pct > 85 ? v("--red") : pct > 70 ? v("--amber") : v("--primary"),
  };
}
