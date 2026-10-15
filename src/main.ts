import { createApp } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./theme.css";
import "./global.css";
import App from "./App.vue";
import FloatWidget from "./FloatWidget.vue";
import { initTheme, watchThemeChanges } from "./utils/theme";
import {
  bindWindowTitle,
  i18n,
  initLanguage,
  watchLanguageChanges,
} from "./i18n";

// 浮窗窗口（label=float）挂载独立的迷你组件，主窗口挂载完整应用
const isFloat = getCurrentWindow().label === "float";
if (isFloat) {
  document.body.classList.add("float-body");
  // 浮窗是独立 webview，需单独监听主窗口的主题/语言切换
  watchThemeChanges();
  watchLanguageChanges();
}

// 先应用主题与语言再挂载，避免闪烁
Promise.all([initTheme(), initLanguage()]).finally(() => {
  const app = createApp(isFloat ? FloatWidget : App);
  app.use(i18n);
  bindWindowTitle(getCurrentWindow());
  app.mount("#app");
});
