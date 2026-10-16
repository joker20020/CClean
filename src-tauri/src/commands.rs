//! Tauri IPC 命令：前端调用的全部后端接口（M1 子集）。

use std::collections::HashMap;
use std::path::PathBuf;

use tauri::{AppHandle, Emitter, State};
use tauri_plugin_autostart::ManagerExt;
use tauri_plugin_opener::OpenerExt;

use crate::ai_report::{self, ModelGroup};
use crate::analyzer::{self, BigFile, DirEntry};
use crate::cleaner::{self, CleanOutcome};
use crate::platform::{self, BrewInfo, Overview};
use crate::privilege;
use crate::resident;
use crate::rules::{self, Rule};
use crate::scanner::{self, RuleStat};
use crate::snapshot;
use crate::storage::{CleanLog, FsEvent, Snapshot, TrendData};
use crate::taskpool;
use crate::AppState;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub risk: String,
    pub default_selected: bool,
    pub delete_mode: String,
    pub description: String,
    pub action: String,
    pub available: bool,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultDto {
    pub id: String,
    pub name: String,
    pub category: String,
    pub risk: String,
    pub action: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub error_count: u64,
    pub targets: Vec<scanner::TargetStat>,
}

#[derive(serde::Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CleanItem {
    pub rule_id: String,
    /// permanent | recycle；为空时使用规则默认值
    #[serde(default)]
    pub mode: String,
}

fn rule_dto(rule: &Rule) -> RuleDto {
    let available = scanner::resolve_rule(rule)
        .iter()
        .any(|t| t.path.exists());
    RuleDto {
        id: rule.id.clone(),
        name: rule.name.clone(),
        category: rule.category.clone(),
        risk: rule.risk.clone(),
        default_selected: rule.default_selected,
        delete_mode: rule.delete_mode().to_string(),
        description: rule.description.clone(),
        action: rule.action.clone(),
        available,
    }
}

fn scan_dto(stat: RuleStat) -> ScanResultDto {
    ScanResultDto {
        id: stat.rule.id.clone(),
        name: stat.rule.name.clone(),
        category: stat.rule.category.clone(),
        risk: stat.rule.risk.clone(),
        action: stat.rule.action.clone(),
        size_bytes: stat.size_bytes,
        file_count: stat.file_count,
        error_count: stat.error_count,
        targets: stat.targets,
    }
}

#[tauri::command]
pub async fn get_system_overview() -> Result<Overview, String> {
    // 磁盘枚举可能因网络盘/移动盘而阻塞数秒，必须离开主线程，否则界面卡死
    tauri::async_runtime::spawn_blocking(platform::overview)
        .await
        .map_err(|e| format!("读取磁盘信息失败: {e}"))
}

#[tauri::command]
pub async fn get_rules(state: State<'_, AppState>) -> Result<Vec<RuleDto>, String> {
    let rules = state.rules.read().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || rules.iter().map(rule_dto).collect())
        .await
        .map_err(|e| format!("读取规则失败: {e}"))
}

/// 扫描/清理进度事件载荷（按规则逐条推进）。
#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub index: usize,
    pub total: usize,
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub async fn scan(
    state: State<'_, AppState>,
    app: AppHandle,
    rule_ids: Option<Vec<String>>,
) -> Result<Vec<ScanResultDto>, String> {
    let rules = state.rules.read().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let targets: Vec<Rule> = rules
            .iter()
            .filter(|r| rule_ids.as_deref().map_or(true, |ids| ids.iter().any(|id| id == &r.id)))
            .cloned()
            .collect();
        let total = targets.len();
        // 逐条并行扫描，但每条完成后发射进度事件，前端可显示确定性进度条。
        // 统一在有界扫描线程池内执行，避免占满 CPU 拖卡界面。
        taskpool::run(|| {
            use rayon::prelude::*;
            targets
                .par_iter()
                .enumerate()
                .map(|(i, rule)| {
                    let dto = scan_dto(scanner::scan_rule(rule));
                    let _ = app.emit(
                        "scan-progress",
                        ProgressPayload {
                            index: i + 1,
                            total,
                            id: rule.id.clone(),
                            name: rule.name.clone(),
                        },
                    );
                    dto
                })
                .collect::<Vec<_>>()
        })
    })
    .await
    .map_err(|e| format!("扫描任务失败: {e}"))
}

#[tauri::command]
pub async fn clean(
    state: State<'_, AppState>,
    app: AppHandle,
    items: Vec<CleanItem>,
) -> Result<Vec<CleanOutcome>, String> {
    let rules = state.rules.read().unwrap().clone();
    let storage = state.storage.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let total = items.len();
        let mut outcomes = Vec::new();
        for (i, item) in items.into_iter().enumerate() {
            let Some(rule) = rules.iter().find(|r| r.id == item.rule_id).cloned() else {
                continue;
            };
            let mode = if item.mode.is_empty() {
                rule.delete_mode()
            } else {
                item.mode.as_str()
            };
            let outcome = match cleaner::clean_rule(&rule, mode) {
                Ok(o) => o,
                Err(e) => CleanOutcome {
                    rule_id: rule.id.clone(),
                    freed_bytes: 0,
                    deleted_files: 0,
                    error_count: 1,
                    errors: vec![e],
                },
            };
            // 写清理日志
            let status = if outcome.error_count == 0 {
                "ok"
            } else if outcome.freed_bytes > 0 {
                "partial"
            } else {
                "failed"
            };
            let detail = outcome.errors.join("; ");
            if let Err(e) = storage.insert_clean_log(
                &outcome.rule_id,
                "",
                outcome.freed_bytes as i64,
                status,
                &detail,
            ) {
                eprintln!("[clean] 清理日志写入失败: {e}");
            }
            let _ = app.emit(
                "clean-progress",
                ProgressPayload {
                    index: i + 1,
                    total,
                    id: rule.id.clone(),
                    name: rule.name.clone(),
                },
            );
            outcomes.push(outcome);
        }
        outcomes
    })
    .await
    .map_err(|e| format!("清理任务失败: {e}"))
}

#[tauri::command]
pub fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    if !std::path::Path::new(&path).exists() {
        return Err(format!("路径不存在: {path}"));
    }
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

/// 打开文件所在的文件夹并选中该文件；对目录则直接打开目录。
#[tauri::command]
pub fn reveal_item(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err(format!("路径不存在: {path}"));
    }
    if p.is_dir() {
        return open_path_folder(&path);
    }

    #[cfg(target_os = "windows")]
    {
        // explorer.exe /select,"C:\path\file"；explorer 即使成功也常返回退出码 1，故不检查状态
        let arg = format!("/select,{path}");
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd.arg(arg);
        crate::platform::no_window(&mut cmd);
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| format!("打开资源管理器失败: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .status()
            .map(|_| ())
            .map_err(|e| format!("打开 Finder 失败: {e}"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let parent = p
            .parent()
            .map(|x| x.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        open_path_folder(&parent)
    }
}

fn open_path_folder(path: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = std::process::Command::new("explorer.exe");
        cmd.arg(path);
        crate::platform::no_window(&mut cmd);
        cmd.spawn()
            .map(|_| ())
            .map_err(|e| format!("打开文件夹失败: {e}"))
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(path)
            .status()
            .map(|_| ())
            .map_err(|e| format!("打开文件夹失败: {e}"))
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(path)
            .status()
            .map(|_| ())
            .map_err(|e| format!("打开文件夹失败: {e}"))
    }
}

// ===== M2：AI 模型报告 / 大文件分析 / brew =====

#[tauri::command]
pub async fn get_ai_report(state: State<'_, AppState>) -> Result<Vec<ModelGroup>, String> {
    let rules = state.rules.read().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || ai_report::build_report(&rules))
        .await
        .map_err(|e| format!("AI 报告生成失败: {e}"))
}

#[tauri::command]
pub async fn list_big_files(
    roots: Option<Vec<String>>,
    min_bytes: u64,
    limit: Option<usize>,
) -> Result<Vec<BigFile>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dirs: Vec<PathBuf> = match roots {
            Some(r) => r.into_iter().map(PathBuf::from).collect(),
            None => analyzer::default_big_file_roots(),
        };
        analyzer::big_files(&dirs, min_bytes, limit.unwrap_or(200))
    })
    .await
    .map_err(|e| format!("大文件扫描失败: {e}"))
}

#[tauri::command]
pub async fn list_dir_sizes(path: String) -> Result<Vec<DirEntry>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let p = PathBuf::from(&path);
        if !p.is_dir() {
            return Vec::new();
        }
        analyzer::child_sizes(&p)
    })
    .await
    .map_err(|e| format!("目录统计失败: {e}"))
}

#[tauri::command]
pub async fn default_roots() -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        analyzer::default_big_file_roots()
            .into_iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect()
    })
    .await
    .map_err(|e| format!("读取默认目录失败: {e}"))
}

#[tauri::command]
pub async fn get_brew_status() -> Result<BrewInfo, String> {
    tauri::async_runtime::spawn_blocking(platform::brew_status)
        .await
        .map_err(|e| format!("读取 Homebrew 状态失败: {e}"))
}

#[tauri::command]
pub async fn run_brew_cleanup() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(platform::brew_cleanup)
        .await
        .map_err(|e| format!("brew 清理任务失败: {e}"))?
}

// ===== M4：用户自定义规则 =====

#[tauri::command]
pub async fn list_custom_rules() -> Result<Vec<rules::UserRuleFile>, String> {
    tauri::async_runtime::spawn_blocking(rules::list_user_files)
        .await
        .map_err(|e| format!("读取用户规则失败: {e}"))
}

#[tauri::command]
pub fn custom_rules_dir() -> Result<String, String> {
    // 打开前确保目录存在，否则资源管理器会报"路径不存在"
    let dir = rules::user_rules_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建规则目录失败: {e}"))?;
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn save_custom_rule(
    state: State<'_, AppState>,
    filename: String,
    content: String,
) -> Result<(), String> {
    let name = if filename.trim().is_empty() {
        // 未指定文件名时，取文件内第一条规则的 id
        let parsed = rules::parse_user_yaml(&content)?;
        format!("{}.yaml", parsed[0].id)
    } else {
        filename
    };
    rules::save_user_file(&name, &content)?;
    state.reload_rules();
    Ok(())
}

#[tauri::command]
pub fn delete_custom_rule(
    state: State<'_, AppState>,
    filename: String,
) -> Result<(), String> {
    rules::delete_user_file(&filename)?;
    state.reload_rules();
    Ok(())
}

/// 校验一段规则 YAML（不落盘），供前端编辑器实时反馈。
#[tauri::command]
pub fn validate_custom_rule(content: String) -> Result<Vec<String>, String> {
    let parsed = rules::parse_user_yaml(&content)?;
    let builtin = rules::load_builtin();
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for rule in &parsed {
        if rule.id.trim().is_empty() || rule.name.trim().is_empty() || rule.targets.is_empty() {
            return Err(format!("规则 {} 缺少 id/name/targets 字段", rule.id));
        }
        if builtin.iter().any(|r| r.id == rule.id) {
            return Err(format!("规则 id {} 与内置规则冲突", rule.id));
        }
        if !seen.insert(rule.id.clone()) {
            return Err(format!("文件内规则 id 重复：{}", rule.id));
        }
        rules::validate_rule(rule)?;
        ids.push(rule.id.clone());
    }
    Ok(ids)
}

// ===== M4：权限提升（UAC 一键提权） =====

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeStatus {
    /// 当前进程是否已具备管理员/root 权限
    pub elevated: bool,
    /// 是否支持图形化一键提权（Windows/macOS）
    pub can_elevate: bool,
    pub platform: &'static str,
}

#[tauri::command]
pub fn get_privilege_status() -> PrivilegeStatus {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };
    PrivilegeStatus {
        elevated: privilege::is_elevated(),
        can_elevate: cfg!(target_os = "windows") || cfg!(target_os = "macos"),
        platform,
    }
}

/// 触发系统授权弹窗，以管理员权限重启应用；成功后退出当前未提权实例。
#[tauri::command]
pub fn relaunch_elevated_app(app: AppHandle) -> Result<(), String> {
    privilege::relaunch_elevated()?;
    // 给新实例一点启动时间，随后退出当前实例
    app.exit(0);
    Ok(())
}

// ===== M3：快照 / 趋势 / 事件 / 设置 =====

#[tauri::command]
pub async fn take_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let storage = state.storage.clone();
    let rules = state.rules.read().unwrap().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let snap = snapshot::take_shared(storage.clone(), rules)?;
        let _ = storage.set_setting(
            resident::SETTING_LAST_SNAPSHOT,
            &snap.created_at.to_string(),
        );
        Ok::<Snapshot, String>(snap)
    })
    .await
    .map_err(|e| format!("快照任务失败: {e}"))?
}

#[tauri::command]
pub async fn get_trend(state: State<'_, AppState>, limit: Option<u32>) -> Result<TrendData, String> {
    let storage = state.storage.clone();
    tauri::async_runtime::spawn_blocking(move || storage.trend(limit.unwrap_or(90)))
        .await
        .map_err(|e| format!("趋势查询失败: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_clean_logs(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<CleanLog>, String> {
    let storage = state.storage.clone();
    tauri::async_runtime::spawn_blocking(move || storage.clean_logs(limit.unwrap_or(100)))
        .await
        .map_err(|e| format!("清理记录查询失败: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_fs_events(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<FsEvent>, String> {
    let storage = state.storage.clone();
    tauri::async_runtime::spawn_blocking(move || storage.fs_events(limit.unwrap_or(100)))
        .await
        .map_err(|e| format!("事件查询失败: {e}"))?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_clean_log(state: State<'_, AppState>, id: i64) -> Result<usize, String> {
    state.storage.delete_clean_log(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_clean_logs(state: State<'_, AppState>) -> Result<usize, String> {
    state.storage.clear_clean_logs().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_fs_event(state: State<'_, AppState>, id: i64) -> Result<usize, String> {
    state.storage.delete_fs_event(id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn clear_fs_events(state: State<'_, AppState>) -> Result<usize, String> {
    state.storage.clear_fs_events().map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub resident_enabled: bool,
    pub snapshot_interval_hours: i64,
    pub low_space_threshold_pct: i64,
    pub autostart_enabled: bool,
    pub default_delete_mode: String,
    pub float_enabled: bool,
    pub float_click_through: bool,
    pub theme: String,
    pub language: String,
}

fn load_settings(storage: &crate::storage::Storage) -> SettingsDto {
    let get = |key: &str, default: &str| {
        storage
            .get_setting(key)
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
    };
    SettingsDto {
        resident_enabled: get(resident::SETTING_RESIDENT, "false") == "true",
        snapshot_interval_hours: get(resident::SETTING_INTERVAL_HOURS, "24").parse().unwrap_or(24),
        low_space_threshold_pct: get(resident::SETTING_LOW_SPACE_PCT, "10").parse().unwrap_or(10),
        autostart_enabled: get("autostart_enabled", "false") == "true",
        default_delete_mode: get("default_delete_mode", "permanent"),
        float_enabled: get(crate::floatwin::SETTING_FLOAT_ENABLED, "false") == "true",
        float_click_through: get(crate::floatwin::SETTING_FLOAT_THROUGH, "false") == "true",
        theme: get("theme", "system"),
        language: get(crate::i18n::SETTING_LANGUAGE, "system"),
    }
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsDto {
    load_settings(&state.storage)
}

/// 用户在低空间提示条上选择“N 小时内不再提醒”。
#[tauri::command]
pub fn snooze_low_space(state: State<'_, AppState>, hours: i64) -> Result<(), String> {
    let hours = hours.clamp(1, 48);
    let until = chrono::Utc::now().timestamp() + hours * 3600;
    state
        .storage
        .set_setting(resident::SETTING_SNOOZE_UNTIL, &until.to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_float_enabled(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::floatwin::set_enabled(&app, enabled)
}

#[tauri::command]
pub fn set_float_click_through(app: AppHandle, enabled: bool) -> Result<bool, String> {
    crate::floatwin::set_click_through(&app, enabled)
}

/// 浮窗"打开主窗口"：显示、还原最小化并聚焦主窗口。
#[tauri::command]
pub fn show_main_window(app: AppHandle) -> Result<(), String> {
    use tauri::Manager;
    let Some(win) = app.get_webview_window("main") else {
        return Err("主窗口不存在".to_string());
    };
    let _ = win.unminimize();
    win.show().map_err(|e| e.to_string())?;
    win.set_focus().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    updates: HashMap<String, String>,
) -> Result<SettingsDto, String> {
    let storage = &state.storage;
    for (key, value) in &updates {
        let validated = match key.as_str() {
            resident::SETTING_RESIDENT => {
                if value != "true" && value != "false" {
                    return Err("resident_enabled 必须为 true/false".into());
                }
                value.clone()
            }
            resident::SETTING_INTERVAL_HOURS => {
                let h: i64 = value.parse().map_err(|_| "间隔必须为整数小时")?;
                h.clamp(1, 168).to_string()
            }
            resident::SETTING_LOW_SPACE_PCT => {
                let p: i64 = value.parse().map_err(|_| "阈值必须为整数百分比")?;
                p.clamp(1, 50).to_string()
            }
            "autostart_enabled" => {
                if value != "true" && value != "false" {
                    return Err("autostart_enabled 必须为 true/false".into());
                }
                value.clone()
            }
            "default_delete_mode" => {
                if value != "permanent" && value != "recycle" {
                    return Err("删除策略必须为 permanent/recycle".into());
                }
                value.clone()
            }
            crate::floatwin::SETTING_FLOAT_ENABLED | crate::floatwin::SETTING_FLOAT_THROUGH => {
                if value != "true" && value != "false" {
                    return Err("浮窗开关必须为 true/false".into());
                }
                value.clone()
            }
            crate::floatwin::SETTING_FLOAT_X | crate::floatwin::SETTING_FLOAT_Y => {
                value.parse::<i32>().map_err(|_| "浮窗坐标必须为整数")?.to_string()
            }
            "theme" => {
                if value != "light" && value != "dark" && value != "system" {
                    return Err("theme 必须为 light/dark/system".into());
                }
                value.clone()
            }
            crate::i18n::SETTING_LANGUAGE => {
                if value != "zh-CN" && value != "en-US" && value != "system" {
                    return Err("language 必须为 zh-CN/en-US/system".into());
                }
                value.clone()
            }
            other => return Err(format!("未知设置项: {other}")),
        };
        storage
            .set_setting(key, &validated)
            .map_err(|e| e.to_string())?;
    }

    // 语言切换后重建托盘菜单，使菜单与提示立即本地化
    if updates.contains_key(crate::i18n::SETTING_LANGUAGE) {
        if let Err(e) = crate::build_tray(&app) {
            eprintln!("[i18n] 托盘菜单重建失败: {e}");
        }
    }

    // 浮窗开关/穿透若被直接写入设置，需同步窗口状态、托盘勾选并广播给前端
    if let Some(v) = updates.get(crate::floatwin::SETTING_FLOAT_ENABLED) {
        if let Err(e) = crate::floatwin::set_enabled(&app, v == "true") {
            eprintln!("[float] 同步启用状态失败: {e}");
        }
    }
    if let Some(v) = updates.get(crate::floatwin::SETTING_FLOAT_THROUGH) {
        if let Err(e) = crate::floatwin::set_click_through(&app, v == "true") {
            eprintln!("[float] 同步穿透状态失败: {e}");
        }
    }

    // 常驻 / 自启联动：两者都开启时才注册开机自启
    let resident_on =
        storage.get_setting(resident::SETTING_RESIDENT).ok().flatten().as_deref() == Some("true");
    let autostart_on =
        storage.get_setting("autostart_enabled").ok().flatten().as_deref() == Some("true");
    let autolaunch = app.autolaunch();
    let autostart_result = if resident_on && autostart_on {
        autolaunch.enable()
    } else {
        autolaunch.disable()
    };
    if let Err(e) = autostart_result {
        eprintln!("[settings] 开机自启设置失败: {e}");
    }

    Ok(load_settings(storage))
}
