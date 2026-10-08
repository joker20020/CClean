import { invoke } from "@tauri-apps/api/core";

export interface DiskInfo {
  mountName: string;
  fileSystem: string;
  totalBytes: number;
  availableBytes: number;
  usedBytes: number;
}

export interface Overview {
  os: string;
  platform: string;
  elevated: boolean;
  disks: DiskInfo[];
}

export interface RuleInfo {
  id: string;
  name: string;
  category: string;
  risk: string;
  defaultSelected: boolean;
  deleteMode: string;
  description: string;
  action: string;
  available: boolean;
}

export interface TargetStat {
  path: string;
  exists: boolean;
  sizeBytes: number;
  fileCount: number;
  errorCount: number;
}

export interface ScanResult {
  id: string;
  name: string;
  category: string;
  risk: string;
  action: string;
  sizeBytes: number;
  fileCount: number;
  errorCount: number;
  targets: TargetStat[];
}

export interface CleanItem {
  ruleId: string;
  mode?: string;
}

export interface CleanOutcome {
  ruleId: string;
  freedBytes: number;
  deletedFiles: number;
  errorCount: number;
  errors: string[];
}

export function getSystemOverview(): Promise<Overview> {
  return invoke<Overview>("get_system_overview");
}

export function getRules(): Promise<RuleInfo[]> {
  return invoke<RuleInfo[]>("get_rules");
}

export function scan(ruleIds?: string[]): Promise<ScanResult[]> {
  return invoke<ScanResult[]>("scan", { ruleIds: ruleIds ?? null });
}

export function clean(items: CleanItem[]): Promise<CleanOutcome[]> {
  return invoke<CleanOutcome[]>("clean", { items });
}

export function openPath(path: string): Promise<void> {
  return invoke("open_path", { path });
}

/** 打开文件所在文件夹并选中该文件；目录则直接打开 */
export function revealItem(path: string): Promise<void> {
  return invoke("reveal_item", { path });
}

// ===== M2 =====
export interface ModelEntry {
  name: string;
  path: string;
  sizeBytes: number;
  isDir: boolean;
}

export interface ModelGroup {
  ruleId: string;
  name: string;
  description: string;
  available: boolean;
  sizeBytes: number;
  fileCount: number;
  paths: string[];
  entries: ModelEntry[];
  guidance: string;
  cliOutput: string;
}

export interface BigFile {
  name: string;
  path: string;
  sizeBytes: number;
  modifiedTs: number | null;
}

export interface DirEntry {
  name: string;
  path: string;
  isDir: boolean;
  sizeBytes: number;
  fileCount: number;
}

export interface BrewInfo {
  available: boolean;
  version: string;
  dryRun: string;
}

export function getAiReport(): Promise<ModelGroup[]> {
  return invoke<ModelGroup[]>("get_ai_report");
}

export function listBigFiles(
  roots: string[] | null,
  minBytes: number,
  limit = 200,
): Promise<BigFile[]> {
  return invoke<BigFile[]>("list_big_files", { roots, minBytes, limit });
}

export function listDirSizes(path: string): Promise<DirEntry[]> {
  return invoke<DirEntry[]>("list_dir_sizes", { path });
}

export function defaultRoots(): Promise<string[]> {
  return invoke<string[]>("default_roots");
}

export function getBrewStatus(): Promise<BrewInfo> {
  return invoke<BrewInfo>("get_brew_status");
}

export function runBrewCleanup(): Promise<string> {
  return invoke<string>("run_brew_cleanup");
}

// ===== M3 =====
export interface Snapshot {
  id: number;
  created_at: number;
  disk_name: string;
  disk_total: number;
  disk_used: number;
  disk_free: number;
}

export interface SnapshotItem {
  snapshot_id: number;
  rule_id: string;
  size_bytes: number;
  file_count: number;
}

export interface TrendData {
  snapshots: Snapshot[];
  items: SnapshotItem[];
}

export interface CleanLog {
  id: number;
  ts: number;
  rule_id: string;
  path: string;
  size_bytes: number;
  status: string;
  detail: string;
}

export interface FsEvent {
  id: number;
  ts: number;
  kind: string;
  kindId?: string;
  rule_id: string;
  path: string;
  size_delta: number | null;
}

export interface Settings {
  residentEnabled: boolean;
  snapshotIntervalHours: number;
  lowSpaceThresholdPct: number;
  autostartEnabled: boolean;
  defaultDeleteMode: string;
  floatEnabled: boolean;
  floatClickThrough: boolean;
  theme: "light" | "dark" | "system";
}

export function takeSnapshot(): Promise<Snapshot> {
  return invoke<Snapshot>("take_snapshot");
}

export function getTrend(limit = 180): Promise<TrendData> {
  return invoke<TrendData>("get_trend", { limit });
}

export function getCleanLogs(limit = 100): Promise<CleanLog[]> {
  return invoke<CleanLog[]>("get_clean_logs", { limit });
}

export function getFsEvents(limit = 100): Promise<FsEvent[]> {
  return invoke<FsEvent[]>("get_fs_events", { limit });
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

export function setSettings(updates: Record<string, string>): Promise<Settings> {
  return invoke<Settings>("set_settings", { updates });
}

/** 低空间告警暂缓 N 小时（到期后自动恢复默认 1 小时提醒节奏） */
export function snoozeLowSpace(hours: number): Promise<void> {
  return invoke("snooze_low_space", { hours });
}

// ===== M4：自定义规则 =====

export interface UserRuleFile {
  filename: string;
  content: string;
}

export function listCustomRules(): Promise<UserRuleFile[]> {
  return invoke<UserRuleFile[]>("list_custom_rules");
}

export function customRulesDir(): Promise<string> {
  return invoke<string>("custom_rules_dir");
}

export function saveCustomRule(filename: string, content: string): Promise<void> {
  return invoke("save_custom_rule", { filename, content });
}

export function deleteCustomRule(filename: string): Promise<void> {
  return invoke("delete_custom_rule", { filename });
}

/** 校验规则 YAML，返回解析出的规则 id 列表 */
export function validateCustomRule(content: string): Promise<string[]> {
  return invoke<string[]>("validate_custom_rule", { content });
}

// ===== M4：桌面浮窗 =====

export function setFloatEnabled(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_float_enabled", { enabled });
}

export function setFloatClickThrough(enabled: boolean): Promise<boolean> {
  return invoke<boolean>("set_float_click_through", { enabled });
}

/** 从浮窗打开并聚焦主窗口 */
export function showMainWindow(): Promise<void> {
  return invoke("show_main_window");
}

// ===== M4：权限提升 =====

export interface PrivilegeStatus {
  elevated: boolean;
  canElevate: boolean;
  platform: string;
}

export function getPrivilegeStatus(): Promise<PrivilegeStatus> {
  return invoke<PrivilegeStatus>("get_privilege_status");
}

/** 触发系统授权弹窗，以管理员权限重启应用（成功后当前实例自动退出） */
export function relaunchElevated(): Promise<void> {
  return invoke("relaunch_elevated_app");
}
