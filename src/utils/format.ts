export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "-";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(value >= 100 ? 0 : value >= 10 ? 1 : 2)} ${units[i]}`;
}

export const CATEGORY_NAMES: Record<string, string> = {
  system: "系统",
  browser: "浏览器",
  dev: "开发工具",
  ai: "AI 专项",
  apps: "应用缓存",
  gpu: "显卡/游戏",
};

export const RISK_NAMES: Record<string, string> = {
  L0: "只报告",
  L1: "安全",
  L2: "需管理员",
};
