//! 应用数据目录统一管理：
//! - 统一应用标识（与 tauri.conf.json 的 identifier 保持一致）
//! - 首次启动时把旧标识目录整体迁移到新标识目录（数据库、设置、用户规则一并保留）
//! - 确保数据目录与用户规则目录存在

/// 当前应用标识目录名（对应 tauri.conf.json `identifier`）。
pub const APP_DIR_NAME: &str = "com.cclean.desktop";
/// 旧版应用标识目录名，仅用于一次性迁移。
pub const LEGACY_APP_DIR_NAME: &str = "com.cclean.app";

/// 应用数据根目录：Windows 为 `%APPDATA%\<标识>`，macOS 为 `~/Library/Application Support/<标识>`。
pub fn app_data_dir() -> std::path::PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(APP_DIR_NAME)
}

/// 用户自定义规则目录：`<数据目录>/rules`。
pub fn user_rules_dir() -> std::path::PathBuf {
    app_data_dir().join("rules")
}

/// 启动时调用：迁移旧目录（若需要）并创建数据目录、规则目录，返回数据目录。
pub fn ensure_app_dirs() -> Result<std::path::PathBuf, String> {
    let base = dirs::data_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let new_dir = base.join(APP_DIR_NAME);
    let legacy_dir = base.join(LEGACY_APP_DIR_NAME);

    // 仅当新目录不存在、旧目录存在时整体迁移（同盘 rename 为原子操作，WAL/SHM 文件一并带走）
    if !new_dir.exists() && legacy_dir.exists() {
        match std::fs::rename(&legacy_dir, &new_dir) {
            Ok(()) => eprintln!("[appdir] 已迁移旧数据目录 {LEGACY_APP_DIR_NAME} -> {APP_DIR_NAME}"),
            Err(e) => {
                eprintln!("[appdir] 旧数据目录迁移失败: {e}；改为在新目录全新初始化");
            }
        }
    }

    std::fs::create_dir_all(new_dir.join("rules"))
        .map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    Ok(new_dir)
}
